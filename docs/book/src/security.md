# Security Model

chain-builder is built so that the *default* path — structured builder methods
with values passed as Rust arguments — is injection-safe by construction, and
every deviation from that path is an explicit, named escape hatch. This page
states the guarantees precisely, inventories every escape hatch and its
contract, and is honest about what the library does **not** protect.

## Guarantees

### 1. Values are always bound, never inlined

Every value — `where_eq`, `insert`, `having`, `limit`, `offset`, everything —
pushes onto the running bind vector and emits a placeholder (`$N` on
Postgres, `?` on MySQL/SQLite). The compiler (`src/compile.rs`) has no code
path that interpolates a value into the SQL string. A malicious value can
therefore never change the *shape* of the query — it travels to the database
as data, in the driver's argument buffer.

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .where_eq("name", "'; DROP TABLE users; --")
    .to_sql();
// SELECT "id" FROM "users" WHERE "name" = $1
// binds == [Value::Text("'; DROP TABLE users; --".into())]
// The payload is inert: it is compared against the column, not executed.
```

See [Binds & Values](binds.md) for the full `Value`/`IntoBind` story.

### 2. Identifiers are always escaped — through one chokepoint

Identifiers (table names, column names, aliases) *are* interpolated into the
SQL text — there is no way to bind an identifier. So every identifier→SQL
site in the compiler routes through a single function, `ctx.esc`, which calls
`escape_identifier` (`src/ident.rs`):

- The identifier is quoted with the dialect's quote character (`"` for
  Postgres/SQLite, `` ` `` for MySQL).
- Any embedded quote character is **doubled** — the standard, injection-safe
  way to neutralize an attempt to terminate the quoted identifier early.
- Dotted paths are split and quoted segment-by-segment: `db.table.col` →
  `"db"."table"."col"`.
- A bare `*` segment passes through unquoted, so `t.*` → `"t".*` and `*`
  stays `*`.

A breakout attempt stays trapped inside one (nonexistent) identifier:

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .select([r#"id" ; DROP TABLE users; --"#])
    .to_sql();
// SELECT "id"" ; DROP TABLE users; --" FROM "users"
// The doubled quote keeps the whole payload inside ONE quoted identifier.
// The query fails (no such column) instead of executing the injection.
```

### 3. `having()` validates its operator against an allowlist

`having(col, op, val)` is the one builder method that takes its operator as a
runtime `&str` (for ergonomics). Because the operator is emitted verbatim —
it is not a value and cannot be escaped without changing its meaning — an
attacker-controlled operator would be an injection vector. So `having()`
checks `op` against a fixed allowlist (`=`, `!=`, `<>`, `>`, `>=`, `<`, `<=`,
`LIKE`, `NOT LIKE`, matched case-insensitively, stored trimmed). A disallowed
operator records a **deferred** `BuildError::InvalidHavingOperator` on the
builder — the chain stays intact, `try_to_sql()` returns `Err`, `to_sql()`
panics with the same message. See
[Error Handling](error-handling.md) for the deferred-error mechanics.

```rust,ignore
let err = QueryBuilder::<Postgres>::table("orders")
    .select(["user_id"])
    .having("amount", ">= 0 UNION SELECT password FROM users --", 0i64)
    .try_to_sql()
    .unwrap_err();
// Err(BuildError::InvalidHavingOperator(...)) — never reaches the SQL.
```

### 4. Every other operator parameter is `op: &'static str`

`where_column(lhs, op, rhs)`, `JoinClause::on(col, op, col2)` and
`JoinClause::on_val(col, op, val)` take `op: &'static str`. A `&'static str`
can only come from a compile-time literal (or a deliberate leak), so an
operator string assembled from request input simply does not type-check.
The injection surface that `having()` guards with an allowlist is closed at
the type level everywhere else.

## Escape-hatch inventory (complete)

Seven methods accept raw SQL. All seven share the same contract:

> The fragment is emitted **verbatim** — not escaped, not validated, not
> renumbered. Its binds are appended to the running bind list in order. On
> **Postgres** the caller must hand-write `$N` placeholders matching the
> actual bind position (number of binds already accumulated `+ 1`, `+ 2`, …);
> on MySQL/SQLite use `?`. A wrong `$N` produces a malformed query.
> **The caller owns the security audit of the fragment.**

| Method | Where the fragment lands |
|---|---|
| `select_raw(sql, Option<Vec<Value>>)` | SELECT column list, after structured columns |
| `where_raw(sql, Vec<Value>)` | WHERE clause, as one predicate (also available on the group builder inside `and_where`/`or_where`) |
| `group_by_raw(sql, Vec<Value>)` | GROUP BY, after structured columns |
| `order_by_raw(sql, Vec<Value>)` | ORDER BY, after structured terms |
| `having_raw(sql, Vec<Value>)` | HAVING, as one term |
| `set_raw(col, sql, Vec<Value>)` | UPDATE SET, as `"col" = <fragment>` after the structured columns (the column identifier IS escaped; only the right-hand side is verbatim) |
| `JoinClause::on_raw(sql, Vec<Value>)` | JOIN … ON, as one condition |

Never feed request input — even "just a column name" — into a raw fragment.
If a raw fragment must vary at runtime, vary it by selecting between
hard-coded fragments, and pass every value through `binds`:

```rust,ignore
// OK: fragment is a compile-time literal, the value is bound.
.having_raw("COUNT(*) > $1", vec![Value::I64(5)])

// NEVER: user_input is interpolated into SQL verbatim.
.where_raw(&format!("{} = $1", user_input), vec![v])
```

## What is NOT protected

**Raw fragments.** Everything passed to the six methods above. The library
deliberately does not parse, validate, or escape them.

**LIKE / ILIKE wildcards in user input.** `where_like` / `where_ilike` bind
the pattern safely — it cannot inject SQL — but `%` and `_` *inside the bound
value* keep their wildcard meaning. A user searching for `%` matches every
row. This is a correctness/DoS concern, not injection; escape the wildcards
yourself if it matters. See
[Case-insensitive Search](cookbook/search.md).

**Identifier *names* from untrusted input.** Escaping guarantees an
attacker-controlled column name cannot break out of the identifier context —
but it is still *used as an identifier*. A caller that passes
`order_by(user_supplied, …)` lets users probe for column existence, sort by
columns they should not see, or trigger errors. Escaping neutralizes
injection; it is not an authorization policy. Allowlist the names yourself:

```rust,ignore
const SORTABLE: &[&str] = &["created_at", "name", "total"];

let col = SORTABLE
    .iter()
    .find(|c| **c == requested_sort)
    .copied()
    .unwrap_or("created_at"); // or reject with 400

let qb = QueryBuilder::<Postgres>::table("orders")
    .select(["id", "total"])
    .order_by_desc(col);
```

## Where this is tested

The guarantees above are pinned by the crate's own test suite:

- `tests/having_guard.rs` — the operator allowlist: all allowed operators
  compile, case-insensitive matching, and two injection attempts via the
  operator (`>= 0 UNION SELECT …`, `; DROP TABLE …`) fail loud.
- `tests/select.rs` (`injection_column_is_neutralized`) — a quote-breakout
  payload in a column name stays inside one quoted identifier.
- `tests/db.rs` — injection through the `.db()` database qualifier is
  neutralized the same way.
- `src/ident.rs` unit tests — quote-doubling for both `"` and `` ` ``,
  dotted-path segmentation, `*` passthrough.

## Related pages

- [Binds & Values](binds.md) — how values travel to the driver.
- [Error Handling](error-handling.md) — deferred errors, HTTP mapping.
- [Internals](internals.md) — the `ctx.esc` chokepoint inside the compiler.
