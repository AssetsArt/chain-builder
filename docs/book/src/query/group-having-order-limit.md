# GROUP BY · HAVING · ORDER · LIMIT

Everything that shapes a result set after filtering: grouping rows
(`group_by` / `group_by_raw`), filtering groups (`having` / `having_raw`),
sorting (`order_by` and friends), and windowing (`limit` / `offset` /
`paginate`). All of these are **SELECT-only** — they are ignored on
INSERT/UPDATE/DELETE. Clauses render in standard SQL order regardless of call
order: `… WHERE … GROUP BY … HAVING … ORDER BY … LIMIT … OFFSET …`.

## `group_by` — escaped columns

`group_by(cols)` takes any iterable of column names; calls accumulate. Every
name is escaped per dialect, dotted identifiers per segment:

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .group_by(["a", "b"])
    .to_sql();
// SELECT "id" FROM "users" GROUP BY "a", "b"
```

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .group_by(["t.col"])
    .to_sql();
// SELECT "id" FROM "users" GROUP BY "t"."col"
```

The typical pairing is with the aggregate selectors from
[SELECT](select.md):

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("orders")
    .select(["status"])
    .select_count_as("*", "cnt")
    .select_sum_as("amount", "total")
    .group_by(["status"])
    .to_sql();
// SELECT "status", COUNT(*) AS "cnt", SUM("amount") AS "total" FROM "orders" GROUP BY "status"
```

## `group_by_raw` — verbatim grouping expressions

`group_by_raw(sql, binds)` is the escape hatch for grouping by an expression
(function call, date truncation, …). The fragment is appended after any
structured `group_by` columns within the same `GROUP BY` clause; if no
structured columns are present it becomes the whole clause. Repeated
`group_by_raw`/`order_by_raw` calls replace the previous fragment (unlike the
accumulating structured methods):

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("t")
    .select(["a"])
    .group_by_raw("date_trunc('day', created_at)", vec![])
    .to_sql();
// SELECT "a" FROM "t" GROUP BY date_trunc('day', created_at)
```

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("t")
    .select(["a"])
    .group_by(["a"])
    .group_by_raw("LOWER(b)", vec![])
    .order_by_asc("a")
    .order_by_raw("LOWER(b)", vec![])
    .to_sql();
// SELECT "a" FROM "t" GROUP BY "a", LOWER(b) ORDER BY "a" ASC, LOWER(b)
```

> **⚠️ Positional placeholder contract**
>
> The `sql` fragment is emitted **verbatim** — it is NOT escaped and NOT
> renumbered. Its binds are appended to the running bind list in order. On
> **Postgres** you must hand-write `$N` numbers matching the actual bind
> position (number of binds already accumulated + 1, + 2, …); on MySQL/SQLite
> use `?`. A wrong `$N` produces a malformed query. The same contract applies
> to every `*_raw` method — see the [Security Model](../security.md).

## `having` — guarded group filter

`having(col, op, val)` emits `HAVING col op ?`: `col` is a real column or
alias (escaped), `val` is bound. Multiple `having` terms are joined with
`AND`:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("orders")
    .select(["user_id"])
    .group_by(["user_id"])
    .having("total", ">", 100i64)
    .to_sql();
// SELECT "user_id" FROM "orders" GROUP BY "user_id" HAVING "total" > $1
```

### The operator allowlist (injection guard)

Unlike `where_column`/`JoinClause::on`/`on_val`, which take
`op: &'static str` (so only compile-time literals are accepted), `having`
takes `op: &str` for ergonomics. Because the operator is emitted **verbatim**
into the SQL — it is not a bound value and cannot be escaped without changing
its meaning — an attacker-controlled operator would be a SQL-injection
vector. So `op` is validated against a fixed allowlist:

`=`, `!=`, `<>`, `>`, `>=`, `<`, `<=`, `LIKE`, `NOT LIKE`

Matching is **case-insensitive** and the operator is stored **trimmed** —
`"  like  "` is accepted and rendered as `like`:

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("orders")
    .select(["user_id"])
    .having("name", "  like  ", "a%")
    .to_sql();
// SELECT "user_id" FROM "orders" HAVING "name" like $1
```

A disallowed operator does **not** panic at the `having()` call. Instead the
builder records a **deferred** `BuildError::InvalidHavingOperator` — the
chain stays intact, and the error surfaces at compile time:
[`try_to_sql()`](../error-handling.md) returns it as `Err`, while `to_sql()`
panics with the same message. If several deferred errors occur, the **first
one wins** (it points at the original misuse):

```rust,ignore
let qb = QueryBuilder::<Postgres>::table("orders")
    .select(["user_id"])
    .having("amount", "; DROP TABLE users", 0i64);
let err = qb.try_to_sql().unwrap_err();
// err == BuildError::InvalidHavingOperator("; DROP TABLE users".to_owned())
// err.to_string() contains "not an allowed"
// to_sql() on the same builder panics with the same message
```

The deferred error also propagates out of nested builders — a bad `having`
inside a CTE, UNION arm, or subquery surfaces from the *outer* `try_to_sql()`.

## `having_raw` — aggregates and everything else

The allowlisted `having` covers `col op value` only. For aggregate
expressions like `COUNT(*) > ?` — or any operator outside the allowlist —
use `having_raw(sql, binds)`, the documented verbatim escape hatch:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("orders")
    .select(["user_id"])
    .group_by(["user_id"])
    .having_raw("COUNT(*) > $1", vec![Value::I64(5)])
    .to_sql();
// SELECT "user_id" FROM "orders" GROUP BY "user_id" HAVING COUNT(*) > $1
```

> **⚠️ Positional placeholder contract**
>
> Same rule as `group_by_raw`: the fragment is emitted verbatim and its binds
> are appended in order. On **Postgres**, count the binds already accumulated
> by earlier clauses (SELECT-list subqueries, WHERE values, …) and number
> from there. With one preceding WHERE bind the aggregate bind is `$2`:
>
> ```rust,ignore
> let (sql, binds) = QueryBuilder::<Postgres>::table("orders")
>     .select(["user_id"])
>     .where_eq("status", "paid")
>     .group_by(["user_id"])
>     .having_raw("COUNT(*) > $2", vec![Value::I64(5)])
>     .to_sql();
> // SELECT "user_id" FROM "orders" WHERE "status" = $1 GROUP BY "user_id" HAVING COUNT(*) > $2
> ```
>
> On MySQL/SQLite write `?` and the position takes care of itself.

## `order_by` / `order_by_asc` / `order_by_desc`

`order_by(col, ord)` takes the `Order` enum (`Order::Asc` / `Order::Desc`);
`order_by_asc` and `order_by_desc` are shorthands. Calls accumulate into one
comma-separated clause; columns are escaped:

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .order_by_asc("a")
    .order_by_desc("b")
    .to_sql();
// SELECT "id" FROM "users" ORDER BY "a" ASC, "b" DESC
```

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .order_by("a", Order::Asc)
    .order_by("b", Order::Desc)
    .to_sql();
// SELECT "id" FROM "users" ORDER BY "a" ASC, "b" DESC
```

## `order_by_raw` — verbatim sort expressions

Same shape and same placeholder contract as `group_by_raw`: the fragment is
appended after any structured `order_by` terms, or becomes the whole clause
on its own:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("t")
    .select(["a"])
    .order_by_raw("CASE WHEN a = $1 THEN 0 ELSE 1 END", vec![Value::I64(5)])
    .to_sql();
// SELECT "a" FROM "t" ORDER BY CASE WHEN a = $1 THEN 0 ELSE 1 END
// binds == [Value::I64(5)]
```

## `limit` / `offset`

`limit(n)` and `offset(n)` render `LIMIT` / `OFFSET` with **bound
placeholders** — the numbers travel as binds, not as literal SQL text:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .where_eq("status", "active")
    .group_by(["dept"])
    .order_by_desc("created")
    .limit(10)
    .offset(20)
    .to_sql();
// SELECT "id" FROM "users" WHERE "status" = $1 GROUP BY "dept" ORDER BY "created" DESC LIMIT $2 OFFSET $3
// binds == [Value::Text("active".into()), Value::I64(10), Value::I64(20)]
```

`limit` works alone; `offset` does **not**. MySQL rejects a bare `OFFSET`, so
the builder makes the rule uniform across dialects: compiling an `offset`
without a `limit` fails with `BuildError::OffsetWithoutLimit` —
[`try_to_sql()`](../error-handling.md) returns it as `Err`, `to_sql()` panics
with `offset(...) requires limit(...)`:

```rust,ignore
let err = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .offset(10)
    .try_to_sql()
    .unwrap_err();
// err == BuildError::OffsetWithoutLimit
// err.to_string() == "offset(...) requires limit(...)"
```

## `paginate` — 1-based pages

`paginate(page, per_page)` is sugar for
`limit(per_page).offset((page - 1).max(0) * per_page)` — the row window
`[(page-1) * per_page, page * per_page)`. Pages are **1-based**; a
`page < 1` is treated as page 1 (offset 0), so callers never get a negative
offset:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .paginate(2, 10)
    .to_sql();
// SELECT "id" FROM "users" LIMIT $1 OFFSET $2
// binds == [Value::I64(10), Value::I64(10)]
```

This is the natural endpoint of a request-driven chain — see
[Dynamic Building](dynamic.md) and the
[HTTP Filters & Pagination](../cookbook/http-filters-pagination.md) recipe.

> **Dialect note** — all of these clauses render identically on MySQL and
> SQLite apart from quoting and placeholders
> (see [Dialect Differences](../dialects.md)):
>
> ```rust,ignore
> let (sql, binds) = QueryBuilder::<MySql>::table("users")
>     .select(["id"])
>     .where_eq("status", "active")
>     .group_by(["dept"])
>     .order_by_desc("created")
>     .limit(10)
>     .offset(20)
>     .to_sql();
> // SELECT `id` FROM `users` WHERE `status` = ? GROUP BY `dept` ORDER BY `created` DESC LIMIT ? OFFSET ?
> ```

## Related pages

- [SELECT](select.md) — aggregates that `GROUP BY` is usually paired with
- [Dynamic Building](dynamic.md) — `when`/`when_else` + `paginate` for request-driven queries
- [Error Handling](../error-handling.md) — the deferred `having` error, `OffsetWithoutLimit`
- [Dialect Differences](../dialects.md) — placeholders and quoting per dialect
- [Security Model](../security.md) — the `*_raw` escape-hatch inventory and the `having` allowlist rationale
