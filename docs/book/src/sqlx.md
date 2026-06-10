# Executing with sqlx

Everything before this page stops at `(String, Vec<Value>)`. With a `sqlx_*`
feature enabled, the builder also hands off directly to
[sqlx](https://docs.rs/sqlx): `to_sqlx_query()` produces a ready-to-execute
`sqlx::query::Query`, and the async helpers (`fetch_all`, `execute`, `count`,
…) go all the way — compile, bind, execute, decode — returning the unified
[`chain_builder::Error`](error-handling.md) so an invalid builder is an `Err`,
never a panic. This page covers the feature flags, the handoff methods, every
execution helper, and a full round-trip example.

## Feature flags

One Cargo feature per backend; each enables the matching sqlx driver and the
`SqlxDialect` impl for that dialect marker:

| Feature | Dialect marker | sqlx database |
|---|---|---|
| `sqlx_postgres` | `Postgres` | `sqlx::Postgres` |
| `sqlx_mysql` *(default)* | `MySql` | `sqlx::MySql` |
| `sqlx_sqlite` | `Sqlite` | `sqlx::Sqlite` |

The flags are freely **combinable** — enable two or three to talk to multiple
databases from one binary. Note the default is `sqlx_mysql`: Postgres and
SQLite users should add `default-features = false` (the full gotcha is in
[Getting Started](getting-started.md)).

Without any `sqlx_*` feature the crate still builds — you just get the pure
builder (`to_sql`/`try_to_sql`) and none of the APIs on this page.

## The handoff: `to_sqlx_query` / `to_sqlx_query_as`

For full control over execution (streaming with `.fetch()`, custom
`Executor` plumbing), take the sqlx query object yourself:

- `to_sqlx_query()` → `sqlx::query::Query` — for statements you execute
  without decoding typed rows.
- `to_sqlx_query_as::<T>()` → `sqlx::query::QueryAs<…, T, …>` — decodes rows
  into any `T: sqlx::FromRow`.
- `try_to_sqlx_query()` / `try_to_sqlx_query_as::<T>()` — the fallible twins,
  returning `Result<_, BuildError>` instead of panicking on an invalid
  builder (see [Error Handling](error-handling.md)).

All four compile the builder, then call `sqlx::query_with` /
`sqlx::query_as_with` with the SQL and the translated arguments. The SQL is
passed through `sqlx::AssertSqlSafe` — sqlx 0.9 requires that assertion for
runtime-built SQL strings, and it is sound here because the SQL is entirely
**builder-generated with bound placeholders**: every user value travels in
the argument buffer, never in the string (see
[Binds & Values](binds.md) and the [Security Model](security.md)).

```rust,no_run
use chain_builder::{MySql, QueryBuilder};

#[derive(sqlx::FromRow)]
struct User { id: i64, name: String }

async fn load(pool: &sqlx::MySqlPool) -> Result<Vec<User>, chain_builder::Error> {
    let users = QueryBuilder::<MySql>::table("users")
        .select(["id", "name"])
        .where_eq("status", "active")
        .try_to_sqlx_query_as::<User>()?   // BuildError → Error via From
        .fetch_all(pool)
        .await?;                            // sqlx::Error → Error via From
    Ok(users)
}
```

### The `SqlxQuery` / `SqlxQueryAs` aliases

The concrete sqlx types are verbose, so the crate root re-exports two
aliases (alongside `SqlxDialect`):

- `SqlxQuery<'q, D>` — what `try_to_sqlx_query()` returns,
- `SqlxQueryAs<'q, D, T>` — what `try_to_sqlx_query_as::<T>()` returns.

Useful when a helper function passes the query object around:

```rust,ignore
use chain_builder::{QueryBuilder, SqlxDialect, SqlxQueryAs, BuildError};

fn prepared<D: SqlxDialect, T>(qb: &QueryBuilder<D>) -> Result<SqlxQueryAs<'_, D, T>, BuildError>
where
    T: for<'r> sqlx::FromRow<'r, <D::Database as sqlx::Database>::Row>,
{
    qb.try_to_sqlx_query_as::<T>()
}
```

## The execution helpers

For the common cases you can skip the handoff entirely — `QueryBuilder<D>`
(for any `D: SqlxDialect`) has async helpers that take any `sqlx::Executor`
(a pool, a connection, or `&mut *tx` inside a transaction). All of them
return `Result<_, chain_builder::Error>`: an invalid builder surfaces as
`Error::Build` **before touching the database**, a driver failure as
`Error::Sqlx`.

| Helper | Returns | Notes |
|---|---|---|
| `fetch_all::<T, _>(exec)` | `Vec<T>` | all rows, each decoded via `T: FromRow` |
| `fetch_one::<T, _>(exec)` | `T` | exactly one row; no row → `sqlx::Error::RowNotFound` as `Error::Sqlx` |
| `fetch_optional::<T, _>(exec)` | `Option<T>` | at most one row |
| `execute(exec)` | the database's `QueryResult` | for INSERT/UPDATE/DELETE — affected rows, last insert id, … |
| `count(exec)` | `i64` | wraps the query in a COUNT — see below |
| `fetch_scalar::<T, _>(exec)` | `T` | first column of the first row; no row → `RowNotFound` as `Error::Sqlx` |
| `fetch_optional_scalar::<T, _>(exec)` | `Option<T>` | first column of the first row, if any |

Details worth knowing:

- **`fetch_one` vs `fetch_optional`** — `fetch_one` treats "no row" as an
  error (`Error::Sqlx(sqlx::Error::RowNotFound)`); `fetch_optional` returns
  `Ok(None)`. Match `Error::Sqlx(sqlx::Error::RowNotFound)` when a missing
  row should become a 404 rather than a 500.
- **`execute`** returns the backend's `QueryResult` type (e.g.
  `MySqlQueryResult`), exposing `rows_affected()` and — where the backend has
  one — the last insert id.
- **`count`** does not modify your builder. It compiles the SQL, wraps it as

  ```sql
  SELECT COUNT(*) FROM (<your sql>) AS __cb_count
  ```

  binds the **same** arguments, and fetches a single `i64`. Your `WHERE`,
  `JOIN`, `GROUP BY`, etc. all apply — handy for a total-count query next to
  a paginated page (see
  [HTTP Filters & Pagination](cookbook/http-filters-pagination.md)).
- **`fetch_scalar` / `fetch_optional_scalar`** decode the first column of the
  first row into any `T: sqlx::Decode + sqlx::Type` — the `pluck`/aggregate
  idiom: `SELECT MAX(age) …` straight into an `i64` without a row struct.

## Full round trip

A complete create-table → insert → fetch → count cycle against in-memory
SQLite (adapted from the crate's live integration tests; requires
`features = ["sqlx_sqlite"]`):

```rust,no_run
use chain_builder::{IntoBind, QueryBuilder, Sqlite};

#[derive(Debug, sqlx::FromRow, PartialEq)]
struct User {
    id: i64,
    name: String,
}

async fn round_trip() -> Result<(), chain_builder::Error> {
    let pool = sqlx::SqlitePool::connect("sqlite::memory:").await?;
    sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL, age INTEGER)")
        .execute(&pool)
        .await?;

    // INSERT — heterogeneous values normalized to `Value` via `IntoBind`.
    QueryBuilder::<Sqlite>::table("users")
        .insert([
            ("id", 1i64.into_bind()),
            ("name", "Ann".into_bind()),
            ("age", 30i64.into_bind()),
        ])
        .execute(&pool)
        .await?;

    // SELECT — rows decoded straight into the struct.
    let rows: Vec<User> = QueryBuilder::<Sqlite>::table("users")
        .select(["id", "name"])
        .order_by_asc("id")
        .fetch_all(&pool)
        .await?;
    assert_eq!(rows, vec![User { id: 1, name: "Ann".into() }]);

    // COUNT — the builder's filters apply inside the COUNT wrapper.
    let n: i64 = QueryBuilder::<Sqlite>::table("users").count(&pool).await?;
    assert_eq!(n, 1);

    // Scalar fetch — no row struct needed.
    let max_age: Option<i64> = QueryBuilder::<Sqlite>::table("users")
        .select_max("age")
        .fetch_optional_scalar(&pool)
        .await?;
    assert_eq!(max_age, Some(30));

    Ok(())
}
```

Both `?` conversions are automatic: the initial `sqlx::query(...)` calls
yield `sqlx::Error` (→ `Error::Sqlx`), and any chain-builder helper that
fails to build yields `BuildError` (→ `Error::Build`). See
[Error Handling](error-handling.md) for matching on the result — including
the mandatory wildcard arm.

## Related pages

- [Error Handling](error-handling.md) — the unified `Error` enum, `try_*` twins, HTTP mapping
- [Binds & Values](binds.md) — how `Vec<Value>` becomes the driver's argument buffer
- [Getting Started](getting-started.md) — feature flags and the `default = ["sqlx_mysql"]` gotcha
- [Row Locking](query/locking.md) — running locking queries inside a transaction
- [HTTP Filters & Pagination](cookbook/http-filters-pagination.md) — `fetch_all` + `count` in a handler
