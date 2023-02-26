use std::collections::HashMap;

use anyhow::Result;
use mysql_async::{BinlogRequest, BinlogStream, Conn, Pool};

use crate::source::MysqlSourceConfig;

pub fn new_conn_pool() -> Result<Pool> {
    let conn_pool = Pool::from_url("mysql://root:password@localhost:3307/db_name")?;
    Ok(conn_pool)
}

pub async fn get_binlog_stream(pool: &Pool, cfg: &MysqlSourceConfig) -> Result<BinlogStream> {
    let conn = pool.get_conn().await?;
    let binlog_stream = conn
        .get_binlog_stream(BinlogRequest::new(cfg.server_id()))
        .await?;
    Ok(binlog_stream)
}

pub struct BinlogPosition {
    filename: String,
    position: u64,
}

pub struct MysqlConn {
    conn: Conn,
}

impl MysqlConn {
    pub async fn new(conn: Conn) -> Self {
        Self { conn }
    }

    /// Determine whether the MySQL server has the binlog_row_image set to 'FULL'.
    /// Returns `true` if the server's `binlog_row_image` is set to `FULL`, or `false` otherwise.
    pub fn is_binlog_row_image_full(&self) -> Result<bool> {
        todo!()
    }

    /// Determine if the current user has the named privilege. Note that if the user has the "ALL"
    /// privilege this method returns {@code true}.
    /// - `grantName`: the name of the MySQL privilege; may not be null.
    /// Returns `true` if the user has the named privilege, or `false` otherwise.
    pub fn user_has_priviledge(&self, _grant_name: String) -> Result<bool> {
        todo!()
    }

    /// Determine whether the MySQL server has the row-level binlog enabled.
    ///
    /// Return `true` if the server's `binlog_format` is set to `ROW`, or `false` otherwise.
    pub fn is_binlog_format_row(&self) -> Result<bool> {
        todo!()
    }

    /// Query the database server to get the list of the binlog files available.
    ///
    /// Returns a list of the binlog files.
    pub fn available_binlog_files(&self) -> Result<Vec<String>> {
        todo!()
    }

    // Read the MySQL charset-related system variables.
    //
    // Returns the system variables that are related to server character sets;
    pub fn mysql_charset_system_variables(&self) -> Result<HashMap<String, String>> {
        todo!()
    }

    /// Read the MySQL system variables.
    ///
    /// Returns the system variables that are related to server character sets;
    pub fn mysql_system_variables(&self) -> Result<HashMap<String, String>> {
        todo!()
    }

    /// Read the SSL version session variable.
    ///
    /// Returns the session variables that are related to session SSL version.
    pub fn session_variable_ssl_version() -> Result<String> {
        todo!()
    }

    /// Determine whether the MySQL server has GTIDs enabled.
    ///
    /// Returns `false` if the server's `gtid_mode` is set and is `OFF`, or `true` otherwise.
    pub fn is_gtid_mode_enabled(&self) -> Result<bool> {
        todo!()
    }

    /// Determine the executed GTID set for MySQL.
    ///
    /// Returns the string representation of MySQL's GTID sets; never null but an empty string if
    /// the server does not use GTIDs.
    pub fn known_gtid_set() -> Result<String> {
        todo!()
    }

    /// Determine the earliest binlog filename that is still available in the server.
    ///
    /// Returns the name of the earliest binlog filename, or `None` if there are none.
    pub fn earliest_binlog_filename() -> Result<Option<String>> {
        todo!()
    }
}
