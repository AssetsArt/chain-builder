# Internals

How a `QueryBuilder` becomes `(sql, binds)`. You do not need this page to use
the library — it exists so that the guarantees in the
[Security Model](security.md) are checkable claims about a small amount of
code, not marketing.

## Scope note: the public AST is documented on docs.rs

Builder methods do not render SQL; they accumulate a small AST. Those AST
types are publicly re-exported from the crate root — `Predicate`, `Conj`,
`Having`, `OnConflict`, `ConflictAction`, `Cte`, `Join`, `JoinCond`,
`JoinKind`, `SelectExpr`, `AggFn`, `Lock`, `LockStrength`, `LockWait`,
`Method`, `Order`, and friends — so advanced callers can inspect or construct
queries structurally. They are deliberately **not** documented page-by-page in
this book: they are an advanced API whose authoritative reference is the
rustdoc on [docs.rs](https://docs.rs/chain-builder). Everything below
describes how the compiler consumes them.

## The single-pass compiler

Compilation lives in `src/compile.rs`. One context struct is threaded through
the entire walk:

```rust,ignore
struct Ctx {
    sql: String,       // the SQL text, appended left to right
    binds: Vec<Value>, // bind values, pushed as their placeholder is written
    quote: char,       // dialect quote char ('"' or '`')
}
```

`try_compile` creates one `Ctx` and calls `compile_into(&mut ctx, qb)`; nested
builders — CTE bodies, UNION arms, subqueries in `where_exists` /
`where_in_subquery` / `select_subquery` — are compiled by recursive
`compile_into` calls **on the same `Ctx`**. There is no second pass, no
renumbering step, no fragment stitching. The invariant that falls out:

> **SQL text order == bind push order.** A placeholder is written at the
> moment its value is pushed, and its number is simply `binds.len()` after
> the push.

That invariant *is* the Postgres `$N` continuity guarantee. Clauses are
emitted in a fixed order — CTE bodies first, then the SELECT list (including
subquery columns), JOIN conditions, WHERE, GROUP BY, HAVING, ORDER BY, LIMIT/OFFSET, then UNION
arms — and the counter just keeps running across all of them:

```rust,ignore
let recent = QueryBuilder::<Postgres>::table("logs")
    .select(["n"])
    .where_gt("n", 100i64);          // $1 — CTE body compiles first

let (sql, binds) = QueryBuilder::<Postgres>::table("recent")
    .with("recent", recent)
    .where_gt("n", 200i64)           // $2 — main WHERE
    .limit(10)                       // $3 — LIMIT binds its value
    .offset(20)                      // $4 — so does OFFSET
    .to_sql();
// WITH "recent" AS (SELECT "n" FROM "logs" WHERE "n" > $1)
//   SELECT * FROM "recent" WHERE "n" > $2 LIMIT $3 OFFSET $4
// binds == [I64(100), I64(200), I64(10), I64(20)]  — same order as $1..$4
```

On MySQL/SQLite the placeholders are all `?`, where only the order matters —
and the order is correct by the same invariant.

This is also why every raw escape hatch carries the "hand-write the correct
`$N`" contract: a raw fragment's text is spliced into `ctx.sql` and its binds
are appended to `ctx.binds`, but the compiler cannot see placeholders inside
the fragment, so it cannot renumber them.

## The `ctx.esc` chokepoint

`Ctx` has one method for turning an identifier into SQL:

```rust,ignore
fn esc(&self, ident: &str) -> String {
    escape_identifier(ident, self.quote)   // src/ident.rs
}
```

Every identifier→SQL site in the compiler — select columns, aliases,
aggregate arguments, table names, `.db()` qualifiers, JOIN columns, WHERE
columns, GROUP BY, HAVING, ORDER BY, CTE names, conflict targets, RETURNING
columns, INSERT/UPDATE column lists — routes through it, so
`grep 'esc(' src/compile.rs` is the complete inventory of identifier writes.
The only sites that bypass it are the six raw escape hatches, which are
verbatim **by documented design** (see the
[escape-hatch inventory](security.md#escape-hatch-inventory-complete)).

## Deferred-error flow

Builder methods never panic mid-chain, and never return `Result` — that would
break chaining. Misuse detected at *build* time (today: `having()` with a
disallowed operator) is recorded on the builder, **first error wins**: a later
mistake must not mask the one that points at the original misuse.

```rust,ignore
// src/builder.rs — inside having():
if self.error.is_none() {
    self.error = Some(BuildError::InvalidHavingOperator(op.to_owned()));
}
return self; // chain continues; the error travels with the builder
```

The very first thing `compile_into` does is check that slot — and because
nested builders are compiled through the same function, an error recorded on
a CTE body, UNION arm, or subquery propagates out of the parent compilation
too:

```rust,ignore
fn compile_into<D: Dialect>(ctx: &mut Ctx, qb: &QueryBuilder<D>) -> Result<(), BuildError> {
    if let Some(e) = &qb.error {
        return Err(e.clone());
    }
    // ...
}
```

Misuse only detectable at *compile* time (offset without limit,
`DISTINCT ON` off Postgres, lock on non-SELECT, lock + UNION, empty
INSERT/UPDATE) returns its `BuildError` from the same walk. Either way the
surfacing is uniform: `try_compile` / `try_to_sql` return `Err`, and the
panicking twins `compile` / `to_sql` panic with exactly the error's `Display`
text — the two paths cannot drift. See
[Error Handling](error-handling.md).

## Determinism

The compiler is deterministic down to the byte — the same builder always
yields the same SQL string, which is what lets the test suite assert exact
SQL and lets you diff generated queries across versions.

Two places need explicit work to get there:

- **Sorted columns.** `insert` / `insert_many` / `update` take key–value
  collections whose iteration order the caller does not control, so the
  compiler sorts column names alphabetically before rendering. For
  `insert_many`, the column list comes from the **first** row's sorted keys;
  a key missing from a later row binds `Value::Null` (ragged rows are
  NULL-padded, never a panic).

  ```rust,ignore
  // INSERT INTO "users" ("age", "email", "name") VALUES ($1, $2, $3)
  // — always this column order, regardless of call order.
  ```

- **Single `WITH` header, one `RECURSIVE`.** All CTEs render in one
  `WITH … AS (…), … AS (…)` clause, in registration order. If *any* CTE was
  added with `with_recursive`, the single header is promoted to
  `WITH RECURSIVE` — the keyword appears once and covers the whole list,
  matching how SQL grammars define it.

## Related pages

- [Security Model](security.md) — what these mechanics guarantee.
- [Error Handling](error-handling.md) — the `BuildError` catalog and HTTP mapping.
- [CTE & UNION](query/cte-union.md) — user-facing bind-ordering notes.
- [docs.rs/chain-builder](https://docs.rs/chain-builder) — rustdoc for the AST types.
