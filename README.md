# Chain Builder

[![Version](https://img.shields.io/crates/v/chain-builder?style=for-the-badge)](https://crates.io/crates/chain-builder)
[![License](https://img.shields.io/crates/l/chain-builder?style=for-the-badge)](LICENSE)

A **typed, dialect-aware SQL query builder** for Rust — "Knex for Rust".

Generic over a `Dialect` (**PostgreSQL / MySQL / SQLite**), with typed bind
parameters, automatic identifier escaping, dialect-correct placeholders
(`$N` for Postgres, `?` for MySQL/SQLite), and an `sqlx` handoff for execution.

> **v2.0** is a ground-up redesign. Upgrading from 1.x? See [CHANGELOG](CHANGELOG.md)
> — the API is entirely new (typed binds, `Dialect`-generic, no `serde_json::Value`
> in the core).

## Features

- **Dialect-generic** — one builder, three dialects; mixing them is a compile error.
- **Typed binds** — pass real Rust values (`i64`, `&str`, `Option<T>`, …) via `IntoBind`.
- **Injection-safe** — identifiers always escaped, values always bound.
- **Full query surface** — SELECT/INSERT/UPDATE/DELETE, WHERE (+ `or`/`and` groups),
  JOINs, CTEs (`WITH`/`RECURSIVE`), `UNION`, GROUP BY/HAVING/ORDER BY, LIMIT/OFFSET.
- **Upsert + RETURNING** — `on_conflict_merge`/`_do_nothing`, dialect-correct.
- **`db()` qualification** — multi-tenant (one connection, many databases).
- **Typed fetch** — `fetch_all::<T>`/`fetch_one`/`count`/`fetch_scalar` via `sqlx`.
- **Dynamic** — `when`/`when_else`, `paginate`, `distinct`/`distinct_on`, `ilike`, jsonb.

## Installation

```toml
[dependencies]
# pick the driver(s) you need; the builder itself is driver-agnostic
chain-builder = { version = "2", features = ["sqlx_postgres"] }
sqlx = { version = "0.9", features = ["postgres", "runtime-tokio-rustls"] }
```

Driver features: `sqlx_mysql` (default), `sqlx_sqlite`, `sqlx_postgres` — enable any
combination. `json` enables `Value::Json`.

## Quick Start

```rust
use chain_builder::{QueryBuilder, Postgres, Order};

let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .db("mydb")
    .select(["id", "name"])
    .where_eq("status", "active")
    .where_in("role", ["admin", "staff"])
    .order_by("created_at", Order::Desc)
    .paginate(2, 20)
    .to_sql();
// SELECT "id", "name" FROM "mydb"."users"
//   WHERE "status" = $1 AND "role" IN ($2, $3)
//   ORDER BY "created_at" DESC LIMIT $4 OFFSET $5
```

```rust
// upsert + RETURNING
use chain_builder::{QueryBuilder, Postgres};
let q = QueryBuilder::<Postgres>::table("users")
    .insert([("email", "a@b.c"), ("name", "A")])
    .on_conflict_merge(["email"])
    .returning(["id"]);
// INSERT INTO "users" ("email", "name") VALUES ($1, $2)
//   ON CONFLICT ("email") DO UPDATE SET "name" = EXCLUDED."name" RETURNING "id"

// execute with sqlx (feature sqlx_postgres)
// let rows: Vec<UserRow> = q.fetch_all(&pool).await?;
```

Same builder, different dialect → MySQL backticks + `?`:

```rust
use chain_builder::{QueryBuilder, MySql};
QueryBuilder::<MySql>::table("users").select(["id"]).where_eq("status", "active");
// SELECT `id` FROM `users` WHERE `status` = ?
```

## Documentation

- **[Guide](docs/guide.md)** — full reference: every WHERE/JOIN/CTE/UNION/aggregate
  method, upsert & RETURNING, typed fetch, dynamic building, and the `Dialect`/
  `IntoBind` model.

## Security

Two axes of SQL-injection safety:

- **Values** are always sent as bound parameters (`?` / `$N`), never inlined.
- **Identifiers** (table/column/alias names) are dialect-escaped automatically;
  qualified names (`users.id`) are escaped segment-by-segment (`"users"."id"`),
  `*` preserved. **Pass bare names** — do not pre-quote (it would be double-escaped).

The `*_raw` methods (`select_raw`, `where_raw`, `group_by_raw`, `order_by_raw`,
`on_raw`, `having_raw`) and the `having*` helpers are verbatim escape hatches —
never pass untrusted input through them.

## Feature Flags

- **`sqlx_mysql`** (default) — MySQL driver + `SqlxDialect for MySql`
- **`sqlx_sqlite`** — SQLite driver + `SqlxDialect for Sqlite`
- **`sqlx_postgres`** — PostgreSQL driver + `SqlxDialect for Postgres`
- **`json`** — `Value::Json` + `IntoBind for serde_json::Value`

The query builder (`to_sql()`) works with **no** driver feature; a driver feature is
only needed for the `sqlx` handoff (`to_sqlx_query`, `fetch_*`). All three drivers
can be enabled simultaneously.

## License

MIT License - see [LICENSE](LICENSE) for details.
