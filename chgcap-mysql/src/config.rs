use std::time::Duration;

use getset::{CopyGetters, Getters};

#[derive(Debug, Clone)]
pub enum SslMode {
    Disabled,
}

impl Default for SslMode {
    fn default() -> Self {
        Self::Disabled
    }
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
pub struct SourceConfig {
    /// The maximum time that the connector should wait after trying to connect to the MySQL
    /// database server before timing out.
    #[getset(get = "pub")]
    #[builder(default)]
    connect_timeout: Duration,

    /// The connection pool size.
    #[getset(get_copy = "pub")]
    #[builder(default)]
    connection_pool_size: i32,

    /// The MySQL database to monitor.
    #[getset(get = "pub")]
    #[builder(default)]
    database: String,

    /// The interval of heartbeat event.
    #[getset(get = "pub")]
    #[builder(default)]
    heartbeat_interval: Duration,

    #[getset(get = "pub")]
    hostname: String,

    /// Whether the [`MySqlSource`] should output the schema changes or not.
    #[getset(get_copy = "pub")]
    #[builder(default)]
    include_schema_changes: bool,

    /// Password to use when connecting to the MySQL database server.
    #[getset(get = "pub")]
    password: String,

    /// Integer port number of the MySQL database server.
    #[getset(get_copy = "pub")]
    port: i32,

    /// Whether the [`MySqlSource`] should output the schema changes or not.
    #[getset(get_copy = "pub")]
    #[builder(default)]
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
    #[builder(default)]
    server_timezone: String,

    /// The group size of split meta, if the meta size exceeds the group size, the meta will be
    /// divided into multiple groups.
    #[getset(get_copy = "pub")]
    #[builder(default)]
    split_meta_group_size: i32,

    /// The split size (number of rows) of table snapshot, captured tables are split into multiple
    /// splits when read the snapshot of table.
    #[getset(get_copy = "pub")]
    #[builder(default)]
    split_size: i32,

    #[getset(get = "pub")]
    #[builder(default)]
    ssl_mode: SslMode,

    /// An optional list of regular expressions that match fully-qualified table identifiers for
    /// tables to be monitored; any table not included in the list will be excluded from
    /// monitoring. Each identifier is of the form databaseName.tableName. By default the
    /// connector will monitor every non-system table in each monitored database.
    #[getset(get = "pub")]
    #[builder(default)]
    table_list: Vec<String>,

    /// Name of the MySQL database to use when connecting to the MySQL database server.
    #[getset(get = "pub")]
    username: String,
}

impl Default for SourceConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(5),
            connection_pool_size: 10,
            database: Default::default(),
            heartbeat_interval: Duration::from_secs(3),
            hostname: Default::default(),
            include_schema_changes: Default::default(),
            scan_newly_added_table_enabled: Default::default(),
            server_id: Default::default(),
            server_timezone: Default::default(),
            split_meta_group_size: Default::default(),
            split_size: Default::default(),
            ssl_mode: SslMode::Disabled,
            table_list: Default::default(),
            username: Default::default(),
            password: Default::default(),
            port: Default::default(),
        }
    }
}
