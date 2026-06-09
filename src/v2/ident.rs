//! Dialect-aware SQL identifier escaping.
//!
//! Values are always sent to the database as bound parameters, but SQL
//! *identifiers* (table names, column names, aliases) are interpolated directly
//! into the generated SQL. If any of those identifiers can be influenced by
//! untrusted input (e.g. a dynamic `ORDER BY` column coming from a request),
//! interpolating them verbatim is a SQL-injection vector.
//!
//! [`escape_identifier`] quotes identifiers using the dialect's quote character
//! and doubles any embedded quote character, which is the standard, injection-safe
//! way to emit an identifier.

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
pub fn escape_identifier(ident: &str, quote: char) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    const BACKTICK: char = '\u{60}';
    const DQUOTE: char = '"';

    #[test]
    fn plain_identifier_mysql() {
        assert_eq!(escape_identifier("name", BACKTICK), "`name`");
    }

    #[test]
    fn qualified_identifier_mysql() {
        assert_eq!(escape_identifier("users.name", BACKTICK), "`users`.`name`");
    }

    #[test]
    fn wildcard_passthrough() {
        assert_eq!(escape_identifier("*", BACKTICK), "*");
        assert_eq!(escape_identifier("users.*", BACKTICK), "`users`.*");
        assert_eq!(escape_identifier("t.*", BACKTICK), "`t`.*");
    }

    #[test]
    fn sqlite_uses_double_quotes() {
        assert_eq!(escape_identifier("name", DQUOTE), "\"name\"");
    }

    #[test]
    fn injection_attempt_is_neutralized_mysql() {
        // A backtick in the input is doubled, keeping it inside one identifier.
        assert_eq!(
            escape_identifier("name` = 1; DROP TABLE users; -- ", BACKTICK),
            "`name`` = 1; DROP TABLE users; --`"
        );
    }

    #[test]
    fn injection_attempt_is_neutralized_double_quote() {
        assert_eq!(
            escape_identifier("name\" OR \"1\"=\"1", DQUOTE),
            "\"name\"\" OR \"\"1\"\"=\"\"1\""
        );
    }

    #[test]
    fn empty_input_yields_empty() {
        assert_eq!(escape_identifier("   ", BACKTICK), "");
    }
}
