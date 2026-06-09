# chain-builder v2 — Query Builder Design

Status: **approved (brainstorm)** · Date: 2026-06-09 · Target crate version: `2.0.0`

## Vision

Become **"Knex.js for Rust"** — a fluent, ergonomic, dialect-aware SQL **query
builder** that produces ready-to-execute `sqlx` queries with correct typed bind
parameters. v2 is the query-builder foundation of that vision. Schema-builder
(DDL), migrations, and seeds are **out of scope for v2** and become later
sub-projects with their own spec → plan → implementation cycles.

## Goal

A clean, type-safe redesign of the query builder around three pillars decided in
brainstorming:

1. **Typed binds via trait** — callers pass real Rust values (`i32`, `&str`,
   `String`, `Option<T>`, dates, `Uuid`, `Decimal`, `Vec<u8>`) through an
   `IntoBind` trait; values are stored in an internal `Value` enum and bound to
   `sqlx` with the correct type at handoff. `serde_json::Value` is **removed from
   the core API** (JSON remains supported as one bind type behind a feature).
2. **sqlx-only execution** — the builder compiles to `(sql, binds)` and hands off
   to `sqlx` as `Query` / `QueryAs<T>`, with dialect-correct placeholders and
   ergonomic `fetch_*` helpers. Async, pooling, and transactions come from `sqlx`
   directly (v2 does not own a runtime).
3. **`Dialect` trait + generic builder** (`QueryBuilder<D: Dialect>`) — per-dialect
   rules (placeholder style `?` vs `$N`, identifier quote char, `RETURNING` /
   upsert syntax, `LIMIT`/`OFFSET` rendering) live in one place; the generic
   parameter makes mixing dialects a compile error and ties each dialect to its
   `sqlx::Database`.

Supported dialects: **PostgreSQL, MySQL, SQLite** (the set `sqlx` 0.9 supports;
MSSQL is not supported by sqlx and is out of scope).

## Non-goals (v2.0)

- Schema builder / DDL (`createTable`, `alterTable`, …) — later sub-project.
- Migrations / seeds — later sub-project.
- Owning connection pools, a runtime, or a transaction manager — use `sqlx`'s.
- MSSQL / Oracle / other dialects sqlx does not support.
- Compile-time schema verification (sqlx `query!` macro style). Binds are dynamic
  at the SQL level (Knex-like); `IntoBind` only adds Rust-type ergonomics on input.
- Backward compatibility with the 1.x `serde_json::Value` API. v2 is a clean
  redesign; a migration guide is provided. (See "Coexistence & cutover".)

## Milestones

v2.0 (the full contract the user approved) spans four feature buckets, all
in-scope for the **2.0.0 release**, sequenced as milestones:

- **M1 — Foundation (THIS plan's implementation target).** `Dialect` trait + 3
  impls; `Value` enum + `IntoBind`; generic `QueryBuilder<D>`; SELECT / INSERT /
  UPDATE / DELETE; `WHERE` (eq/ne/in/null/between/like/comparisons/raw/and-or
  groups); identifier escaping (ported from 1.x `dialect.rs`); dialect-correct
  placeholder numbering (`?` for MySQL/SQLite, `$N` for Postgres); `to_sql()` +
  `to_sqlx_query()` / `to_sqlx_query_as::<T>()`; per-dialect tests.
- **M2 — Joins / CTE / UNION / GROUP BY / HAVING / ORDER BY** ported onto the
  generic builder.
- **M3 — Upsert + RETURNING.** Real `on_conflict()` (Postgres `ON CONFLICT … DO
  UPDATE/NOTHING`, MySQL `ON DUPLICATE KEY UPDATE`, SQLite `ON CONFLICT`) and
  `returning()` with row fetch. (Replaces 1.x's broken no-op `insert_ignore` /
  `insert_or_update`.)
- **M4 — Typed fetch.** `fetch_all::<T>()`, `fetch_one::<T>()`, `first::<T>()`,
  `pluck::<C>()`, scalar `count() -> i64`, `execute()`.
- **M5 — Postgres parity extras.** `DISTINCT ON`, jsonb operators
  (`->`, `->>`, `@>`), `ILIKE`.
- **M6 — Dynamic building + pagination.** `when(cond, |q| …)` / `modify`,
  pagination helpers.

> **Scope honesty:** This document's *implementation plan* (and the autonomous
> run that follows) targets **M1** and lands it behind a `v2` cargo feature so the
> published 1.x API on `main` is untouched. M2–M6 are real v2.0 scope but ship as
> subsequent plans. Anything not implemented this session is named in the wrap-up.

## High-level architecture

```
src/v2/                         (all behind feature = "v2"; 1.x untouched)
├── mod.rs                      pub use surface; module wiring
├── value.rs                   Value enum + IntoBind trait + From/IntoBind impls
├── dialect/
│   ├── mod.rs                 Dialect trait + Placeholder writer
│   ├── postgres.rs            Postgres marker type + Dialect impl ($N, ", RETURNING…)
│   ├── mysql.rs               MySql marker + Dialect impl (?, `)
│   └── sqlite.rs              Sqlite marker + Dialect impl (?, ")
├── ident.rs                   escape_identifier (ported from src/dialect.rs)
├── builder.rs                 QueryBuilder<D> — select/from/insert/update/delete entry
├── where_.rs                  WhereBuilder<D> — predicate tree (eq/in/between/group/raw)
├── compile.rs                 clause → SQL + Vec<Value> (dialect-driven placeholders)
└── sqlx_bind.rs               Value → DB::Arguments; to_sqlx_query / _as (feature sqlx_*)
```

### `Dialect` + `SqlxDialect` traits (the seam)

`sqlx` is an **unconditional** dependency of the crate (`Cargo.toml`: `sqlx =
"0.9"`), but the concrete `sqlx::Postgres` / `sqlx::MySql` / `sqlx::Sqlite` types
only exist when their sqlx dialect feature is on. So the dialect contract is split
in two (resolves spec-review B1/R3/Q1): a base trait with **no** sqlx dependency,
and a **sealed** sub-trait that carries the sqlx binding, available per feature.

```rust
// Always available — no sqlx in scope.
pub trait Dialect: Sized + Send + Sync + 'static {
    /// Identifier quote character: '`' for MySQL, '"' for Postgres/SQLite.
    fn quote_char() -> char;

    /// Write the placeholder for the 1-based bind index `n`.
    /// MySQL/SQLite ignore `n` and write `?`; Postgres writes `$n`.
    fn write_placeholder(out: &mut String, n: usize);

    /// Whether the dialect supports `RETURNING` (Postgres/SQLite: true, MySQL: false).
    fn supports_returning() -> bool;
}

// Sealed sub-trait carrying the sqlx binding. Implemented for each marker behind
// its own `sqlx_<dialect>` feature, so the sqlx DB type is only named when present.
pub trait SqlxDialect: Dialect {
    type Database: sqlx::Database;
    /// Bind a slice of `Value`s into this database's argument buffer.
    fn bind_arguments(
        binds: &[crate::v2::Value],
    ) -> <Self::Database as sqlx::Database>::Arguments<'static>;
}
```

`Dialect` is implemented by marker types `Postgres`, `MySql`, `Sqlite`
(always). `SqlxDialect` is implemented for `Postgres` behind `sqlx_postgres`, for
`MySql` behind `sqlx_mysql`, for `Sqlite` behind `sqlx_sqlite`. `to_sqlx_query()`
is only available `where D: SqlxDialect`. `QueryBuilder<D>` is generic over
`D: Dialect`. Constructors: `QueryBuilder::<Postgres>::table("t")` plus convenience
aliases (`postgres()`, `mysql()`, `sqlite()`).

### `Value` + `IntoBind`

```rust
#[non_exhaustive]
pub enum Value {
    Null,
    Bool(bool),
    I64(i64),
    F64(f64),
    Text(String),
    Bytes(Vec<u8>),
    // feature-gated variants added in their own steps:
    #[cfg(feature = "json")]    Json(serde_json::Value),
    #[cfg(feature = "uuid")]    Uuid(uuid::Uuid),
    #[cfg(feature = "chrono")]  // DateTime/Date/Time …
    // …
}

pub trait IntoBind { fn into_bind(self) -> Value; }
```

`IntoBind` is implemented for the primitive Rust types, `&str`/`String`,
`Option<T: IntoBind>` (→ `Null` for `None`), and the feature-gated types. A blanket
`impl<T: IntoBind> IntoBind for Option<T>` covers nullability. `where_in` takes
`IntoIterator<Item = impl IntoBind>`.

### Compilation & placeholders

`compile.rs` walks the clause tree and writes SQL into a `String` while pushing
binds into a `Vec<Value>`. A running 1-based counter feeds
`D::write_placeholder`. Identifier slots go through `ident::escape_identifier`.
The **logic and tests** are ported verbatim from 1.x `src/dialect.rs`, but the
**signature is adapted**: 1.x is `escape_identifier(ident: &str, client: &Client)`
using `client.quote_char()`; v2 has no `Client`, so it becomes
`escape_identifier(ident: &str, quote: char)` and callers pass `D::quote_char()`
(spec-review R1). Values never appear inline.

### sqlx handoff

`SqlxDialect::bind_arguments` converts `&[Value]` into
`Database::Arguments<'static>` (one impl per dialect, mirroring 1.x
`value_to_arguments`). `to_sqlx_query()` / `to_sqlx_query_as::<T>()` (defined
`where D: SqlxDialect`) return `sqlx::query::Query` / `QueryAs`, wrapping the
SQL with `sqlx::AssertSqlSafe` (builder-generated) as in 1.x. `to_sql() ->
(String, Vec<Value>)` stays driver-free for testing/debugging.

**Bind-error disposition (spec-review R2/Q2):** sqlx 0.9 `Arguments::add` returns
`Result`. The M1 `Value` variants (`Null` → `Option::<String>::None`, `Bool`,
`I64`, `F64`, `Text`, `Bytes`) all have **infallible** encoders for pg/mysql/
sqlite, so `bind_arguments` follows 1.x and discards the (never-hit-in-practice)
`add` error with a documenting comment rather than threading `Result` through every
`to_sqlx_query()` call site. If a future fallible `Value` variant is added (e.g. a
large-decimal), this decision is revisited then — it is **not** silent data loss
for the M1 type set. `Value::Null` binding as SQL `NULL` via `Option::None` is the
proven 1.x behavior (Q4).

## Public API surface (M1)

```rust
use chain_builder::v2::{QueryBuilder, Postgres, MySql, Sqlite};

// SELECT
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["id", "name"])
    .where_eq("status", "active")
    .where_in("role", ["admin", "staff"])
    .where_gt("age", 18)
    .to_sql();
// SELECT "id", "name" FROM "users"
//   WHERE "status" = $1 AND "role" IN ($2, $3) AND "age" > $4

// same builder, MySQL → backticks + `?`
QueryBuilder::<MySql>::table("users").select(["id"]).where_eq("status", "active");
// SELECT `id` FROM `users` WHERE `status` = ?

// INSERT / UPDATE / DELETE
QueryBuilder::<Sqlite>::table("users").insert([("name", "John"), ("age", 30)]);
QueryBuilder::<Sqlite>::table("users").update([("age", 31)]).where_eq("id", 1);
QueryBuilder::<Sqlite>::table("users").delete().where_eq("id", 1);

// sqlx handoff (feature sqlx_postgres)
let q = QueryBuilder::<Postgres>::table("users").select(["*"]).to_sqlx_query();
// q.fetch_all(&pool).await? …
```

WHERE methods in M1: `where_eq`, `where_ne`, `where_in`, `where_not_in`,
`where_null`, `where_not_null`, `where_between`, `where_like`, `where_gt`,
`where_gte`, `where_lt`, `where_lte`, `where_raw`, and `and_where`/`or_where`
group combinators (spec-review Q3): **closure-based sub-groups** that compile to a
parenthesized predicate joined to the outer chain by `AND`/`OR` respectively:

```rust
QueryBuilder::<Postgres>::table("users").select(["*"])
    .where_eq("active", true)
    .or_where(|q| { q.where_eq("role", "admin").where_gt("age", 40); });
// ... WHERE "active" = $1 OR ("role" = $2 AND "age" > $3)
```

The closure receives a fresh `WhereBuilder<D>`; its predicates are wrapped in `(…)`
and appended. This is the M1 form of OR chaining (1.x used `Box<QueryBuilder>`
`OrChain`/`SubChain`; v2 uses closures for ergonomics). An OR-group test is part of
M1 acceptance (Tests item 8).

## Cargo features

Additive only. `serde_json` stays an **unconditional** dependency (it is used
throughout 1.x), so the `json` feature does **not** use `dep:` activation
(spec-review B2). `sqlx_postgres` is **new** and added as an M1 setup step
(spec-review B3). `default` stays `["mysql", "sqlx_mysql"]` and must **not** pull
in `v2` (R4).

```toml
v2            = []                # gates the entire src/v2 module (off by default)
json          = ["v2"]           # enables Value::Json (+ IntoBind for serde_json::Value)
sqlx_postgres = ["sqlx/postgres"] # NEW — sqlx Postgres driver for SqlxDialect<Postgres>
# existing: sqlx_mysql = ["sqlx/mysql"], sqlx_sqlite = ["sqlx/sqlite"]
# uuid / chrono / rust_decimal Value variants: their own steps (may slip past M1;
#   Value is #[non_exhaustive] to allow adding them without a breaking change)
```

`v2` is **off by default**; `main` keeps building/publishing 1.x. The only `src/`
change outside `src/lib.rs` is none — `lib.rs` gains one
`#[cfg(feature = "v2")] pub mod v2;`. `Cargo.toml` changes are **additive feature
entries only** (no reclassification of existing deps). When v2 reaches parity it
becomes the crate root and 1.x is removed in the real `2.0.0` release.

## Behavior invariants

- **Injection safety preserved.** Identifiers escaped via the ported, tested
  `escape_identifier`; values always bound, never inlined. The `*_raw` methods are
  the documented verbatim escape hatch.
- **Dialect correctness.** Postgres emits `$1..$n` in first-appearance order;
  MySQL/SQLite emit `?`. A bind pushed for clause K must get the placeholder index
  matching its position in the `binds` vec.
- **`Option::None` → SQL `NULL` bind** (not omitted).
- **Empty `IN ()`** compiles to `1 = 0` (and `NOT IN ()` to `1 = 1`), matching 1.x.

## Tests (M1 acceptance)

Per dialect (pg/mysql/sqlite), `tests/v2_*.rs` (run with `--features v2,...`):

1. SELECT with columns + multiple WHERE → exact SQL string + bind vec + (for pg)
   `$1..$n` ordering.
2. INSERT / UPDATE / DELETE → exact SQL + binds; keys escaped, values bound.
3. Identifier escaping: dotted `t.col`, `*`, and an injection payload neutralized.
4. `IntoBind`: `i64`/`&str`/`String`/`Option<T>`(None→Null)/`bool`/`f64`/bytes
   produce the right `Value` variants and bind order.
5. Empty `IN` / `NOT IN` → `1 = 0` / `1 = 1`.
6. (sqlx feature on) `to_sqlx_query()` compiles and `.sql()` equals `to_sql().0`
   for each enabled dialect. **Compile/string-level only — no running database**
   (Q5); pg path is verified by `cargo build --features v2,sqlx_postgres` + a
   `.sql()` string assertion, not an integration query.
7. Unit tests for `write_placeholder` per dialect.
8. OR-group: `or_where(|q| …)` produces a parenthesized `OR (…)` with correct
   placeholder ordering across the outer + inner predicates (Q3).

Baseline: existing 1.x suite (63 tests) must stay green — v2 is additive behind a
feature, zero changes to `src/*` outside `src/lib.rs` (one `#[cfg(feature="v2")]
pub mod v2;`).

## Acceptance criteria (M1)

- [ ] `cargo build` (default) unchanged; `cargo build --features v2` compiles.
- [ ] `cargo build --features v2,sqlx_postgres` / `,sqlx_mysql` / `,sqlx_sqlite`
      each compile.
- [ ] `cargo test` (default, 1.x) still 63 passing.
- [ ] `cargo test --features v2,sqlx_mysql,sqlx_sqlite,sqlx_postgres` passes the
      new v2 suite.
- [ ] Postgres placeholder ordering verified (`$1..$n`).
- [ ] No `serde_json::Value` in the v2 core WHERE/INSERT signatures.
- [ ] `cargo clippy --features v2` produces no new warnings **in `src/v2/`**.
      Pre-existing 1.x `clippy -D warnings` / `fmt` drift (`method_compiler.rs:197`,
      `types.rs:58`, `src/builder.rs` fmt) is **out of scope** for this plan and is
      not "fixed" here (spec-review R5); the v2 check is scoped to v2 files. `cargo
      fmt` is applied to `src/v2/` and the new tests only.

## Known limitations / deferred (named, not hidden)

- M2–M6 (joins, CTE, union, group/having/order, upsert+RETURNING, typed fetch,
  Postgres jsonb/distinctOn, dynamic when/modify, pagination) are **not** in M1.
  They are v2.0 scope, shipped by later plans.
- `uuid`/`chrono`/`rust_decimal` bind types may land in M1 if cheap, else deferred
  to their own step; the `Value` enum is `#[non_exhaustive]` to allow adding them.
- Postgres `sqlx_postgres` feature wires `sqlx/postgres`; CI/publish workflow
  changes for it are out of scope here.

## Open questions resolved at plan-time

- **Builder ownership style** (by-value chaining vs `&mut self`): plan picks
  by-value (`self`) chaining for v2 ergonomics — decided in Phase 4.
- **`select()` arg type** (`IntoIterator<Item = impl AsRef<str>>`): decided in
  Phase 4.
