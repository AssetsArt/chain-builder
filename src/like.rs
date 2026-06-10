//! `LIKE`-pattern escaping helper.

/// Escape `\`, `%`, and `_` so user input matches **literally** inside a
/// `LIKE`/`ILIKE` pattern. The caller still wraps the result in `%…%` /
/// `…%` as needed.
///
/// Backslash is escaped first (order matters), then `%` and `_`:
///
/// ```
/// use chain_builder::escape_like;
/// assert_eq!(escape_like("100%_a\\b"), "100\\%\\_a\\\\b");
/// assert_eq!(format!("%{}%", escape_like("50%")), "%50\\%%");
/// ```
///
/// # Portability caveat
///
/// What `\` means inside a `LIKE` pattern is dialect-defined: Postgres and
/// MySQL treat backslash as the default escape character, but **SQLite has
/// no default escape character at all**. For fully portable literal
/// matching, pair the escaped pattern with an explicit `ESCAPE '\'` clause
/// via `where_raw` (on MySQL write the literal as `ESCAPE '\\'`). See the
/// search-pattern chapter of the book for the full recipe.
pub fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}
