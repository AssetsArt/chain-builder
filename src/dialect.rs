//! Dialect-aware SQL identifier escaping.
//!
//! Values are always sent to the database as bound parameters (`?`), but SQL
//! *identifiers* (table names, column names, aliases) are interpolated directly
//! into the generated SQL. If any of those identifiers can be influenced by
//! untrusted input (e.g. a dynamic `ORDER BY` column coming from a request),
//! interpolating them verbatim is a SQL-injection vector.
//!
//! [`escape_identifier`] quotes identifiers using the dialect's quote character
//! and doubles any embedded quote character, which is the standard, injection-safe
//! way to emit an identifier.

use crate::types::Client;

impl Client {
    /// The identifier quote character for this dialect.
    ///
    /// MySQL uses backticks; SQLite and PostgreSQL use ANSI double quotes.
    pub(crate) fn quote_char(&self) -> char {
        match self {
            Client::Mysql => '`',
            Client::Sqlite | Client::Postgres => '"',
        }
    }
}

/// Escape a SQL identifier so that attacker-controlled table/column/alias names
/// cannot break out of the identifier context.
///
/// Behaviour:
/// - The identifier is split on `.` so qualified names like `db.table.col` are
///   quoted segment-by-segment (`` `db`.`table`.`col` ``).
/// - A bare `*` segment (wildcard) is passed through unquoted, so `t.*` becomes
///   `` `t`.* `` and `*` stays `*`.
/// - Any occurrence of the quote character inside a segment is doubled, which
///   neutralizes attempts to terminate the quoted identifier early.
/// - Surrounding whitespace is trimmed; empty input yields an empty string.
pub(crate) fn escape_identifier(ident: &str, client: &Client) -> String {
    let quote = client.quote_char();
    let trimmed = ident.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let mut out = String::with_capacity(trimmed.len() + 4);
    for (i, part) in trimmed.split('.').enumerate() {
        if i > 0 {
            out.push('.');
        }
        let part = part.trim();
        if part == "*" {
            out.push('*');
            continue;
        }
        out.push(quote);
        for ch in part.chars() {
            if ch == quote {
                // Double the quote char to embed it safely.
                out.push(quote);
            }
            out.push(ch);
        }
        out.push(quote);
    }
    out
}

/// Escape each identifier in a list and join them with `, `.
pub(crate) fn escape_identifier_list(idents: &[String], client: &Client) -> String {
    idents
        .iter()
        .map(|ident| escape_identifier(ident, client))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_identifier_mysql() {
        assert_eq!(escape_identifier("name", &Client::Mysql), "`name`");
    }

    #[test]
    fn qualified_identifier_mysql() {
        assert_eq!(
            escape_identifier("users.name", &Client::Mysql),
            "`users`.`name`"
        );
    }

    #[test]
    fn wildcard_passthrough() {
        assert_eq!(escape_identifier("*", &Client::Mysql), "*");
        assert_eq!(escape_identifier("users.*", &Client::Mysql), "`users`.*");
    }

    #[test]
    fn sqlite_uses_double_quotes() {
        assert_eq!(escape_identifier("name", &Client::Sqlite), "\"name\"");
    }

    #[test]
    fn injection_attempt_is_neutralized_mysql() {
        // A backtick in the input is doubled, keeping it inside one identifier.
        assert_eq!(
            escape_identifier("name` = 1; DROP TABLE users; -- ", &Client::Mysql),
            "`name`` = 1; DROP TABLE users; --`"
        );
    }

    #[test]
    fn injection_attempt_is_neutralized_sqlite() {
        assert_eq!(
            escape_identifier("name\" OR \"1\"=\"1", &Client::Sqlite),
            "\"name\"\" OR \"\"1\"\"=\"\"1\""
        );
    }

    #[test]
    fn empty_input_yields_empty() {
        assert_eq!(escape_identifier("   ", &Client::Mysql), "");
    }
}
