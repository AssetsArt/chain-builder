//! `having()` operator allowlist guard.
//!
//! Unlike every other operator-bearing method (`where_eq`, `where_column`,
//! `JoinClause::on`, …) which take `op: &'static str` and so cannot receive a
//! runtime string, `having()` takes `op: &str` for ergonomics. To keep that
//! from becoming a SQL-injection vector when a caller wires an
//! attacker-controlled operator into the HAVING clause, the operator is
//! validated against a fixed allowlist and a disallowed operator panics
//! (fail-loud, matching the `offset`/`distinct_on`/lock guards). Arbitrary
//! aggregate expressions go through `having_raw` instead.

use chain_builder::{Postgres, QueryBuilder};

#[test]
fn allowed_operators_compile() {
    for op in ["=", "!=", "<>", ">", ">=", "<", "<=", "LIKE", "NOT LIKE"] {
        let (sql, _) = QueryBuilder::<Postgres>::table("orders")
            .select(["user_id"])
            .group_by(["user_id"])
            .having("total", op, 100i64)
            .to_sql();
        assert_eq!(
            sql,
            format!(r#"SELECT "user_id" FROM "orders" GROUP BY "user_id" HAVING "total" {op} $1"#)
        );
    }
}

#[test]
fn operator_is_case_insensitive_and_trimmed() {
    // `like` (lowercase, padded) is accepted and stored trimmed.
    let (sql, _) = QueryBuilder::<Postgres>::table("orders")
        .select(["user_id"])
        .having("name", "  like  ", "a%")
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "user_id" FROM "orders" HAVING "name" like $1"#
    );
}

#[test]
#[should_panic(expected = "not an allowed")]
fn injection_via_operator_panics() {
    // The classic generic-filter-API injection: an attacker-controlled operator.
    let _ = QueryBuilder::<Postgres>::table("orders")
        .select(["user_id"])
        .having("amount", ">= 0 UNION SELECT password FROM users --", 0i64)
        .to_sql();
}

#[test]
#[should_panic(expected = "not an allowed")]
fn arbitrary_operator_panics() {
    let _ = QueryBuilder::<Postgres>::table("orders")
        .select(["user_id"])
        .having("amount", "; DROP TABLE users", 0i64)
        .to_sql();
}
