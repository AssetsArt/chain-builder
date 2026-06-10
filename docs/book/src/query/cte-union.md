# CTE & UNION

Common table expressions (`with` / `with_recursive`) prepend a
`WITH … AS (…)` header built from another `QueryBuilder`, and `union` /
`union_all` append further SELECT arms after the main query. Both take whole
builders by value, and both are **SELECT-only** features of the compiler. The
one thing to internalize on this page is **bind ordering**: CTE bodies
compile *first*, so their binds — and on Postgres their `$N` numbers — come
first.

## `with(name, query)` — common table expressions

`with` adds one `WITH name AS (body)` entry; multiple calls accumulate into a
single comma-separated `WITH` header. The CTE name is escaped like any
identifier; the body is a complete sub-builder:

```rust,ignore
let cte = QueryBuilder::<Postgres>::table("logs")
    .select(["n"])
    .where_gt("n", 1i64)
    .where_lt("n", 10i64);

let (sql, binds) = QueryBuilder::<Postgres>::table("recent")
    .with("recent", cte)
    .select(["*"])
    .where_gte("n", 5i64)
    .where_lte("n", 8i64)
    .to_sql();
// WITH "recent" AS (SELECT "n" FROM "logs" WHERE "n" > $1 AND "n" < $2) SELECT * FROM "recent" WHERE "n" >= $3 AND "n" <= $4
// binds == [Value::I64(1), Value::I64(10), Value::I64(5), Value::I64(8)]
```

Note the numbering: the CTE body owns `$1`/`$2`, the main query continues at
`$3` — automatically, with no manual counting.

## `with_recursive` — recursive CTEs

`with_recursive(name, query)` marks the CTE as recursive. SQL has a single
`WITH` keyword per statement, so if **any** CTE in the chain is recursive,
that one `WITH` carries `RECURSIVE` for all of them:

```rust,ignore
let cte = QueryBuilder::<Postgres>::table("t").select(["n"]);
let (sql, _) = QueryBuilder::<Postgres>::table("t")
    .with_recursive("t", cte)
    .select(["*"])
    .to_sql();
// WITH RECURSIVE "t" AS (SELECT "n" FROM "t") SELECT * FROM "t"
```

(The recursive *step* — the body's self-`UNION` — is built like any other
union; the builder does not validate recursion structure, the database does.)

## `union(query)` / `union_all(query)`

Each call appends one ` UNION ` / ` UNION ALL ` arm after the main query;
arms render in call order:

```rust,ignore
let arm = QueryBuilder::<Postgres>::table("b").select(["id"]);
let (sql, _) = QueryBuilder::<Postgres>::table("a")
    .select(["id"])
    .union_all(arm)
    .to_sql();
// SELECT "id" FROM "a" UNION ALL SELECT "id" FROM "b"
```

`union` deduplicates rows (SQL `UNION`); `union_all` keeps duplicates and is
cheaper. As in plain SQL, the arms must produce compatible column lists —
the builder does not check that for you.

## Bind ordering: CTEs first, then main, then UNION arms

The compiler emits SQL in a single pass, pushing each bind the moment its
placeholder is written. Since the `WITH` header renders before the main query
and UNION arms render after it, the bind list is always
**CTE bodies → main query → UNION arms**, and on Postgres the `$N` numbers
follow the same sequence across all nesting:

```rust,ignore
// THE CRUX: $1 (cte body) -> $2 (main where) -> $3 (union arm).
let cte = QueryBuilder::<Postgres>::table("logs")
    .select(["n"])
    .where_gt("n", 1i64);
let arm = QueryBuilder::<Postgres>::table("recent")
    .select(["n"])
    .where_lt("n", 99i64);

let (sql, binds) = QueryBuilder::<Postgres>::table("recent")
    .with("recent", cte)
    .select(["*"])
    .where_gt("n", 5i64)
    .union(arm)
    .to_sql();
// WITH "recent" AS (SELECT "n" FROM "logs" WHERE "n" > $1) SELECT * FROM "recent" WHERE "n" > $2 UNION SELECT "n" FROM "recent" WHERE "n" < $3
// binds == [Value::I64(1), Value::I64(5), Value::I64(99)]
```

This matters when you combine CTEs with a `*_raw` method: the raw fragment's
hand-written `$N` must account for every bind the CTE bodies contributed
before it — see the placeholder contract in
[GROUP BY · HAVING · ORDER · LIMIT](group-having-order-limit.md).

Errors propagate through the same nesting: an invalid CTE body or UNION arm
(e.g. a deferred `having` error) surfaces from the outer
[`try_to_sql()`](../error-handling.md).

> **Dialect note** — CTEs and unions render with the same shape on every
> dialect; only quoting and placeholders differ (see
> [Dialect Differences](../dialects.md)):
>
> ```rust,ignore
> let cte = QueryBuilder::<MySql>::table("logs")
>     .select(["n"])
>     .where_gt("n", 1i64);
> let arm = QueryBuilder::<MySql>::table("recent")
>     .select(["n"])
>     .where_lt("n", 99i64);
>
> let (sql, binds) = QueryBuilder::<MySql>::table("recent")
>     .with("recent", cte)
>     .select(["*"])
>     .where_gt("n", 5i64)
>     .union(arm)
>     .to_sql();
> // WITH `recent` AS (SELECT `n` FROM `logs` WHERE `n` > ?) SELECT * FROM `recent` WHERE `n` > ? UNION SELECT `n` FROM `recent` WHERE `n` < ?
> // binds == [Value::I64(1), Value::I64(5), Value::I64(99)]
> ```
>
> SQLite is identical to Postgres except for `?` placeholders.

> **SELECT-only** — `with`/`with_recursive`/`union`/`union_all` are compiled
> only for SELECT queries; on INSERT/UPDATE/DELETE they are ignored. Also
> note that combining `UNION` with a [row lock](locking.md) is rejected with
> `BuildError::LockWithUnion`.

## Related pages

- [SELECT](select.md) — `select_subquery` uses the same placeholder-continuity rule
- [WHERE](where.md) — `where_exists` / `where_in_subquery` for nested builders in predicates
- [Row Locking](locking.md) — why `FOR UPDATE` + `UNION` is an error
- [Error Handling](../error-handling.md) — nested error propagation
- [Dialect Differences](../dialects.md) — quoting and placeholders
