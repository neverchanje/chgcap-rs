use std::time::Duration;

use anyhow::Result;
use futures::TryStreamExt;
use getset::{CopyGetters, Getters};
use mysql_async::binlog::events::{
    DeleteRowsEvent, DeleteRowsEventV1, EventData, GtidEvent, IncidentEvent, QueryEvent,
    RotateEvent, RowsEvent, RowsEventData, RowsQueryEvent, UpdateRowsEvent, UpdateRowsEventV1,
    WriteRowsEvent, WriteRowsEventV1,
};
use mysql_async::BinlogStream;

use crate::connection::{get_binlog_stream, MysqlConn};

#[derive(Debug, Clone)]
pub enum SslMode {
    Disabled,
}

/// TODO: Allow to load configurations from a YAML file.
#[derive(Builder, Debug, Clone, Getters, CopyGetters)]
pub struct MysqlSourceConfig {
    /// The maximum time that the connector should wait after trying to connect to the MySQL
    /// database server before timing out.
    #[getset(get = "pub")]
    connect_timeout: Duration,

    /// The connection pool size.
    #[getset(get_copy = "pub")]
    connection_pool_size: i32,

    /// An optional list of regular expressions that match database names to be monitored; any
    /// database name not included in the whitelist will be excluded from monitoring. By default
    /// all databases will be monitored.
    #[getset(get = "pub")]
    database_list: Vec<String>,

    /// The interval of heartbeat event.
    #[getset(get = "pub")]
    heartbeat_interval: Duration,

    #[getset(get = "pub")]
    hostname: String,

    /// Whether the [`MySqlSource`] should output the schema changes or not.
    #[getset(get_copy = "pub")]
    include_schema_changes: bool,

    /// Password to use when connecting to the MySQL database server.
    #[getset(get = "pub")]
    password: String,

    /// Integer port number of the MySQL database server.
    #[getset(get_copy = "pub")]
    port: i32,

    /// Whether the [`MySqlSource`] should output the schema changes or not.
    #[getset(get_copy = "pub")]
    scan_newly_added_table_enabled: bool,

    /// A numeric ID of this database client, which must be unique across all currently-running
    /// database processes in the MySQL cluster. This connector joins the MySQL database cluster
    /// as another server (with this unique ID) so it can read the binlog. By default, a random
    /// number is generated between 5400 and 6400, though we recommend setting an explicit value.
    #[getset(get_copy = "pub")]
    server_id: u32,

    /// The session time zone in database server, e.g. "America/Los_Angeles". It controls how the
    /// TIMESTAMP type in MYSQL converted to STRING. See more
    /// https://debezium.io/documentation/reference/1.5/connectors/mysql.html#mysql-temporal-types
    #[getset(get = "pub")]
    server_timezone: String,

    /// The group size of split meta, if the meta size exceeds the group size, the meta will be
    /// divided into multiple groups.
    #[getset(get_copy = "pub")]
    split_meta_group_size: i32,

    /// The split size (number of rows) of table snapshot, captured tables are split into multiple
    /// splits when read the snapshot of table.
    #[getset(get_copy = "pub")]
    split_size: i32,

    #[getset(get = "pub")]
    ssl_mode: SslMode,

    /// An optional list of regular expressions that match fully-qualified table identifiers for
    /// tables to be monitored; any table not included in the list will be excluded from
    /// monitoring. Each identifier is of the form databaseName.tableName. By default the
    /// connector will monitor every non-system table in each monitored database.
    #[getset(get = "pub")]
    table_list: Vec<String>,

    /// Name of the MySQL database to use when connecting to the MySQL database server.
    #[getset(get = "pub")]
    username: String,
}

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
}

impl MysqlSource {
    /// Handle a [mysql_async::binlog::events::XidEvent] or a COMMIT statement.
    fn handle_txn_completion(&self, _ctx: MysqlOffsetContext) {}

    /// Handle the supplied event that signals that mysqld has stopped.
    fn handle_server_stop(&self) {}

    /// Handle the supplied event that is sent by a primary to a replica to let the replica
    /// know that the primary is still alive. Not written to a binary log.
    fn handle_server_heartbeat(&self) {}

    /// Handle the supplied event that signals that an out of the ordinary event that occurred on
    /// the master. It notifies the replica that something happened on the primary that might
    /// cause data to be in an inconsistent state.
    fn handle_server_incident(&self, _event: IncidentEvent) {}

    /// Handle the supplied event that signals the logs are being rotated. This means that either
    /// the server was restarted, or the binlog has transitioned to a new file.
    /// In either case, subsequent table numbers will be different than those seen to this point.
    fn handle_rotate_logs(&self, _event: RotateEvent) {}

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
    fn handle_gtid_event(&self, _event: GtidEvent) {}

    /// Handle the supplied event [RowsQueryEvent] by recording the original SQL query that
    /// generated the event.
    fn handle_rows_query(&self, _event: RowsQueryEvent) {}

    /// Handle the supplied event with an [QueryEvent] by possibly recording the DDL statements
    /// as changes in the MySQL schemas.
    fn handle_query(&self, _event: QueryEvent) {}

    /// Generate source records for the supplied event.
    fn handle_rows_event(&self, _event: RowsEventData) {}
}

pub struct MysqlBinlogSplit {
    binlog_stream: BinlogStream,
}

impl MysqlBinlogSplit {
    pub async fn new(source: &MysqlSource) -> Result<Self> {
        let binlog_stream = get_binlog_stream(&source.pool, &source.cfg).await?;

        Ok(Self { binlog_stream })
    }

    pub async fn start(self, s: &MysqlSource) -> Result<()> {
        self.binlog_stream
            .try_for_each(|event| async move {
                let event_data = match event.read_data()? {
                    Some(data) => data,
                    None => return Ok(()),
                };
                match event_data {
                    EventData::UnknownEvent => todo!(),
                    EventData::StartEventV3(_) => todo!(),
                    EventData::QueryEvent(e) => s.handle_query(e),
                    EventData::StopEvent => todo!(),
                    EventData::RotateEvent(e) => s.handle_rotate_logs(e),
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
                    EventData::HeartbeatEvent => todo!(),
                    EventData::IgnorableEvent(_) => todo!(),
                    EventData::RowsQueryEvent(_) => todo!(),
                    EventData::GtidEvent(_) => todo!(),
                    EventData::AnonymousGtidEvent(_) => todo!(),
                    EventData::PreviousGtidsEvent(_) => todo!(),
                    EventData::TransactionContextEvent(_) => todo!(),
                    EventData::ViewChangeEvent(_) => todo!(),
                    EventData::XaPrepareLogEvent(_) => todo!(),
                    EventData::RowsEvent(e) => s.handle_rows_event(e),
                };
                Ok(())
            })
            .await?;
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
pub struct MysqlOffsetContext {
    current_gtid: Option<String>,

    current_binlog_filename: String,

    current_binlog_pos: u64,

    /// The original SQL query that generated the event.
    current_query: String,

    /// The server ID found within the binary log file.
    server_id: u32,

    /// The identifier of the MySQL thread that generated the most recent event.
    /// `None` if not known.
    thread_id: Option<i64>,
}
