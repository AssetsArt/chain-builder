# chain-builder — Guide (1.x API)

Detailed usage for the stable **1.x** API. See the [README](../README.md) for installation and a quick start, and [v2.md](v2.md) for the typed, dialect-generic v2 preview.

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

### Advanced WHERE Clauses

```rust
builder.query(|qb| {
    qb.where_eq("status", Value::String("active".to_string()));
    qb.where_in("department", vec![
        Value::String("IT".to_string()),
        Value::String("HR".to_string()),
    ]);
    
    // Case-insensitive LIKE
    qb.where_ilike("name", Value::String("john".to_string()));
    
    // Column-to-column comparison
    qb.where_column("users.age", ">", "profiles.min_age");
    
    // EXISTS subquery
    qb.where_exists(|sub| {
        sub.db("mydb")
            .table("orders")
            .select(Select::Columns(vec!["id".into()]))
            .query(|sub_qb| {
                sub_qb.where_column("orders.user_id", "=", "users.id");
                sub_qb.where_eq("status", Value::String("completed".to_string()));
            });
    });
    
    // JSON contains (MySQL only)
    qb.where_json_contains("metadata", Value::String("premium".to_string()));
    
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

### Advanced JOINs

```rust
builder.query(|qb| {
    qb.left_join("profiles", |join| {
        join.on("users.id", "=", "profiles.user_id");
    });
    
    qb.inner_join("departments", |join| {
        join.on("users.department_id", "=", "departments.id");
        join.or()
            .on("users.role", "=", "departments.manager_role");
    });
    
    qb.full_outer_join("orders", |join| {
        join.on("users.id", "=", "orders.user_id");
    });
    
    qb.cross_join("roles", |join| {
        join.on("users.role_id", "=", "roles.id");
    });
    
    qb.join_using("permissions", vec!["user_id".to_string()]);
});
```

### Aggregate Functions and HAVING

```rust
let mut builder = ChainBuilder::new(Client::Mysql);
builder
    .db("mydb")
    .table("orders")
    .query(|qb| {
        qb.group_by(vec!["user_id".to_string()]);
        qb.having("COUNT(*)", ">", Value::Number(5.into()));
        qb.having_between("SUM(amount)", [
            Value::Number(100.into()),
            Value::Number(1000.into())
        ]);
    });

// Add aggregate functions
builder
    .select_count("id")
    .select_sum("amount")
    .select_avg("amount")
    .select_max("created_at")
    .select_min("created_at")
    .select_alias("user_id", "uid")
    .select_raw("CONCAT(first_name, ' ', last_name) AS full_name", None);
```

## sqlx Integration

### MySQL with sqlx

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

### SQLite with sqlx

```rust
use chain_builder::{ChainBuilder, Client, Select};
use sqlx::sqlite::SqlitePool;

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let pool = SqlitePool::connect("sqlite://path/to/database.db").await?;
    
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.where_eq("status", Value::String("active".to_string()));
        });
    
    // Convert to sqlx query (available with sqlx_sqlite feature)
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
- `new_mysql()` - Create a new MySQL builder
- `new_sqlite()` - Create a new SQLite builder
- `db(name: &str)` - Set database name
- `table(name: &str)` - Set table name
- `select(select: Select)` - Add SELECT clause
- `insert(data: Value)` - Set INSERT data
- `update(data: Value)` - Set UPDATE data
- `delete()` - Set DELETE operation
- `query(closure)` - Configure WHERE, JOIN, etc.
- `to_sql()` - Generate SQL string and bind parameters

#### SELECT Methods

- `select(select: Select)` - Basic SELECT
- `select_raw(sql, binds)` - Raw SELECT expression
- `select_distinct(columns)` - DISTINCT SELECT
- `select_count(column)` - COUNT aggregate
- `select_sum(column)` - SUM aggregate
- `select_avg(column)` - AVG aggregate
- `select_max(column)` - MAX aggregate
- `select_min(column)` - MIN aggregate
- `select_alias(column, alias)` - SELECT with alias

#### sqlx Integration Methods (Conditional)

- `to_sqlx_query()` - Convert to sqlx query (requires sqlx_mysql or sqlx_sqlite feature)
- `to_sqlx_query_as<T>()` - Convert to typed sqlx query (requires sqlx_mysql or sqlx_sqlite feature)
- `count(column, pool)` - Count rows (MySQL only, requires sqlx_mysql feature)

### QueryBuilder

Used for WHERE clauses and other query parts.

#### WHERE Methods

- `where_eq(column, value)` - Equal condition
- `where_ne(column, value)` - Not equal condition
- `where_in(column, values)` - IN condition
- `where_not_in(column, values)` - NOT IN condition
- `where_gt(column, value)` - Greater than
- `where_gte(column, value)` - Greater than or equal
- `where_lt(column, value)` - Less than
- `where_lte(column, value)` - Less than or equal
- `where_between(column, [min, max])` - BETWEEN condition
- `where_not_between(column, [min, max])` - NOT BETWEEN condition
- `where_like(column, pattern)` - LIKE condition
- `where_not_like(column, pattern)` - NOT LIKE condition
- `where_ilike(column, pattern)` - Case-insensitive LIKE
- `where_null(column)` - IS NULL
- `where_not_null(column)` - IS NOT NULL
- `where_exists(closure)` - EXISTS subquery
- `where_not_exists(closure)` - NOT EXISTS subquery
- `where_column(lhs, op, rhs)` - Column-to-column comparison
- `where_json_contains(column, value)` - JSON contains (MySQL)
- `where_subquery(closure)` - Subquery condition
- `or()` - Start OR chain
- `where_raw(sql, binds)` - Raw SQL condition

#### HAVING Methods

- `having(column, operator, value)` - HAVING condition
- `having_between(column, [min, max])` - HAVING BETWEEN
- `having_in(column, values)` - HAVING IN
- `having_not_in(column, values)` - HAVING NOT IN
- `having_raw(sql, binds)` - Raw HAVING SQL

#### JOIN Methods

- `join(table, closure)` - INNER JOIN
- `left_join(table, closure)` - LEFT JOIN
- `right_join(table, closure)` - RIGHT JOIN
- `left_outer_join(table, closure)` - LEFT OUTER JOIN
- `right_outer_join(table, closure)` - RIGHT OUTER JOIN
- `full_outer_join(table, closure)` - FULL OUTER JOIN
- `cross_join(table, closure)` - CROSS JOIN
- `join_using(table, columns)` - JOIN USING

#### Other Methods

- `limit(n)` - LIMIT clause
- `offset(n)` - OFFSET clause
- `order_by(column, direction)` - ORDER BY
- `group_by(columns)` - GROUP BY
- `with(alias, builder)` - WITH clause
- `union(builder)` - UNION clause

## Architecture

The library is organized into several modules:

- **`src/types.rs`** - Core types and enums
- **`src/builder.rs`** - Main ChainBuilder implementation
- **`src/query/`** - Query building functionality
  - **`src/query/common.rs`** - Common query operations (WHERE, HAVING, etc.)
  - **`src/query/join/`** - JOIN functionality
- **`src/common/`** - Shared compilation logic
- **`src/mysql/`** - MySQL-specific compilation
- **`src/sqlite/`** - SQLite-specific compilation
- **`src/sqlx_mysql.rs`** - MySQL sqlx integration (conditional compilation)
- **`src/sqlx_sqlite.rs`** - SQLite sqlx integration (conditional compilation)
- **`src/dialect.rs`** - Dialect-aware identifier escaping

