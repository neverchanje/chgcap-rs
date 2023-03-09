#[derive(Debug)]
pub enum MysqlChange {
    Insert(mysql_async::Row),
}

#[derive(Debug)]
pub struct MysqlTableEvent {
    table_name: String,

    changes: Vec<MysqlChange>,
}
