use getset::{CopyGetters, Getters};
use itertools::Itertools;
use mysql_async::binlog::row::BinlogRow;
use mysql_async::binlog::value::BinlogValue;
use mysql_async::consts::ColumnType;

#[derive(Clone, PartialEq)]
pub enum RowChange {
    Insert(BinlogRow),
    Delete(BinlogRow),
}

fn fmt_column_type(c: &ColumnType) -> String {
    let s = match c {
        ColumnType::MYSQL_TYPE_DECIMAL => "DECIMAL",
        ColumnType::MYSQL_TYPE_TINY => "TINYINT",
        ColumnType::MYSQL_TYPE_SHORT => "SMALLINT",
        ColumnType::MYSQL_TYPE_LONG => "INT",
        ColumnType::MYSQL_TYPE_FLOAT => "FLOAT",
        ColumnType::MYSQL_TYPE_DOUBLE => "DOUBLE",
        ColumnType::MYSQL_TYPE_NULL => "NULL",
        ColumnType::MYSQL_TYPE_TIMESTAMP => "TIMESTAMP",
        ColumnType::MYSQL_TYPE_LONGLONG => "BIGINT",
        ColumnType::MYSQL_TYPE_INT24 => "MEDIUMINT",
        ColumnType::MYSQL_TYPE_DATE => "DATE",
        ColumnType::MYSQL_TYPE_TIME => "TIME",
        ColumnType::MYSQL_TYPE_DATETIME => "DATETIME",
        ColumnType::MYSQL_TYPE_YEAR => "YEAR",
        ColumnType::MYSQL_TYPE_NEWDATE => "NEWDATE", // Internal to MySql
        ColumnType::MYSQL_TYPE_VARCHAR => "VARCHAR",
        ColumnType::MYSQL_TYPE_BIT => "BIT",
        ColumnType::MYSQL_TYPE_TIMESTAMP2 => "TIMESTAMP2",
        ColumnType::MYSQL_TYPE_DATETIME2 => "DATETIME2",
        ColumnType::MYSQL_TYPE_TIME2 => "TIME2",
        ColumnType::MYSQL_TYPE_TYPED_ARRAY => "TYPED ARRAY", // Used for replication only
        ColumnType::MYSQL_TYPE_UNKNOWN => "UNKNOWN",
        ColumnType::MYSQL_TYPE_JSON => "JSON",
        ColumnType::MYSQL_TYPE_NEWDECIMAL => "NEWDECIMAL",
        ColumnType::MYSQL_TYPE_ENUM => "ENUM",
        ColumnType::MYSQL_TYPE_SET => "SET",
        ColumnType::MYSQL_TYPE_TINY_BLOB => "TINYBLOB",
        ColumnType::MYSQL_TYPE_MEDIUM_BLOB => "MEDIUMBLOB",
        ColumnType::MYSQL_TYPE_LONG_BLOB => "LONGBLOB",
        ColumnType::MYSQL_TYPE_BLOB => "BLOB",
        ColumnType::MYSQL_TYPE_VAR_STRING => "VAR_STRING",
        ColumnType::MYSQL_TYPE_STRING => "STRING",
        ColumnType::MYSQL_TYPE_GEOMETRY => "GEOMETRY",
    };
    s.to_string()
}

fn fmt_value(val: &BinlogValue, ty: &ColumnType) -> String {
    match val {
        BinlogValue::Value(v) => {
            format!("{}({:?})", fmt_column_type(ty), v)
        }
        BinlogValue::Jsonb(_) => todo!(),
        BinlogValue::JsonDiff(_) => todo!(),
    }
}

fn fmt_row(row: &BinlogRow) -> String {
    (0..row.len())
        .map(|i| match row.as_ref(i) {
            Some(v) => fmt_value(v, &row.columns_ref().get(i).unwrap().column_type()),
            None => "NULL".to_string(),
        })
        .join(",")
}

impl std::fmt::Display for RowChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Insert(row) => write!(f, "Insert({})", fmt_row(row)),
            Self::Delete(row) => write!(f, "Delete({})", fmt_row(row)),
        }
    }
}

#[derive(Clone, PartialEq)]
pub enum SchemaChange {
    Create,
    Alter,
    Drop,
}

#[derive(Clone, PartialEq)]
pub enum EventData {
    DataChange(Vec<RowChange>),
    SchemaChange(SchemaChange),
}

#[derive(Clone, PartialEq, Getters, CopyGetters)]
pub struct Event {
    #[getset(get_copy = "pub")]
    pub(crate) pos: u32,

    #[getset(get = "pub")]
    pub(crate) database_name: String,

    #[getset(get = "pub")]
    pub(crate) schema_name: String,

    #[getset(get_copy = "pub")]
    pub(crate) table_id: u64,

    #[getset(get = "pub")]
    pub(crate) table_name: String,

    #[getset(get = "pub")]
    pub(crate) sql: String,

    #[getset(get = "pub")]
    pub(crate) data: EventData,
}
