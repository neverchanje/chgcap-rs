use mysql_async::binlog::events::GtidEvent;

use crate::config::MysqlSourceConfig;
use crate::connection::MysqlConn;

/// The MySQL CDC Source which supports parallel reading snapshot of table
/// and then continue to capture data change from binlog.
///
/// 1. The source supports parallel capturing table change.
/// 2. The source supports checkpoint in split level when read snapshot data.
/// 3. The source doesn't need apply any lock of MySQL.
pub struct MysqlSource {
    pub(crate) cfg: MysqlSourceConfig,
    pub(crate) conn: MysqlConn,
    pub(crate) pool: mysql_async::Pool,
}

pub async fn get_mysql_cdc_stream(_cfg: &MysqlSourceConfig) {}

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
#[derive(Default)]
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
