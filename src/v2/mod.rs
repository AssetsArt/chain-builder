//! chain-builder v2 — typed, dialect-aware query builder (feature = "v2").
pub mod value;
pub mod ident;
pub mod dialect;
pub mod builder;
pub mod where_;
pub mod compile;
#[cfg(any(feature = "sqlx_mysql", feature = "sqlx_sqlite", feature = "sqlx_postgres"))]
pub mod sqlx_bind;

pub use builder::{Method, QueryBuilder};
pub use compile::compile;
pub use dialect::{Dialect, MySql, Postgres, Sqlite};
pub use value::{IntoBind, Value};
pub use where_::{Conj, Predicate, WhereBuilder};
