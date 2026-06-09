//! MySQL dialect marker.

use super::{Dialect, UpsertStyle};

/// MySQL dialect marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MySql;

impl Dialect for MySql {
    fn quote_char() -> char {
        '\u{60}' // backtick
    }

    fn write_placeholder(out: &mut String, _n: usize) {
        out.push('?');
    }

    fn supports_returning() -> bool {
        false
    }

    fn upsert_style() -> UpsertStyle {
        UpsertStyle::OnDuplicateKey
    }
}
