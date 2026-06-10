# Binds & Values

Every value you pass to the builder — to `where_eq`, `insert`, `having`,
`limit`, anywhere — ends up as a bound parameter, **never** inlined into the
SQL string. Compiling yields a `(sql, binds)` tuple: the SQL contains only
placeholders (`$N` on Postgres, `?` on MySQL/SQLite), and `binds` is a
`Vec<Value>` handed to the driver separately. This page covers `Value` — the
dialect-agnostic representation of a bound parameter — and `IntoBind`, the
trait that converts ordinary Rust types into it.

```rust,ignore
use chain_builder::{Postgres, QueryBuilder, Value};
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .where_eq("status", "active")
    .where_gte("age", 18i64)
    .to_sql();
// SELECT "id" FROM "users" WHERE "status" = $1 AND "age" >= $2
// binds == [Value::Text("active".into()), Value::I64(18)]
```

The string `"active"` never appears in `sql` — only in `binds`. That
separation is the core SQL-injection guarantee (see the
[Security Model](security.md)); when executing through sqlx, the same
`Vec<Value>` is translated into the driver's argument buffer (see
[Executing with sqlx](sqlx.md)).

## The `Value` enum

```rust,ignore
#[non_exhaustive]
pub enum Value {
    Null,
    Bool(bool),
    I64(i64),     // the canonical integer binding type
    F64(f64),
    Text(String),
    Bytes(Vec<u8>),
    // feature-gated:
    Json(serde_json::Value),               // feature = "json"
    Uuid(uuid::Uuid),                      // feature = "uuid"
    DateTimeUtc(chrono::DateTime<Utc>),    // feature = "chrono"
    NaiveDateTime(chrono::NaiveDateTime),  // feature = "chrono"
    NaiveDate(chrono::NaiveDate),          // feature = "chrono"
    NaiveTime(chrono::NaiveTime),          // feature = "chrono"
    Decimal(rust_decimal::Decimal),        // feature = "decimal"
}
```

`Value` is `#[non_exhaustive]`: new variants can appear without a breaking
change, so any `match` you write over it must include a wildcard arm (the
same rule as [`BuildError`](error-handling.md)).

## `IntoBind` conversions

Everything that can be bound implements `IntoBind`. The complete table:

| Rust type | `Value` variant | Notes |
|---|---|---|
| `i8`, `i16`, `i32`, `i64` | `I64` | lossless |
| `u8`, `u16`, `u32` | `I64` | lossless |
| `u64`, `usize`, `isize` | `I64` | **wraps above `i64::MAX`** — see below |
| `f32`, `f64` | `F64` | `f32` widened |
| `bool` | `Bool` | |
| `&str`, `String`, `&String` | `Text` | |
| `Vec<u8>`, `&[u8]` | `Bytes` | BLOB |
| `Option<T>` where `T: IntoBind` | `Null` / inner | `None` → `Null`, `Some(v)` → `v`'s conversion |
| `Value` | itself | identity pass-through |
| `serde_json::Value` | `Json` | feature `json` |
| `uuid::Uuid` | `Uuid` | feature `uuid` |
| `chrono::DateTime<Utc>` | `DateTimeUtc` | feature `chrono` |
| `chrono::NaiveDateTime` | `NaiveDateTime` | feature `chrono` |
| `chrono::NaiveDate` | `NaiveDate` | feature `chrono` |
| `chrono::NaiveTime` | `NaiveTime` | feature `chrono` |
| `rust_decimal::Decimal` | `Decimal` | feature `decimal` |

### Integers — and the `u64` wrap

All integer types convert to `Value::I64`, the canonical integer binding
type. For `i8`–`i64` and `u8`–`u32` this is lossless.

> **⚠️ `u64` / `usize` / `isize` values above `i64::MAX` wrap silently.**
>
> The narrowing uses Rust's `as i64` truncation semantics — silent
> two's-complement wrap, not an error and not saturation:
>
> ```rust,ignore
> use chain_builder::{IntoBind, Value};
> assert_eq!(((i64::MAX as u64) + 1).into_bind(), Value::I64(i64::MIN));
> assert_eq!(u64::MAX.into_bind(), Value::I64(-1));
> ```
>
> This is intentional (the conversions exist for ergonomics), but it means a
> `u64` row count or hash above `9_223_372_036_854_775_807` binds as a
> *negative* number. If your values can exceed `i64::MAX`, bind them
> explicitly — e.g. as `Value::Text(v.to_string())` — instead of relying on
> the blanket impl.

### `Option<T>` — nullable binds

`None` binds SQL `NULL`; `Some(v)` binds exactly what `v` alone would:

```rust,ignore
use chain_builder::{IntoBind, Value};
assert_eq!(Option::<i64>::None.into_bind(), Value::Null);
assert_eq!(Some(5i64).into_bind(), Value::I64(5));
```

This composes with the feature-gated types too
(`Option::<uuid::Uuid>::None.into_bind() == Value::Null`). One semantic trap:
`where_eq("col", None::<i64>)` emits `"col" = $1` with a NULL bind — and
`col = NULL` is never true in SQL. For "IS NULL" tests use
[`where_null`](query/where.md); for "filter only when present" guard the call
with [`when`](query/dynamic.md).

### `Value` pass-through

`Value` implements `IntoBind` as the identity, so you can construct a variant
explicitly whenever you need to override the blanket conversions (e.g. the
`u64` case above):

```rust,ignore
assert_eq!(Value::I64(1).into_bind(), Value::I64(1));
```

## Feature-gated types

Each optional integration adds `Value` variants and `IntoBind` impls. Enable
them in `Cargo.toml` (`features = ["json", "uuid", "chrono", "decimal"]` —
see [Getting Started](getting-started.md)).

### `json` — `serde_json::Value`

Binds as `Value::Json`. On **all three backends** the JSON is serialized with
`to_string` and stored as TEXT — it is not bound as a native `jsonb`
parameter. (Intended for TEXT columns; Postgres has no implicit cast from a
text parameter to `json`/`jsonb`, so inserting into a `jsonb` column needs an
explicit `::jsonb` cast in raw SQL. For jsonb *querying* see
[`where_jsonb_contains`](query/where.md).)

### `uuid` — `uuid::Uuid`

```rust,ignore
let id = uuid::Uuid::nil();
let (sql, binds) = QueryBuilder::<Postgres>::table("t")
    .select(["id"])
    .where_eq("id", id)
    .to_sql();
// SELECT "id" FROM "t" WHERE "id" = $1
// binds == [Value::Uuid(id)]
```

Native `uuid` binding on Postgres; on MySQL/SQLite it binds through sqlx's
`Uuid` encoding for those backends.

### `chrono` — dates and times

Four variants, one per chrono type: `DateTimeUtc`
(`chrono::DateTime<chrono::Utc>` — timezone-aware, UTC), `NaiveDateTime`,
`NaiveDate`, `NaiveTime`:

```rust,ignore
let nd = chrono::NaiveDate::from_ymd_opt(2026, 6, 9).unwrap();
let (sql, binds) = QueryBuilder::<Postgres>::table("t")
    .select(["id"])
    .where_eq("day", nd)
    .to_sql();
// SELECT "id" FROM "t" WHERE "day" = $1
// binds == [Value::NaiveDate(nd)]
```

### `decimal` — `rust_decimal::Decimal`

For money and exact-numeric columns:

```rust,ignore
use rust_decimal::Decimal;
use std::str::FromStr;

let price = Decimal::from_str("19.99").unwrap();
let (sql, binds) = QueryBuilder::<Postgres>::table("products")
    .select(["id"])
    .where_eq("price", price)
    .to_sql();
// SELECT "id" FROM "products" WHERE "price" = $1
// binds == [Value::Decimal(price)]
```

Bound **natively** on Postgres (`NUMERIC`) and MySQL (`DECIMAL`).

> **⚠️ SQLite stores `Decimal` as TEXT — comparisons are lexicographic.**
>
> SQLite has no native decimal type, so the exact value is bound as its
> string form (`d.to_string()`). The round-trip is exact, but any SQL-side
> comparison or `ORDER BY` against a TEXT column compares **strings, not
> numbers**: `"19.99" < "5"` lexicographically. Use `Decimal` on SQLite for
> exact storage/round-trip only; for numeric range queries, store a scaled
> integer (cents) or compare with `CAST(price AS REAL)`.

## Inspecting a query: `to_sql_pretty`

For logs and debugging, `to_sql_pretty()` (3.1.0+) renders the SQL plus one
line per bind. `try_to_sql_pretty()` is the fallible twin and surfaces the
same `BuildError` as `try_to_sql()`:

```rust,ignore
let qb = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .where_eq("status", "active")
    .where_gt("age", 21i64);
println!("{}", qb.to_sql_pretty());
// SELECT "id" FROM "users" WHERE "status" = $1 AND "age" > $2
// binds:
//   $1 = Text("active")
//   $2 = I64(21)
```

On `?`-placeholder dialects (MySQL/SQLite) the bind labels carry a 1-based
ordinal for readability (`?1 = …`); the SQL itself still uses bare `?`. The
output includes the literal bind values — don't log it if a bind may carry
sensitive data. The output format is for humans — not a stability contract.

## Where the binds go from here

`to_sql()` / `try_to_sql()` stop at `(String, Vec<Value>)` — useful for
logging, testing, or driving a non-sqlx driver yourself. With a `sqlx_*`
feature enabled, `to_sqlx_query()` and the `fetch_*` helpers translate each
`Value` into the backend's argument buffer and execute it — the SQL string
still never contains a value. See [Executing with sqlx](sqlx.md).

## Related pages

- [Security Model](security.md) — why values are always bound, and what raw fragments do not protect
- [Executing with sqlx](sqlx.md) — how `Vec<Value>` becomes driver arguments
- [Dynamic Building](query/dynamic.md) — `Option` filters done right with `when`
- [Getting Started](getting-started.md) — enabling the `json`/`uuid`/`chrono`/`decimal` features
- [Error Handling](error-handling.md) — the matching `#[non_exhaustive]` rule for `BuildError`
