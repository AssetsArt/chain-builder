# Error Handling

chain-builder fails **loud** on invalid query construction — never by emitting
broken SQL. Every guard surfaces in one of two ways: the panicking API
(`to_sql()` and friends) panics, and the fallible `try_*` twins return a typed
[`BuildError`](#builderror-variants). The async execution helpers go one step
further and fold build failures and database failures into a single unified
[`Error`](#the-unified-error-enum) enum, so a `fetch_all` call can never panic
on a bad builder. This page covers all three layers, the deferred `having()`
error, the `#[non_exhaustive]` matching rule, and how to map errors to HTTP
status codes.

## The twin API: panic or `Result`

Every compilation entry point exists in two flavors:

| Fallible (returns `Result<_, BuildError>`) | Panicking twin |
|---|---|
| `try_to_sql()` | `to_sql()` |
| `try_compile(&qb)` | `compile(&qb)` |
| `try_to_sqlx_query()` | `to_sqlx_query()` |
| `try_to_sqlx_query_as::<T>()` | `to_sqlx_query_as::<T>()` |

The panicking twin panics with **exactly** the `Display` message of the
`BuildError` the fallible twin would return — the two paths cannot drift
apart.

**Policy: the panicking API is kept deliberately.** It is not deprecated and
not a legacy shim. It stays the ergonomic path for **static, hand-written
queries** — tests, migrations, fixed reports — where invalid construction is a
programmer error and a panic message is the fastest signal. Use the `try_*`
family whenever any part of the query is **shaped by runtime input** (filter
parameters, pagination, user-selected operators): there, an invalid builder is
a request problem, not a crash-worthy bug.

```rust
use chain_builder::{BuildError, Postgres, QueryBuilder};

// offset() without limit() is invalid — try_to_sql returns the typed error:
let err = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .offset(10)
    .try_to_sql()
    .unwrap_err();
assert_eq!(err, BuildError::OffsetWithoutLimit);
assert_eq!(err.to_string(), "offset(...) requires limit(...)");

// The same builder's to_sql() would panic with that exact message.
```

## `BuildError` variants

`BuildError` (`#[non_exhaustive]`, implements `std::error::Error`) describes
*caller* mistakes — a query that cannot be rendered as valid SQL — never
internal compiler state. The complete set in 3.0:

| Variant | Trigger | Display message |
|---|---|---|
| `LockRequiresSelect` | `for_update()`/`for_share()` on a non-SELECT query | `for_update()/for_share() is only valid on SELECT` |
| `DistinctOnRequiresPostgres` | `distinct_on(...)` compiled for a dialect without `DISTINCT ON` | `DISTINCT ON requires PostgreSQL` |
| `EmptyInsert` | `insert()` with no columns | `insert() requires at least one column` |
| `EmptyUpdate` | `update()` with no columns | `update() requires at least one column` |
| `OffsetWithoutLimit` | `offset(...)` without `limit(...)` | `offset(...) requires limit(...)` |
| `LockWithUnion` | `for_update()`/`for_share()` combined with `UNION` | `for_update()/for_share() cannot be combined with UNION` |
| `InvalidHavingOperator(String)` | `having()` operator outside the fixed allowlist (carries the rejected operator) | `having() operator "<op>" is not an allowed comparison operator (use having_raw() for arbitrary aggregate expressions)` |

(`<op>` is the operator exactly as the caller passed it, in Rust `{:?}` debug
quoting.) Where each guard comes from:
[Row Locking](query/locking.md) for the two lock rules,
[SELECT](query/select.md) for `distinct_on`,
[INSERT · UPDATE · DELETE](query/insert-update-delete.md) for the empty-column
guards, and
[GROUP BY · HAVING · ORDER · LIMIT](query/group-having-order-limit.md) for
`offset`/`limit` and the `having` allowlist.

## The deferred `having()` error

`having()` validates its operator against a fixed allowlist (`=`, `!=`, `<>`,
`>`, `>=`, `<`, `<=`, `LIKE`, `NOT LIKE`, matched case-insensitively). A
disallowed operator does **not** panic at the call site — `having()` returns
`self` like every other method, so the chain stays intact. Instead the error
is **recorded on the builder** (the first recorded error wins if several
calls fail) and surfaces when you compile:

```rust
use chain_builder::{BuildError, Postgres, QueryBuilder};

let qb = QueryBuilder::<Postgres>::table("orders")
    .select(["user_id"])
    .having("amount", "; DROP TABLE users", 0i64); // no panic here

let err = qb.try_to_sql().unwrap_err();
assert_eq!(
    err,
    BuildError::InvalidHavingOperator("; DROP TABLE users".to_owned())
);
```

The deferred error also **propagates from nested builders**: if the offending
builder is attached as a [CTE](query/cte-union.md), a
[UNION arm](query/cte-union.md), or a subquery (`select_subquery`,
`where_exists`, `where_in_subquery`, …), compiling the *outer* builder
surfaces the inner error:

```rust
let bad_inner = QueryBuilder::<Postgres>::table("orders")
    .select(["user_id"])
    .having("amount", "UNION SELECT", 0i64);

let err = QueryBuilder::<Postgres>::table("top")
    .select(["user_id"])
    .with("top", bad_inner)
    .try_to_sql()
    .unwrap_err();
// err == BuildError::InvalidHavingOperator("UNION SELECT".to_owned())
```

## The unified `Error` enum

The execution helpers ([`fetch_all`/`fetch_one`/`fetch_optional`/`execute`/
`count`/`fetch_scalar`/`fetch_optional_scalar`](sqlx.md)) can fail in two
ways: the query failed to **build**, or it failed to **execute**. Both fold
into one enum:

```rust
#[non_exhaustive]
pub enum Error {
    Build(BuildError),  // invalid construction — returned BEFORE touching the DB
    Sqlx(sqlx::Error),  // database / driver failure
}
```

- `Error::Build` is returned **before** any database round-trip — an invalid
  builder never reaches the pool, and never panics on this path.
- `From<BuildError>` and `From<sqlx::Error>` are both implemented, so `?`
  works directly in functions returning `Result<_, chain_builder::Error>`.
- `Error` implements `std::error::Error`, and `source()` returns the inner
  `BuildError` or `sqlx::Error`, so it composes with `anyhow`, `thiserror`,
  and error-report chains.

```rust
use chain_builder::{BuildError, Error};

let e = Error::from(BuildError::OffsetWithoutLimit);
assert_eq!(e.to_string(), "offset(...) requires limit(...)");
assert!(std::error::Error::source(&e).is_some());

let _: Error = sqlx::Error::RowNotFound.into();
```

## `#[non_exhaustive]`: always include a wildcard arm

**Both** `BuildError` and `Error` are `#[non_exhaustive]`: future versions may
add variants without a semver-major bump. Outside the chain-builder crate, the
compiler therefore *requires* a wildcard arm in every `match` over them —
write your matches so a new variant lands somewhere sensible (usually the
500 bucket):

```rust
match qb.try_to_sql() {
    Ok((sql, binds)) => { /* run it */ }
    Err(BuildError::InvalidHavingOperator(op)) => { /* reject the input */ }
    Err(e) => { /* wildcard arm — required, catches future variants */ }
}
```

## Mapping to HTTP status codes

All `BuildError` variants describe caller mistakes, which splits cleanly along
one question: *did the offending value come from the request?* Variants an
end-user can trigger (an operator or pagination parameter taken from input)
map to **400**; the rest are programmer errors and database failures — **500**:

```rust
use chain_builder::{BuildError, Error};

match qb.fetch_all::<Row, _>(&pool).await {
    Ok(rows) => ok(rows),
    // input-driven: a filter API let the client pick the operator → 400
    Err(Error::Build(BuildError::InvalidHavingOperator(_))) => bad_request(),
    // also 400 if your pagination params come straight from the request:
    Err(Error::Build(BuildError::OffsetWithoutLimit)) => bad_request(),
    // remaining build errors are bugs in query construction → 500
    Err(Error::Build(e)) => internal_error(e),
    // connection / query / decode failure → 500
    Err(Error::Sqlx(e)) => internal_error(e),
    // #[non_exhaustive]: wildcard arm required downstream
    Err(e) => internal_error(e),
}
```

A complete axum version of this pattern lives in
[Mapping Errors to HTTP Status](cookbook/http-error-mapping.md).

## Since 3.0: the execution helpers changed error type

In 2.x, `fetch_*` / `execute` / `count` returned `Result<_, sqlx::Error>` and
**panicked** on an invalid builder. Since 3.0 they return
`Result<_, chain_builder::Error>` and never panic. Migrating a 2.x call site
is one of:

- switch your function's error type to `chain_builder::Error` (or any type
  with `From<chain_builder::Error>`) and keep using `?`, or
- match `Error::Sqlx(e)` to recover the inner `sqlx::Error` (e.g. to check
  for `RowNotFound`) — plus the mandatory wildcard arm.

Also since 3.0: `having()` with a bad operator no longer panics at the call
site (it was a call-time panic in 2.1.2) — the error is deferred to
compilation as described above.

## Related pages

- [Executing with sqlx](sqlx.md) — the helpers that return the unified `Error`
- [Row Locking](query/locking.md) — `LockRequiresSelect` / `LockWithUnion` in context
- [GROUP BY · HAVING · ORDER · LIMIT](query/group-having-order-limit.md) — the `having` allowlist and `offset`-requires-`limit`
- [SELECT](query/select.md) — `distinct_on` and `DistinctOnRequiresPostgres`
- [Mapping Errors to HTTP Status](cookbook/http-error-mapping.md) — the full axum recipe
- [Security Model](security.md) — why the `having` operator is validated at all
