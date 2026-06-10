# Getting Started

chain-builder is published on crates.io as
[`chain-builder`](https://crates.io/crates/chain-builder); the current version
is **3.0.0**. The builder itself is driver-agnostic — the `sqlx_*` feature
flags only control which sqlx driver the execution layer compiles against.

## Installation

Pick the line that matches your database:

| Backend | `Cargo.toml` |
|---|---|
| MySQL *(default)* | `chain-builder = "3.0.0"` |
| PostgreSQL | `chain-builder = { version = "3.0.0", default-features = false, features = ["sqlx_postgres"] }` |
| SQLite | `chain-builder = { version = "3.0.0", default-features = false, features = ["sqlx_sqlite"] }` |

> **⚠️ The `default = ["sqlx_mysql"]` gotcha**
>
> The crate's default feature set is `["sqlx_mysql"]`. If you target Postgres
> or SQLite and write
> `chain-builder = { version = "3.0.0", features = ["sqlx_postgres"] }`
> **without** `default-features = false`, your build silently compiles the
> MySQL driver *in addition to* the one you asked for. Everything still
> compiles, so the mistake is easy to miss — always add
> `default-features = false` when you are not on MySQL.

The driver features (`sqlx_mysql`, `sqlx_postgres`, `sqlx_sqlite`) may be
enabled in any combination if you talk to more than one database.

### Optional value features

These features extend the set of Rust types you can pass as bind values:

| Feature | Enables binding | Pulls in |
|---|---|---|
| `json` | `serde_json::Value` | `serde_json` |
| `uuid` | `uuid::Uuid` | `uuid`, `sqlx/uuid` |
| `chrono` | `chrono` date/time types | `chrono`, `sqlx/chrono` |
| `decimal` | `rust_decimal::Decimal` | `rust_decimal`, `sqlx/rust_decimal` |

See [Binds & Values](binds.md) for the full type-mapping table.

## Your first query

A builder is created with `QueryBuilder::<Dialect>::table(...)`, configured by
chaining methods, and turned into SQL with `to_sql()`:

```rust,ignore
use chain_builder::{QueryBuilder, Postgres};

let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["id", "name"])
    .where_eq("status", "active")
    .to_sql();

assert_eq!(sql, r#"SELECT "id", "name" FROM "users" WHERE "status" = $1"#);
// binds == [Value::Text("active")]
```

Step by step:

1. **`QueryBuilder::<Postgres>::table("users")`** — picks the dialect at the
   type level and names the table. The dialect decides quoting (`"users"`) and
   placeholder style (`$1`).
2. **`.select(["id", "name"])` / `.where_eq("status", "active")`** — chain as
   many clauses as you need; every method returns the builder. The value
   `"active"` is *not* spliced into the string — it is recorded as a typed
   bind.
3. **`.to_sql()`** — compiles the chain into a `(String, Vec<Value>)` pair:
   the SQL text with placeholders, and the bind values in placeholder order.
   Hand both to your database layer, or skip straight to the built-in
   [sqlx integration](sqlx.md).

## `to_sql()` vs `try_to_sql()`

A handful of builder combinations cannot be compiled — for example
`distinct_on` on a non-Postgres dialect, `offset` without `limit`, or a
`having()` operator outside the allowlist. `to_sql()` **panics** on these
misuse errors (they are programmer bugs, and the panicking form keeps simple
code simple), while `try_to_sql()` returns
`Result<(String, Vec<Value>), BuildError>` so you can surface the problem
gracefully — the right choice when any part of the query is shaped by runtime
input. Every panicking method has a `try_` twin; the variants and the policy
behind them are covered in [Error Handling](error-handling.md).

## Executing with sqlx

You rarely need to touch the `(sql, binds)` pair yourself: with the matching
`sqlx_*` feature enabled, the builder hands off directly to
[sqlx](https://github.com/launchbadge/sqlx):

```rust,no_run
# async fn demo(pool: sqlx::PgPool) -> Result<(), chain_builder::Error> {
use chain_builder::{QueryBuilder, Postgres};

#[derive(sqlx::FromRow)]
struct User { id: i64, name: String }

let users: Vec<User> = QueryBuilder::<Postgres>::table("users")
    .select(["id", "name"])
    .where_eq("status", "active")
    .fetch_all(&pool)
    .await?;
# Ok(()) }
```

`fetch_all`, `fetch_one`, `fetch_optional`, `execute`, `count`, scalar
fetches, and the lower-level `to_sqlx_query` family are all documented in
[Executing with sqlx](sqlx.md).
