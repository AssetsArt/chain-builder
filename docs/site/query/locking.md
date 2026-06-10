# Row Locking

`for_update()` / `for_share()` append ` FOR UPDATE` / ` FOR SHARE` to a
SELECT, and `skip_locked()` / `no_wait()` add the ` SKIP LOCKED` / ` NOWAIT`
modifier. Row locks only mean something **inside a transaction**: you lock the
rows a SELECT returns so that concurrent transactions cannot modify (or, for
`FOR UPDATE`, even lock) them until you commit. Honored on Postgres and MySQL (`FOR SHARE` needs MySQL 8.0+; older versions
only have `LOCK IN SHARE MODE`, which the builder does not emit);
a **silent no-op on SQLite** (details below). Locking is strictly a SELECT
feature — attaching it to anything else, or combining it with `UNION` on a
locking dialect, is a [`BuildError`](../error-handling.md), not a silent drop.

## `for_update()` / `for_share()`

`for_update` takes an exclusive row lock; `for_share` takes a shared one
(other transactions can still read and `FOR SHARE` the same rows, but cannot
update or delete them). The clause renders after everything else, including
`LIMIT`:

```rust
let (sql, _) = QueryBuilder::<Postgres>::table("jobs")
    .select(["id"])
    .where_eq("status", "queued")
    .for_update()
    .to_sql();
// SELECT "id" FROM "jobs" WHERE "status" = $1 FOR UPDATE
```

```rust
let (sql, _) = QueryBuilder::<Postgres>::table("jobs")
    .select(["id"])
    .for_share()
    .to_sql();
// SELECT "id" FROM "jobs" FOR SHARE
```

Calling one after the other replaces the lock strength; the last call wins.
Any `SKIP LOCKED` / `NOWAIT` modifier already set is preserved.

## `skip_locked()` / `no_wait()`

By default a locking SELECT **blocks** until conflicting locks are released.
Two modifiers change that:

- `skip_locked()` — silently skip rows that are already locked
  (` SKIP LOCKED`),
- `no_wait()` — error immediately if any row is already locked (` NOWAIT`).

```rust
let (sql, _) = QueryBuilder::<Postgres>::table("jobs")
    .select(["id"])
    .for_update()
    .skip_locked()
    .to_sql();
// SELECT "id" FROM "jobs" FOR UPDATE SKIP LOCKED
```

```rust
let (sql, _) = QueryBuilder::<Postgres>::table("jobs")
    .select(["id"])
    .for_update()
    .no_wait()
    .to_sql();
// SELECT "id" FROM "jobs" FOR UPDATE NOWAIT
```

If no lock strength was set yet, both modifiers default it to `FOR UPDATE` —
the job-queue idiom in one call:

```rust
let (sql, _) = QueryBuilder::<Postgres>::table("jobs")
    .select(["id"])
    .skip_locked()
    .to_sql();
// SELECT "id" FROM "jobs" FOR UPDATE SKIP LOCKED
```

An existing `for_share()` is preserved — the modifier never downgrades or
upgrades a strength you chose explicitly:

```rust
let (sql, _) = QueryBuilder::<Postgres>::table("jobs")
    .select(["id"])
    .for_share()
    .skip_locked()
    .to_sql();
// SELECT "id" FROM "jobs" FOR SHARE SKIP LOCKED
```

The lock clause renders last, after `LIMIT`/`OFFSET`:

```rust
let (sql, binds) = QueryBuilder::<Postgres>::table("jobs")
    .select(["id"])
    .limit(1)
    .for_update()
    .skip_locked()
    .to_sql();
// SELECT "id" FROM "jobs" LIMIT $1 FOR UPDATE SKIP LOCKED
// binds.len() == 1
```

## Rules: SELECT-only, no UNION

Two misuses are rejected instead of silently dropped, because a lock the
caller believes is held — but is not — is a data-corruption bug waiting to
happen:

- **Lock on a non-SELECT** (INSERT/UPDATE/DELETE):
  [`try_to_sql()`](../error-handling.md) returns
  `BuildError::LockRequiresSelect`; `to_sql()` panics with
  `for_update()/for_share() is only valid on SELECT`. This guard is
  dialect-independent — it fires on SQLite too, because the misuse is
  structural, not a dialect capability.

  ```rust
  let err = QueryBuilder::<Postgres>::table("users")
      .update([("status", "x")])
      .for_update()
      .try_to_sql()
      .unwrap_err();
  // err == BuildError::LockRequiresSelect
  // err.to_string() == "for_update()/for_share() is only valid on SELECT"
  ```

- **Lock combined with `UNION`** on a locking dialect: Postgres and MySQL
  reject `FOR UPDATE` on a `UNION` result, so emitting it would produce
  invalid SQL. `try_to_sql()` returns `BuildError::LockWithUnion`;
  `to_sql()` panics with
  `for_update()/for_share() cannot be combined with UNION`.

  ```rust
  let err = QueryBuilder::<Postgres>::table("a")
      .select(["id"])
      .union(QueryBuilder::<Postgres>::table("b").select(["id"]))
      .for_update()
      .try_to_sql()
      .unwrap_err();
  // err == BuildError::LockWithUnion
  ```

## Dialect note: SQLite is a silent no-op

> **Dialect note** — SQLite has no row-level locks: it locks the **whole
> database** per transaction, so `FOR UPDATE` does not exist in its grammar.
> Rather than emit invalid SQL, the compiler silently drops the entire lock
> clause (strength and modifier) on SQLite:
>
> ```rust
> let (sql, _) = QueryBuilder::<Sqlite>::table("jobs")
>     .select(["id"])
>     .for_update()
>     .skip_locked()
>     .to_sql();
> // SELECT "id" FROM "jobs"
> ```
>
> Because the lock is dropped before the UNION check runs, lock + `UNION` on
> SQLite is also a harmless no-op rather than a `LockWithUnion` error:
>
> ```rust
> let arm = QueryBuilder::<Sqlite>::table("archived_jobs").select(["id"]);
> let (sql, _) = QueryBuilder::<Sqlite>::table("jobs")
>     .select(["id"])
>     .for_update()
>     .union(arm)
>     .to_sql();
> // SELECT "id" FROM "jobs" UNION SELECT "id" FROM "archived_jobs"
> ```
>
> The SELECT-only guard, by contrast, still fires on SQLite (see above).

MySQL renders the same clauses with its own quoting:

```rust
let (sql, _) = QueryBuilder::<MySql>::table("jobs")
    .select(["id"])
    .for_update()
    .skip_locked()
    .to_sql();
// SELECT `id` FROM `jobs` FOR UPDATE SKIP LOCKED
```

## Typical use: claim a job inside a transaction

The canonical pattern is `SELECT … FOR UPDATE SKIP LOCKED` inside a
transaction: each worker claims a different row, already-claimed rows are
skipped, and the lock is released at commit:

```rust
use chain_builder::{Postgres, QueryBuilder};

async fn claim_next_job(pool: &sqlx::PgPool) -> Result<(), chain_builder::Error> {
    let mut tx = pool.begin().await.map_err(chain_builder::Error::Sqlx)?;

    // Lock one queued job; concurrent workers skip it instead of blocking.
    let job_id: i64 = QueryBuilder::<Postgres>::table("jobs")
        .select(["id"])
        .where_eq("status", "queued")
        .limit(1)
        .for_update()
        .skip_locked()
        .fetch_scalar(&mut *tx)
        .await?;

    QueryBuilder::<Postgres>::table("jobs")
        .update([("status", "running")])
        .where_eq("id", job_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await.map_err(chain_builder::Error::Sqlx)?;
    Ok(())
}
```

Outside a transaction the lock is released as soon as the statement finishes —
which is almost never what you want.

## Related pages

- [CTE & UNION](cte-union.md) — why lock + `UNION` cannot combine
- [Error Handling](../error-handling.md) — `BuildError::LockRequiresSelect` / `LockWithUnion`, `to_sql()` vs `try_to_sql()`
- [Executing with sqlx](../sqlx.md) — running locking queries in a transaction
- [Dialect Differences](../dialects.md) — the row-locking support matrix
