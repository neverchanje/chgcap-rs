use std::time::Duration;

use anyhow::Result;
use getset::Getters;

#[derive(Debug, Clone)]
pub enum SslMode {
    Disabled,
}

/// TODO: Allow to load configurations from a YAML file.
#[derive(Builder, Debug, Clone, Getters)]
pub struct MysqlSourceConfig {
    /// Name of the MySQL database to use when connecting to the MySQL database server.
    #[getset(get = "pub")]
    username: String,

    /// Password to use when connecting to the MySQL database server.
    #[getset(get = "pub")]
    password: String,

    /// The session time zone in database server, e.g. "America/Los_Angeles". It controls how the
    /// TIMESTAMP type in MYSQL converted to STRING. See more
    /// https://debezium.io/documentation/reference/1.5/connectors/mysql.html#mysql-temporal-types
    #[getset(get = "pub")]
    server_timezone: String,

    /// A numeric ID of this database client, which must be unique across all currently-running
    /// database processes in the MySQL cluster. This connector joins the MySQL database cluster
    /// as another server (with this unique ID) so it can read the binlog. By default, a random
    /// number is generated between 5400 and 6400, though we recommend setting an explicit value.
    #[getset(get = "pub")]
    server_id: i32,

    /// An optional list of regular expressions that match fully-qualified table identifiers for
    /// tables to be monitored; any table not included in the list will be excluded from
    /// monitoring. Each identifier is of the form databaseName.tableName. By default the
    /// connector will monitor every non-system table in each monitored database.
    #[getset(get = "pub")]
    table_list: Vec<String>,

    /// An optional list of regular expressions that match database names to be monitored; any
    /// database name not included in the whitelist will be excluded from monitoring. By default
    /// all databases will be monitored.
    #[getset(get = "pub")]
    database_list: Vec<String>,

    /// The split size (number of rows) of table snapshot, captured tables are split into multiple
    /// splits when read the snapshot of table.
    #[getset(get = "pub")]
    split_size: i32,

    /// The group size of split meta, if the meta size exceeds the group size, the meta will be
    /// divided into multiple groups.
    #[getset(get = "pub")]
    split_meta_group_size: i32,

    /// The maximum time that the connector should wait after trying to connect to the MySQL database
    /// server before timing out.
    #[getset(get = "pub")]
    connect_timeout: Duration,

    /// The connection pool size.
    #[getset(get = "pub")]
    connection_pool_size: i32,

    /// Whether the [`MySqlSource`] should output the schema changes or not.
    #[getset(get = "pub")]
    include_schema_changes: bool,

    /// Whether the [`MySqlSource`] should output the schema changes or not.
    #[getset(get = "pub")]
    scan_newly_added_table_enabled: bool,

    /// The interval of heartbeat event.
    #[getset(get = "pub")]
    heartbeat_interval: Duration,

    #[getset(get = "pub")]
    hostname: String,

    /// Integer port number of the MySQL database server.
    #[getset(get = "pub")]
    port: i32,

    #[getset(get = "pub")]
    ssl_mode: SslMode,
}

/// The MySQL CDC Source which supports parallel reading snapshot of table
/// and then continue to capture data change from binlog.
///
/// 1. The source supports parallel capturing table change.
/// 2. The source supports checkpoint in split level when read snapshot data.
/// 3. The source doesn't need apply any lock of MySQL.
pub struct MysqlSource {}

impl MysqlSource {
    pub fn create_reader(_subtask_idx: i32) -> Result<MysqlReaderSplit> {
        todo!()
    }
}

pub struct MysqlReaderSplit {}
