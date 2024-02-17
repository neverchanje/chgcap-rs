use std::{iter::Peekable, str::Chars};

pub fn is_identifier_start(ch: char) -> bool {
    // See https://dev.mysql.com/doc/refman/8.0/en/identifiers.html.
    // Identifiers which begin with a digit are recognized while tokenizing numbers,
    // so they can be distinguished from exponent numeric literals.
    ch.is_alphabetic()
        || ch == '_'
        || ch == '$'
        || ch == '@'
        || ('\u{0080}'..='\u{ffff}').contains(&ch)
}

pub fn is_identifier_part(ch: char) -> bool {
    is_identifier_start(ch) || ch.is_ascii_digit()
}

pub fn is_delimited_identifier_start(ch: char) -> bool {
    ch == '`'
}

pub fn is_proper_identifier_inside_quotes(mut _chars: Peekable<Chars<'_>>) -> bool {
    true
}
