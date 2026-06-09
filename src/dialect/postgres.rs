//! PostgreSQL dialect marker.

use super::Dialect;

/// PostgreSQL dialect marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Postgres;

impl Dialect for Postgres {
    fn quote_char() -> char {
        '"'
    }

    fn write_placeholder(out: &mut String, n: usize) {
        out.push('$');
        out.push_str(&n.to_string());
    }

    fn supports_returning() -> bool {
        true
    }

    fn supports_distinct_on() -> bool {
        true
    }

    fn ilike_is_native() -> bool {
        true
    }
}
