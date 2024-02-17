use std::fmt;

use crate::ast::value::escape_single_quote_string;

use super::{display_comma_separated, Ident};

/// A table-level constraint, specified in a `CREATE TABLE` or an
/// `ALTER TABLE ADD <constraint>` statement.
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum TableConstraint {
    /// `[CONSTRAINT [symbol]] PRIMARY KEY [index_type] (key_part,...) [index_option] ...`
    PrimaryKeys { columns: Vec<Ident> },
}

impl fmt::Display for TableConstraint {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TableConstraint::PrimaryKeys { columns } => {
                write!(f, "PRIMARY KEY ({})", display_comma_separated(columns))
            }
        }
    }
}

/// `ColumnOption`s are modifiers that follow a column definition in a `CREATE
/// TABLE` statement.
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub enum ColumnOption {
    /// `NULL`
    Null,
    /// `NOT NULL`
    NotNull,
    // [PRIMARY] KEY
    PrimaryKey,
    // COMMENT 'string'
    Comment(String),
}

impl fmt::Display for ColumnOption {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use ColumnOption::*;
        match self {
            Null => write!(f, "NULL"),
            NotNull => write!(f, "NOT NULL"),
            Comment(v) => write!(f, "COMMENT '{}'", escape_single_quote_string(v)),
            PrimaryKey => write!(f, "PRIMARY KEY"),
        }
    }
}
