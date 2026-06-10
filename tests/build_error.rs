//! `BuildError` / `try_to_sql()` — fallible compilation.
//!
//! Every fail-loud guard that panics in `to_sql()` must surface as a typed
//! `Err(BuildError)` from `try_to_sql()` (and the sqlx handoff twins
//! `try_to_sqlx_query` / `try_to_sqlx_query_as`), so web apps can map invalid
//! query construction to an HTTP 4XX/5XX instead of crashing. `to_sql()`
//! keeps its panicking behavior (same messages) for backward compatibility.

use chain_builder::{BuildError, MySql, Postgres, QueryBuilder, Value};

#[test]
fn offset_without_limit_errs() {
    let err = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .offset(10)
        .try_to_sql()
        .unwrap_err();
    assert_eq!(err, BuildError::OffsetWithoutLimit);
    assert_eq!(err.to_string(), "offset(...) requires limit(...)");
}

#[test]
fn lock_on_non_select_errs() {
    let err = QueryBuilder::<Postgres>::table("users")
        .update([("status", "x")])
        .for_update()
        .try_to_sql()
        .unwrap_err();
    assert_eq!(err, BuildError::LockRequiresSelect);
    assert_eq!(
        err.to_string(),
        "for_update()/for_share() is only valid on SELECT"
    );
}

#[test]
fn distinct_on_requires_postgres_errs() {
    let err = QueryBuilder::<MySql>::table("users")
        .select(["id"])
        .distinct_on(["id"])
        .try_to_sql()
        .unwrap_err();
    assert_eq!(err, BuildError::DistinctOnRequiresPostgres);
    assert_eq!(err.to_string(), "DISTINCT ON requires PostgreSQL");
}

#[test]
fn empty_insert_errs() {
    let err = QueryBuilder::<Postgres>::table("users")
        .insert(std::iter::empty::<(&str, Value)>())
        .try_to_sql()
        .unwrap_err();
    assert_eq!(err, BuildError::EmptyInsert);
    assert_eq!(err.to_string(), "insert() requires at least one column");
}

#[test]
fn empty_update_errs() {
    let err = QueryBuilder::<Postgres>::table("users")
        .update(std::iter::empty::<(&str, Value)>())
        .try_to_sql()
        .unwrap_err();
    assert_eq!(err, BuildError::EmptyUpdate);
    assert_eq!(err.to_string(), "update() requires at least one column");
}

#[test]
fn lock_with_union_errs() {
    let err = QueryBuilder::<Postgres>::table("a")
        .select(["id"])
        .union(QueryBuilder::<Postgres>::table("b").select(["id"]))
        .for_update()
        .try_to_sql()
        .unwrap_err();
    assert_eq!(err, BuildError::LockWithUnion);
    assert_eq!(
        err.to_string(),
        "for_update()/for_share() cannot be combined with UNION"
    );
}

#[test]
fn invalid_having_operator_is_deferred_not_a_call_time_panic() {
    // Building with a bad operator must NOT panic at the `having()` call —
    // the error is recorded and surfaces from `try_to_sql()`.
    let qb = QueryBuilder::<Postgres>::table("orders")
        .select(["user_id"])
        .having("amount", "; DROP TABLE users", 0i64);
    let err = qb.try_to_sql().unwrap_err();
    assert_eq!(
        err,
        BuildError::InvalidHavingOperator("; DROP TABLE users".to_owned())
    );
    assert!(err.to_string().contains("not an allowed"));
}

#[test]
fn first_deferred_error_wins() {
    let err = QueryBuilder::<Postgres>::table("orders")
        .select(["user_id"])
        .having("a", "BAD-OP-1", 1i64)
        .having("b", "BAD-OP-2", 2i64)
        .try_to_sql()
        .unwrap_err();
    assert_eq!(
        err,
        BuildError::InvalidHavingOperator("BAD-OP-1".to_owned())
    );
}

#[test]
fn deferred_error_propagates_from_cte() {
    let bad_inner = QueryBuilder::<Postgres>::table("orders")
        .select(["user_id"])
        .having("amount", "UNION SELECT", 0i64);
    let err = QueryBuilder::<Postgres>::table("top")
        .select(["user_id"])
        .with("top", bad_inner)
        .try_to_sql()
        .unwrap_err();
    assert_eq!(
        err,
        BuildError::InvalidHavingOperator("UNION SELECT".to_owned())
    );
}

#[test]
fn deferred_error_propagates_from_union_arm() {
    let bad_arm = QueryBuilder::<Postgres>::table("b")
        .select(["id"])
        .having("x", "NOPE", 1i64);
    let err = QueryBuilder::<Postgres>::table("a")
        .select(["id"])
        .union(bad_arm)
        .try_to_sql()
        .unwrap_err();
    assert_eq!(err, BuildError::InvalidHavingOperator("NOPE".to_owned()));
}

#[test]
fn deferred_error_propagates_from_select_subquery() {
    let bad_sub = QueryBuilder::<Postgres>::table("orders")
        .select(["amount"])
        .having("amount", "OOPS", 1i64);
    let err = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .select_subquery("total", bad_sub)
        .try_to_sql()
        .unwrap_err();
    assert_eq!(err, BuildError::InvalidHavingOperator("OOPS".to_owned()));
}

#[test]
fn deferred_error_propagates_from_where_exists() {
    let bad_sub = QueryBuilder::<Postgres>::table("orders")
        .select(["id"])
        .having("id", "OOPS", 1i64);
    let err = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .where_exists(bad_sub)
        .try_to_sql()
        .unwrap_err();
    assert_eq!(err, BuildError::InvalidHavingOperator("OOPS".to_owned()));
}

#[test]
fn deferred_error_propagates_from_where_in_subquery() {
    let bad_sub = QueryBuilder::<Postgres>::table("orders")
        .select(["user_id"])
        .having("user_id", "OOPS", 1i64);
    let err = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .where_in_subquery("id", bad_sub)
        .try_to_sql()
        .unwrap_err();
    assert_eq!(err, BuildError::InvalidHavingOperator("OOPS".to_owned()));
}

#[test]
fn valid_query_try_to_sql_ok() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .where_eq("status", "active")
        .limit(10)
        .offset(20)
        .try_to_sql()
        .unwrap();
    assert_eq!(
        sql,
        r#"SELECT "id" FROM "users" WHERE "status" = $1 LIMIT $2 OFFSET $3"#
    );
    assert_eq!(binds.len(), 3);
}

#[test]
fn valid_having_still_compiles_via_try_to_sql() {
    let (sql, _) = QueryBuilder::<Postgres>::table("orders")
        .select(["user_id"])
        .group_by(["user_id"])
        .having("total", ">", 100i64)
        .try_to_sql()
        .unwrap();
    assert_eq!(
        sql,
        r#"SELECT "user_id" FROM "orders" GROUP BY "user_id" HAVING "total" > $1"#
    );
}

#[test]
fn build_error_is_std_error_send_sync() {
    fn assert_traits<E: std::error::Error + Send + Sync + 'static>() {}
    assert_traits::<BuildError>();
}

#[test]
fn try_compile_is_exported() {
    let qb = QueryBuilder::<Postgres>::table("users").select(["id"]);
    let (sql, _) = chain_builder::try_compile(&qb).unwrap();
    assert_eq!(sql, r#"SELECT "id" FROM "users""#);
}

// ---- sqlx handoff twins (MySql = default `sqlx_mysql` feature) ----

#[cfg(feature = "sqlx_mysql")]
mod sqlx_handoff {
    use super::*;

    #[test]
    fn try_to_sqlx_query_errs_on_invalid_builder() {
        let err = QueryBuilder::<MySql>::table("users")
            .select(["id"])
            .offset(5)
            .try_to_sqlx_query()
            .map(|_| ())
            .unwrap_err();
        assert_eq!(err, BuildError::OffsetWithoutLimit);
    }

    #[test]
    fn try_to_sqlx_query_ok_on_valid_builder() {
        assert!(QueryBuilder::<MySql>::table("users")
            .select(["id"])
            .where_eq("status", "active")
            .try_to_sqlx_query()
            .is_ok());
    }

    #[derive(sqlx::FromRow)]
    #[allow(dead_code)]
    struct UserRow {
        id: i64,
    }

    #[test]
    fn try_to_sqlx_query_as_errs_on_invalid_builder() {
        let err = QueryBuilder::<MySql>::table("users")
            .select(["id"])
            .having("id", "BAD", 1i64)
            .try_to_sqlx_query_as::<UserRow>()
            .map(|_| ())
            .unwrap_err();
        assert_eq!(err, BuildError::InvalidHavingOperator("BAD".to_owned()));
    }

    #[test]
    fn try_to_sqlx_query_as_ok_on_valid_builder() {
        assert!(QueryBuilder::<MySql>::table("users")
            .select(["id"])
            .try_to_sqlx_query_as::<UserRow>()
            .is_ok());
    }
}
