# Upsert & RETURNING

Conflict handling for `INSERT` comes through exactly **two entry points**:
`on_conflict_do_nothing(targets)` (skip the conflicting row) and
`on_conflict_merge(targets)` (update it from the proposed row). You never
construct conflict actions yourself — the `ConflictAction` type is AST-only.
Both methods are INSERT-only (ignored on UPDATE/DELETE) and compose with
[`insert` / `insert_many`](insert-update-delete.md). This page also covers
`returning`, which is per-dialect: real on Postgres/SQLite, a **silent no-op
on MySQL**.

## `on_conflict_do_nothing(targets)`

`targets` are the conflict-target columns (escaped; the list may be empty).

**Postgres / SQLite** render `ON CONFLICT (…) DO NOTHING` — or bare
`ON CONFLICT DO NOTHING` when `targets` is empty:

```rust
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .insert([("id", 1i64), ("email", 0), ("name", 0)])
    .on_conflict_do_nothing(["id"])
    .to_sql();
// INSERT INTO "users" ("email", "id", "name") VALUES ($1, $2, $3) ON CONFLICT ("id") DO NOTHING
```

```rust
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .insert([("id", 1i64), ("name", 0)])
    .on_conflict_do_nothing(Vec::<&str>::new())
    .to_sql();
// INSERT INTO "users" ("id", "name") VALUES ($1, $2) ON CONFLICT DO NOTHING
```

SQLite is identical apart from `?` placeholders:

```rust
let (sql, _) = QueryBuilder::<Sqlite>::table("users")
    .insert([("id", 1i64), ("email", 0), ("name", 0)])
    .on_conflict_do_nothing(["id"])
    .to_sql();
// INSERT INTO "users" ("email", "id", "name") VALUES (?, ?, ?) ON CONFLICT ("id") DO NOTHING
```

**MySQL** has no `ON CONFLICT`; `DoNothing` becomes the `INSERT IGNORE INTO`
keyword with no trailing clause:

```rust
let (sql, _) = QueryBuilder::<MySql>::table("users")
    .insert([("id", 1i64), ("email", 0), ("name", 0)])
    .on_conflict_do_nothing(["id"])
    .to_sql();
// INSERT IGNORE INTO `users` (`email`, `id`, `name`) VALUES (?, ?, ?)
```

> **Dialect note** — MySQL `IGNORE` suppresses *more* than duplicate-key
> errors: it also downgrades truncation and bad-value-coercion errors to
> warnings. That is broader than Postgres/SQLite `DO NOTHING`. If you rely on
> those errors surfacing, `INSERT IGNORE` is not a drop-in equivalent.

## `on_conflict_merge(targets)`

On conflict, update the existing row from the values the `INSERT` proposed.

**Postgres / SQLite** render
`ON CONFLICT (targets) DO UPDATE SET col = EXCLUDED.col, …` where the SET
list is **the inserted columns minus the conflict targets** (you don't
re-assign the key you matched on):

```rust
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .insert([("id", 1i64), ("email", 0), ("name", 0)]) // values irrelevant for SQL shape
    .on_conflict_merge(["id"])
    .to_sql();
// INSERT INTO "users" ("email", "id", "name") VALUES ($1, $2, $3) ON CONFLICT ("id") DO UPDATE SET "email" = EXCLUDED."email", "name" = EXCLUDED."name"
```

```rust
let (sql, _) = QueryBuilder::<Sqlite>::table("users")
    .insert([("id", 1i64), ("email", 0), ("name", 0)])
    .on_conflict_merge(["id"])
    .to_sql();
// INSERT INTO "users" ("email", "id", "name") VALUES (?, ?, ?) ON CONFLICT ("id") DO UPDATE SET "email" = EXCLUDED."email", "name" = EXCLUDED."name"
```

Postgres and SQLite require a conflict target for `DO UPDATE`, and an empty
SET list is invalid SQL. So when `targets` is empty, or the targets cover
**all** inserted columns (nothing left to set), the merge **falls back to the
`DO NOTHING` rendering**:

```rust
// Targets cover every inserted column → nothing to merge.
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .insert([("id", 1i64)])
    .on_conflict_merge(["id"])
    .to_sql();
// INSERT INTO "users" ("id") VALUES ($1) ON CONFLICT ("id") DO NOTHING
```

```rust
// Empty targets → bare DO NOTHING.
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .insert([("id", 1i64), ("name", 0)])
    .on_conflict_merge(Vec::<&str>::new())
    .to_sql();
// INSERT INTO "users" ("id", "name") VALUES ($1, $2) ON CONFLICT DO NOTHING
```

### MySQL merge — targets are IGNORED

> **⚠️ MySQL ignores your conflict targets.** MySQL's
> `ON DUPLICATE KEY UPDATE` has no target list — the *table's own*
> primary/unique keys decide what counts as a duplicate. Whatever you pass as
> `targets` does not appear in the SQL and does not influence which conflicts
> fire. The SET list is also different: it covers **ALL inserted columns**,
> not "inserted minus targets".

```rust
let (sql, _) = QueryBuilder::<MySql>::table("users")
    .insert([("id", 1i64), ("email", 0), ("name", 0)])
    .on_conflict_merge(["id"]) // "id" target: IGNORED on MySQL
    .to_sql();
// INSERT INTO `users` (`email`, `id`, `name`) VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE `email` = VALUES(`email`), `id` = VALUES(`id`), `name` = VALUES(`name`)
```

`VALUES(col)` is used (rather than the 8.0.20+ row-alias syntax) for
MySQL 5.7/8.x compatibility. Including a primary-key column in the insert set
yields a redundant-but-harmless `pk = VALUES(pk)`. If your code must behave
identically across dialects, make sure the MySQL table's unique keys match
the `targets` you pass for Postgres/SQLite — the builder cannot check that
for you.

## `returning(cols)`

`returning` appends a ` RETURNING …` column list to INSERT, UPDATE, or
DELETE. Columns are escaped; the special column `"*"` is emitted
**unescaped** (`RETURNING *`):

```rust
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .insert([("name", "x")])
    .returning(["id"])
    .to_sql();
// INSERT INTO "users" ("name") VALUES ($1) RETURNING "id"
```

```rust
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .insert([("name", "x")])
    .returning(["*"])
    .to_sql();
// INSERT INTO "users" ("name") VALUES ($1) RETURNING *
```

```rust
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .update([("name", "x")])
    .where_eq("id", 1i64)
    .returning(["id"])
    .to_sql();
// UPDATE "users" SET "name" = $1 WHERE "id" = $2 RETURNING "id"
```

```rust
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .delete()
    .where_eq("id", 1i64)
    .returning(["id"])
    .to_sql();
// DELETE FROM "users" WHERE "id" = $1 RETURNING "id"
```

> **Dialect note** — MySQL has no `RETURNING`, and on MySQL this method is a
> **silent no-op**: the clause is simply omitted, no error, no panic. Code
> that expects rows back from an insert must fetch them separately on MySQL
> (e.g. via `last_insert_id`):
>
> ```rust
> let (sql, _) = QueryBuilder::<MySql>::table("users")
>     .insert([("name", "x")])
>     .returning(["id"])
>     .to_sql();
> // INSERT INTO `users` (`name`) VALUES (?)        -- no RETURNING
> ```
>
> On **SQLite**, `RETURNING` requires SQLite ≥ 3.35.0 (2021). The builder's
> dialect support flag is compile-time, not a runtime version check — on an
> older SQLite the statement fails at execution time.

```rust
let (sql, _) = QueryBuilder::<Sqlite>::table("users")
    .insert([("name", "x")])
    .returning(["id"])
    .to_sql();
// INSERT INTO "users" ("name") VALUES (?) RETURNING "id"
```

## Related pages

- [INSERT · UPDATE · DELETE](insert-update-delete.md) — the statements these clauses attach to
- [Bulk Insert & Upsert](../cookbook/bulk-insert-upsert.md) — `insert_many` + `on_conflict_merge` in practice
- [Dialect Differences](../dialects.md) — upsert style and `RETURNING` support per dialect
- [Executing with sqlx](../sqlx.md) — fetching the rows `RETURNING` produces
- [Error Handling](../error-handling.md) — `to_sql()` vs `try_to_sql()`
