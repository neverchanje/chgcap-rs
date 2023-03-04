use anyhow::Result;
use futures::StreamExt;
use log::{debug, error, info, warn};
use mysql_async::binlog::events::{
    DeleteRowsEvent, DeleteRowsEventV1, Event, EventData, GtidEvent, IncidentEvent, QueryEvent,
    RotateEvent, RowsEventData, RowsQueryEvent, UpdateRowsEvent, UpdateRowsEventV1, WriteRowsEvent,
    WriteRowsEventV1,
};
use mysql_async::BinlogStream;

use crate::config::MysqlSourceConfig;
use crate::connection::{get_binlog_stream, MysqlConn};

/// The MySQL CDC Source which supports parallel reading snapshot of table
/// and then continue to capture data change from binlog.
///
/// 1. The source supports parallel capturing table change.
/// 2. The source supports checkpoint in split level when read snapshot data.
/// 3. The source doesn't need apply any lock of MySQL.
pub struct MysqlSource {
    cfg: MysqlSourceConfig,
    conn: MysqlConn,
    pool: mysql_async::Pool,
    ctx: SourceContext,
}

impl MysqlSource {
    pub fn handle_event(&mut self, event: Event) -> Result<()> {
        let event_data = match event.read_data()? {
            Some(data) => data,
            None => return Ok(()), // Skip empty event.
        };
        self.ctx.server_id = event.header().server_id();

        match event_data {
            EventData::UnknownEvent => todo!(),
            EventData::StartEventV3(_) => todo!(),
            EventData::QueryEvent(e) => self.handle_query(e),
            EventData::StopEvent => todo!(),
            EventData::RotateEvent(e) => self.handle_rotate(e),
            EventData::IntvarEvent(_) => todo!(),
            EventData::LoadEvent(_) => todo!(),
            EventData::SlaveEvent => todo!(),
            EventData::CreateFileEvent(_) => todo!(),
            EventData::AppendBlockEvent(_) => todo!(),
            EventData::ExecLoadEvent(_) => todo!(),
            EventData::DeleteFileEvent(_) => todo!(),
            EventData::NewLoadEvent(_) => todo!(),
            EventData::RandEvent(_) => todo!(),
            EventData::UserVarEvent(_) => todo!(),
            EventData::FormatDescriptionEvent(_) => todo!(),
            EventData::XidEvent(_) => todo!(),
            EventData::BeginLoadQueryEvent(_) => todo!(),
            EventData::ExecuteLoadQueryEvent(_) => todo!(),
            EventData::TableMapEvent(_) => todo!(),
            EventData::PreGaWriteRowsEvent(_) => todo!(),
            EventData::PreGaUpdateRowsEvent(_) => todo!(),
            EventData::PreGaDeleteRowsEvent(_) => todo!(),
            EventData::IncidentEvent(_) => todo!(),
            EventData::HeartbeatEvent => self.handle_server_heartbeat(),
            EventData::IgnorableEvent(_) => todo!(),
            EventData::RowsQueryEvent(e) => self.handle_rows_query(e),
            EventData::GtidEvent(e) => self.handle_gtid_event(e),
            EventData::AnonymousGtidEvent(_) => todo!(),
            EventData::PreviousGtidsEvent(_) => todo!(),
            EventData::TransactionContextEvent(_) => todo!(),
            EventData::ViewChangeEvent(_) => todo!(),
            EventData::XaPrepareLogEvent(_) => todo!(),
            EventData::RowsEvent(e) => self.handle_rows_event(e),
        };
        Ok(())
    }

    /// Handle a [mysql_async::binlog::events::XidEvent] or a COMMIT statement.
    fn handle_txn_completion(&self) {}

    /// Handle the supplied event that signals that mysqld has stopped.
    fn handle_server_stop(&self) {
        info!("Server stopped")
    }

    /// Handle the supplied event that is sent by a primary to a replica to let the replica
    /// know that the primary is still alive. Not written to a binary log.
    fn handle_server_heartbeat(&self) {}

    /// Handle the supplied event that signals that an out of the ordinary event that occurred on
    /// the master. It notifies the replica that something happened on the primary that might
    /// cause data to be in an inconsistent state.
    fn handle_server_incident(&self, e: IncidentEvent) {
        // TODO: Failure handling: https://github.com/neverchanje/chgcap-rs/issues/2.
        error!("server incident: {}", e.message())
    }

    /// Handle the supplied event that signals the logs are being rotated. This means that either
    /// the server was restarted, or the binlog has transitioned to a new file.
    /// In either case, subsequent table numbers will be different than those seen to this point.
    fn handle_rotate(&mut self, e: RotateEvent) {
        self.ctx.current_binlog_pos = e.position();
        self.ctx.current_binlog_filename = e.name().to_string();
    }

    /// Handle the supplied event that signals the beginning of a GTID transaction.
    /// We don't yet know whether this transaction contains any events we're interested in,
    /// but we have to record it so that we know the position of this event and know we've
    /// processed the binlog to this point.
    ///
    /// Note that this captures the current GTID and complete GTID set, regardless of whether
    /// the connector is filtering the GTID set upon connection. We do this because we actually
    /// want to capture all GTID set values found in the binlog, whether or not we process them.
    /// However, only when we connect do we actually want to pass to MySQL only those GTID ranges
    /// that are applicable per the configuration.
    fn handle_gtid_event(&mut self, e: GtidEvent) {
        self.ctx.current_gtid = Some(e);
    }

    /// Handle the supplied event [RowsQueryEvent] by recording the original SQL query that
    /// generated the event.
    fn handle_rows_query(&mut self, e: RowsQueryEvent) {
        self.ctx.current_query = e.query().to_string();
    }

    /// Handle the supplied event with an [QueryEvent] by possibly recording the DDL statements
    /// as changes in the MySQL schemas.
    fn handle_query(&mut self, e: QueryEvent) {
        debug!("Received query command: {:?}", e);

        let query = e.query().to_string().trim().to_string();
        if query.eq_ignore_ascii_case("BEGIN") {
            self.ctx.thread_id = Some(e.thread_id());
            return;
        }
        if query.eq_ignore_ascii_case("COMMIT") {
            self.handle_txn_completion();
            return;
        }

        let stmt_prefix = query[..7].to_uppercase();
        if stmt_prefix.starts_with("XA ") {
            info!("Ignore XA transaction");
            return;
        } else if stmt_prefix == "INSERT " || stmt_prefix == "UPDATE " || stmt_prefix == "DELETE " {
            warn!("Received DML '{}' for processing, binlog probably contains events generated with statement or mixed based replication format", query);
            return;
        }
        if query.eq_ignore_ascii_case("ROLLBACK") {
            unimplemented!("{}", query)
        }
    }

    /// Generate source records for the supplied event.
    fn handle_rows_event(&self, e: RowsEventData) {
        match e {
            RowsEventData::WriteRowsEventV1(e) => self.handle_write_rows_v1(e),
            RowsEventData::WriteRowsEvent(e) => self.handle_write_rows(e),
            RowsEventData::UpdateRowsEventV1(e) => self.handle_update_rows_v1(e),
            RowsEventData::UpdateRowsEvent(e) => self.handle_update_rows(e),
            RowsEventData::DeleteRowsEventV1(e) => self.handle_delete_rows_v1(e),
            RowsEventData::DeleteRowsEvent(e) => self.handle_delete_rows(e),
            RowsEventData::PartialUpdateRowsEvent(_) => todo!(),
        }
    }

    fn handle_write_rows_v1(&self, _e: WriteRowsEventV1) {}

    fn handle_write_rows(&self, _e: WriteRowsEvent) {}

    fn handle_update_rows_v1(&self, _e: UpdateRowsEventV1) {}

    fn handle_update_rows(&self, _e: UpdateRowsEvent) {}

    fn handle_delete_rows_v1(&self, _e: DeleteRowsEventV1) {}

    fn handle_delete_rows(&self, _e: DeleteRowsEvent) {}
}

pub struct MysqlBinlogSplit {
    binlog_stream: BinlogStream,
}

impl MysqlBinlogSplit {
    pub async fn new(source: &MysqlSource) -> Result<Self> {
        let binlog_stream = get_binlog_stream(&source.pool, &source.cfg).await?;

        Ok(Self { binlog_stream })
    }

    pub async fn start(mut self, s: &mut MysqlSource) -> Result<()> {
        while let Some(r) = self.binlog_stream.next().await {
            let event = r?;
            s.handle_event(event)?;
        }
        Ok(())
    }
}

pub struct MysqlSnapshotSplit {}

/// Startup modes for the MySQL CDC Consumer.
pub enum StartupMode {
    Earliest,
    Latest,
    SpecificOffset,
}

/// Information about the source, which includes the position in the source binary log we have
/// previously processed.
///
/// The [MySqlPartition::getSourcePartition()] source partition information describes the database
/// whose log is being consumed. Typically, the database is identified by the host address port
/// number of the MySQL server and the name of the database.
///
/// The `gtids` field only appears in offsets produced when GTIDs are enabled. The "{@code
/// snapshot}" field only appears in offsets produced when the connector is in the middle of a
/// snapshot. And finally, the "{@code ts}" field contains the <em>seconds</em> since Unix epoch
/// (since Jan 1, 1970) of the MySQL event; the message [Envelope] envelopes also have a timestamp,
/// but that timestamp is the <em>milliseconds</em> since since Jan 1, 1970.
///
/// Each change event envelope also includes the [MySqlSource::struct()] source struct that contains
/// MySQL information about that particular event, including a mixture the fields from the binlog
/// filename and position where the event can be found, and when GTIDs are enabled the GTID of the
/// transaction in which the event occurs. Like with the offset, the "{@code snapshot}" field only
/// appears for events produced when the connector is in the middle of a snapshot. Note that this
/// information is likely different than the offset information, since the connector may need to
/// restart from either just after the most recently completed transaction or the beginning
/// of the most recently started transaction (whichever appears later in the binlog).
pub struct SourceContext {
    pub current_gtid: Option<GtidEvent>,

    pub current_binlog_filename: String,

    pub current_binlog_pos: u64,

    /// The original SQL query that generated the event.
    pub current_query: String,

    /// The server ID found within the binary log file.
    pub server_id: u32,

    /// The identifier of the MySQL thread that generated the most recent event.
    /// `None` if not known.
    pub thread_id: Option<u32>,
}
