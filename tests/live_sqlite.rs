#![cfg(feature = "sqlx_sqlite")]
//! Live end-to-end SQLite (`:memory:`) integration tests.
//!
//! Proves the chain-builder → sqlx round-trip actually executes against a real
//! database: insert, fetch, count, upsert (`ON CONFLICT DO NOTHING`) and
//! `RETURNING`.

use chain_builder::{IntoBind, QueryBuilder, Sqlite, Value};

#[derive(Debug, sqlx::FromRow, PartialEq)]
struct User {
    id: i64,
    name: String,
}

/// `insert` requires a homogeneous value type across the row, so heterogeneous
/// columns (`i64`, `&str`, `i64`) are normalized to [`Value`] via [`IntoBind`].
fn row(pairs: Vec<(&str, Value)>) -> Vec<(&str, Value)> {
    pairs
}

#[tokio::test]
async fn sqlite_round_trip() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, age INTEGER)")
        .execute(&pool)
        .await
        .unwrap();

    // insert via chain-builder
    QueryBuilder::<Sqlite>::table("users")
        .insert(row(vec![
            ("id", 1i64.into_bind()),
            ("name", "Ann".into_bind()),
            ("age", 30i64.into_bind()),
        ]))
        .execute(&pool)
        .await
        .unwrap();

    // fetch via chain-builder
    let rows: Vec<User> = QueryBuilder::<Sqlite>::table("users")
        .select(["id", "name"])
        .order_by_asc("id")
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

    // count
    let n: i64 = QueryBuilder::<Sqlite>::table("users")
        .count(&pool)
        .await
        .unwrap();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn sqlite_upsert_and_returning() {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL)")
        .execute(&pool)
        .await
        .unwrap();

    // First insert with RETURNING — proves RETURNING decodes end-to-end.
    let returned: Vec<(i64,)> = QueryBuilder::<Sqlite>::table("users")
        .insert(row(vec![
            ("id", 1i64.into_bind()),
            ("name", "Ann".into_bind()),
        ]))
        .returning(["id"])
        .fetch_all(&pool)
        .await
        .unwrap();
    assert_eq!(returned, vec![(1,)]);

    // Conflicting insert with ON CONFLICT DO NOTHING — no row produced, no error.
    let conflicted: Vec<(i64,)> = QueryBuilder::<Sqlite>::table("users")
        .insert(row(vec![
            ("id", 1i64.into_bind()),
            ("name", "Bob".into_bind()),
        ]))
        .on_conflict_do_nothing(["id"])
        .returning(["id"])
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(conflicted.is_empty());

    // The original row is untouched and there is still exactly one row.
    let rows: Vec<User> = QueryBuilder::<Sqlite>::table("users")
        .select(["id", "name"])
        .order_by_asc("id")
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
