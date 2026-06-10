# Bulk Insert & Upsert

**Problem:** you have hundreds or thousands of rows to load — an import job,
a sync from an external API — and they must land in one of two ways: insert
new rows and *update* the ones that already exist (upsert), without one
statement per row, and without blowing the backend's placeholder limit.

The pieces: [`insert_many`](../query/insert-update-delete.md) renders one
multi-row `INSERT`, [`on_conflict_merge`](../query/upsert-returning.md) turns
it into an upsert, and a small `chunks()` loop keeps each statement under the
bind-count ceiling.

## `insert_many` + `on_conflict_merge`

```rust,ignore
use chain_builder::{Postgres, QueryBuilder};

let rows = vec![
    vec![("email", "a@x.io"), ("name", "Ann")],
    vec![("email", "b@x.io"), ("name", "Bob")],
];

let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .insert_many(rows)
    .on_conflict_merge(["email"])
    .to_sql();
// INSERT INTO "users" ("email", "name") VALUES ($1, $2), ($3, $4) ON CONFLICT ("email") DO UPDATE SET "name" = EXCLUDED."name"
```

One statement, one round trip: existing `email`s get their `name` updated
from the proposed row, new ones are inserted. The SET list is the inserted
columns *minus* the conflict targets — you never re-assign the key you
matched on.

Two `insert_many` rules to keep in mind (full detail in
[INSERT · UPDATE · DELETE](../query/insert-update-delete.md)):

1. **The first row defines the column set** (sorted alphabetically, like
   `insert`). A key that appears only in a later row is silently dropped.
2. **Ragged rows are NULL-padded, never an error.** A later row missing a key
   binds `Value::Null` for that slot instead of panicking — a malformed
   record in an import batch cannot take the process down:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("u")
    .insert_many([vec![("a", 1i64), ("b", 2i64)], vec![("a", 3i64)]])
    .to_sql();
// INSERT INTO "u" ("a", "b") VALUES ($1, $2), ($3, $4)
// binds == [Value::I64(1), Value::I64(2), Value::I64(3), Value::Null]
```

For "insert new, leave existing untouched" use `on_conflict_do_nothing`
instead — same shape, different conflict action.

## Chunking: respect the placeholder ceiling

Every cell is a bound placeholder, so a batch binds `rows × columns`
parameters — and each backend caps the count per statement:

| Backend | Limit | Source |
|---|---|---|
| Postgres | 65,535 | wire protocol encodes the parameter count as 16-bit |
| MySQL | 65,535 | prepared-statement placeholder count is 16-bit |
| SQLite | 32,766 default (999 before 3.32.0) | `SQLITE_MAX_VARIABLE_NUMBER` compile-time option |

Exceed it and the statement fails at execute time. Stay comfortably under it
by chunking — with the bonus that chunks keep statement size, lock duration,
and memory per round trip bounded even when the hard limit is far away:

```rust,no_run
use chain_builder::{Postgres, QueryBuilder};

const COLS: usize = 2;          // ("email", "name")
const CHUNK: usize = 1_000;     // 1_000 rows × 2 cols = 2_000 binds ≪ 65_535

async fn upsert_users(
    pool: &sqlx::PgPool,
    rows: &[(String, String)],  // (email, name)
) -> Result<u64, chain_builder::Error> {
    let mut tx = pool.begin().await?;
    let mut affected = 0u64;

    for chunk in rows.chunks(CHUNK) {
        let batch = chunk
            .iter()
            .map(|(email, name)| [("email", email.clone()), ("name", name.clone())]);

        let result = QueryBuilder::<Postgres>::table("users")
            .insert_many(batch)
            .on_conflict_merge(["email"])
            .execute(&mut *tx)
            .await?;
        affected += result.rows_affected();
    }

    tx.commit().await?;
    Ok(affected)
}
```

Pick `CHUNK` so that `CHUNK × COLS` clears the *smallest* limit you deploy
against (999 if old SQLite builds are in play), with margin for any extra
binds on the statement (`WHERE`/`RETURNING` expressions, etc.). The
transaction makes the chunked upsert all-or-nothing; drop it if partial
progress is acceptable for your job.

## Notes & caveats

- **MySQL ignores your conflict targets.** `on_conflict_merge(["email"])`
  compiles to `ON DUPLICATE KEY UPDATE …` against **whatever unique/primary
  keys the table has**, and the SET list covers *all* inserted columns. If
  the table carries more than one unique key, the matched row may not be the
  one `["email"]` suggests — see the
  [MySQL merge note](../query/upsert-returning.md) before relying on target
  semantics there.
- **Get the upserted rows back** with `.returning([...])` on Postgres/SQLite
  (per chunk). On MySQL `returning` is a silent no-op — query separately.
- **`rows_affected()` counts differently per backend.** Notably MySQL's
  `ON DUPLICATE KEY UPDATE` reports 2 for an updated row and 1 for an
  inserted one, so the sum above is not a row count there.
- **Don't pad with empty batches:** `insert_many` over zero rows leaves the
  builder with no columns, which fails as `BuildError::EmptyInsert` at
  compile time. `chunks()` never yields an empty slice, so the loop above is
  safe — but guard any other path that might hand the builder an empty
  iterator.

## Related pages

- [INSERT · UPDATE · DELETE](../query/insert-update-delete.md) — `insert_many`, NULL padding, sorted columns
- [Upsert & RETURNING](../query/upsert-returning.md) — both conflict actions, per-dialect rendering
- [Executing with sqlx](../sqlx.md) — `execute`, transactions, `rows_affected`
