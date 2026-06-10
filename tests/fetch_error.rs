#![cfg(feature = "sqlx_sqlite")]
//! `fetch_*` / `execute` / `count` return the unified [`chain_builder::Error`]:
//! an invalid builder surfaces as `Error::Build(BuildError)` BEFORE touching
//! the database, and a database failure surfaces as `Error::Sqlx(sqlx::Error)`
//! — no panic on either path.

use chain_builder::{BuildError, Error, QueryBuilder, Sqlite};

#[derive(Debug, sqlx::FromRow, PartialEq)]
struct User {
    id: i64,
    name: String,
}

async fn pool() -> sqlx::SqlitePool {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")
        .execute(&pool)
        .await
        .unwrap();
    pool
}

#[tokio::test]
async fn invalid_builder_fetch_all_errs_with_build_error() {
    let pool = pool().await;
    let err = QueryBuilder::<Sqlite>::table("users")
        .select(["id", "name"])
        .offset(5) // offset without limit → BuildError, not a panic
        .fetch_all::<User, _>(&pool)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Build(BuildError::OffsetWithoutLimit)));
}

#[tokio::test]
async fn invalid_builder_execute_errs_with_build_error() {
    let pool = pool().await;
    let err = QueryBuilder::<Sqlite>::table("users")
        .update(std::iter::empty::<(&str, i64)>())
        .execute(&pool)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Build(BuildError::EmptyUpdate)));
}

#[tokio::test]
async fn invalid_builder_count_errs_with_build_error() {
    let pool = pool().await;
    let err = QueryBuilder::<Sqlite>::table("users")
        .select(["id"])
        .having("id", "BAD-OP", 1i64)
        .count(&pool)
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        Error::Build(BuildError::InvalidHavingOperator(_))
    ));
}

#[tokio::test]
async fn database_failure_errs_with_sqlx_error() {
    let pool = pool().await;
    let err = QueryBuilder::<Sqlite>::table("no_such_table")
        .select(["id", "name"])
        .fetch_all::<User, _>(&pool)
        .await
        .unwrap_err();
    assert!(matches!(err, Error::Sqlx(_)));
}

#[tokio::test]
async fn valid_query_round_trips_ok() {
    let pool = pool().await;
    use chain_builder::Value;
    QueryBuilder::<Sqlite>::table("users")
        .insert([("id", Value::I64(1)), ("name", Value::Text("Ann".into()))])
        .execute(&pool)
        .await
        .unwrap();
    let rows: Vec<User> = QueryBuilder::<Sqlite>::table("users")
        .select(["id", "name"])
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(
        rows,
        vec![User {
            id: 1,
            name: "Ann".into()
        }]
    );
}

#[test]
fn error_is_std_error_with_source() {
    fn assert_traits<E: std::error::Error + Send + Sync + 'static>() {}
    assert_traits::<Error>();

    let e = Error::from(BuildError::OffsetWithoutLimit);
    assert_eq!(e.to_string(), "offset(...) requires limit(...)");
    assert!(std::error::Error::source(&e).is_some());
}

#[test]
fn error_converts_from_both_sides() {
    let _: Error = BuildError::EmptyInsert.into();
    let _: Error = sqlx::Error::RowNotFound.into();
}
