# chain-builder v2 — M4–M7 completion plan

Base: `ef7c48f` (`v2`). Each milestone = one PR into `v2`. TDD; `feature = "v2"`;
1.x untouched; M1–M3 SQL byte-identical (new fields default empty). Per-single-
sqlx-backend testing (1.x multi-sqlx limitation).

## M4 — Typed fetch (execution helpers)

On `impl<D: SqlxDialect> QueryBuilder<D>` (in `src/v2/fetch.rs`, gated like
`sqlx_bind`). Thin async delegations to the sqlx query objects:
- `fetch_all::<T>(executor)`, `fetch_one::<T>(executor)`,
  `fetch_optional::<T>(executor)` where `T: for<'r> FromRow<'r, <D::Database as
  Database>::Row> + Send + Unpin`, `executor: Executor<'e, Database=D::Database>`.
  → delegate to `self.to_sqlx_query_as::<T>().fetch_*`.
- `execute(executor) -> QueryResult` → `self.to_sqlx_query().execute`.
- `count(executor) -> i64` — wrap: `SELECT COUNT(*) FROM (<sql>) AS __c`, bind the
  same args, `fetch_one` scalar `i64` (mirrors 1.x `count`).
- `fetch_scalar::<T>(executor) -> T` and `fetch_optional_scalar::<T>` — first
  column of first row (for pluck/aggregate use).
**Tests:** compile-tests per sqlx feature — a `#[derive(sqlx::FromRow)]` row +
an `async fn` (never run, `#[allow(dead_code)]`) that calls every helper, proving
the generic plumbing typechecks for pg/mysql/sqlite. (Live DB integration =
deferred to a CI/runtime task — needs a sqlx runtime feature + tokio; named, not
hidden.)

## M5 — Postgres parity extras

- `distinct(self)` — `SELECT DISTINCT …` (all dialects).
- `distinct_on(self, cols)` — pg `SELECT DISTINCT ON (cols) …`. Capability:
  `Dialect::supports_distinct_on() -> bool` (default false; Postgres true). On a
  dialect without it → `panic!("DISTINCT ON requires PostgreSQL")` (explicit).
- `where_ilike(col, val)` — dialect-aware: pg `{col} ILIKE {ph}`; mysql/sqlite
  `LOWER({col}) LIKE LOWER({ph})`. Capability `Dialect::ilike_is_native() -> bool`
  (pg true). Add `Predicate::ILike{col,val}` (col escaped; bound val) OR build via
  the dialect at construction. Use a `Predicate` variant so placeholder ordering
  is correct.
- jsonb (pg-oriented, emitted verbatim operator): `where_jsonb_contains(col, json)`
  → `{esc col} @> {ph}` (bind json text/Value::Json). Keep minimal; advanced json
  paths deferred.
**Tests:** distinct/distinct_on (pg + panic on mysql), ilike per dialect,
jsonb_contains (pg). M1–M3 unchanged.

## M6 — Dynamic building + pagination

- `when(self, cond: bool, f: impl FnOnce(Self) -> Self) -> Self` — apply `f` only
  if `cond`. `when_else(cond, f_true, f_false)`.
- `paginate(self, page: i64, per_page: i64) -> Self` — `limit(per_page)
  .offset((page-1).max(0) * per_page)` (1-based page). Document.
**Tests:** `when(true/false, …)` toggles a where; `paginate(2, 10)` → `LIMIT ?
OFFSET ?` binds `[10, 10]`. Pure builder — low risk.

## M7 — Fill remaining gaps

- **Multi-row insert:** `insert_many<rows>(self, rows)` where a row is the same
  `(col, val)` shape; all rows must share the first row's (sorted) key set; compile
  `INSERT INTO t (cols) VALUES (…), (…), …`. Missing key in a later row → bind
  `Value::Null` (no panic — DoS-safe, matches 1.x hardening). Composes with
  on_conflict/returning.
- **Raw group/order:** `group_by_raw(sql, binds)`, `order_by_raw(sql, binds)` —
  verbatim (pg `$N` caller-responsibility, same Warning as `where_raw`).
**Tests:** insert_many (single + multi-row + ragged→NULL) per dialect; group/order
raw. M1–M3 unchanged.

## Final
- v2 module-level doc / short `docs/v2.md` usage guide (optional, time-permitting).
- Honest wrap-up: what shipped, what's deferred (live fetch integration, uuid/
  chrono/decimal Value variants, json path ops, named-constraint upsert targets).

## Cross-cutting acceptance (every milestone)
- M1–M3 byte-identical (no new field set by default).
- 1.x **63**; per-backend v2 suites green; no new `src/v2` clippy.
