# chain-builder v2 — M2 plan: JOIN / CTE / UNION / GROUP BY / HAVING / ORDER BY

Spec: `2026-06-09-chain-builder-v2-query-builder-design.md` (Milestone M2) · Base:
`d313031` (branch `feat/v2-m2`, off `v2`) · TDD throughout · all under
`feature = "v2"`; 1.x untouched.

## Goal

Port the remaining 1.x SELECT-shaping clauses onto the generic `QueryBuilder<D>`:
JOINs, CTEs (`WITH` / `WITH RECURSIVE`), `UNION` / `UNION ALL`, `GROUP BY`,
`HAVING`, `ORDER BY`, `LIMIT` / `OFFSET` — with **placeholder continuity** (pg
`$1..$n` in first-appearance order) across nested sub-queries.

## Foundational refactor (M2-T1, do first)

The current `compile.rs::compile<D>(qb) -> (String, Vec<Value>)` allocates a fresh
`Ctx` each call, so a nested sub-query (CTE body, UNION arm) would restart pg
placeholder numbering at `$1`. Refactor to thread one `Ctx`:

```rust
struct Ctx { sql: String, binds: Vec<Value>, quote: char }
impl Ctx {
    fn esc(&self, ident: &str) -> String { escape_identifier(ident, self.quote) }
    fn placeholder<D: Dialect>(&mut self, v: Value) {
        self.binds.push(v); D::write_placeholder(&mut self.sql, self.binds.len());
    }
}
/// Write `qb`'s SQL into an existing ctx (binds + placeholder counter continue).
fn compile_into<D: Dialect>(ctx: &mut Ctx, qb: &QueryBuilder<D>) { … }
/// Public entry: fresh ctx → compile_into → (sql, binds).
pub fn compile<D: Dialect>(qb: &QueryBuilder<D>) -> (String, Vec<Value>) { … }
```

`quote` lives on `Ctx` (set from `D::quote_char()` once). All existing `esc(col,
quote)` sites become `ctx.esc(col)`. **Behavior must be byte-identical** for every
existing M1 test (run the full M1 suite — must stay green unchanged).

Statement assembly order inside `compile_into` (SELECT):
`[WITH ctes] SELECT cols FROM table [JOINs] [WHERE] [GROUP BY] [HAVING] [ORDER BY]
[LIMIT] [OFFSET] [UNION arms]`. INSERT/UPDATE/DELETE unchanged (no joins/group/etc
in M2).

## Builder additions (`QueryBuilder<D>` new fields, all raw-stored)

```rust
joins:   Vec<Join>,                       // Join { kind: JoinKind, table: String, on: Vec<JoinCond> }
groups:  Vec<String>,                     // raw col idents
havings: Vec<Having>,                     // Having::Expr{sql,binds} | Having::Col{col,op,val}
orders:  Vec<(String, Order)>,            // (raw col, Asc|Desc)
limit:   Option<i64>, offset: Option<i64>,
ctes:    Vec<Cte<D>>,                     // Cte { name: String, recursive: bool, query: QueryBuilder<D> }
unions:  Vec<(bool /*all*/, QueryBuilder<D>)>,
```
(`Vec<QueryBuilder<D>>` provides the heap indirection — no `Box` needed.)

## M2-T1 — refactor + ORDER BY / GROUP BY / LIMIT / OFFSET

- Refactor as above.
- `order_by(col, Order)` (enum `Order { Asc, Desc }`) and `order_by_asc/desc(col)`
  convenience; render `ORDER BY {esc col} ASC|DESC, …`.
- `group_by<I: IntoIterator<Item=impl AsRef<str>>>(cols)`; render
  `GROUP BY {esc c}, …`.
- `limit(n: i64)`, `offset(n: i64)`; render `LIMIT ` + placeholder, `OFFSET ` +
  placeholder (bound as `Value::I64`, uniform across pg/mysql/sqlite — pg gets
  `$n`). **Decision:** bind limit/offset (not inline) for injection-uniformity.
- Tests `tests/v2_clauses.rs`: per-dialect ORDER BY (asc+desc), GROUP BY (dotted +
  multi), LIMIT+OFFSET placeholder numbering after a WHERE (pg: WHERE `$1` →
  LIMIT `$2` OFFSET `$3`).

## M2-T2 — JOINs

- `enum JoinKind { Inner, Left, Right, FullOuter, Cross }` → keywords
  `INNER JOIN`/`LEFT JOIN`/`RIGHT JOIN`/`FULL OUTER JOIN`/`CROSS JOIN`.
- Builder: `join(table, |j| …)`, `left_join`, `right_join`, `full_outer_join`,
  `cross_join`. Closure gets a `JoinClause` exposing `on(col, op, col2)` (both
  idents escaped), `on_val(col, op, impl IntoBind)` (col escaped, value bound),
  `on_raw(sql, Vec<Value>)` (verbatim). Multiple `on` joined by `AND`.
- Render: `{KIND} {esc table} ON {cond AND cond}` (CROSS JOIN emits no `ON` if
  no conds). Placeholders from `on_val` continue the counter.
- Tests: pg/mysql/sqlite inner+left join with `on`; `on_val` placeholder ordering
  relative to WHERE; dotted table+cols escaped.

## M2-T3 — HAVING

- `having(col, op, impl IntoBind)` → escapes `col`, binds value: `{esc col} {op}
  {ph}`.
- `having_raw(sql, Vec<Value>)` → verbatim expression (for `COUNT(*) > ?`),
  documented escape hatch. Multiple HAVING joined by `AND`.
- Render after GROUP BY: `HAVING …`.
- Tests: `having_raw("COUNT(*) > ?", …)` + `having("total", ">", …)` per dialect;
  placeholder continuity GROUP BY has none, so HAVING `$n` continues from WHERE.

## M2-T4 — CTE (WITH) + UNION

- `with(name, query)`, `with_recursive(name, query)` → push `Cte`. Render
  `WITH [RECURSIVE] {esc name} AS ({sub}), … ` BEFORE the main SELECT, compiling
  each sub via `compile_into` so binds/placeholders continue (CTE binds appear
  first → pg numbers them `$1..` before the main query's). If any cte is
  recursive, the single `WITH` carries `RECURSIVE` (SQL: `WITH RECURSIVE a AS
  (...), b AS (...)`).
- `union(query)`, `union_all(query)` → push `(all, query)`. Render after the main
  query: ` UNION [ALL] {sub}` per arm, via `compile_into` (placeholders continue).
- Tests (the crux): pg statement with a CTE bind + main WHERE bind + UNION-arm
  bind → assert exact `$1`(cte) `$2`(main) `$3`(union) ordering and the bind vec
  matches; mysql/sqlite `?` equivalents; `WITH RECURSIVE`.

## Out of M2 (deferred)

`DISTINCT ON`/jsonb/ILIKE (M5), upsert+RETURNING (M3), typed fetch (M4),
when/modify+pagination helpers (M6), lateral joins, `UNION` of differing dialects
(N/A — same `D`).

## Acceptance

- M1 suite green **unchanged** after the refactor (byte-identical SQL).
- New `tests/v2_clauses.rs` + `tests/v2_join.rs` + `tests/v2_cte_union.rs` pass for
  pg (default+`sqlx_postgres`), mysql, sqlite (isolated `--no-default-features`).
- Per single sqlx backend (1.x multi-sqlx limitation): `cargo test --features
  v2,sqlx_postgres`, `…,sqlx_mysql`, `--no-default-features …,v2,sqlite,sqlx_sqlite`.
- 1.x baseline still **63**; no new `src/v2` clippy warnings.
- **Placeholder continuity across CTE→main→UNION verified** (pg `$1..$n` first
  appearance) — the load-bearing invariant.
