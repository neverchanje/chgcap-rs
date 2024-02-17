// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::vec;

use log::debug;

use crate::ast::data_type::{
    CharLengthUnits, CharacterLength, DataType, ExactNumberInfo, TimezoneInfo,
};
use crate::ast::ddl::{ColumnOption, TableConstraint};
use crate::ast::ColumnDef;
use crate::keywords::{self};
use crate::tokenizer::{Location, Word};
use crate::{
    ast::{Ident, ObjectName, Statement},
    keywords::Keyword,
    tokenizer::{Token, TokenWithLocation, Tokenizer},
};

pub const TRAILING_COMMAS: bool = false;

pub struct Parser {
    tokens: Vec<TokenWithLocation>,
    /// The index of the first unprocessed token in `self.tokens`
    index: usize,
}

impl Parser {
    /// Create a parser.
    ///
    /// See also [`Parser::parse_sql`]
    ///
    /// Example:
    /// ```
    /// use sqlparser::{parser::{Parser, ParserError}};
    /// fn main() -> Result<(), ParserError> {
    ///   let statements = Parser::parse_sql("CREATE TABLE t (v INT);")?;
    /// }
    /// ```
    fn new() -> Self {
        Self {
            tokens: vec![],
            index: 0,
        }
    }

    /// Tokenize the sql string and sets this [`Parser`]'s state to
    /// parse the resulting tokens
    ///
    /// See example on [`Parser::new()`] for an example
    fn try_with_sql(self, sql: &str) -> Option<Self> {
        debug!("Parsing sql '{}'...", sql);
        Tokenizer::new(sql)
            .tokenize_with_location()
            .map(|tokens| self.with_tokens_with_locations(tokens))
            .ok()
    }

    /// Reset this parser to parse the specified token stream
    fn with_tokens_with_locations(mut self, tokens: Vec<TokenWithLocation>) -> Self {
        self.tokens = tokens;
        self.index = 0;
        self
    }

    /// Convenience method to parse a string with one or more SQL
    /// statements into produce an Abstract Syntax Tree (AST).
    /// NOTE: We assume that the upstream database has validated the query.
    /// Therefore, the user can simply skip the query if this function returns `None`.
    ///
    /// Example
    /// ```
    /// use chgcap_mysql_ddl_parser::parser::Parser;
    /// fn test_parser() {
    ///   Parser::parse_sql("CREATE TABLE t (v INT)").unwrap();
    /// }
    /// ```
    pub fn parse_sql(sql: &str) -> Option<Statement> {
        Parser::new()
            .try_with_sql(sql)
            .as_mut()
            .and_then(|p| p.parse_statement())
    }

    /// Parse a single top-level statement (such as ALTER, CREATE, etc.),
    /// stopping before the statement separator, if any.
    pub fn parse_statement(&mut self) -> Option<Statement> {
        let next_token = self.next_token();
        match &next_token.token {
            Token::Word(w) => match w.keyword {
                Keyword::CREATE => self.parse_create(),
                Keyword::ALTER => self.parse_alter(),
                Keyword::DROP => self.parse_drop(),
                _ => None,
            },
            _ => None,
        }
    }

    /// Parse a SQL CREATE statement
    pub fn parse_create(&mut self) -> Option<Statement> {
        if self.parse_keyword(Keyword::TABLE) {
            self.parse_create_table()
        } else if self.parse_keyword(Keyword::SCHEMA) {
            self.parse_create_schema()
        } else if self.parse_keyword(Keyword::DATABASE) {
            self.parse_create_database()
        } else {
            None
        }
    }

    pub fn parse_create_table(&mut self) -> Option<Statement> {
        let if_not_exists = self.parse_keywords(&[Keyword::IF, Keyword::NOT, Keyword::EXISTS]);
        let table_name = self.parse_object_name();

        if self.parse_keyword(Keyword::LIKE) {
            // CREATE TABLE ... LIKE is unsupported yet, see https://github.com/neverchanje/chgcap-rs/issues/14 for details.
            return None;
        }

        // parse optional column list (schema)
        let (columns, constraints) = self.parse_columns();

        // Table options and partition options are ending clauses, hence that we don't have to parse them.

        // Parse optional `AS ( query )`
        if self.parse_keyword(Keyword::AS) {
            // CREATE TABLE ... AS is not supported.
            return None;
        }

        Some(Statement::CreateTable {
            if_not_exists,
            name: table_name,
            columns,
            constraints,
        })
    }

    pub fn parse_create_schema(&mut self) -> Option<Statement> {
        todo!()
    }

    pub fn parse_create_database(&mut self) -> Option<Statement> {
        todo!()
    }

    pub fn parse_alter(&mut self) -> Option<Statement> {
        todo!()
    }

    pub fn parse_drop(&mut self) -> Option<Statement> {
        todo!()
    }

    pub fn parse_columns(&mut self) -> (Vec<ColumnDef>, Vec<TableConstraint>) {
        let mut columns = vec![];
        let mut constraints = vec![];
        if !self.consume_token(&Token::LParen) || self.consume_token(&Token::RParen) {
            return (columns, constraints);
        }

        loop {
            if let Some(constraint) = self.parse_optional_table_constraint() {
                constraints.push(constraint);
            } else if let Token::Word(_) = self.peek_token().token {
                columns.push(self.parse_column_def());
            } else {
                panic!(
                    "Expected column name or constraint definition, found: {}",
                    self.peek_token()
                );
            }
            let comma = self.consume_token(&Token::Comma);
            if self.consume_token(&Token::RParen) {
                // allow a trailing comma, even though it's not in standard
                break;
            }
            if !comma {
                panic!(
                    "Expected ',' or ')' after column definition, found: {}",
                    self.peek_token()
                );
            }
        }

        (columns, constraints)
    }

    /// Parse single table constraint.
    pub fn parse_optional_table_constraint(&mut self) -> Option<TableConstraint> {
        // [CONSTRAINT [symbol]]
        if self.parse_keyword(Keyword::CONSTRAINT) {
            self.parse_optional_identifier();
        }

        // PRIMARY KEY [index_type] (key_part,...) [index_option] ...
        if self.parse_keywords(&[Keyword::PRIMARY, Keyword::KEY]) {
            self.parse_optional_index_type();
            let columns = self.parse_parenthesized_column_list(false);
            self.parse_ending_of_create_definition();
            return Some(TableConstraint::PrimaryKeys { columns });
        }

        // UNIQUE [INDEX | KEY] [index_name] [index_type] (key_part,...) [index_option] ...
        if self.parse_keyword(Keyword::UNIQUE) {
            let _ = self.parse_one_of_keywords(&[Keyword::INDEX, Keyword::KEY]);
            self.parse_optional_identifier();
            self.parse_optional_index_type();
            self.parse_parenthesized_column_list(false);
            self.parse_ending_of_create_definition();
        }

        // FOREIGN KEY [index_name] (col_name,...) reference_definition
        if self.parse_keywords(&[Keyword::FOREIGN, Keyword::KEY]) {
            self.parse_optional_identifier();
            self.parse_parenthesized_column_list(false);
            self.expect_reference_definition();
        }

        // CHECK (expr) [[NOT] ENFORCED]
        if self.parse_keyword(Keyword::CHECK) {
            self.expect_token(&Token::LParen);
            self.parse_parenthesized_expr();
            self.expect_token(&Token::RParen);
            self.parse_ending_of_create_definition();
        }

        None
    }

    // index_type:
    //   `USING {BTREE | HASH}`
    pub fn parse_optional_index_type(&mut self) -> bool {
        if self.parse_keyword(Keyword::USING) {
            let _ = self.parse_one_of_keywords(&[Keyword::BTREE, Keyword::HASH]);
            return true;
        }
        false
    }

    // According to https://dev.mysql.com/doc/refman/8.0/en/create-table.html, a create definition can be
    // a column definition or a table constraints.  Its ending is supposed to be a comma or a right parenthesis.
    pub fn parse_ending_of_create_definition(&mut self) {
        loop {
            let next_token = self.next_token().token;
            if next_token == Token::Comma || next_token == Token::RParen {
                self.prev_token();
                break;
            }
            if next_token == Token::EOF {
                panic!("Expected comma or right parenthesis, found EOF, which is an expected ending of a column definition")
            }
        }
    }

    // reference_definition:
    //     REFERENCES tbl_name (key_part,...)
    //     [MATCH FULL | MATCH PARTIAL | MATCH SIMPLE]
    //     [ON DELETE reference_option]
    //     [ON UPDATE reference_option]
    //
    // Returns false if reference definition doesn't exist.
    #[must_use]
    pub fn parse_reference_definition(&mut self) -> bool {
        if !self.parse_keyword(Keyword::REFERENCES) {
            return false;
        }
        self.parse_object_name();
        self.parse_parenthesized_column_list(false);

        if self.parse_keyword(Keyword::MATCH) {
            let _ = self.parse_one_of_keywords(&[Keyword::FULL, Keyword::PARTIAL, Keyword::SIMPLE]);
        }

        if self.parse_keyword(Keyword::ON)
            && (self.parse_keyword(Keyword::DELETE) || self.parse_keyword(Keyword::UPDATE))
        {
            self.parse_reference_option();
        }
        true
    }

    pub fn expect_reference_definition(&mut self) {
        if !self.parse_reference_definition() {
            panic!("Expected REFERENCES, found: {}", self.peek_token());
        }
    }

    // reference_option:
    // RESTRICT | CASCADE | SET NULL | NO ACTION | SET DEFAULT
    pub fn parse_reference_option(&mut self) {
        let _ = self.parse_keyword(Keyword::RESTRICT)
            || self.parse_keyword(Keyword::CASCADE)
            || self.parse_keywords(&[Keyword::SET, Keyword::NULL])
            || self.parse_keywords(&[Keyword::SET, Keyword::DEFAULT])
            || self.parse_keywords(&[Keyword::NO, Keyword::ACTION]);
    }

    pub fn parse_column_def(&mut self) -> ColumnDef {
        // data_type [NOT NULL | NULL]
        let name = self.expect_identifier();
        let data_type = self.parse_data_type();

        let mut options = vec![];
        while let Some(option) = self.parse_optional_column_option() {
            options.push(option);
        }
        ColumnDef {
            name,
            data_type,
            options,
        }
    }

    /// Parse a SQL datatype (in the context of a CREATE TABLE statement for example)
    pub fn parse_data_type(&mut self) -> DataType {
        let next_token = self.next_token();
        match next_token.token {
            Token::Word(w) => match w.keyword {
                Keyword::BOOLEAN => DataType::Boolean,
                Keyword::BOOL => DataType::Bool,
                Keyword::FLOAT => DataType::Float(self.parse_optional_precision()),
                Keyword::REAL => DataType::Real,
                Keyword::FLOAT4 => DataType::Float4,
                Keyword::FLOAT64 => DataType::Float64,
                Keyword::FLOAT8 => DataType::Float8,
                Keyword::DOUBLE => {
                    if self.parse_keyword(Keyword::PRECISION) {
                        DataType::DoublePrecision
                    } else {
                        DataType::Double
                    }
                }
                Keyword::TINYINT => {
                    let optional_precision = self.parse_optional_precision();
                    if self.parse_keyword(Keyword::UNSIGNED) {
                        DataType::UnsignedTinyInt(optional_precision)
                    } else {
                        DataType::TinyInt(optional_precision)
                    }
                }
                Keyword::INT2 => {
                    let optional_precision = self.parse_optional_precision();
                    if self.parse_keyword(Keyword::UNSIGNED) {
                        DataType::UnsignedInt2(optional_precision)
                    } else {
                        DataType::Int2(optional_precision)
                    }
                }
                Keyword::SMALLINT => {
                    let optional_precision = self.parse_optional_precision();
                    if self.parse_keyword(Keyword::UNSIGNED) {
                        DataType::UnsignedSmallInt(optional_precision)
                    } else {
                        DataType::SmallInt(optional_precision)
                    }
                }
                Keyword::MEDIUMINT => {
                    let optional_precision = self.parse_optional_precision();
                    if self.parse_keyword(Keyword::UNSIGNED) {
                        DataType::UnsignedMediumInt(optional_precision)
                    } else {
                        DataType::MediumInt(optional_precision)
                    }
                }
                Keyword::INT => {
                    let optional_precision = self.parse_optional_precision();
                    if self.parse_keyword(Keyword::UNSIGNED) {
                        DataType::UnsignedInt(optional_precision)
                    } else {
                        DataType::Int(optional_precision)
                    }
                }
                Keyword::INT4 => {
                    let optional_precision = self.parse_optional_precision();
                    if self.parse_keyword(Keyword::UNSIGNED) {
                        DataType::UnsignedInt4(optional_precision)
                    } else {
                        DataType::Int4(optional_precision)
                    }
                }
                Keyword::INT64 => DataType::Int64,
                Keyword::INTEGER => {
                    let optional_precision = self.parse_optional_precision();
                    if self.parse_keyword(Keyword::UNSIGNED) {
                        DataType::UnsignedInteger(optional_precision)
                    } else {
                        DataType::Integer(optional_precision)
                    }
                }
                Keyword::BIGINT => {
                    let optional_precision = self.parse_optional_precision();
                    if self.parse_keyword(Keyword::UNSIGNED) {
                        DataType::UnsignedBigInt(optional_precision)
                    } else {
                        DataType::BigInt(optional_precision)
                    }
                }
                Keyword::INT8 => {
                    let optional_precision = self.parse_optional_precision();
                    if self.parse_keyword(Keyword::UNSIGNED) {
                        DataType::UnsignedInt8(optional_precision)
                    } else {
                        DataType::Int8(optional_precision)
                    }
                }
                Keyword::VARCHAR => DataType::Varchar(self.parse_optional_character_length()),
                Keyword::NVARCHAR => DataType::Nvarchar(self.parse_optional_precision()),
                Keyword::CHARACTER => {
                    if self.parse_keyword(Keyword::VARYING) {
                        DataType::CharacterVarying(self.parse_optional_character_length())
                    } else if self.parse_keywords(&[Keyword::LARGE, Keyword::OBJECT]) {
                        DataType::CharacterLargeObject(self.parse_optional_precision())
                    } else {
                        DataType::Character(self.parse_optional_character_length())
                    }
                }
                Keyword::CHAR => {
                    if self.parse_keyword(Keyword::VARYING) {
                        DataType::CharVarying(self.parse_optional_character_length())
                    } else if self.parse_keywords(&[Keyword::LARGE, Keyword::OBJECT]) {
                        DataType::CharLargeObject(self.parse_optional_precision())
                    } else {
                        DataType::Char(self.parse_optional_character_length())
                    }
                }
                Keyword::CLOB => DataType::Clob(self.parse_optional_precision()),
                Keyword::BINARY => DataType::Binary(self.parse_optional_precision()),
                Keyword::VARBINARY => DataType::Varbinary(self.parse_optional_precision()),
                Keyword::BLOB => DataType::Blob(self.parse_optional_precision()),
                Keyword::BYTES => DataType::Bytes(self.parse_optional_precision()),
                Keyword::UUID => DataType::Uuid,
                Keyword::DATE => DataType::Date,
                Keyword::DATETIME => DataType::Datetime(self.parse_optional_precision()),
                Keyword::TIMESTAMP => {
                    let precision = self.parse_optional_precision();
                    let tz = if self.parse_keyword(Keyword::WITH) {
                        self.expect_keywords(&[Keyword::TIME, Keyword::ZONE]);
                        TimezoneInfo::WithTimeZone
                    } else if self.parse_keyword(Keyword::WITHOUT) {
                        self.expect_keywords(&[Keyword::TIME, Keyword::ZONE]);
                        TimezoneInfo::WithoutTimeZone
                    } else {
                        TimezoneInfo::None
                    };
                    DataType::Timestamp(precision, tz)
                }
                Keyword::TIMESTAMPTZ => {
                    DataType::Timestamp(self.parse_optional_precision(), TimezoneInfo::Tz)
                }
                Keyword::TIME => {
                    let precision = self.parse_optional_precision();
                    let tz = if self.parse_keyword(Keyword::WITH) {
                        self.expect_keywords(&[Keyword::TIME, Keyword::ZONE]);
                        TimezoneInfo::WithTimeZone
                    } else if self.parse_keyword(Keyword::WITHOUT) {
                        self.expect_keywords(&[Keyword::TIME, Keyword::ZONE]);
                        TimezoneInfo::WithoutTimeZone
                    } else {
                        TimezoneInfo::None
                    };
                    DataType::Time(precision, tz)
                }
                Keyword::TIMETZ => {
                    DataType::Time(self.parse_optional_precision(), TimezoneInfo::Tz)
                }
                // Interval types can be followed by a complicated interval
                // qualifier that we don't currently support. See
                // parse_interval for a taste.
                Keyword::INTERVAL => DataType::Interval,
                Keyword::JSON => DataType::JSON,
                Keyword::JSONB => DataType::JSONB,
                Keyword::REGCLASS => DataType::Regclass,
                Keyword::STRING => DataType::String(self.parse_optional_precision()),
                Keyword::TEXT => DataType::Text,
                Keyword::BYTEA => DataType::Bytea,
                Keyword::NUMERIC => {
                    DataType::Numeric(self.parse_exact_number_optional_precision_scale())
                }
                Keyword::DECIMAL => {
                    DataType::Decimal(self.parse_exact_number_optional_precision_scale())
                }
                Keyword::DEC => DataType::Dec(self.parse_exact_number_optional_precision_scale()),
                Keyword::BIGNUMERIC => {
                    DataType::BigNumeric(self.parse_exact_number_optional_precision_scale())
                }
                Keyword::BIGDECIMAL => {
                    DataType::BigDecimal(self.parse_exact_number_optional_precision_scale())
                }
                Keyword::ENUM => DataType::Enum(self.parse_string_values()),
                Keyword::SET => DataType::Set(self.parse_string_values()),
                _ => {
                    self.prev_token();
                    let type_name = self.parse_object_name();
                    if let Some(modifiers) = self.parse_optional_type_modifiers() {
                        DataType::Custom(type_name, modifiers)
                    } else {
                        DataType::Custom(type_name, vec![])
                    }
                }
            },
            _ => panic!("Expected a data type name, found {}", next_token),
        }
    }

    pub fn parse_optional_type_modifiers(&mut self) -> Option<Vec<String>> {
        if self.consume_token(&Token::LParen) {
            let mut modifiers = Vec::new();
            loop {
                let next_token = self.next_token();
                match next_token.token {
                    Token::Word(w) => modifiers.push(w.to_string()),
                    Token::Number(n, _) => modifiers.push(n),
                    Token::SingleQuotedString(s) => modifiers.push(s),

                    Token::Comma => {
                        continue;
                    }
                    Token::RParen => {
                        break;
                    }
                    _ => panic!("Expected type modifiers, found {}", next_token),
                }
            }

            Some(modifiers)
        } else {
            None
        }
    }

    pub fn parse_precision(&mut self) -> u64 {
        self.expect_token(&Token::LParen);
        let n = self.parse_literal_uint();
        self.expect_token(&Token::RParen);
        n
    }

    pub fn parse_optional_precision(&mut self) -> Option<u64> {
        if self.consume_token(&Token::LParen) {
            let n = self.parse_literal_uint();
            self.expect_token(&Token::RParen);
            Some(n)
        } else {
            None
        }
    }

    pub fn parse_optional_character_length(&mut self) -> Option<CharacterLength> {
        if self.consume_token(&Token::LParen) {
            let character_length = self.parse_character_length();
            self.expect_token(&Token::RParen);
            Some(character_length)
        } else {
            None
        }
    }

    pub fn parse_character_length(&mut self) -> CharacterLength {
        if self.parse_keyword(Keyword::MAX) {
            return CharacterLength::Max;
        }
        let length = self.parse_literal_uint();
        let unit = if self.parse_keyword(Keyword::CHARACTERS) {
            Some(CharLengthUnits::Characters)
        } else if self.parse_keyword(Keyword::OCTETS) {
            Some(CharLengthUnits::Octets)
        } else {
            None
        };
        CharacterLength::IntegerLength { length, unit }
    }

    pub fn parse_optional_precision_scale(&mut self) -> (Option<u64>, Option<u64>) {
        if self.consume_token(&Token::LParen) {
            let n = self.parse_literal_uint();
            let scale = if self.consume_token(&Token::Comma) {
                Some(self.parse_literal_uint())
            } else {
                None
            };
            self.expect_token(&Token::RParen);
            (Some(n), scale)
        } else {
            (None, None)
        }
    }

    pub fn parse_exact_number_optional_precision_scale(&mut self) -> ExactNumberInfo {
        if self.consume_token(&Token::LParen) {
            let precision = self.parse_literal_uint();
            let scale = if self.consume_token(&Token::Comma) {
                Some(self.parse_literal_uint())
            } else {
                None
            };

            self.expect_token(&Token::RParen);

            match scale {
                None => ExactNumberInfo::Precision(precision),
                Some(scale) => ExactNumberInfo::PrecisionAndScale(precision, scale),
            }
        } else {
            ExactNumberInfo::None
        }
    }

    /// Parse a parenthesized comma-separated list of unqualified, possibly quoted identifiers
    pub fn parse_parenthesized_column_list(&mut self, allow_empty: bool) -> Vec<Ident> {
        if self.consume_token(&Token::LParen) {
            if allow_empty && self.peek_token().token == Token::RParen {
                self.next_token();
                vec![]
            } else {
                let cols = self.parse_comma_separated(|p| p.expect_identifier());
                self.expect_token(&Token::RParen);
                cols
            }
        } else {
            panic!(
                "Expected a list of columns in parentheses, found {}",
                self.peek_token()
            );
        }
    }

    // [NOT NULL | NULL]
    // [DEFAULT {literal | (expr)} ]
    // [VISIBLE | INVISIBLE] | [VIRTUAL | STORED] | [AUTO_INCREMENT]
    // [UNIQUE [KEY]]
    // [[PRIMARY] KEY]
    // [COMMENT 'string']
    // [COLLATE collation_name]
    // [COLUMN_FORMAT {FIXED | DYNAMIC | DEFAULT}]
    // [ENGINE_ATTRIBUTE [=] 'string']
    // [SECONDARY_ENGINE_ATTRIBUTE [=] 'string']
    // [STORAGE {DISK | MEMORY}]
    // [GENERATED ALWAYS] AS (expr)
    // [reference_definition]
    // [check_constraint_definition]
    //
    // This method returns `None` if it parses a skippable column option.
    pub fn parse_optional_column_option(&mut self) -> Option<ColumnOption> {
        if self.parse_keywords(&[Keyword::NOT, Keyword::NULL]) {
            Some(ColumnOption::NotNull)
        } else if self.parse_keyword(Keyword::NULL) {
            Some(ColumnOption::Null)
        } else if self.parse_keyword(Keyword::DEFAULT) {
            if !self.parse_literal() {
                self.parse_parenthesized_expr();
            }
            None
        } else if self
            .parse_one_of_keywords(&[
                Keyword::VISIBLE,
                Keyword::INVISIBLE,
                Keyword::AUTO_INCREMENT,
                Keyword::VIRTUAL,
                Keyword::STORED,
            ])
            .is_some()
        {
            None
        } else if self.parse_keyword(Keyword::UNIQUE) {
            let _ = self.parse_keyword(Keyword::KEY);
            None
        } else if self.parse_keywords(&[Keyword::PRIMARY, Keyword::KEY])
            || self.parse_keyword(Keyword::KEY)
        {
            Some(ColumnOption::PrimaryKey)
        } else if self.parse_keyword(Keyword::COMMENT) {
            Some(ColumnOption::Comment(self.parse_literal_string()))
        } else if self.parse_keyword(Keyword::COLLATE) {
            self.expect_identifier();
            None
        } else if self.parse_keyword(Keyword::COLUMN_FORMAT) {
            let _ =
                self.parse_one_of_keywords(&[Keyword::FIXED, Keyword::DYNAMIC, Keyword::DEFAULT]);
            None
        } else if self.parse_keyword(Keyword::ENGINE_ATTRIBUTE)
            || self.parse_keyword(Keyword::SECONDARY_ENGINE_ATTRIBUTE)
        {
            let _ = self.consume_token(&Token::Eq);
            self.parse_literal_string();
            None
        } else if self.parse_keyword(Keyword::STORAGE) {
            let _ = self.parse_one_of_keywords(&[Keyword::DISK, Keyword::MEMORY]);
            None
        } else if self.parse_keywords(&[Keyword::GENERATED, Keyword::ALWAYS, Keyword::AS])
            || self.parse_keyword(Keyword::AS)
        {
            self.parse_parenthesized_expr();
            None
        } else {
            let _ = self.parse_reference_definition() || self.parse_check_constraint_definition();
            None
        }
    }

    // check_constraint_definition:
    // [CONSTRAINT [symbol]] CHECK (expr) [[NOT] ENFORCED]
    //
    // Returns false if check constraint doesn't exist.
    pub fn parse_check_constraint_definition(&mut self) -> bool {
        if self.parse_keyword(Keyword::CONSTRAINT) {
            self.parse_optional_identifier();
        }
        if self.parse_keyword(Keyword::CHECK) {
            return false;
        }
        self.parse_parenthesized_expr();
        let _ = self.parse_keyword(Keyword::NOT);
        let _ = self.parse_keyword(Keyword::ENFORCED);
        true
    }

    /// Parse a literal string
    pub fn parse_literal_string(&mut self) -> String {
        let next_token = self.next_token();
        match next_token.token {
            Token::Word(Word {
                value,
                keyword: Keyword::NoKeyword,
                ..
            }) => value,
            Token::SingleQuotedString(s) => s,
            Token::DoubleQuotedString(s) => s,
            _ => panic!("Expected a literal string, found {}", next_token),
        }
    }

    // Parse `(expr)`.
    pub fn parse_parenthesized_expr(&mut self) {
        self.expect_token(&Token::LParen);
        let mut paren = 1;
        loop {
            let next_token = self.next_token();
            match next_token.token {
                Token::LParen => paren += 1,
                Token::RParen => {
                    paren -= 1;
                    if paren == 0 {
                        break;
                    }
                }
                _ => (),
            }
        }
    }

    pub fn parse_literal(&mut self) -> bool {
        matches!(
            self.next_token().token,
            Token::Number(_, _)
                | Token::Char(_)
                | Token::SingleQuotedString(_)
                | Token::DoubleQuotedString(_)
                | Token::DollarQuotedString(_)
                | Token::SingleQuotedByteStringLiteral(_)
                | Token::DoubleQuotedByteStringLiteral(_)
                | Token::RawStringLiteral(_)
                | Token::NationalStringLiteral(_)
                | Token::EscapedStringLiteral(_)
                | Token::HexStringLiteral(_)
        )
    }

    /// Parse an unsigned literal integer/long
    pub fn parse_literal_uint(&mut self) -> u64 {
        let next_token = self.next_token();
        match next_token.token {
            Token::Number(s, _) => s.parse::<u64>().unwrap(),
            _ => panic!("Expected literal int, found {}", next_token),
        }
    }

    pub fn parse_string_values(&mut self) -> Vec<String> {
        self.expect_token(&Token::LParen);
        let mut values = Vec::new();
        loop {
            let next_token = self.next_token();
            match next_token.token {
                Token::SingleQuotedString(value) => values.push(value),
                _ => panic!("Expected a string, found {}", next_token),
            }
            let next_token = self.next_token();
            match next_token.token {
                Token::Comma => (),
                Token::RParen => break,
                _ => panic!("Expected , or }}, found {}", next_token),
            }
        }
        values
    }

    /// Return the first non-whitespace token that has not yet been processed
    /// (or None if reached end-of-file)
    pub fn peek_token(&self) -> TokenWithLocation {
        self.peek_nth_token(0)
    }

    /// Return nth non-whitespace token that has not yet been processed
    pub fn peek_nth_token(&self, mut n: usize) -> TokenWithLocation {
        let mut index = self.index;
        loop {
            index += 1;
            match self.tokens.get(index - 1) {
                Some(TokenWithLocation {
                    token: Token::Whitespace(_),
                    location: _,
                }) => continue,
                non_whitespace => {
                    if n == 0 {
                        return non_whitespace.cloned().unwrap_or(TokenWithLocation {
                            token: Token::EOF,
                            location: Location { line: 0, column: 0 },
                        });
                    }
                    n -= 1;
                }
            }
        }
    }

    /// Return the first token, possibly whitespace, that has not yet been processed
    /// (or None if reached end-of-file).
    pub fn peek_token_no_skip(&self) -> TokenWithLocation {
        self.peek_nth_token_no_skip(0)
    }

    /// Return nth token, possibly whitespace, that has not yet been processed.
    pub fn peek_nth_token_no_skip(&self, n: usize) -> TokenWithLocation {
        self.tokens
            .get(self.index + n)
            .cloned()
            .unwrap_or(TokenWithLocation {
                token: Token::EOF,
                location: Location { line: 0, column: 0 },
            })
    }

    /// Return the first non-whitespace token that has not yet been processed
    /// (or None if reached end-of-file) and mark it as processed. OK to call
    /// repeatedly after reaching EOF.
    pub fn next_token(&mut self) -> TokenWithLocation {
        loop {
            self.index += 1;
            match self.tokens.get(self.index - 1) {
                Some(TokenWithLocation {
                    token: Token::Whitespace(_),
                    location: _,
                }) => continue,
                token => {
                    return token
                        .cloned()
                        .unwrap_or_else(|| TokenWithLocation::wrap(Token::EOF))
                }
            }
        }
    }

    /// Return the first unprocessed token, possibly whitespace.
    pub fn next_token_no_skip(&mut self) -> Option<&TokenWithLocation> {
        self.index += 1;
        self.tokens.get(self.index - 1)
    }

    /// Push back the last one non-whitespace token. Must be called after
    /// `next_token()`, otherwise might panic. OK to call after
    /// `next_token()` indicates an EOF.
    pub fn prev_token(&mut self) {
        loop {
            assert!(self.index > 0);
            self.index -= 1;
            if let Some(TokenWithLocation {
                token: Token::Whitespace(_),
                location: _,
            }) = self.tokens.get(self.index)
            {
                continue;
            }
            return;
        }
    }

    /// If the current token is the `expected` keyword, consume it and returns
    /// true. Otherwise, no tokens are consumed and returns false.
    #[must_use]
    pub fn parse_keyword(&mut self, expected: Keyword) -> bool {
        match self.peek_token().token {
            Token::Word(w) if expected == w.keyword => {
                self.next_token();
                true
            }
            _ => false,
        }
    }

    /// If the current and subsequent tokens exactly match the `keywords`
    /// sequence, consume them and returns true. Otherwise, no tokens are
    /// consumed and returns false
    #[must_use]
    pub fn parse_keywords(&mut self, keywords: &[Keyword]) -> bool {
        let index = self.index;
        for &keyword in keywords {
            if !self.parse_keyword(keyword) {
                // println!("parse_keywords aborting .. did not find {:?}", keyword);
                // reset index and return immediately
                self.index = index;
                return false;
            }
        }
        true
    }

    /// If the current token is one of the given `keywords`, consume the token
    /// and return the keyword that matches. Otherwise, no tokens are consumed
    /// and returns `None`.
    #[must_use]
    pub fn parse_one_of_keywords(&mut self, keywords: &[Keyword]) -> Option<Keyword> {
        match self.peek_token().token {
            Token::Word(w) => {
                keywords
                    .iter()
                    .find(|keyword| **keyword == w.keyword)
                    .map(|keyword| {
                        self.next_token();
                        *keyword
                    })
            }
            _ => None,
        }
    }

    /// If the current token is one of the expected keywords, consume the token
    /// and return the keyword that matches. Otherwise, return an error.
    pub fn expect_one_of_keywords(&mut self, keywords: &[Keyword]) -> Keyword {
        if let Some(keyword) = self.parse_one_of_keywords(keywords) {
            keyword
        } else {
            let keywords: Vec<String> = keywords.iter().map(|x| format!("{x:?}")).collect();
            panic!(
                "Expected one of {}, found: {}",
                keywords.join(" or "),
                self.peek_token()
            );
        }
    }

    /// If the current token is the `expected` keyword, consume the token.
    /// Otherwise return an error.
    pub fn expect_keyword(&mut self, expected: Keyword) {
        if !self.parse_keyword(expected) {
            panic!("Expected {:?}, found: {}", expected, self.peek_token())
        }
    }

    pub fn expect_keywords(&mut self, expected: &[Keyword]) {
        for &kw in expected {
            self.expect_keyword(kw);
        }
    }

    /// Consume the next token if it matches the expected token, otherwise return false
    #[must_use]
    pub fn consume_token(&mut self, expected: &Token) -> bool {
        if self.peek_token() == *expected {
            self.next_token();
            true
        } else {
            false
        }
    }

    /// Bail out if the current token is not an expected keyword, or consume it if it is
    pub fn expect_token(&mut self, expected: &Token) {
        if !self.consume_token(expected) {
            panic!(
                "Expected: {}, found: {}",
                &expected.to_string(),
                self.peek_token()
            )
        }
    }

    /// Parse a comma-separated list of 1+ items accepted by `F`
    pub fn parse_comma_separated<T, F>(&mut self, mut f: F) -> Vec<T>
    where
        F: FnMut(&mut Parser) -> T,
    {
        let mut values = vec![];
        loop {
            values.push(f(self));
            if !self.consume_token(&Token::Comma) {
                break;
            } else if TRAILING_COMMAS {
                match self.peek_token().token {
                    Token::Word(kw)
                        if keywords::RESERVED_FOR_COLUMN_ALIAS
                            .iter()
                            .any(|d| kw.keyword == *d) =>
                    {
                        break;
                    }
                    Token::RParen
                    | Token::SemiColon
                    | Token::EOF
                    | Token::RBracket
                    | Token::RBrace => break,
                    _ => continue,
                }
            }
        }
        values
    }

    /// Parse a possibly qualified, possibly quoted identifier, e.g.
    /// `foo` or `myschema."table".
    pub fn parse_object_name(&mut self) -> ObjectName {
        let mut idents = vec![];
        loop {
            idents.push(self.expect_identifier());
            if !self.consume_token(&Token::Period) {
                break;
            }
        }
        ObjectName(idents)
    }

    /// Parse identifiers
    pub fn parse_identifiers(&mut self) -> Vec<Ident> {
        let mut idents = vec![];
        loop {
            match self.peek_token().token {
                Token::Word(w) => {
                    idents.push(w.to_ident());
                }
                Token::EOF | Token::Eq => break,
                _ => {}
            }
            self.next_token();
        }
        idents
    }

    /// Parse a simple one-word identifier (possibly quoted, possibly a keyword)
    pub fn parse_optional_identifier(&mut self) -> Option<Ident> {
        let next_token = self.peek_token();
        let ident = match next_token.token {
            Token::Word(w) => w.to_ident(),
            Token::SingleQuotedString(s) => Ident::with_quote('\'', s),
            Token::DoubleQuotedString(s) => Ident::with_quote('\"', s),
            _ => return None,
        };
        self.next_token();
        Some(ident)
    }

    pub fn expect_identifier(&mut self) -> Ident {
        self.parse_optional_identifier()
            .unwrap_or_else(|| panic!("Expected identifier, found: {}", self.peek_token()))
    }
}
