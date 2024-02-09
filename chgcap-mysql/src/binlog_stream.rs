use std::pin::Pin;
use std::task::Poll;

use anyhow::{anyhow, bail, Result};
use log::{debug, error, info, warn};
use mysql_async::binlog::events::{
    DeleteRowsEvent, Event, EventData, GtidEvent, IncidentEvent, QueryEvent, RotateEvent,
    RowsEventData, RowsQueryEvent, TableMapEvent, TransactionPayloadEvent, UpdateRowsEvent,
    WriteRowsEvent,
};
use mysql_async::prelude::Query;
use mysql_async::{BinlogStream as MysqlBinlogStream, BinlogStreamRequest, Conn, Pool, Row};

use crate::event::{Event as ChgcapEvent, EventData as ChgcapEventData, RowChange};
use crate::source::{Source, SourceContext};
use crate::SourceConfig;

pub struct BinlogStream {
    binlog_stream: MysqlBinlogStream,

    ctx: SourceContext,
    cfg: SourceConfig,
}

impl BinlogStream {
    pub async fn new(source: &Source) -> Result<Self> {
        let cfg = source.cfg.clone();
        let pool = &source.pool;
        let (conn, filename, position) = create_binlog_stream_conn(pool).await?;
        let request = BinlogStreamRequest::new(cfg.server_id())
            .with_filename(&filename)
            .with_pos(position);
        let binlog_stream = conn
            .get_binlog_stream(request)
            .await
            .map_err(|e| anyhow!(e))?;
        let ctx = SourceContext {
            current_binlog_filename: String::from_utf8(filename)?,
            ..Default::default()
        };
        Ok(Self {
            binlog_stream,
            ctx,
            cfg,
        })
    }

    pub fn config(&self) -> &SourceConfig {
        &self.cfg
    }
}

/// Ensures that the MySQL server has GTIDs enabled.
async fn check_gtid_mode_enabled(conn: &mut Conn) -> Result<()> {
    let opt_gtid_mode = "SELECT @@GLOBAL.GTID_MODE".first::<String, _>(conn).await?;
    if let Some(gtid_mode) = opt_gtid_mode {
        if gtid_mode.starts_with("ON") {
            return Ok(());
        }
    }
    bail!("GTID_MODE is disabled (enable using --gtid_mode=ON --enforce_gtid_consistency=ON)");
}

async fn create_binlog_stream_conn(pool: &Pool) -> Result<(Conn, Vec<u8>, u64)> {
    let mut conn = pool.get_conn().await.unwrap();
    check_gtid_mode_enabled(&mut conn).await?;

    if conn.server_version() >= (8, 0, 31) && conn.server_version() < (9, 0, 0) {
        let _ = "SET binlog_transaction_compression=ON"
            .ignore(&mut conn)
            .await;
    }

    let row: Row = "SHOW BINARY LOGS".first(&mut conn).await.unwrap().unwrap();
    let filename = row.get(0).unwrap();
    let position = row.get(1).unwrap();

    Ok((conn, filename, position))
}

impl BinlogStream {
    /// Returns `Ok(None)` if the event is skipped.
    /// See [https://dev.mysql.com/doc/dev/mysql-server/latest/page_protocol_replication_binlog_event.html] for the description of each event type.
    /// We only support binlog version 4, which corresponds to MySQL 5.0 and later.
    fn handle_event(&mut self, event: Event) -> Result<Option<ChgcapEvent>> {
        let event_data = match event.read_data()? {
            Some(data) => data,
            None => return Ok(None), // Skip empty event.
        };
        self.ctx.server_id = event.header().server_id();

        match event_data {
            EventData::QueryEvent(e) => self.handle_query(e),
            EventData::RotateEvent(e) => self.handle_rotate(e),
            EventData::HeartbeatEvent => self.handle_server_heartbeat(),
            EventData::RowsQueryEvent(e) => self.handle_rows_query(e),
            EventData::GtidEvent(e) => self.handle_gtid_event(e),
            EventData::RowsEvent(e) => {
                return Ok(Some(self.handle_rows_event(
                    &self.binlog_stream,
                    e,
                    event.header().log_pos(),
                )?))
            }
            EventData::TableMapEvent(_) => {
                // [`BinlogStream`] has already handled this event. Internally, it maintains a table
                // mapping from table ID to table name.
            }
            EventData::XidEvent(_) => self.handle_txn_completion(),
            EventData::TransactionPayloadEvent(e) => self.handle_txn_payload(e),
            _ => {
                // EventData::UnknownEvent => todo!(),
                // EventData::StartEventV3(_) => todo!(), // Deprecated.
                // EventData::StopEvent => todo!(),
                // EventData::IntvarEvent(_) => todo!(),
                // EventData::LoadEvent(_) => todo!(), // Deprecated.
                // EventData::SlaveEvent => todo!(), // Deprecated.
                // EventData::CreateFileEvent(_) => todo!(), // Deprecated.
                // EventData::AppendBlockEvent(_) => todo!(), // Ignored.
                // EventData::ExecLoadEvent(_) => todo!(), // Deprecated.
                // EventData::DeleteFileEvent(_) => todo!(), // Ignored.
                // EventData::NewLoadEvent(_) => todo!(), // Deprecated.
                // EventData::RandEvent(_) => todo!(),
                // EventData::UserVarEvent(_) => todo!(),
                // EventData::FormatDescriptionEvent(_) => todo!(),
                // EventData::BeginLoadQueryEvent(_) => todo!(),
                // EventData::ExecuteLoadQueryEvent(_) => todo!(),
                // EventData::PreGaWriteRowsEvent(_) => todo!(), // Deprecated.
                // EventData::PreGaUpdateRowsEvent(_) => todo!(), // Deprecated.
                // EventData::PreGaDeleteRowsEvent(_) => todo!(), // Deprecated.
                // EventData::IncidentEvent(_) => todo!(),
                // EventData::IgnorableEvent(_) => todo!(), // Ignored.
                // EventData::AnonymousGtidEvent(_) => todo!(), // Ignored.
                // EventData::PreviousGtidsEvent(_) => todo!(), // Ignored.
                // EventData::TransactionContextEvent(_) => todo!(), // Ignored.
                // EventData::ViewChangeEvent(_) => todo!(), // Ignored.
                // EventData::XaPrepareLogEvent(_) => todo!(), // Ignored.
            }
        };
        Ok(None)
    }

    fn handle_txn_payload(&self, e: TransactionPayloadEvent) {
        debug!("Received transaction payload: {:?}", e)
    }

    /// Handle a [mysql_async::binlog::events::XidEvent] or a COMMIT statement.
    fn handle_txn_completion(&self) {
        todo!()
    }

    /// Handle the supplied event that signals that mysqld has stopped.
    fn handle_server_stop(&self) {
        info!("Server stopped")
    }

    /// Handle the supplied event that is sent by a primary to a replica to let the replica
    /// know that the primary is still alive. Not written to a binary log.
    fn handle_server_heartbeat(&self) {
        debug!("server heartbeat")
    }

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

        debug!("Rotated to binlog file: {}", e.name());
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
    fn handle_rows_event(
        &self,
        stream: &MysqlBinlogStream,
        e: RowsEventData,
        pos: u32,
    ) -> Result<ChgcapEvent> {
        let tme = stream.get_tme(e.table_id()).ok_or_else(|| {
            anyhow!(
                "Received a rows event for table id {} but no table metadata was found",
                e.table_id()
            )
        })?;
        let changes = match e {
            RowsEventData::WriteRowsEvent(e) => self.handle_write_rows(tme, e),
            RowsEventData::UpdateRowsEvent(e) => self.handle_update_rows(tme, e),
            RowsEventData::DeleteRowsEvent(e) => self.handle_delete_rows(tme, e),
            RowsEventData::PartialUpdateRowsEvent(_) => todo!(),

            RowsEventData::DeleteRowsEventV1(_)
            | RowsEventData::WriteRowsEventV1(_)
            | RowsEventData::UpdateRowsEventV1(_) => panic!(
                "Received a V1 rows event, but V1 events are not supported. You are perhaps using an unsupported MySQL version (5.1.15-5.6.x)."
            ),
        }?;

        Ok(ChgcapEvent {
            table_name: tme.table_name().to_string(),
            table_id: tme.table_id(),
            database_name: self.config().database().clone(),
            schema_name: Default::default(),
            sql: Default::default(),
            pos,
            data: ChgcapEventData::DataChange(changes),
        })
    }

    fn handle_write_rows(&self, tme: &TableMapEvent, e: WriteRowsEvent) -> Result<Vec<RowChange>> {
        e.rows(tme)
            .map(|r| {
                let row = r?;
                if row.0.is_some() {
                    bail!("unexpected 'before' in the UpdateRowsEvent")
                }
                let after = row
                    .1
                    .ok_or_else(|| anyhow!("'after' is missing in the UpdateRowsEvent"))?;

                Ok(RowChange::Insert(after))
            })
            .collect::<Result<Vec<RowChange>>>()
    }

    fn handle_update_rows(
        &self,
        tme: &TableMapEvent,
        e: UpdateRowsEvent,
    ) -> Result<Vec<RowChange>> {
        let mut changes: Vec<RowChange> = vec![];
        for r in e.rows(tme) {
            let row = r?;
            let before = row
                .0
                .ok_or_else(|| anyhow!("'before' is missing in the UpdateRowsEvent"))?;
            let after = row
                .1
                .ok_or_else(|| anyhow!("'after' is missing in the UpdateRowsEvent"))?;
            changes.push(RowChange::Delete(before));
            changes.push(RowChange::Insert(after));
        }
        Ok(changes)
    }

    fn handle_delete_rows(
        &self,
        tme: &TableMapEvent,
        e: DeleteRowsEvent,
    ) -> Result<Vec<RowChange>> {
        let mut changes: Vec<RowChange> = vec![];
        for r in e.rows(tme) {
            let row = r?;
            let before = row
                .0
                .ok_or_else(|| anyhow!("'before' is missing in the UpdateRowsEvent"))?;
            if row.1.is_some() {
                bail!("unexpected 'after' in the UpdateRowsEvent")
            }
            changes.push(RowChange::Delete(before));
        }
        Ok(changes)
    }
}

impl futures_core::stream::Stream for BinlogStream {
    type Item = Result<ChgcapEvent>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        // TODO: Support rate limiting.
        loop {
            let binlog_stream = Pin::new(&mut this.binlog_stream);
            return match binlog_stream.poll_next(cx) {
                Poll::Ready(t) => match t {
                    Some(event_result) => match event_result {
                        Ok(event) => match this.handle_event(event) {
                            Ok(change) => match change {
                                Some(c) => Poll::Ready(Some(Ok(c))),
                                None => continue, // Skip this event.
                            },
                            Err(e) => Poll::Ready(Some(Err(e))),
                        },
                        Err(err) => Poll::Ready(Some(Err(anyhow!(err)))),
                    },
                    None => Poll::Ready(None), // Completed.
                },
                Poll::Pending => Poll::Pending,
            };
        }
    }
}
