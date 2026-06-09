//! chain-builder v2 — typed, dialect-aware query builder (feature = "v2").
pub mod value;
pub mod ident;
pub mod dialect;
pub mod builder;
#[cfg(any(feature = "sqlx_mysql", feature = "sqlx_sqlite", feature = "sqlx_postgres"))]
pub mod sqlx_bind;

pub use builder::QueryBuilder;
pub use dialect::{Dialect, MySql, Postgres, Sqlite};
pub use value::{IntoBind, Value};
