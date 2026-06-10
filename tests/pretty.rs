//! 3.1.0: `to_sql_pretty` / `try_to_sql_pretty`.

use chain_builder::{Postgres, QueryBuilder, Sqlite, Value};

#[test]
fn pg_pretty_multi_bind() {
    let out = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .where_eq("status", "active")
        .where_gt("age", 21i64)
        .to_sql_pretty();
    assert_eq!(
        out,
        "SELECT \"id\" FROM \"users\" WHERE \"status\" = $1 AND \"age\" > $2\nbinds:\n  $1 = Text(\"active\")\n  $2 = I64(21)"
    );
}

#[test]
fn sqlite_pretty_labels_ordinals() {
    let out = QueryBuilder::<Sqlite>::table("users")
        .select(["id"])
        .where_eq("status", "active")
        .where_gt("age", 21i64)
        .to_sql_pretty();
    assert_eq!(
        out,
        "SELECT \"id\" FROM \"users\" WHERE \"status\" = ? AND \"age\" > ?\nbinds:\n  ?1 = Text(\"active\")\n  ?2 = I64(21)"
    );
}

#[test]
fn pretty_zero_binds_is_sql_only() {
    let out = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .to_sql_pretty();
    assert_eq!(out, "SELECT \"id\" FROM \"users\"");
}

#[test]
fn try_pretty_surfaces_same_build_error() {
    let qb = QueryBuilder::<Postgres>::table("t").update(std::iter::empty::<(&str, Value)>());
    assert_eq!(
        qb.try_to_sql_pretty().unwrap_err(),
        qb.try_to_sql().unwrap_err()
    );
}

#[test]
#[should_panic(expected = "update() requires at least one column")]
fn pretty_panic_twin_message_parity() {
    let _ = QueryBuilder::<Postgres>::table("t")
        .update(std::iter::empty::<(&str, Value)>())
        .to_sql_pretty();
}
