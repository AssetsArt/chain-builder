//! T5 — v2 sqlx handoff tests.
//!
//! Each test is gated by a single backend feature so the file compiles under
//! one-backend feature sets (the 1.x crate cannot build with 2+ sqlx backends
//! at once). Asserts the sqlx query's SQL equals the builder's `to_sql().0`.

#![cfg(feature = "v2")]

// A trivial row type used only for the `to_sqlx_query_as` compile-smoke checks.
#[cfg(any(
    feature = "sqlx_postgres",
    feature = "sqlx_mysql",
    feature = "sqlx_sqlite"
))]
#[derive(Debug)]
#[allow(dead_code)]
struct IdRow {
    id: i64,
}

#[cfg(feature = "sqlx_postgres")]
impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for IdRow {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(IdRow { id: row.try_get("id")? })
    }
}

#[cfg(feature = "sqlx_mysql")]
impl<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow> for IdRow {
    fn from_row(row: &'r sqlx::mysql::MySqlRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(IdRow { id: row.try_get("id")? })
    }
}

#[cfg(feature = "sqlx_sqlite")]
impl<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow> for IdRow {
    fn from_row(row: &'r sqlx::sqlite::SqliteRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(IdRow { id: row.try_get("id")? })
    }
}

#[cfg(feature = "sqlx_postgres")]
#[test]
fn postgres_to_sqlx_sql_matches_to_sql() {
    use chain_builder::v2::{Postgres, QueryBuilder};
    use sqlx::Execute;
    let qb = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .where_eq("status", "active");
    let (sql, _) = qb.to_sql();
    let q = qb.to_sqlx_query();
    assert_eq!(q.sql(), sql);
}

#[cfg(feature = "sqlx_postgres")]
#[test]
fn postgres_to_sqlx_query_as_compiles() {
    use chain_builder::v2::{Postgres, QueryBuilder};
    use sqlx::Execute;
    let qb = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .where_eq("status", "active");
    let (sql, _) = qb.to_sql();
    let q = qb.to_sqlx_query_as::<IdRow>();
    assert_eq!(q.sql(), sql);
}

#[cfg(feature = "sqlx_mysql")]
#[test]
fn mysql_to_sqlx_sql_matches_to_sql() {
    use chain_builder::v2::{MySql, QueryBuilder};
    use sqlx::Execute;
    let qb = QueryBuilder::<MySql>::table("users")
        .select(["id"])
        .where_eq("status", "active");
    let (sql, _) = qb.to_sql();
    let q = qb.to_sqlx_query();
    assert_eq!(q.sql(), sql);
}

#[cfg(feature = "sqlx_mysql")]
#[test]
fn mysql_to_sqlx_query_as_compiles() {
    use chain_builder::v2::{MySql, QueryBuilder};
    use sqlx::Execute;
    let qb = QueryBuilder::<MySql>::table("users")
        .select(["id"])
        .where_eq("status", "active");
    let (sql, _) = qb.to_sql();
    let q = qb.to_sqlx_query_as::<IdRow>();
    assert_eq!(q.sql(), sql);
}

#[cfg(feature = "sqlx_sqlite")]
#[test]
fn sqlite_to_sqlx_sql_matches_to_sql() {
    use chain_builder::v2::{QueryBuilder, Sqlite};
    use sqlx::Execute;
    let qb = QueryBuilder::<Sqlite>::table("users")
        .select(["id"])
        .where_eq("status", "active");
    let (sql, _) = qb.to_sql();
    let q = qb.to_sqlx_query();
    assert_eq!(q.sql(), sql);
}

#[cfg(feature = "sqlx_sqlite")]
#[test]
fn sqlite_to_sqlx_query_as_compiles() {
    use chain_builder::v2::{QueryBuilder, Sqlite};
    use sqlx::Execute;
    let qb = QueryBuilder::<Sqlite>::table("users")
        .select(["id"])
        .where_eq("status", "active");
    let (sql, _) = qb.to_sql();
    let q = qb.to_sqlx_query_as::<IdRow>();
    assert_eq!(q.sql(), sql);
}
