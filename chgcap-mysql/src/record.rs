use mysql_async::binlog::row::BinlogRow;

#[derive(Debug, Clone, PartialEq)]
pub enum MysqlChange {
    Insert(BinlogRow),
}

#[derive(Debug, Clone, PartialEq)]
pub struct MysqlTableEvent {
    pub table_name: String,
    pub table_id: u64,
    pub changes: Vec<MysqlChange>,
}
