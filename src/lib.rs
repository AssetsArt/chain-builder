//! # Chain Builder
//!
//! A typed, dialect-aware SQL query builder for Rust.
//!
//! Generic over a [`Dialect`] (PostgreSQL / MySQL / SQLite), with typed bind
//! parameters via [`IntoBind`], dialect-correct placeholders (`$N` for Postgres,
//! `?` for MySQL/SQLite), automatic identifier escaping, and an optional `sqlx`
//! handoff for execution.
//!
//! # Example
//! ```rust
//! use chain_builder::{QueryBuilder, Postgres, Order};
//!
//! let (sql, binds) = QueryBuilder::<Postgres>::table("users")
//!     .db("mydb")                       // multi-tenant: one connection, many DBs
//!     .select(["id", "name"])
//!     .where_eq("status", "active")
//!     .where_in("role", ["admin", "staff"])
//!     .order_by("name", Order::Desc)
//!     .paginate(2, 20)
//!     .to_sql();
//!
//! assert!(sql.starts_with(r#"SELECT "id", "name" FROM "mydb"."users""#));
//! assert_eq!(binds.len(), 5); // active, admin, staff, limit, offset
//! ```
//!
//! With a dialect's `sqlx_*` feature enabled, [`QueryBuilder::to_sqlx_query`] and
//! the `fetch_*` helpers turn a builder into a ready-to-execute `sqlx` query.

pub mod builder;
pub mod compile;
pub mod dialect;
pub mod error;
pub mod ident;
pub mod like;
pub mod value;
pub mod where_;

#[cfg(any(
    feature = "sqlx_mysql",
    feature = "sqlx_sqlite",
    feature = "sqlx_postgres"
))]
pub mod fetch;
#[cfg(any(
    feature = "sqlx_mysql",
    feature = "sqlx_sqlite",
    feature = "sqlx_postgres"
))]
pub mod sqlx_bind;

pub use builder::{
    AggFn, ConflictAction, Cte, Having, Join, JoinClause, JoinCond, JoinKind, Lock, LockStrength,
    LockWait, Method, OnConflict, Order, QueryBuilder, SelectExpr,
};
pub use compile::{compile, try_compile};
pub use dialect::{Dialect, MySql, Postgres, Sqlite, UpsertStyle};
pub use error::{BuildError, Error};
pub use like::escape_like;
pub use value::{IntoBind, Value};
pub use where_::{Conj, Predicate, WhereBuilder};

#[cfg(any(
    feature = "sqlx_mysql",
    feature = "sqlx_sqlite",
    feature = "sqlx_postgres"
))]
pub use sqlx_bind::{SqlxDialect, SqlxQuery, SqlxQueryAs};
