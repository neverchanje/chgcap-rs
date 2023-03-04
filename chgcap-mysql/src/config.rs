use std::time::Duration;

use getset::{CopyGetters, Getters};

#[derive(Debug, Clone)]
pub enum SslMode {
    Disabled,
}

/// The set of predefined modes for dealing with failures during event processing.
#[derive(Debug)]
pub enum FailureHandlingMode {
    /// Problematic events will be skipped.
    Skip,

    /// The position of problematic events will be logged and events will be skipped.
    Warn,

    /// An exception indicating the problematic events and their position is raised, causing the
    /// connector to be stopped.
    Fail,

    /// Problematic events will be skipped - for transitional period only, scheduled to be removed.
    Ignore,
}

pub struct CommonConfig {
    failure_handling_mode: FailureHandlingMode,
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
