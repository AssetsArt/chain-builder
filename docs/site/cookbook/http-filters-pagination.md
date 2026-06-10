# HTTP Filters & Pagination

**Problem:** a `GET /users` endpoint takes optional query parameters —
`?status=active&role=admin&role=editor&page=2&per_page=20` — and must turn
them into one SQL query. Each filter applies only when present, pagination
must be safe against hostile values, and a malformed query must become an
error response, never a panic.

This recipe is the HTTP-shaped version of
[Dynamic Building](../query/dynamic.md): `when` toggles each filter,
`paginate` handles the page window, and the [sqlx helpers](../sqlx.md) return
the unified `Error` so nothing in the handler can panic.

> axum is used for illustration only — it is not a dependency of
> chain-builder. The builder calls are identical under any framework.

## The parameters

```rust
use serde::Deserialize;

#[derive(Deserialize)]
struct ListParams {
    /// Exact-match filter, applied only when present.
    status: Option<String>,
    /// Repeated parameter (`?role=admin&role=editor`) → `WHERE role IN (…)`.
    #[serde(default)]
    role: Vec<String>,
    /// 1-based page number.
    #[serde(default = "default_page")]
    page: i64,
    #[serde(default = "default_per_page")]
    per_page: i64,
}

fn default_page() -> i64 { 1 }
fn default_per_page() -> i64 { 20 }
```

## The builder chain

Build the *filtered* query once, then fork it (`QueryBuilder` is `Clone`) into
the count query and the page query — the filters can never drift apart:

```rust
use chain_builder::{Postgres, QueryBuilder};

fn list_query(p: &ListParams) -> QueryBuilder<Postgres> {
    QueryBuilder::<Postgres>::table("users")
        .when(p.status.is_some(), |q| q.where_eq("status", p.status.clone()))
        .when(!p.role.is_empty(), |q| q.where_in("role", p.role.clone()))
}
```

With `status = Some("active")` and `role = ["admin", "editor"]`, the page
query compiles to:

```rust
let per_page = p.per_page.clamp(1, 100); // never trust per_page from the wire
let (sql, binds) = list_query(&p)
    .select(["id", "name", "role"])
    .order_by_desc("created_at")
    .paginate(p.page, per_page)
    .try_to_sql()?;
// SELECT "id", "name", "role" FROM "users" WHERE "status" = $1 AND "role" IN ($2, $3) ORDER BY "created_at" DESC LIMIT $4 OFFSET $5
```

Safety properties you get for free:

- Every value — including `LIMIT`/`OFFSET` — travels as a bind, never inside
  the SQL string.
- `paginate` clamps `page < 1` to page 1, so a hostile `?page=-5` cannot
  produce a negative offset. Clamping `per_page` is **your** job (the one
  `clamp(1, 100)` above) — otherwise `?per_page=1000000` is a free
  table-dump request.
- An empty `role` list is guarded by `when` here, but even unguarded,
  `where_in("role", Vec::<String>::new())` compiles to the always-false
  `1 = 0` rather than invalid SQL (see [WHERE](../query/where.md)).
- `try_to_sql`, not `to_sql`: the query shape is driven by request input, so
  an invalid builder must surface as a `Result`, not a panic — the
  [documented policy](../error-handling.md).

## The full axum handler

```rust
use axum::{extract::{Query, State}, Json};
use chain_builder::{Postgres, QueryBuilder};
use serde::Serialize;

#[derive(Serialize, sqlx::FromRow)]
struct UserRow {
    id: i64,
    name: String,
    role: String,
}

#[derive(Serialize)]
struct Page {
    items: Vec<UserRow>,
    total: i64,
    page: i64,
    per_page: i64,
}

// AppError wraps chain_builder::Error and implements IntoResponse —
// the full mapping is the next recipe (http-error-mapping.md).
async fn list_users(
    State(pool): State<sqlx::PgPool>,
    Query(p): Query<ListParams>,
) -> Result<Json<Page>, AppError> {
    let per_page = p.per_page.clamp(1, 100);
    let base = list_query(&p);

    // Total count: same filters, wrapped as SELECT COUNT(*) FROM (…) by the
    // count() helper — no second copy of the WHERE clause to maintain.
    let total = base.clone().count(&pool).await?;

    let items: Vec<UserRow> = base
        .select(["id", "name", "role"])
        .order_by_desc("created_at")
        .paginate(p.page, per_page)
        .fetch_all(&pool)
        .await?;

    Ok(Json(Page { items, total, page: p.page.max(1), per_page }))
}
```

Both `?` operators convert into `chain_builder::Error` automatically: a build
failure surfaces as `Error::Build` *before* any database round-trip, a driver
failure as `Error::Sqlx`. The `fetch_all`/`count` helpers never panic on an
invalid builder.

## Notes & caveats

- **Two queries, one source of truth.** `base.clone().count(…)` plus the
  paginated fetch is the standard pair; because both fork from `list_query`,
  adding a filter later updates the total and the page together.
- **Free-text search** (`?q=ali`) belongs in the same `when` chain via
  `where_ilike` — but read [Case-insensitive Search](search.md) first: `%`
  and `_` in user input are **not** escaped by the builder.
- **Sortable columns from input?** Never pass a request value to
  `order_by(…)` directly — column names are identifiers, not binds. Map the
  input through an allowlist first (same policy as tenant names in
  [Multi-tenant with `.db()`](multi-tenant.md)).
- **MySQL/SQLite:** identical chain; only placeholders (`?`) and quoting
  change. See [Dialect Differences](../dialects.md).

## Related pages

- [Dynamic Building](../query/dynamic.md) — `when`, `when_else`, `paginate`, builder reuse
- [Executing with sqlx](../sqlx.md) — `fetch_all`, `count`, and the unified `Error`
- [Mapping Errors to HTTP Status](http-error-mapping.md) — the `AppError` used above
- [Error Handling](../error-handling.md) — why `try_*` for request-driven queries
