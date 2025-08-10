# Chain Builder

A flexible and easy-to-use query builder for MySQL in Rust. This library provides a fluent interface for building SQL queries with support for complex operations like JOINs, CTEs, and subqueries.

## Features

- **Fluent API**: Chain methods for intuitive query building
- **Type Safety**: Compile-time safety with Rust's type system
- **Complex Queries**: Support for JOINs, CTEs, UNIONs, and subqueries
- **Raw SQL**: Fallback to raw SQL when needed
- **Multiple Operations**: SELECT, INSERT, UPDATE, DELETE
- **sqlx Integration**: Direct integration with sqlx for async database operations

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
chain-builder = "0.1.25"
serde_json = "1.0"
```

For sqlx integration:

```toml
[dependencies]
chain-builder = { version = "0.1.25", features = ["sqlx_mysql"] }
sqlx = { version = "0.8", features = ["mysql", "runtime-tokio-rustls"] }
```

## Quick Start

```rust
use chain_builder::{ChainBuilder, Client, Select};
use serde_json::Value;

// Create a new query builder
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

## Examples

### Basic SELECT Query

```rust
use chain_builder::{ChainBuilder, Client, Select};
use serde_json::Value;

let mut builder = ChainBuilder::new(Client::Mysql);
builder
    .db("mydb")
    .select(Select::Columns(vec!["id".into(), "name".into(), "email".into()]))
    .table("users")
    .query(|qb| {
        qb.where_eq("status", Value::String("active".to_string()));
        qb.where_gt("age", Value::Number(18.into()));
        qb.limit(10);
        qb.offset(5);
        qb.order_by("name", "ASC");
    });

let (sql, binds) = builder.to_sql();
```

### JOIN Queries

```rust
let mut builder = ChainBuilder::new(Client::Mysql);
builder
    .db("mydb")
    .select(Select::Columns(vec!["users.name".into(), "profiles.bio".into()]))
    .table("users")
    .query(|qb| {
        qb.join("profiles", |join| {
            join.on("users.id", "=", "profiles.user_id");
        });
        qb.where_eq("users.status", Value::String("active".to_string()));
    });
```

### Complex WHERE Clauses

```rust
builder.query(|qb| {
    qb.where_eq("status", Value::String("active".to_string()));
    qb.where_in("department", vec![
        Value::String("IT".to_string()),
        Value::String("HR".to_string()),
    ]);
    
    // Subquery
    qb.where_subquery(|sub| {
        sub.where_eq("status", Value::String("pending".to_string()));
        sub.or()
            .where_eq("status", Value::String("approved".to_string()))
            .where_between(
                "created_at",
                [
                    Value::String("2024-01-01".to_string()),
                    Value::String("2024-01-31".to_string()),
                ],
            );
    });
    
    // Raw SQL
    qb.where_raw(
        "(latitude BETWEEN ? AND ?) AND (longitude BETWEEN ? AND ?)",
        Some(vec![
            Value::Number(40.0.into()),
            Value::Number(41.0.into()),
            Value::Number(70.0.into()),
            Value::Number(71.0.into()),
        ]),
    );
});
```

### INSERT Operations

```rust
let mut builder = ChainBuilder::new(Client::Mysql);
builder
    .db("mydb")
    .table("users")
    .insert(serde_json::json!({
        "name": "John Doe",
        "email": "john@example.com",
        "age": 30,
        "status": "active"
    }));

let (sql, binds) = builder.to_sql();
```

### UPDATE Operations

```rust
let mut builder = ChainBuilder::new(Client::Mysql);
builder
    .db("mydb")
    .table("users")
    .update(serde_json::json!({
        "status": "inactive",
        "updated_at": "2024-01-15"
    }))
    .query(|qb| {
        qb.where_eq("id", Value::Number(1.into()));
    });
```

### DELETE Operations

```rust
let mut builder = ChainBuilder::new(Client::Mysql);
builder
    .db("mydb")
    .table("users")
    .delete()
    .query(|qb| {
        qb.where_eq("status", Value::String("inactive".to_string()));
    });
```

### WITH Clauses (CTEs)

```rust
// Create a CTE for active users
let mut active_users = ChainBuilder::new(Client::Mysql);
active_users
    .db("mydb")
    .table("users")
    .select(Select::Columns(vec!["*".into()]))
    .query(|qb| {
        qb.where_eq("status", Value::String("active".to_string()));
    });

// Use the CTE in main query
let mut builder = ChainBuilder::new(Client::Mysql);
builder
    .with("active_users", active_users)
    .select(Select::Columns(vec!["*".into()]))
    .table("active_users")
    .query(|qb| {
        qb.where_gt("age", Value::Number(25.into()));
    });
```

### UNION Queries

```rust
let mut pending_users = ChainBuilder::new(Client::Mysql);
pending_users
    .db("mydb")
    .table("users")
    .select(Select::Columns(vec!["*".into()]))
    .query(|qb| {
        qb.where_eq("status", Value::String("pending".to_string()));
    });

let mut builder = ChainBuilder::new(Client::Mysql);
builder
    .union(pending_users)
    .db("mydb")
    .select(Select::Columns(vec!["*".into()]))
    .table("users")
    .query(|qb| {
        qb.where_eq("status", Value::String("active".to_string()));
    });
```

## sqlx Integration

```rust
use chain_builder::{ChainBuilder, Client, Select};
use sqlx::mysql::MySqlPool;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let pool = MySqlPool::connect("mysql://user:pass@localhost/db").await?;
    
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .db("mydb")
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.where_eq("status", Value::String("active".to_string()));
        });
    
    // Convert to sqlx query
    let query = builder.to_sqlx_query();
    
    // Execute
    let rows = query.fetch_all(&pool).await?;
    
    Ok(())
}
```

## API Reference

### ChainBuilder

The main query builder class.

#### Methods

- `new(client: Client)` - Create a new builder
- `db(name: &str)` - Set database name
- `table(name: &str)` - Set table name
- `select(select: Select)` - Add SELECT clause
- `insert(data: Value)` - Set INSERT data
- `update(data: Value)` - Set UPDATE data
- `delete()` - Set DELETE operation
- `query(closure)` - Configure WHERE, JOIN, etc.
- `to_sql()` - Generate SQL string and bind parameters

### QueryBuilder

Used for WHERE clauses and other query parts.

#### WHERE Methods

- `where_eq(column, value)` - Equal condition
- `where_ne(column, value)` - Not equal condition
- `where_in(column, values)` - IN condition
- `where_gt(column, value)` - Greater than
- `where_lt(column, value)` - Less than
- `where_between(column, [min, max])` - BETWEEN condition
- `where_like(column, pattern)` - LIKE condition
- `where_null(column)` - IS NULL
- `where_subquery(closure)` - Subquery condition
- `or()` - Start OR chain
- `where_raw(sql, binds)` - Raw SQL condition

#### JOIN Methods

- `join(table, closure)` - INNER JOIN
- `left_join(table, closure)` - LEFT JOIN
- `right_join(table, closure)` - RIGHT JOIN
- `raw_join(sql, binds)` - Raw JOIN

#### Other Methods

- `limit(n)` - LIMIT clause
- `offset(n)` - OFFSET clause
- `order_by(column, direction)` - ORDER BY
- `group_by(columns)` - GROUP BY
- `with(alias, builder)` - WITH clause
- `union(builder)` - UNION clause

## License

MIT License - see LICENSE file for details.