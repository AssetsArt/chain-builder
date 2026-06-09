//! SQL dialect markers and the [`Dialect`] trait.
//!
//! Each supported database is represented by a zero-sized marker type
//! (`MySql`, `Postgres`, `Sqlite`) implementing [`Dialect`]. The trait exposes
//! the dialect-specific bits the builder needs: the identifier quote character,
//! placeholder syntax, and whether `RETURNING` is supported.

mod mysql;
mod postgres;
mod sqlite;

pub use mysql::MySql;
pub use postgres::Postgres;
pub use sqlite::Sqlite;

/// Compile-time description of a SQL dialect.
pub trait Dialect: Sized + Send + Sync + 'static {
    /// The identifier quote character (e.g. backtick for MySQL, `"` for ANSI).
    fn quote_char() -> char;

    /// Write the bind placeholder for the `n`-th parameter (1-based) into `out`.
    ///
    /// Dialects with positional placeholders (`?`) ignore `n`.
    fn write_placeholder(out: &mut String, n: usize);

    /// Whether this dialect supports the `RETURNING` clause.
    fn supports_returning() -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_placeholders_are_numbered() {
        let mut out = String::new();
        Postgres::write_placeholder(&mut out, 1);
        Postgres::write_placeholder(&mut out, 2);
        assert_eq!(out, "$1$2");
    }

    #[test]
    fn mysql_placeholders_are_question_marks() {
        let mut out = String::new();
        MySql::write_placeholder(&mut out, 1);
        MySql::write_placeholder(&mut out, 2);
        assert_eq!(out, "??");
    }

    #[test]
    fn sqlite_placeholders_are_question_marks() {
        let mut out = String::new();
        Sqlite::write_placeholder(&mut out, 1);
        Sqlite::write_placeholder(&mut out, 2);
        assert_eq!(out, "??");
    }

    #[test]
    fn quote_chars() {
        assert_eq!(MySql::quote_char(), '\u{60}');
        assert_eq!(Postgres::quote_char(), '"');
        assert_eq!(Sqlite::quote_char(), '"');
    }

    #[test]
    fn returning_support() {
        assert!(Postgres::supports_returning());
        assert!(Sqlite::supports_returning());
        assert!(!MySql::supports_returning());
    }
}
