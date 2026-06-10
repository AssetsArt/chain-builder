# SELECT

Every builder starts as a `SELECT` — `QueryBuilder::<Dialect>::table("users")`
with no other method calls compiles to `SELECT * FROM "users"`. This page
covers everything that shapes the SELECT list: plain columns, aliases,
aggregates, raw expressions, subquery columns, and `DISTINCT` /
`DISTINCT ON`. Filtering belongs to [WHERE](where.md), combining tables to
[JOIN](join.md).

## `select` — plain columns

`select` takes any iterable of column names and replaces the column list.
Every name is escaped per dialect; dotted identifiers are escaped per segment;
a literal `"*"` passes through unquoted; an empty list selects `*`:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["id", "name"])
    .where_eq("status", "active")
    .to_sql();
// SELECT "id", "name" FROM "users" WHERE "status" = $1
```

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .select(["users.id"])
    .to_sql();
// SELECT "users"."id" FROM "users"
```

Escaping is not optional decoration — a hostile column name is neutralized by
quote-doubling rather than spliced in:

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .select([r#"id" ; DROP TABLE users; --"#])
    .to_sql();
// SELECT "id"" ; DROP TABLE users; --" FROM "users"
```

## `select_as` — column aliases

`select_as(col, alias)` emits `col AS alias`, with **both** identifiers
escaped:

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .select_as("created_at", "joined")
    .to_sql();
// SELECT "created_at" AS "joined" FROM "users"
```

## Aggregates

Five aggregate helpers — `select_count`, `select_sum`, `select_avg`,
`select_min`, `select_max` — each with an `_as` twin that adds an escaped
alias. The column is escaped; for `select_count` (and `select_count_as`) the
special column `"*"` passes through as `COUNT(*)`:

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .select_count("*")
    .to_sql();
// SELECT COUNT(*) FROM "users"
```

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("orders")
    .select_sum("amount")
    .select_avg("amount")
    .select_min("amount")
    .select_max("amount")
    .to_sql();
// SELECT SUM("amount"), AVG("amount"), MIN("amount"), MAX("amount") FROM "orders"
```

Aggregates mix freely with plain columns and `GROUP BY` (see
[GROUP BY · HAVING · ORDER · LIMIT](group-having-order-limit.md)):

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("orders")
    .select(["status"])
    .select_count_as("*", "cnt")
    .select_sum_as("amount", "total")
    .group_by(["status"])
    .to_sql();
// SELECT "status", COUNT(*) AS "cnt", SUM("amount") AS "total" FROM "orders" GROUP BY "status"
```

Qualified columns work inside aggregates too:
`select_sum("orders.amount")` → `SUM("orders"."amount")`.

## `select_raw` — verbatim expressions

`select_raw(sql, binds)` is the escape hatch for anything the structured API
does not cover (functions, casts, window expressions). The fragment is
appended to the column list after any `select` columns and aggregate/alias
expressions; multiple calls
accumulate; `binds` is an `Option<Vec<Value>>` (`None` for no binds).

> **⚠️ Positional placeholder contract**
>
> The `sql` fragment is emitted **verbatim** — it is NOT escaped and NOT
> renumbered. Its binds are appended to the running bind list in order. On
> **Postgres** you must hand-write `$N` numbers that match the actual bind
> position (number of binds already accumulated + 1, + 2, …); on MySQL/SQLite
> use `?`. A wrong `$N` produces a malformed query. The same contract applies
> to every `*_raw` method — see the [Security Model](../security.md) for the
> full escape-hatch inventory.

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("orders")
    .select_raw("COUNT(*)", None)
    .to_sql();
// SELECT COUNT(*) FROM "orders"
```

## `select_subquery` — scalar subquery columns

`select_subquery(alias, sub)` takes another builder by value and emits
`(<sub>) AS "alias"` after the regular columns and any `select_raw`
expressions. The subquery is compiled with **placeholder continuity**: since
the SELECT list renders before `WHERE`, the subquery's binds take the earlier
`$N` numbers — automatically, no manual numbering involved:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .select_subquery(
        "cnt",
        QueryBuilder::<Postgres>::table("orders")
            .select_raw("COUNT(*)", None)
            .where_eq("status", 1i64),
    )
    .where_eq("active", true)
    .to_sql();
// SELECT "id", (SELECT COUNT(*) FROM "orders" WHERE "status" = $1) AS "cnt" FROM "users" WHERE "active" = $2
// binds == [Value::I64(1), Value::Bool(true)]
```

Correlated subqueries use `where_column` to reference the outer table —
see [WHERE](where.md).

## `distinct`

`distinct()` emits `SELECT DISTINCT …` and works on all dialects:

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("t")
    .distinct()
    .select(["a"])
    .to_sql();
// SELECT DISTINCT "a" FROM "t"
```

## `distinct_on` — Postgres only

`distinct_on(cols)` emits Postgres' `SELECT DISTINCT ON (cols) …`; the columns
are escaped like any identifier:

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("t")
    .distinct_on(["a", "b"])
    .select(["a"])
    .to_sql();
// SELECT DISTINCT ON ("a", "b") "a" FROM "t"
```

> **Dialect note** — MySQL and SQLite have no `DISTINCT ON`. Compiling a
> `distinct_on` builder against either dialect fails:
> [`try_to_sql()`](../error-handling.md) returns
> `BuildError::DistinctOnRequiresPostgres`, and `to_sql()` panics with
> `DISTINCT ON requires PostgreSQL`. If the dialect is decided at runtime,
> use the `try_` form.

## Related pages

- [WHERE](where.md) — filtering the rows you select
- [JOIN](join.md) — combining tables
- [GROUP BY · HAVING · ORDER · LIMIT](group-having-order-limit.md) — shaping the result set
- [Error Handling](../error-handling.md) — `to_sql()` vs `try_to_sql()` and `BuildError`
- [Security Model](../security.md) — what `select_raw` does and does not protect
