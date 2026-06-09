//! MySQL dialect marker.

use super::Dialect;

/// MySQL dialect marker.
#[derive(Debug, Clone, Copy)]
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
}
