# Dynamic Building

The builder is consumed and returned **by value** — every method takes `self`
and gives it back. That makes a plain `if` awkward in the middle of a chain,
so the builder ships two combinators, `when` and `when_else`, that apply a
closure conditionally while keeping the chain intact. Together with
`QueryBuilder` being `Clone` and `paginate`, they cover the everyday job of
turning runtime input — HTTP query parameters, feature flags, role checks —
into SQL without ever concatenating strings.

## `when(cond, f)`

Returns `f(self)` when `cond` is true, otherwise `self` unchanged:

```rust
use chain_builder::{Postgres, QueryBuilder};
let only_active = true;
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .when(only_active, |q| q.where_eq("status", "active"))
    .to_sql();
assert_eq!(sql, r#"SELECT "id" FROM "users" WHERE "status" = $1"#);
```

When the condition is false, nothing is added — no `WHERE` keyword, no bind:

```rust
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .when(false, |q| q.where_eq("a", 1i64))
    .to_sql();
// SELECT "id" FROM "users"
// binds is empty — no dangling WHERE, no orphaned bind
```

The closure receives the whole builder, so it can add anything — extra
predicates, joins, ordering — and the rest of the chain continues unaffected.
Bind numbering stays correct in both branches:

```rust
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .where_eq("status", "active")
    .when(true, |q| q.where_eq("a", 1i64))
    .order_by_desc("created")
    .limit(5)
    .to_sql();
// SELECT "id" FROM "users" WHERE "status" = $1 AND "a" = $2 ORDER BY "created" DESC LIMIT $3
// binds == [Value::Text("active"), Value::I64(1), Value::I64(5)]
```

With `false`, the same chain compiles to
`SELECT "id" FROM "users" WHERE "status" = $1 ORDER BY "created" DESC LIMIT $2`
— the placeholders renumber automatically.

## `when_else(cond, if_true, if_false)`

Applies `if_true` when `cond` holds, otherwise `if_false` — an inline
`if/else` for the chain:

```rust
use chain_builder::{Postgres, QueryBuilder};
let active = false;
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .when_else(
        active,
        |q| q.where_eq("status", "active"),
        |q| q.where_eq("status", "inactive"),
    )
    .to_sql();
assert_eq!(sql, r#"SELECT "id" FROM "users" WHERE "status" = $1"#);
```

The SQL shape is identical either way here — only the bound value differs
(`"active"` vs `"inactive"`), which is exactly what you want: the *structure*
is decided by your code, the *data* travels as a bind.

## Building from request parameters

The classic use case: a search endpoint with optional filters. Guard each
optional filter with `when(param.is_some(), …)` — note that the guard is what
gives `None` its intended meaning ("no filter"). Without it,
`where_eq("status", None::<String>)` would bind a `NULL` and emit
`"status" = $1`, which never matches any row (use
[`where_null`](where.md) for explicit NULL tests):

```rust
struct Filters {
    status: Option<String>,
    min_age: Option<i64>,
    search: Option<String>,
}

let f = Filters {
    status: Some("active".into()),
    min_age: None,
    search: Some("ali".into()),
};

let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["id", "name"])
    .when(f.status.is_some(), |q| q.where_eq("status", f.status.clone()))
    .when(f.min_age.is_some(), |q| q.where_gte("age", f.min_age))
    .when(f.search.is_some(), |q| {
        q.where_like("name", format!("%{}%", f.search.clone().unwrap_or_default()))
    })
    .order_by_asc("name")
    .paginate(1, 20)
    .to_sql();
// SELECT "id", "name" FROM "users" WHERE "status" = $1 AND "name" LIKE $2 ORDER BY "name" ASC LIMIT $3 OFFSET $4
```

`Option<T>` implements `IntoBind` (`Some(v)` binds `v`, `None` binds NULL —
see [Binds & Values](../binds.md)), so the guarded `f.status.clone()` can be
passed as-is without unwrapping. A full axum version of this pattern lives in
the [HTTP filters & pagination cookbook](../cookbook/http-filters-pagination.md);
the `where_like` wildcard caveat is covered in [WHERE](where.md).

## Reusing a base query — `QueryBuilder` is `Clone`

`QueryBuilder` derives `Clone`, so you can build the shared part once and fork
it. The standard pagination pair — one COUNT, one page fetch — from a single
source of truth for the filters:

```rust
let base = QueryBuilder::<Postgres>::table("users")
    .where_eq("status", "active");

let count_q = base.clone().select_count("*");
let page_q = base
    .select(["id", "name"])
    .order_by_desc("created")
    .paginate(2, 10);

let (count_sql, _) = count_q.to_sql();
// SELECT COUNT(*) FROM "users" WHERE "status" = $1
let (page_sql, _) = page_q.to_sql();
// SELECT "id", "name" FROM "users" WHERE "status" = $1 ORDER BY "created" DESC LIMIT $2 OFFSET $3
```

Because the filters live in one place, the count and the page can never drift
apart. (When executing with sqlx, the [`count()` helper](../sqlx.md) wraps a
builder in `SELECT COUNT(*)` for you.)

## `paginate(page, per_page)`

`paginate` is `limit(per_page).offset((page - 1).max(0) * per_page)` — a
**1-based** page window. Both numbers are bound as placeholders, never
inlined:

```rust
use chain_builder::{Postgres, QueryBuilder, Value};
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .paginate(2, 10)
    .to_sql();
assert_eq!(sql, r#"SELECT "id" FROM "users" LIMIT $1 OFFSET $2"#);
assert_eq!(binds, vec![Value::I64(10), Value::I64(10)]);
```

`page < 1` is clamped to page 1, so hostile or buggy input can never produce
a negative offset:

```rust
let (_, binds) = QueryBuilder::<Postgres>::table("users")
    .paginate(0, 10)   // also -5, -100, …
    .to_sql();
// binds == [Value::I64(10), Value::I64(0)]  — LIMIT 10 OFFSET 0
```

> **Dialect note** — only the placeholder style differs:
> MySQL renders ``SELECT `id` FROM `users` LIMIT ? OFFSET ?``, SQLite
> `SELECT "id" FROM "users" LIMIT ? OFFSET ?`. The bind order
> (`[per_page, offset]`) is the same everywhere.

## Related pages

- [Binds & Values](../binds.md) — `Option<T>` binding, `IntoBind`
- [WHERE](where.md) — the predicates you toggle with `when`
- [GROUP BY · HAVING · ORDER · LIMIT](group-having-order-limit.md) — `limit`/`offset` underneath `paginate`
- [Error Handling](../error-handling.md) — prefer `try_to_sql()` when input is runtime-driven
- [Cookbook: HTTP filters & pagination](../cookbook/http-filters-pagination.md) — the full axum recipe
