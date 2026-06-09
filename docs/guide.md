# chain-builder — Guide

A typed, dialect-aware query builder. Generic over a `Dialect`
(PostgreSQL / MySQL / SQLite), with **typed binds** via `IntoBind`, automatic
identifier escaping, dialect-correct placeholders (`$N` for Postgres, `?` for
MySQL/SQLite), and an `sqlx` handoff. See the [README](../README.md) for install
and a quick start.

```toml
chain-builder = { version = "2", features = ["sqlx_postgres"] }
```

## The builder

`QueryBuilder::<D>::table(name)` starts a query for dialect `D`
(`Postgres` / `MySql` / `Sqlite`). Every method is by-value chaining; `to_sql()`
returns `(String, Vec<Value>)`.

```rust
use chain_builder::{QueryBuilder, Postgres, Order};

let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .db("mydb")                              // multi-tenant: one connection, many DBs
    .select(["id", "name"])
    .left_join("orders", |j| j.on("users.id", "=", "orders.user_id"))
    .where_eq("status", "active")
    .where_in("role", ["admin", "staff"])
    .group_by(["users.id"])
    .order_by("name", Order::Desc)
    .paginate(2, 20)                          // LIMIT/OFFSET
    .to_sql();
```

## Typed binds (`IntoBind` / `Value`)

Bind arguments accept any `IntoBind`: integers, `f32`/`f64`, `bool`, `&str`/
`String`, `Vec<u8>`/`&[u8]`, `Option<T>` (`None` → SQL `NULL`), and the
feature-gated types `serde_json::Value` (`json`), `uuid::Uuid` (`uuid`), the
`chrono` date/time types (`chrono`), and `rust_decimal::Decimal` (`decimal`).
They are stored in an internal `Value` enum and bound to `sqlx` with the right
type — never inlined into SQL. `Decimal` binds natively on Postgres/MySQL and as
TEXT on SQLite (which has no native decimal type) — note that on SQLite this makes
comparisons/`ORDER BY` lexicographic, not numeric (`"19.99" < "5"`).

## WHERE

`where_eq/ne/gt/gte/lt/lte/like`, `where_in/not_in`, `where_null/not_null`,
`where_between`, `where_ilike` (native `ILIKE` on Postgres, `LOWER() LIKE LOWER()`
elsewhere), `where_jsonb_contains` (Postgres `@>`), `where_raw(sql, binds)`.
Group with `and_where(|w| …)` / `or_where(|w| …)` (parenthesized, AND-joined
inner):

```rust
QueryBuilder::<Postgres>::table("users").select(["*"])
    .where_eq("active", true)
    .or_where(|w| w.where_eq("role", "admin").where_gt("age", 40));
// ... WHERE "active" = $1 OR ("role" = $2 AND "age" > $3)
```

Empty `IN ()` → `1 = 0`; empty `NOT IN ()` → `1 = 1`.

## SELECT / DISTINCT / aggregates

`select([cols])` (bare identifiers, dotted ok), `select_raw(expr, binds)` for
arbitrary expressions, `distinct()`, `distinct_on([cols])` (Postgres only —
panics elsewhere).

Structured aggregate helpers escape the column at compile time (`*` passed
through) and accept an optional alias via the `_as` variants:

```rust
QueryBuilder::<Postgres>::table("orders")
    .select(["status"])
    .select_count_as("*", "cnt")        // COUNT(*) AS "cnt"
    .select_sum_as("amount", "total")   // SUM("amount") AS "total"
    .group_by(["status"]);
// SELECT "status", COUNT(*) AS "cnt", SUM("amount") AS "total" FROM "orders" GROUP BY "status"
```

`select_count`/`select_sum`/`select_avg`/`select_min`/`select_max` (no alias),
their `_as` variants, and `select_as(col, alias)` for plain column aliasing.

## Row locking

`for_update()` / `for_share()` append a locking clause to a `SELECT`;
`skip_locked()` / `no_wait()` add the corresponding modifier (and default the
strength to `FOR UPDATE` if called alone). Honored by Postgres / MySQL; a
**silent no-op on SQLite** (which locks the whole database, not rows) — ideal for
job-queue / multi-tenant claim patterns:

```rust
QueryBuilder::<Postgres>::table("jobs")
    .select(["id"])
    .where_eq("status", "queued")
    .limit(1)
    .for_update()
    .skip_locked();
// SELECT "id" FROM "jobs" WHERE "status" = $1 LIMIT $2 FOR UPDATE SKIP LOCKED
```

## JOIN / CTE / UNION

- `join` / `inner_join` / `left_join` / `right_join` / `full_outer_join` /
  `cross_join`; conditions via the closure: `.on(col, op, col2)`,
  `.on_val(col, op, value)`, `.on_raw(sql, binds)`.
- `with(name, query)` / `with_recursive(name, query)` — `WITH [RECURSIVE] …`.
- `union(query)` / `union_all(query)`.

Bind placeholders stay in first-appearance order across CTE → main → UNION (so
Postgres `$1..$n` is correct across nested sub-queries).

## GROUP BY / HAVING / ORDER BY / LIMIT

`group_by([cols])`, `group_by_raw(sql, binds)`, `having(col, op, val)`,
`having_raw(sql, binds)` (for `COUNT(*) > ?`), `order_by(col, Order::Asc|Desc)`
(+ `order_by_asc/desc`), `order_by_raw`, `limit(n)`, `offset(n)`,
`paginate(page, per_page)` (1-based).

## INSERT / UPDATE / DELETE

```rust
QueryBuilder::<Sqlite>::table("users").insert([("name", "John"), ("age", 30)]);
QueryBuilder::<Sqlite>::table("users").insert_many([
    [("name", "A"), ("age", 1)],
    [("name", "B"), ("age", 2)],
]); // ragged rows bind NULL for missing keys
QueryBuilder::<Sqlite>::table("users").update([("age", 31)]).where_eq("id", 1);
QueryBuilder::<Sqlite>::table("users").delete().where_eq("id", 1);
```

### Upsert + RETURNING

```rust
QueryBuilder::<Postgres>::table("users")
    .insert([("email", "a@b.c"), ("name", "A")])
    .on_conflict_merge(["email"])   // DO UPDATE SET <non-target cols> = EXCLUDED.<col>
    .returning(["id"]);
// .on_conflict_do_nothing(["email"]) → DO NOTHING (Postgres/SQLite) / INSERT IGNORE (MySQL)
```

Dialect map: Postgres/SQLite `ON CONFLICT (targets) DO UPDATE/NOTHING`; MySQL
`ON DUPLICATE KEY UPDATE c = VALUES(c)` (merge) / `INSERT IGNORE` (do-nothing).
`RETURNING` is emitted for Postgres/SQLite and **omitted on MySQL** (no support).

## Dynamic building

```rust
let qb = QueryBuilder::<Postgres>::table("users").select(["*"])
    .when(only_active, |q| q.where_eq("status", "active"))
    .when_else(by_name, |q| q.order_by_asc("name"), |q| q.order_by_desc("id"));
```

## Execution (sqlx, requires a `sqlx_*` feature)

```rust
// to_sqlx_query() / to_sqlx_query_as::<T>() → sqlx Query / QueryAs
let rows: Vec<UserRow> = qb.fetch_all(&pool).await?;       // T: sqlx::FromRow
let one:  UserRow      = qb.fetch_one(&pool).await?;
let maybe: Option<UserRow> = qb.fetch_optional(&pool).await?;
let n: i64 = qb.count(&pool).await?;                        // SELECT COUNT(*) FROM (..)
let id: i64 = qb.fetch_scalar(&pool).await?;               // first column
qb.execute(&pool).await?;                                   // INSERT/UPDATE/DELETE
```

All three drivers can be enabled at once (`sqlx_mysql` + `sqlx_sqlite` +
`sqlx_postgres`).

## Raw escape hatches

`select_raw`, `where_raw`, `group_by_raw`, `order_by_raw`, `on_raw`, the `having*`
helpers, and `table` set via a dotted string are emitted **verbatim** (not
escaped). For **Postgres**, raw SQL must use the correct `$N` matching the bind
position — the builder does not renumber raw fragments. Never pass untrusted input
through raw methods.

## Known limitations

- Most bundled tests are string-level; a live `sqlite::memory:` round-trip test
  exercises the real `sqlx` handoff.
- JSON path operators, named-constraint upsert targets, derived tables
  (subquery in `FROM`), and `INSERT … SELECT` are not yet implemented. The
  `Value` enum is `#[non_exhaustive]`, so further variants can be added without a
  breaking change.
