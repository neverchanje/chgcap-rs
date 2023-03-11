use mysql_async::binlog::row::BinlogRow;

#[derive(Debug, Clone, PartialEq)]
pub enum MysqlChange {
    Insert(BinlogRow),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MysqlTableEvent {
    pub table_name: String,

    pub changes: Vec<MysqlChange>,
}
