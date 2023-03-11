#[derive(Debug)]
pub enum MysqlChange {
    Insert(mysql_async::Row),
}

#[derive(Debug)]
pub struct MysqlTableEvent {
    pub table_name: String,

    pub changes: Vec<MysqlChange>,
}
