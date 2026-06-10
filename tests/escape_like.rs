//! 3.1.0: `escape_like` — LIKE-pattern metacharacter escaping.

use chain_builder::escape_like;

#[test]
fn pinned_cookbook_assertion() {
    // Byte-identical to the recipe in docs/book/src/cookbook/search.md.
    assert_eq!(escape_like("100%_a\\b"), "100\\%\\_a\\\\b");
}

#[test]
fn empty_input_is_empty() {
    assert_eq!(escape_like(""), "");
}

#[test]
fn input_without_metachars_is_unchanged() {
    assert_eq!(escape_like("hello world"), "hello world");
}

#[test]
fn each_metachar_escaped() {
    assert_eq!(escape_like("%"), "\\%");
    assert_eq!(escape_like("_"), "\\_");
    assert_eq!(escape_like("\\"), "\\\\");
}

#[test]
fn adjacent_metachars_validate_escape_ordering() {
    // `\%_` must become `\\` + `\%` + `\_` — a wrong replace order would
    // double-escape the backslashes produced for `%`/`_`.
    assert_eq!(escape_like("\\%_"), "\\\\\\%\\_");
}
