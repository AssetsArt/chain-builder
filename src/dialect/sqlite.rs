//! SQLite dialect marker.

use super::Dialect;

/// SQLite dialect marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sqlite;

impl Dialect for Sqlite {
    fn quote_char() -> char {
        '"'
    }

    fn write_placeholder(out: &mut String, _n: usize) {
        out.push('?');
    }

    fn supports_returning() -> bool {
        true
    }

    fn supports_row_locking() -> bool {
        false
    }
}
