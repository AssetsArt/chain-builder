# chain-builder v2 — M1 Foundation implementation plan

Spec: `2026-06-09-chain-builder-v2-query-builder-design.md` · Base: `f22391d` ·
Branch: `feat/v2-foundation`

TDD throughout: write the test(s) named in each task, watch them fail, implement,
watch them pass. All v2 code lives under `src/v2/` behind `feature = "v2"`; 1.x is
untouched except one line in `src/lib.rs`.

## Spec → task coverage

| Spec section | Task |
|---|---|
| Cargo features (`v2`, `json`, `sqlx_postgres`); `lib.rs` wiring | T0 |
| `Value` enum + `IntoBind` | T1 |
| `Dialect` trait + markers; `ident::escape_identifier` | T2 |
| `QueryBuilder<D>` SELECT + WHERE (+ OR group, empty IN, escaping) | T3 |
| INSERT / UPDATE / DELETE | T4 |
| `SqlxDialect` + `to_sqlx_query` / `_as` | T5 |
| Acceptance criteria, full suite green | T6 (orchestrator verify) |

## Conventions

- Builder uses **by-value (`self`) chaining** (resolved open question): every
  mutator is `fn x(mut self, …) -> Self`. Terminal: `to_sql(&self)`,
  `to_sqlx_query(&self)`.
- `select()` / column lists accept `IntoIterator<Item = impl AsRef<str>>`.
- Bind values accept `impl IntoBind`; `where_in` accepts
  `IntoIterator<Item = impl IntoBind>`.
- Module layout exactly as the spec's "High-level architecture" tree.

---

## T0 — Feature wiring & module skeleton

**Files:** `Cargo.toml`, `src/lib.rs`, `src/v2/mod.rs`

1. `Cargo.toml` `[features]`, additive only (do NOT touch `default`, do NOT add
   `dep:`):
   ```toml
   v2            = []
   json          = ["v2"]
   sqlx_postgres = ["sqlx/postgres"]
   ```
   (`sqlx_mysql`, `sqlx_sqlite` already exist.)
2. `src/lib.rs`: add near the other module decls:
   ```rust
   #[cfg(feature = "v2")]
   pub mod v2;
   ```
3. `src/v2/mod.rs`: declare submodules + re-exports (stub modules created in later
   tasks; for T0 just create `mod.rs` with `pub mod value;` etc. added as files
   land — for T0 create an empty-but-compiling `mod.rs` and a placeholder so
   `--features v2` builds). Minimal T0 content:
   ```rust
   //! chain-builder v2 — typed, dialect-aware query builder (feature = "v2").
   pub mod value;
   pub mod ident;
   pub mod dialect;
   pub mod builder;
   #[cfg(any(feature = "sqlx_mysql", feature = "sqlx_sqlite", feature = "sqlx_postgres"))]
   pub mod sqlx_bind;

   pub use builder::QueryBuilder;
   pub use dialect::{Dialect, MySql, Postgres, Sqlite};
   pub use value::{IntoBind, Value};
   ```
   Since later modules don't exist yet, T0 lands the module files as **minimal
   compiling stubs** (each with at least the public type the `pub use` needs), and
   subsequent tasks fill them in. The implementer creates all five files as
   compiling stubs in T0 so `cargo build --features v2` is green.

**Verify:**
```
cargo build                                   # 1.x default, unchanged
cargo build --features v2                      # compiles (stubs)
cargo test 2>&1 | tail -1                       # 1.x: 63 passed
```
**BLOCKED fallback:** if stub cross-references cause cycles, make `mod.rs`
re-export only types that exist; add `pub use` lines as each task lands.

---

## T1 — `Value` + `IntoBind`

**File:** `src/v2/value.rs` · **Test:** inline `#[cfg(test)] mod tests`

`Value` enum (`#[non_exhaustive]`, `Debug, Clone, PartialEq`):
`Null, Bool(bool), I64(i64), F64(f64), Text(String), Bytes(Vec<u8>)`, plus
`#[cfg(feature = "json")] Json(serde_json::Value)`.

`pub trait IntoBind { fn into_bind(self) -> Value; }` with impls:
- integers `i8 i16 i32 i64 u8 u16 u32` → `I64` (via `as i64` / `i64::from`);
  `u64`/`usize`/`isize` → `I64` (document narrowing; clamp not required for M1).
- `f32 f64` → `F64`.
- `bool` → `Bool`.
- `&str`, `String`, `&String` → `Text`.
- `Vec<u8>`, `&[u8]` → `Bytes`.
- `Value` → itself (identity).
- `Option<T: IntoBind>` → `None ⇒ Null`, `Some(v) ⇒ v.into_bind()`.
- `#[cfg(feature = "json")] serde_json::Value` → `Json`.

**Tests (write first):**
1. each primitive maps to the right variant (`30i64`, `"x"`, `true`, `1.5f64`,
   `vec![1u8,2]`).
2. `Option::<i64>::None.into_bind() == Value::Null`; `Some(5)` → `I64(5)`.
3. `String` and `&str` both → `Text`.
4. `Value::I64(1).into_bind() == Value::I64(1)` (identity).

**Verify:** `cargo test --features v2 value:: 2>&1 | tail -1`

---

## T2 — `Dialect` trait, markers, `ident::escape_identifier`

**Files:** `src/v2/dialect/mod.rs`, `dialect/postgres.rs`, `dialect/mysql.rs`,
`dialect/sqlite.rs`, `src/v2/ident.rs`

`Dialect` (base, no sqlx) exactly as spec. Markers `Postgres`, `MySql`, `Sqlite`
(unit structs, `Debug, Clone, Copy`):
- `quote_char`: `` ` `` for MySql, `"` for Postgres & Sqlite.
- `write_placeholder(out, n)`: MySql/Sqlite push `'?'` (ignore `n`); Postgres
  `out.push('$'); out.push_str(&n.to_string());`.
- `supports_returning`: Postgres/Sqlite `true`, MySql `false`.

`ident.rs`: port `escape_identifier` from 1.x `src/dialect.rs` **verbatim logic**,
signature `pub fn escape_identifier(ident: &str, quote: char) -> String` (split on
`.`, `*` passthrough, double the quote char, trim, empty→empty). Port the 1.x unit
tests, adapting calls to pass `'`'`/`'"'`.

**Tests (write first):**
1. `write_placeholder`: Postgres `$1`,`$2`; MySql/Sqlite `?`,`?`.
2. `escape_identifier("users.name", '`') == "`users`.`name`"`; `"*"`→`*`;
   `"t.*"`→`` `t`.* ``; injection `name`+backtick doubled.
3. double-quote variant for `"` (sqlite/pg).

**Verify:** `cargo test --features v2 dialect:: ident:: 2>&1 | tail -1`

---

## T3 — `QueryBuilder<D>`: SELECT + WHERE

**Files:** `src/v2/builder.rs`, `src/v2/where_.rs`, `src/v2/compile.rs`

`QueryBuilder<D: Dialect>` holds: `table: String`, `select_cols: Vec<String>`
(empty ⇒ `*`), `wheres: Vec<Predicate>`, `method: Method` (enum
`Select|Insert|Update|Delete`, default Select), plus insert/update payload fields
(filled in T4). `PhantomData<D>`.

`Predicate` (in `where_.rs`): enum covering
`Binary{col, op: &'static str, val: Value}`,
`In{col, neg: bool, vals: Vec<Value>}`,
`Null{col, neg: bool}`,
`Between{col, lo: Value, hi: Value}`,
`Raw{sql: String, binds: Vec<Value>}`,
`Group{conj: Conj, preds: Vec<Predicate>}` (Conj = And|Or).

Builder WHERE methods (each `mut self … -> Self`): `where_eq/ne/gt/gte/lt/lte`
(→ `Binary` with `= != > >= < <=`), `where_like` (`LIKE`), `where_in/not_in`
(→ `In`), `where_null/not_null` (→ `Null`), `where_between` (→ `Between`),
`where_raw(sql, binds: Vec<Value>)`, `and_where(|w| …)` / `or_where(|w| …)` where
the closure gets a `WhereBuilder<D>` accumulating into a `Group`.

`compile.rs::compile(qb) -> (String, Vec<Value>)`:
- A `Ctx { sql: String, binds: Vec<Value>, quote: char }` plus a placeholder
  counter derived from `binds.len()+1` fed to `D::write_placeholder`.
- SELECT: `SELECT <cols|*> FROM <table>` with each col + table through
  `escape_identifier`; then ` WHERE ` + compiled predicate list joined by ` AND `.
- Predicate compilation:
  - `Binary`: `<col> <op> <ph>` push val.
  - `In`: empty vals ⇒ `1 = 0` (neg ⇒ `1 = 1`); else `<col> [NOT ]IN (<ph,…>)`.
  - `Null`: `<col> IS [NOT ]NULL`.
  - `Between`: `<col> [NOT ]BETWEEN <ph> AND <ph>`.
  - `Raw`: push sql verbatim + extend binds.
  - `Group`: `(<inner joined by AND/OR>)`.
- Placeholder ordering: the counter increments as each bind is pushed, so pg gets
  `$1..$n` in first-appearance order across nested groups (Tests item 8).

`to_sql(&self) -> (String, Vec<Value>)` calls `compile`.

**Tests (write first, `tests/v2_select.rs`, `--features v2`):** per spec Tests
items 1,3,5,8 — exact SQL + binds for Postgres (`$n`), MySql (backtick+`?`),
Sqlite (`"`+`?`); empty IN→`1 = 0`; OR group placeholder ordering; dotted/`*`/
injection escaping.

**Verify:** `cargo test --features v2 --test v2_select 2>&1 | tail -1`
**BLOCKED fallback:** if `and_where`/`or_where` closure borrow fighting the
by-value builder is awkward, give `WhereBuilder<D>` its own `Vec<Predicate>` and
have the closure return/accumulate it, then wrap into `Group` — don't thread the
whole `QueryBuilder` into the closure.

---

## T4 — INSERT / UPDATE / DELETE

**Files:** `src/v2/builder.rs`, `src/v2/compile.rs` (extend)

- `insert(rows)` where a row is `IntoIterator<Item = (impl AsRef<str>, impl
  IntoBind)>`; store `Vec<(String, Value)>` (single row for M1; multi-row deferred
  to M-later — name it). Sets `method = Insert`.
- `update(set)` same shape → `method = Update` (WHERE applies).
- `delete()` → `method = Delete` (WHERE applies).
- compile:
  - Insert: `INSERT INTO <t> (<cols>) VALUES (<ph,…>)`, cols escaped, keys **sorted**
    for determinism (match 1.x behavior), values bound.
  - Update: `UPDATE <t> SET <col> = <ph>, …` + WHERE.
  - Delete: `DELETE FROM <t>` + WHERE.

**Tests (write first, `tests/v2_crud.rs`):** spec Tests item 2 per dialect —
exact SQL + binds; keys escaped, values bound; UPDATE/DELETE carry WHERE.

**Verify:** `cargo test --features v2 --test v2_crud 2>&1 | tail -1`

---

## T5 — `SqlxDialect` + sqlx handoff

**File:** `src/v2/sqlx_bind.rs` (gated
`any(sqlx_mysql, sqlx_sqlite, sqlx_postgres)`)

`pub trait SqlxDialect: Dialect { type Database: sqlx::Database; fn
bind_arguments(binds: &[Value]) -> <Self::Database as
sqlx::Database>::Arguments<'static>; }`

Impls (each behind its feature), mirroring 1.x `value_to_arguments` (match each
`Value` variant → `arguments.add(...)`, `Null` → `Option::<String>::None`,
discard the infallible `add` error with a documenting `let _ =` per spec R2):
- `#[cfg(feature="sqlx_postgres")] impl SqlxDialect for Postgres { type Database =
  sqlx::Postgres; … }`
- `#[cfg(feature="sqlx_mysql")] … MySql = sqlx::MySql`
- `#[cfg(feature="sqlx_sqlite")] … Sqlite = sqlx::Sqlite`

On `QueryBuilder<D> where D: SqlxDialect`:
- `to_sqlx_query(&self) -> sqlx::query::Query<'_, D::Database, …Arguments>`:
  `let (sql, binds) = self.to_sql(); sqlx::query_with(sqlx::AssertSqlSafe(sql),
  D::bind_arguments(&binds))`.
- `to_sqlx_query_as::<T>(&self) -> QueryAs<…>` analogously with `query_as_with`.

**Tests (write first, `tests/v2_sqlx.rs`, gated per feature):** spec Tests item 6 —
`to_sqlx_query().sql() == to_sql().0` for each enabled dialect (string/compile
level, no DB).

**Verify (compile-only for pg):**
```
cargo build --features v2,sqlx_postgres
cargo test --features v2,sqlx_mysql,sqlx_sqlite,sqlx_postgres --test v2_sqlx 2>&1 | tail -1
```
**BLOCKED fallback:** if `Arguments<'static>` lifetime fights pg (it differs from
the sqlite-lifetime story in 1.x), bind into an owned `PgArguments` exactly as 1.x
does for MySql (`MySqlArguments::default()` + `add`), which is `'static`-friendly.

---

## T6 — Orchestrator final verification (Phase 6 gate)

> **Correction (found at verify time):** the combined
> `--features v2,sqlx_mysql,sqlx_sqlite,sqlx_postgres` command does **not**
> compile — a **pre-existing 1.x defect**: `ChainBuilder::to_sqlx_query` is defined
> in *both* `src/sqlx_mysql.rs` and `src/sqlx_sqlite.rs`, so enabling ≥2 sqlx
> backends at once = duplicate definitions. This is orthogonal to v2. Verify v2 sqlx
> **per single backend** (sqlite needs `--no-default-features` because `default`
> pulls in `sqlx_mysql`):

```
cargo build                                                   # 1.x default
cargo build --features v2
cargo build --features v2,sqlx_postgres
cargo test 2>&1                                               # 63 (1.x baseline)
cargo test --features v2,sqlx_postgres                         # core + pg sqlx
cargo test --features v2,sqlx_mysql                            # core + mysql sqlx
cargo test --no-default-features --features v2,sqlite,sqlx_sqlite  # core + sqlite sqlx
cargo clippy --features v2 2>&1 | grep 'src/v2'               # no new v2 warnings
```
Acceptance checklist from the spec must be ✓ or the gap is named in the wrap-up.

## Out of M1 (deferred, named)

Joins, CTE, UNION, GROUP BY/HAVING/ORDER BY, upsert+RETURNING, typed
`fetch_*`/`pluck`/scalar, Postgres jsonb/`DISTINCT ON`/ILIKE, `when`/`modify`,
pagination, multi-row insert, uuid/chrono/decimal `Value` variants. These are
M2–M6 per the spec and ship in later plans.
