# chain-builder v2 — M3 plan: Upsert (`on_conflict`) + `RETURNING`

Spec: M3 milestone · Base: `bf208e1` (branch `feat/v2-m3`, off `v2`) · TDD ·
`feature = "v2"`; 1.x untouched. Replaces 1.x's broken no-op `insert_ignore` /
`insert_or_update`.

## Goal

Real, dialect-correct upsert and `RETURNING` on `QueryBuilder<D>` INSERT (and
`RETURNING` on UPDATE/DELETE for pg/sqlite). No new binds are introduced by either
feature (excluded/VALUES references + returning column lists carry no parameters),
so placeholder ordering is unaffected.

## Dialect divergence — the crux

Add a capability to the `Dialect` trait so `compile` can branch without a runtime
dialect discriminant:

```rust
pub enum UpsertStyle { OnConflict, OnDuplicateKey } // pg/sqlite vs mysql
// on Dialect:
fn upsert_style() -> UpsertStyle;
```
- `Postgres`/`Sqlite` → `OnConflict`; `MySql` → `OnDuplicateKey`.
- `supports_returning()` already exists (pg/sqlite true, mysql false) — reuse for RETURNING.

### Exact SQL per dialect (verify in plan review)

INSERT base: `INSERT INTO {tbl} ({cols}) VALUES ({ph…})`.

**Postgres / SQLite (`OnConflict`):**
- merge: `… ON CONFLICT ({targets}) DO UPDATE SET {c} = EXCLUDED.{c}, …`
  - SET list = inserted columns **minus** the conflict targets (don't re-set the
    key). If that list is empty → emit `DO NOTHING` instead.
  - `EXCLUDED` is uppercase (works in both pg and sqlite).
- ignore: `… ON CONFLICT ({targets}) DO NOTHING` (if no targets → `ON CONFLICT DO
  NOTHING`).
- targets + SET columns are identifier-escaped.

**MySQL (`OnDuplicateKey`):**
- merge: `… ON DUPLICATE KEY UPDATE {c} = VALUES({c}), …` (SET list = all inserted
  columns; MySQL uses its own unique key, **targets are ignored** — document).
- ignore: emit `INSERT IGNORE INTO …` (change the INSERT keyword; do NOT append an
  ON-clause). This is the idiomatic MySQL ignore (matches Knex).

### RETURNING
- pg/sqlite: ` RETURNING {cols}` (escaped; `["*"]` → `*`) appended to INSERT /
  UPDATE / DELETE.
- mysql (`supports_returning()==false`): **omitted** (no-op), documented (MySQL has
  no RETURNING; matches Knex). No panic.

## Builder API (`QueryBuilder<D>`)

New fields (raw-stored): `on_conflict: Option<OnConflict>`, `returning:
Vec<String>`.

```rust
pub struct OnConflict { pub targets: Vec<String>, pub action: ConflictAction }
pub enum ConflictAction { DoNothing, Merge } // Merge = update non-target inserted cols
```

Methods (by-value chaining; only meaningful on INSERT):
- `on_conflict_do_nothing<I,S>(self, targets: I) -> Self` (targets may be empty).
- `on_conflict_merge<I,S>(self, targets: I) -> Self`.
- `returning<I,S>(self, cols: I) -> Self` (e.g. `.returning(["id"])` /
  `.returning(["*"])`).

(Targets/cols: `I: IntoIterator<Item=S>, S: AsRef<str>`, stored owned raw.)

## Compile changes (`compile.rs`)

- INSERT arm: after the `VALUES (...)`:
  - keyword: if `on_conflict == DoNothing` AND `D::upsert_style()==OnDuplicateKey`
    → the INSERT keyword is `INSERT IGNORE INTO` (decide at the top of the arm) and
    **no** trailing conflict clause.
  - else render the conflict clause per `upsert_style()` + action as above.
  - then RETURNING (if `supports_returning()` && `!returning.is_empty()`).
- UPDATE / DELETE arms: append RETURNING (if supported & non-empty) at the end.
- `on_conflict` on a non-INSERT method: ignored (documented; like M2 F5).
- Use the existing sorted-key order for the SET list so it matches the inserted
  column order (deterministic).

## Tests — `tests/v2_upsert.rs`

Per dialect, exact SQL:
1. **pg merge:** `.table("users").insert([("id",1),("email","a"),("name","x")])
   .on_conflict_merge(["id"])` →
   `INSERT INTO "users" ("email", "id", "name") VALUES ($1, $2, $3) ON CONFLICT
   ("id") DO UPDATE SET "email" = EXCLUDED."email", "name" = EXCLUDED."name"`
   (note: `id` excluded from SET; binds `[a,1,x]` in sorted-key order).
2. **pg ignore:** `.on_conflict_do_nothing(["id"])` → `… ON CONFLICT ("id") DO
   NOTHING`.
3. **sqlite** equivalents (`"`-quoted, `?`).
4. **mysql merge:** `… ON DUPLICATE KEY UPDATE \`email\` = VALUES(\`email\`),
   \`id\` = VALUES(\`id\`), \`name\` = VALUES(\`name\`)` (all cols; targets ignored).
5. **mysql ignore:** `INSERT IGNORE INTO \`users\` (...) VALUES (...)` (no ON-clause).
6. **pg/sqlite RETURNING:** `.insert(...).returning(["id"])` → `… RETURNING "id"`;
   `.returning(["*"])` → `… RETURNING *`; on UPDATE and DELETE too.
7. **mysql RETURNING omitted:** `.insert(...).returning(["id"])` → no `RETURNING`
   in output.
8. **merge where all cols are targets** (pg) → falls back to `DO NOTHING`.
9. Regression: M1/M2 insert SQL unchanged when no on_conflict/returning set.

## Acceptance
- M1/M2 suites byte-identical (no conflict/returning by default).
- New `tests/v2_upsert.rs` green per dialect (pg `--features v2,sqlx_postgres`,
  mysql `--features v2,sqlx_mysql`, sqlite `--no-default-features
  --features v2,sqlite,sqlx_sqlite`).
- 1.x baseline **63**; no new `src/v2` clippy warnings.

## Out of M3 (deferred)
- `on_conflict_merge_with([(col, value)])` (update to explicit bound values, not
  EXCLUDED) — M-later if needed.
- Named-constraint conflict targets (`ON CONFLICT ON CONSTRAINT name`) — deferred.
- Fetching the RETURNING rows (that's M4 typed-fetch; M3 only emits the clause).
