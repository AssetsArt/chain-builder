//! Chain Builder - A flexible and easy-to-use query builder for MySQL in Rust
//! 
//! This library provides a fluent interface for building SQL queries with support for:
//! - SELECT, INSERT, UPDATE, DELETE operations
//! - Complex WHERE clauses with subqueries
//! - JOIN operations
//! - WITH clauses (CTEs)
//! - UNION operations
//! - Raw SQL support
//! 
//! # Example
//! ```rust
//! use chain_builder::{ChainBuilder, Client, Select, WhereClauses};
//! use serde_json::Value;
//! 
//! let mut builder = ChainBuilder::new(Client::Mysql);
//! builder
//!     .db("mydb")
//!     .select(Select::Columns(vec!["*".into()]))
//!     .table("users")
//!     .query(|qb| {
//!         qb.where_eq("name", Value::String("John".to_string()));
//!         qb.where_eq("status", Value::String("active".to_string()));
//!     });
//! 
//! let (sql, binds) = builder.to_sql();
//! ```

// Core modules
mod types;
mod builder;
mod query;

// Database-specific modules
#[cfg(feature = "mysql")]
mod mysql;
#[cfg(all(feature = "mysql", feature = "sqlx_mysql"))]
mod sqlx_mysql;
#[cfg(feature = "sqlite")]
mod sqlite;
#[cfg(all(feature = "sqlite", feature = "sqlx_sqlite"))]
mod sqlx_sqlite;

// Re-export main types
pub use types::{Client, Method, Statement, Select, Common};
pub use builder::ChainBuilder;
pub use query::{QueryBuilder, Operator};

// Re-export database-specific types
#[cfg(feature = "mysql")]
pub use mysql::ToSql;

// Re-export join functionality
pub use query::join::{JoinBuilder, JoinMethods};

// Re-export query builder functionality
pub use query::common::{QueryCommon, WhereClauses, HavingClauses};
