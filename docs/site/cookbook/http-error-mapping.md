# Mapping Errors to HTTP Status

**Problem:** a handler that executes chain-builder queries can fail three
ways — the query was invalid because of *request input* (the client's fault),
invalid because of a *bug in your chain* (your fault), or the *database*
failed. Those need to become 400, 500, and 500/404 respectively, in one place,
instead of ad-hoc matches in every handler.

The split is principled, not heuristic: every
[`BuildError`](../error-handling.md) variant describes a caller mistake, so
the only question is *did the offending value come from the request?* Two
variants can be triggered by end-user input in a typical filter/pagination
API — `InvalidHavingOperator` (an operator string taken from input) and
`OffsetWithoutLimit` (pagination parameters taken from input) — they map to
**400**. Every other build error is a bug in query construction → **500**.
`Error::Sqlx` is a database/driver failure → **500**, with `RowNotFound`
refined to **404** where "no such row" is an expected outcome.

> axum is used for illustration only — it is not a dependency of
> chain-builder. The match arms are the portable part; swap the
> `IntoResponse` glue for your framework's equivalent.

## The `AppError` wrapper

One newtype over `chain_builder::Error`, one `From` impl so `?` works in
every handler, one `IntoResponse` impl holding the whole mapping:

```rust
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use chain_builder::{BuildError, Error};

struct AppError(Error);

// `?` on any fetch_*/execute/count call lands here.
impl From<Error> for AppError {
    fn from(e: Error) -> Self {
        Self(e)
    }
}

// `?` on a bare try_to_sql()/try_to_sqlx_query() also works:
// BuildError → chain_builder::Error → AppError.
impl From<BuildError> for AppError {
    fn from(e: BuildError) -> Self {
        Self(Error::from(e))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match &self.0 {
            // ---- 4XX: the offending value came from the request ----------

            // A filter API let the client pick the operator; the allowlist
            // rejected it. The variant carries the rejected operator, but
            // echo it back with care (it is attacker-controlled text).
            Error::Build(BuildError::InvalidHavingOperator(_)) => (
                StatusCode::BAD_REQUEST,
                "unsupported filter operator".to_owned(),
            ),

            // Pagination params came straight from the request: an offset
            // without a limit is a client error here. If YOUR code computes
            // offsets independently of input, move this arm to the 500 bucket.
            Error::Build(BuildError::OffsetWithoutLimit) => (
                StatusCode::BAD_REQUEST,
                "offset requires a limit".to_owned(),
            ),

            // ---- 404 refinement: expected "no such row" -------------------

            // fetch_one / fetch_scalar on zero rows. Only sensible on
            // lookup-by-id style endpoints; drop this arm if a missing row
            // means a broken invariant instead.
            Error::Sqlx(sqlx::Error::RowNotFound) => {
                (StatusCode::NOT_FOUND, "not found".to_owned())
            }

            // ---- 5XX: programmer errors and infrastructure ----------------

            // Remaining build errors (LockRequiresSelect, EmptyInsert, …) are
            // bugs in query construction, not client input. Log loudly.
            Error::Build(e) => {
                tracing::error!(error = %e, "invalid query construction");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".to_owned())
            }

            // Connection / query / decode failure.
            Error::Sqlx(e) => {
                tracing::error!(error = %e, "database error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".to_owned())
            }

            // Error is #[non_exhaustive]: this wildcard arm is REQUIRED by
            // the compiler outside the chain-builder crate, and it is where
            // any future variant lands — default it to 500, never 200.
            e => {
                tracing::error!(error = %e, "unhandled chain-builder error");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal error".to_owned())
            }
        };
        (status, message).into_response()
    }
}
```

A handler then needs no error code at all:

```rust
async fn get_user(
    State(pool): State<sqlx::PgPool>,
    Path(id): Path<i64>,
) -> Result<Json<UserRow>, AppError> {
    let user: UserRow = QueryBuilder::<Postgres>::table("users")
        .select(["id", "name", "role"])
        .where_eq("id", id)
        .fetch_one(&pool)        // no row → Error::Sqlx(RowNotFound) → 404
        .await?;
    Ok(Json(user))
}
```

## Notes & caveats

- **Wildcard arms are mandatory, twice.** Both `Error` *and* `BuildError` are
  `#[non_exhaustive]` — any `match` on either, anywhere outside the crate,
  needs a wildcard arm. Above, the inner `BuildError` matching happens inside
  `Error::Build(…)` patterns, and the trailing `e => …` arm covers future
  `Error` variants; the `Error::Build(e)` arm covers future `BuildError`
  variants. Route both wildcards to 500: a variant you did not anticipate is
  by definition not a request error you understood.
- **The 400 set is about *your* API surface.** `InvalidHavingOperator` and
  `OffsetWithoutLimit` are 400 only because those values plausibly arrive
  from a request. If your handlers never expose operators or raw offsets to
  clients, map everything in `Error::Build` to 500 and treat any occurrence
  as a bug.
- **Don't leak internals.** `BuildError`'s `Display` messages name builder
  methods, and `sqlx::Error` can include SQL fragments and table names —
  log them, but send a generic body. The one variant carrying user input,
  `InvalidHavingOperator(String)`, contains attacker-controlled text; if you
  echo it, treat it as untrusted output (encode/escape for the response
  format).
- **404 is a refinement, not a default.** `fetch_optional` returning
  `Ok(None)` is usually the cleaner way to express "missing is fine"; the
  `RowNotFound` arm catches the `fetch_one`/`fetch_scalar` paths.
- **Prefer the `try_*`/`fetch_*` family in handlers.** The panicking
  `to_sql()` path bypasses this whole mapping — it is for static,
  hand-written queries (see the [policy](../error-handling.md)).

## Related pages

- [Error Handling](../error-handling.md) — the variant table and the 4XX/5XX reasoning
- [Executing with sqlx](../sqlx.md) — which helpers return `RowNotFound` vs `Ok(None)`
- [HTTP Filters & Pagination](http-filters-pagination.md) — the handler that uses this `AppError`
