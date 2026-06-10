# WHERE

Predicates are the heart of the builder: every `where_*` method records a
typed predicate, and every value you pass becomes a **bound parameter** — never
text spliced into the SQL. Consecutive top-level predicates are joined with
`AND`; `OR` and explicit parentheses come from the
[group closures](#groups-and_where--or_where) below. This page lists every
predicate method, the empty-`IN` semantics, the dialect-aware ones
(`where_ilike`, `where_jsonb_contains`), subquery predicates, and the
`where_raw` escape hatch.

## Comparison predicates

`where_eq`, `where_ne`, `where_gt`, `where_gte`, `where_lt`, `where_lte` emit
`col = ?`, `col != ?`, `col > ?`, `col >= ?`, `col < ?`, `col <= ?`
respectively. `where_like` emits `col LIKE ?`:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("t")
    .where_ne("a", 1i64)
    .where_gte("b", 2i64)
    .where_lt("c", 3i64)
    .where_lte("d", 4i64)
    .where_like("e", "%x%")
    .to_sql();
// SELECT * FROM "t" WHERE "a" != $1 AND "b" >= $2 AND "c" < $3 AND "d" <= $4 AND "e" LIKE $5
```

> **⚠️ `where_like` caveat** — the pattern is bound as a value (no SQL
> injection possible), but SQL `LIKE` wildcards `%` and `_` inside user input
> are **NOT escaped**. A user who types `%` matches everything. If you build
> patterns from untrusted input, escape the wildcards yourself — see
> [Case-insensitive Search](../cookbook/search.md).

## `where_in` / `where_not_in`

Take any iterable of bindable values and emit one placeholder per element:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["id", "name"])
    .where_eq("status", "active")
    .where_in("role", ["admin", "staff"])
    .where_gt("age", 18i64)
    .to_sql();
// SELECT "id", "name" FROM "users" WHERE "status" = $1 AND "role" IN ($2, $3) AND "age" > $4
```

**Empty-`IN` semantics.** SQL has no valid `IN ()`, so the builder substitutes
a constant predicate with the logically correct truth value:

- empty `where_in` → `1 = 0` (nothing matches — no value is in an empty set)
- empty `where_not_in` → `1 = 1` (everything matches)

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .where_in("x", Vec::<i64>::new())
    .to_sql();
// SELECT * FROM "users" WHERE 1 = 0

let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .where_not_in("x", Vec::<i64>::new())
    .to_sql();
// SELECT * FROM "users" WHERE 1 = 1
```

This makes list-driven filters (e.g. from an HTTP query string) safe to chain
without special-casing the empty list.

## `where_null` / `where_not_null`

Emit `col IS NULL` / `col IS NOT NULL` — no binds:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("t")
    .where_null("a")
    .where_not_null("b")
    .to_sql();
// SELECT * FROM "t" WHERE "a" IS NULL AND "b" IS NOT NULL
// binds is empty
```

## `where_between`

`where_between(col, lo, hi)` emits `col BETWEEN ? AND ?` with two binds:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("t")
    .where_between("age", 18i64, 65i64)
    .to_sql();
// SELECT * FROM "t" WHERE "age" BETWEEN $1 AND $2
```

## `where_ilike` — dialect-aware case-insensitive match

On **Postgres** this compiles to the native `ILIKE` operator. **MySQL** and
**SQLite** have no `ILIKE`, so the builder lowers it to
`LOWER(col) LIKE LOWER(?)` — same semantics, portable chain:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("t")
    .select(["a"])
    .where_ilike("name", "%jo%")
    .to_sql();
// SELECT "a" FROM "t" WHERE "name" ILIKE $1
```

```rust,ignore
let (sql, binds) = QueryBuilder::<MySql>::table("t")
    .select(["a"])
    .where_ilike("name", "%jo%")
    .to_sql();
// SELECT `a` FROM `t` WHERE LOWER(`name`) LIKE LOWER(?)
```

The `where_like` wildcard caveat applies here too — see
[Case-insensitive Search](../cookbook/search.md) for the full recipe.

## `where_jsonb_contains` — JSONB containment

`where_jsonb_contains(col, val)` emits `col @> ?`. The value is typically a
JSON text string (or `Value::Json` behind the `json` feature — see
[Binds & Values](../binds.md)):

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("t")
    .select(["a"])
    .where_jsonb_contains("meta", "{\"a\":1}")
    .to_sql();
// SELECT "a" FROM "t" WHERE "meta" @> $1
```

> **Dialect note** — the `@>` operator is emitted **verbatim on ALL
> dialects**; the builder does not lower it for MySQL/SQLite, where the
> resulting SQL is invalid or means something else. This predicate is only
> meaningful on Postgres `jsonb` columns.

## `where_column` — column-to-column comparison

`where_column(lhs, op, rhs)` compares two columns (both escaped, no bind).
`op` is a `&'static str`, so the operator cannot be built from runtime input:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("t")
    .select(["x"])
    .where_column("a.x", "=", "b.y")
    .to_sql();
// SELECT "x" FROM "t" WHERE "a"."x" = "b"."y"
```

Its main job is correlating subqueries with the outer query (next section).

## Subquery predicates

`where_exists` / `where_not_exists` and `where_in_subquery` /
`where_not_in_subquery` take an already-built sub-builder by value. The
subquery is compiled with **placeholder continuity** — its binds continue the
outer query's `$N` numbering at the point the predicate is emitted:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .where_eq("active", true)
    .where_exists(
        QueryBuilder::<Postgres>::table("orders")
            .select(["1"])
            .where_column("orders.user_id", "=", "users.id")
            .where_gt("total", 100i64),
    )
    .to_sql();
// SELECT "id" FROM "users" WHERE "active" = $1 AND EXISTS (SELECT "1" FROM "orders" WHERE "orders"."user_id" = "users"."id" AND "total" > $2)
```

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .where_in_subquery(
        "id",
        QueryBuilder::<Postgres>::table("ban")
            .select(["user_id"])
            .where_eq("k", 7i64),
    )
    .to_sql();
// SELECT "id" FROM "users" WHERE "id" IN (SELECT "user_id" FROM "ban" WHERE "k" = $1)
```

`where_not_exists` renders `NOT EXISTS (…)`; `where_not_in_subquery` renders
`col NOT IN (…)`. For subqueries in the SELECT list, see
[`select_subquery`](select.md).

## Groups: `and_where` / `or_where`

To get parentheses and `OR`, pass a closure to `and_where` or `or_where`. The
closure receives a `WhereBuilder` exposing the predicate methods, plus
`and_where`/`or_where` again, so groups nest. The method name decides how the
group attaches to what precedes it (`AND (…)` vs `OR (…)`); inside the group,
predicates are joined with `AND` unless they are themselves `or_where`
sub-groups:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .where_eq("active", true)
    .or_where(|w| w.where_eq("role", "admin").where_gt("age", 40i64))
    .to_sql();
// SELECT * FROM "users" WHERE "active" = $1 OR ("role" = $2 AND "age" > $3)
```

Nesting — an `or_where` inside an `and_where` group attaches with `OR`:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("t")
    .select(["*"])
    .and_where(|g| g.where_eq("a", 1i64).or_where(|h| h.where_eq("b", 2i64)))
    .to_sql();
// SELECT * FROM "t" WHERE ("a" = $1 OR ("b" = $2))
```

Since 3.1.0 the four subquery predicates are also available **inside**
groups, with the same contracts (placeholder continuity included):

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .and_where(|g| {
        g.where_in_subquery(
            "id",
            QueryBuilder::<Postgres>::table("ban")
                .select(["user_id"])
                .where_eq("k", 7i64),
        )
        .where_not_exists(
            QueryBuilder::<Postgres>::table("audit")
                .select(["1"])
                .where_eq("level", 3i64),
        )
    })
    .to_sql();
// SELECT "id" FROM "users" WHERE ("id" IN (SELECT "user_id" FROM "ban" WHERE "k" = $1) AND NOT EXISTS (SELECT "1" FROM "audit" WHERE "level" = $2))
```

Two edge cases are handled for you:

- **A group as the first predicate** has nothing to attach to, so no leading
  `AND`/`OR` is emitted: `.or_where(|w| w.where_eq("x", 1i64))` alone renders
  `WHERE ("x" = $1)`.
- **Empty groups are omitted entirely.** A closure that adds no predicates
  (common with conditional chains) produces no `()`, no dangling `AND`/`OR` —
  and if the empty group was the only predicate, no `WHERE` at all:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("t")
    .where_eq("a", 1i64)
    .and_where(|w| w)
    .to_sql();
// SELECT * FROM "t" WHERE "a" = $1

let (sql, binds) = QueryBuilder::<Postgres>::table("t")
    .and_where(|w| w)
    .to_sql();
// SELECT * FROM "t"
```

This makes groups safe to combine with [`when`](dynamic.md)-style conditional
building.

## `where_raw` — the escape hatch

`where_raw(sql, binds)` records a raw SQL predicate with its own binds, for
operators and expressions the structured API does not model:

> **⚠️ Positional placeholder contract**
>
> The `sql` fragment is emitted **verbatim** — it is NOT escaped and NOT
> renumbered. `binds` are appended to the running bind list in order. On
> **Postgres** you must hand-write `$N` numbers matching the actual bind
> position (number of binds already accumulated + 1, + 2, …); on MySQL/SQLite
> use `?`. A wrong `$N` produces a malformed query. Never build the fragment
> itself from untrusted input — see the [Security Model](../security.md).

```rust,ignore
// Seven binds precede the raw predicate (a, b, c, d, e, and h's two BETWEEN
// bounds), so its single bind is the 8th → `$8`.
let (sql, binds) = QueryBuilder::<Postgres>::table("t")
    .where_ne("a", 1i64)
    .where_gte("b", 2i64)
    .where_lt("c", 3i64)
    .where_lte("d", 4i64)
    .where_like("e", "%x%")
    .where_null("f")
    .where_not_null("g")
    .where_between("h", 5i64, 6i64)
    .where_raw("j @> $8", vec![Value::Text("raw".into())])
    .to_sql();
// SELECT * FROM "t" WHERE "a" != $1 AND "b" >= $2 AND "c" < $3 AND "d" <= $4 AND "e" LIKE $5 AND "f" IS NULL AND "g" IS NOT NULL AND "h" BETWEEN $6 AND $7 AND j @> $8
```

## Related pages

- [SELECT](select.md) — shaping the column list
- [JOIN](join.md) — `ON` conditions use the same bound-value discipline
- [Dynamic Building](dynamic.md) — `when`/`when_else` for request-driven filters
- [Case-insensitive Search](../cookbook/search.md) — `where_ilike` + LIKE-wildcard escaping
- [Error Handling](../error-handling.md) — `to_sql()` vs `try_to_sql()`
- [Security Model](../security.md) — guarantees and the `*_raw` inventory
