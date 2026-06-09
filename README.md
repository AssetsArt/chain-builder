# Chain Builder

[![Documentation](https://img.shields.io/badge/docs.rs-chain--builder-66c2a5?style=for-the-badge&labelColor=555555&logoColor=white)](https://docs.rs/chain-builder)
[![Version](https://img.shields.io/crates/v/chain-builder?style=for-the-badge)](https://crates.io/crates/chain-builder)
[![License](https://img.shields.io/crates/l/chain-builder?style=for-the-badge)](https://crates.io/crates/chain-builder)

A flexible and easy-to-use query builder for MySQL and SQLite in Rust. This library provides a fluent interface for building SQL queries with support for complex operations like JOINs, CTEs, and subqueries.

## Features

- **Fluent API**: Chain methods for intuitive query building
- **Type Safety**: Compile-time safety with Rust's type system
- **Multi-Database Support**: MySQL and SQLite with dedicated compilers
- **Complex Queries**: Support for JOINs, CTEs, UNIONs, and subqueries
- **Advanced WHERE Clauses**: EXISTS, NOT EXISTS, ILIKE, column comparisons, JSON operations
- **HAVING Clauses**: Support for aggregate function filtering
- **Aggregate Functions**: COUNT, SUM, AVG, MAX, MIN with aliases
- **Advanced JOINs**: FULL OUTER JOIN, CROSS JOIN, JOIN USING
- **Raw SQL**: Fallback to raw SQL when needed
- **Multiple Operations**: SELECT, INSERT, UPDATE, DELETE
- **Injection-safe identifiers**: Table/column/alias names are dialect-escaped automatically (see [Security](#security))
- **sqlx Integration**: Direct integration with sqlx for async database operations
- **Modern Architecture**: Clean, modular codebase with better maintainability

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
chain-builder = "1.0.2"
serde_json = "1.0"
```

For MySQL with sqlx integration:

```toml
[dependencies]
chain-builder = { version = "1.0.2", features = ["sqlx_mysql"] }
sqlx = { version = "0.9", features = ["mysql", "runtime-tokio-rustls"] }
```

For SQLite with sqlx integration:

```toml
[dependencies]
chain-builder = { version = "1.0.2", features = ["sqlx_sqlite"] }
sqlx = { version = "0.9", features = ["sqlite", "runtime-tokio-rustls"] }
```

For both MySQL and SQLite with sqlx integration:

```toml
[dependencies]
chain-builder = { version = "1.0.2", features = ["sqlx_mysql", "sqlx_sqlite"] }
sqlx = { version = "0.9", features = ["mysql", "sqlite", "runtime-tokio-rustls"] }
```

## Quick Start

### MySQL Example

```rust
use chain_builder::{ChainBuilder, Client, Select};
use serde_json::Value;

// Create a new query builder for MySQL
let mut builder = ChainBuilder::new(Client::Mysql);

// Build a simple SELECT query
builder
    .db("mydb")
    .select(Select::Columns(vec!["*".into()]))
    .table("users")
    .query(|qb| {
        qb.where_eq("name", Value::String("John".to_string()));
        qb.where_eq("status", Value::String("active".to_string()));
    });

// Generate SQL
let (sql, binds) = builder.to_sql();
println!("SQL: {}", sql);
println!("Binds: {:?}", binds);
```

### SQLite Example

```rust
use chain_builder::{ChainBuilder, Client, Select};
use serde_json::Value;

// Create a new query builder for SQLite
let mut builder = ChainBuilder::new(Client::Sqlite);

// Build a simple SELECT query
builder
    .select(Select::Columns(vec!["*".into()]))
    .table("users")
    .query(|qb| {
        qb.where_eq("name", Value::String("John".to_string()));
        qb.where_eq("status", Value::String("active".to_string()));
    });

// Generate SQL
let (sql, binds) = builder.to_sql();
println!("SQL: {}", sql);
println!("Binds: {:?}", binds);
```

## Documentation

- **[Guide](docs/guide.md)** — full examples (SELECT/INSERT/UPDATE/DELETE, JOINs, CTEs, UNION, aggregates & HAVING), sqlx integration, API reference, and architecture for the stable **1.x** API.
- **[v2 preview](docs/v2.md)** — the typed, dialect-generic builder (`feature = "v2"`): typed binds, `db()` multi-tenant, upsert + RETURNING, typed fetch, and more.

## Security

Chain Builder is designed to be safe against SQL injection on **two** axes:

- **Values** are always sent as bound parameters (`?`), never inlined into SQL.
- **Identifiers** (table, column, and alias names) passed to the structured API
  are automatically escaped for the active dialect — backticks for MySQL,
  double quotes for SQLite/PostgreSQL — with any embedded quote character doubled.
  Qualified names like `users.id` are escaped segment-by-segment
  (`` `users`.`id` ``) and a `*` segment is preserved (`` `users`.* ``).

This means you can safely pass an untrusted column name (e.g. a dynamic
`ORDER BY` coming from a request) without it being able to break out of the
identifier context:

```rust
// "name`; DROP TABLE users; --" becomes a single quoted identifier:
//   ... ORDER BY `name``; DROP TABLE users; --` ASC
qb.order_by(user_supplied_column, "ASC");
```

> **Pass bare identifiers.** Do **not** pre-quote names yourself (e.g. `` "`name`" ``);
> the builder quotes them for you, and a pre-quoted name would be double-escaped.

The `*_raw` methods (`select_raw`, `where_raw`, `group_by_raw`, `order_by_raw`,
`having_raw`, `add_raw`, `table_raw`, `raw_join`, `on_raw`) and the `having*`
helpers are **expression** escape hatches and are emitted verbatim — never pass
untrusted input through them.

## Feature Flags

The library uses feature flags to control functionality:

- **`mysql`** (default) - Enable MySQL support
- **`sqlite`** - Enable SQLite support
- **`sqlx_mysql`** (default) - Enable MySQL sqlx integration
- **`sqlx_sqlite`** - Enable SQLite sqlx integration
- **`sqlx_postgres`** - Enable PostgreSQL sqlx integration (v2)
- **`postgres`** - Enable PostgreSQL support (future, 1.x)
- **`v2`** - Enable the typed, dialect-generic v2 builder (preview)
- **`json`** - Enable `v2::Value::Json` (v2)

## License

