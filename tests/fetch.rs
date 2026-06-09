//! M4 typed-fetch compile tests.
//!
//! These prove the generic plumbing of the `fetch_*` / `execute` / `count`
//! helpers typechecks for each sqlx backend. They never touch a live database —
//! the `async fn _typechecks` bodies are compiled (which is the assertion) but
//! never run; the `#[test]` only needs to exist so the module is built under its
//! backend feature. Live DB integration is deferred to a runtime/CI task (needs a
//! sqlx runtime feature + an async executor).

#[cfg(feature = "sqlx_postgres")]
#[allow(dead_code)]
mod pg {
    use chain_builder::{Postgres, QueryBuilder};

    #[derive(sqlx::FromRow)]
    struct UserRow {
        id: i64,
        name: String,
    }

    async fn _typechecks(pool: sqlx::PgPool) -> Result<(), sqlx::Error> {
        let qb = QueryBuilder::<Postgres>::table("users")
            .select(["id", "name"])
            .where_gt("id", 0i64);
        let rows: Vec<UserRow> = qb.fetch_all(&pool).await?;
        let _ = rows;
        let one: UserRow = qb.fetch_one(&pool).await?;
        let _ = (one.id, one.name);
        let _opt: Option<UserRow> = qb.fetch_optional(&pool).await?;
        let _ = qb.execute(&pool).await?;
        let _: i64 = qb.count(&pool).await?;
        let _: i64 = QueryBuilder::<Postgres>::table("users")
            .select(["id"])
            .fetch_scalar(&pool)
            .await?;
        let _: Option<i64> = QueryBuilder::<Postgres>::table("users")
            .select(["id"])
            .fetch_optional_scalar(&pool)
            .await?;
        Ok(())
    }

    #[test]
    fn pg_typechecks_compile() {
        // Presence proves `_typechecks` compiled for Postgres.
    }
}

#[cfg(feature = "sqlx_mysql")]
#[allow(dead_code)]
mod mysql {
    use chain_builder::{MySql, QueryBuilder};

    #[derive(sqlx::FromRow)]
    struct UserRow {
        id: i64,
        name: String,
    }

    async fn _typechecks(pool: sqlx::MySqlPool) -> Result<(), sqlx::Error> {
        let qb = QueryBuilder::<MySql>::table("users")
            .select(["id", "name"])
            .where_gt("id", 0i64);
        let rows: Vec<UserRow> = qb.fetch_all(&pool).await?;
        let _ = rows;
        let one: UserRow = qb.fetch_one(&pool).await?;
        let _ = (one.id, one.name);
        let _opt: Option<UserRow> = qb.fetch_optional(&pool).await?;
        let _ = qb.execute(&pool).await?;
        let _: i64 = qb.count(&pool).await?;
        let _: i64 = QueryBuilder::<MySql>::table("users")
            .select(["id"])
            .fetch_scalar(&pool)
            .await?;
        let _: Option<i64> = QueryBuilder::<MySql>::table("users")
            .select(["id"])
            .fetch_optional_scalar(&pool)
            .await?;
        Ok(())
    }

    #[test]
    fn mysql_typechecks_compile() {
        // Presence proves `_typechecks` compiled for MySql.
    }
}

#[cfg(feature = "sqlx_sqlite")]
#[allow(dead_code)]
mod sqlite {
    use chain_builder::{QueryBuilder, Sqlite};

    #[derive(sqlx::FromRow)]
    struct UserRow {
        id: i64,
        name: String,
    }

    async fn _typechecks(pool: sqlx::SqlitePool) -> Result<(), sqlx::Error> {
        let qb = QueryBuilder::<Sqlite>::table("users")
            .select(["id", "name"])
            .where_gt("id", 0i64);
        let rows: Vec<UserRow> = qb.fetch_all(&pool).await?;
        let _ = rows;
        let one: UserRow = qb.fetch_one(&pool).await?;
        let _ = (one.id, one.name);
        let _opt: Option<UserRow> = qb.fetch_optional(&pool).await?;
        let _ = qb.execute(&pool).await?;
        let _: i64 = qb.count(&pool).await?;
        let _: i64 = QueryBuilder::<Sqlite>::table("users")
            .select(["id"])
            .fetch_scalar(&pool)
            .await?;
        let _: Option<i64> = QueryBuilder::<Sqlite>::table("users")
            .select(["id"])
            .fetch_optional_scalar(&pool)
            .await?;
        Ok(())
    }

    #[test]
    fn sqlite_typechecks_compile() {
        // Presence proves `_typechecks` compiled for Sqlite.
    }
}
