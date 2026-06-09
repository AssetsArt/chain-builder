//! sqlx handoff for v2 (`feature = "sqlx_*"`).
//!
//! [`SqlxDialect`] is a sealed sub-trait of [`Dialect`] that carries the sqlx
//! `Database` for a dialect and knows how to turn the builder's dialect-agnostic
//! [`Value`] binds into that database's owned `Arguments`. With it, any
//! [`QueryBuilder<D>`] whose `D: SqlxDialect` can produce a ready-to-execute
//! `sqlx::query::Query` / `QueryAs` via [`to_sqlx_query`](QueryBuilder::to_sqlx_query)
//! and [`to_sqlx_query_as`](QueryBuilder::to_sqlx_query_as).
//!
//! These mirror the 1.x `value_to_arguments` / `to_sqlx_query` integration in
//! `src/sqlx_mysql.rs` and `src/sqlx_sqlite.rs`.

use crate::builder::QueryBuilder;
use crate::dialect::Dialect;
use crate::value::Value;

#[cfg(feature = "sqlx_mysql")]
use crate::dialect::MySql;
#[cfg(feature = "sqlx_postgres")]
use crate::dialect::Postgres;
#[cfg(feature = "sqlx_sqlite")]
use crate::dialect::Sqlite;

/// Sealed sub-trait carrying the sqlx binding for a dialect.
///
/// Sealed via the private [`private::Sealed`] supertrait so only the dialect
/// markers in this crate can implement it.
pub trait SqlxDialect: Dialect + private::Sealed {
    /// The sqlx `Database` this dialect binds against.
    ///
    /// The database's owned `Arguments` must be `IntoArguments<Self::Database>`
    /// (it always is for sqlx's built-in databases) so the produced query type
    /// is executable.
    type Database: sqlx::Database<Arguments: sqlx::IntoArguments<Self::Database>>;

    /// Build owned `Arguments` for `binds`.
    ///
    /// Mirrors 1.x `value_to_arguments`: each [`Value`] is appended with
    /// `arguments.add(..)`. The sqlx encoders for this M1 [`Value`] set are
    /// infallible, so the `add` result is discarded with `let _ =`.
    fn bind_arguments(binds: &[Value]) -> <Self::Database as sqlx::Database>::Arguments;
}

mod private {
    /// Sealing marker — implemented only for this crate's dialect markers.
    pub trait Sealed {}

    #[cfg(feature = "sqlx_postgres")]
    impl Sealed for crate::dialect::Postgres {}
    #[cfg(feature = "sqlx_mysql")]
    impl Sealed for crate::dialect::MySql {}
    #[cfg(feature = "sqlx_sqlite")]
    impl Sealed for crate::dialect::Sqlite {}
}

#[cfg(feature = "sqlx_postgres")]
impl SqlxDialect for Postgres {
    type Database = sqlx::Postgres;

    fn bind_arguments(binds: &[Value]) -> sqlx::postgres::PgArguments {
        use sqlx::Arguments;
        let mut arguments = sqlx::postgres::PgArguments::default();
        for bind in binds {
            // These encoders are infallible for the M1 `Value` set.
            match bind {
                Value::Null => {
                    let _ = arguments.add(Option::<String>::None);
                }
                Value::Bool(b) => {
                    let _ = arguments.add(*b);
                }
                Value::I64(i) => {
                    let _ = arguments.add(*i);
                }
                Value::F64(f) => {
                    let _ = arguments.add(*f);
                }
                Value::Text(s) => {
                    let _ = arguments.add(s.clone());
                }
                Value::Bytes(b) => {
                    let _ = arguments.add(b.clone());
                }
                #[cfg(feature = "json")]
                Value::Json(j) => {
                    // Match 1.x: store JSON as text.
                    let _ = arguments.add(serde_json::to_string(j).unwrap_or_default());
                }
                #[cfg(feature = "uuid")]
                Value::Uuid(u) => {
                    let _ = arguments.add(*u);
                }
                #[cfg(feature = "chrono")]
                Value::DateTimeUtc(dt) => {
                    let _ = arguments.add(*dt);
                }
                #[cfg(feature = "chrono")]
                Value::NaiveDateTime(dt) => {
                    let _ = arguments.add(*dt);
                }
                #[cfg(feature = "chrono")]
                Value::NaiveDate(d) => {
                    let _ = arguments.add(*d);
                }
                #[cfg(feature = "chrono")]
                Value::NaiveTime(t) => {
                    let _ = arguments.add(*t);
                }
                #[cfg(feature = "decimal")]
                Value::Decimal(d) => {
                    // Native NUMERIC on Postgres.
                    let _ = arguments.add(*d);
                }
            }
        }
        arguments
    }
}

#[cfg(feature = "sqlx_mysql")]
impl SqlxDialect for MySql {
    type Database = sqlx::MySql;

    fn bind_arguments(binds: &[Value]) -> sqlx::mysql::MySqlArguments {
        use sqlx::Arguments;
        let mut arguments = sqlx::mysql::MySqlArguments::default();
        for bind in binds {
            // These encoders are infallible for the M1 `Value` set.
            match bind {
                Value::Null => {
                    let _ = arguments.add(Option::<String>::None);
                }
                Value::Bool(b) => {
                    let _ = arguments.add(*b);
                }
                Value::I64(i) => {
                    let _ = arguments.add(*i);
                }
                Value::F64(f) => {
                    let _ = arguments.add(*f);
                }
                Value::Text(s) => {
                    let _ = arguments.add(s.clone());
                }
                Value::Bytes(b) => {
                    let _ = arguments.add(b.clone());
                }
                #[cfg(feature = "json")]
                Value::Json(j) => {
                    // Match 1.x: store JSON as text.
                    let _ = arguments.add(serde_json::to_string(j).unwrap_or_default());
                }
                #[cfg(feature = "uuid")]
                Value::Uuid(u) => {
                    let _ = arguments.add(*u);
                }
                #[cfg(feature = "chrono")]
                Value::DateTimeUtc(dt) => {
                    let _ = arguments.add(*dt);
                }
                #[cfg(feature = "chrono")]
                Value::NaiveDateTime(dt) => {
                    let _ = arguments.add(*dt);
                }
                #[cfg(feature = "chrono")]
                Value::NaiveDate(d) => {
                    let _ = arguments.add(*d);
                }
                #[cfg(feature = "chrono")]
                Value::NaiveTime(t) => {
                    let _ = arguments.add(*t);
                }
                #[cfg(feature = "decimal")]
                Value::Decimal(d) => {
                    // Native DECIMAL on MySQL.
                    let _ = arguments.add(*d);
                }
            }
        }
        arguments
    }
}

#[cfg(feature = "sqlx_sqlite")]
impl SqlxDialect for Sqlite {
    type Database = sqlx::Sqlite;

    fn bind_arguments(binds: &[Value]) -> sqlx::sqlite::SqliteArguments {
        use sqlx::Arguments;
        // sqlx 0.9 dropped the lifetime param from `SqliteArguments`; this is an
        // owned set, matching 1.x `src/sqlx_sqlite.rs`.
        let mut arguments = sqlx::sqlite::SqliteArguments::default();
        for bind in binds {
            // These encoders are infallible for the M1 `Value` set.
            match bind {
                Value::Null => {
                    let _ = arguments.add(Option::<String>::None);
                }
                Value::Bool(b) => {
                    let _ = arguments.add(*b);
                }
                Value::I64(i) => {
                    let _ = arguments.add(*i);
                }
                Value::F64(f) => {
                    let _ = arguments.add(*f);
                }
                Value::Text(s) => {
                    let _ = arguments.add(s.clone());
                }
                Value::Bytes(b) => {
                    let _ = arguments.add(b.clone());
                }
                #[cfg(feature = "json")]
                Value::Json(j) => {
                    // Match 1.x: store JSON as text.
                    let _ = arguments.add(serde_json::to_string(j).unwrap_or_default());
                }
                #[cfg(feature = "uuid")]
                Value::Uuid(u) => {
                    let _ = arguments.add(*u);
                }
                #[cfg(feature = "chrono")]
                Value::DateTimeUtc(dt) => {
                    let _ = arguments.add(*dt);
                }
                #[cfg(feature = "chrono")]
                Value::NaiveDateTime(dt) => {
                    let _ = arguments.add(*dt);
                }
                #[cfg(feature = "chrono")]
                Value::NaiveDate(d) => {
                    let _ = arguments.add(*d);
                }
                #[cfg(feature = "chrono")]
                Value::NaiveTime(t) => {
                    let _ = arguments.add(*t);
                }
                #[cfg(feature = "decimal")]
                Value::Decimal(d) => {
                    // SQLite has no native decimal type and sqlx provides no
                    // `Decimal` encoder for it, so bind the exact value as TEXT.
                    let _ = arguments.add(d.to_string());
                }
            }
        }
        arguments
    }
}

#[cfg(any(
    feature = "sqlx_postgres",
    feature = "sqlx_mysql",
    feature = "sqlx_sqlite"
))]
impl<D: SqlxDialect> QueryBuilder<D> {
    /// Build an executable `sqlx::query::Query` from this builder.
    ///
    /// The SQL is builder-generated with bound placeholders, so it is asserted
    /// safe via `sqlx::AssertSqlSafe` (which also satisfies sqlx 0.9's `'static`
    /// SQL bound). Mirrors 1.x `ChainBuilder::to_sqlx_query`.
    pub fn to_sqlx_query(
        &self,
    ) -> sqlx::query::Query<'_, D::Database, <D::Database as sqlx::Database>::Arguments> {
        let (sql, binds) = self.to_sql();
        sqlx::query_with(sqlx::AssertSqlSafe(sql), D::bind_arguments(&binds))
    }

    /// Build an executable `sqlx::query::QueryAs` decoding rows into `T`.
    pub fn to_sqlx_query_as<T>(
        &self,
    ) -> sqlx::query::QueryAs<'_, D::Database, T, <D::Database as sqlx::Database>::Arguments>
    where
        T: for<'r> sqlx::FromRow<'r, <D::Database as sqlx::Database>::Row>,
    {
        let (sql, binds) = self.to_sql();
        sqlx::query_as_with(sqlx::AssertSqlSafe(sql), D::bind_arguments(&binds))
    }
}
