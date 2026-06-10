# Dialect Differences

The dialect is a type parameter — `QueryBuilder<Postgres>`,
`QueryBuilder<MySql>`, `QueryBuilder<Sqlite>` — so every difference below is
decided at compile time, with no runtime dialect switch to misconfigure. The
same chain compiles to the right SQL for each backend; where a feature does
not exist on a backend, the builder either fails loud with a
[`BuildError`](error-handling.md), lowers the feature to an equivalent form,
or (in two documented cases) silently drops it. This page is the single
comparison table, with a short note per row pointing at the detailed page.

## The comparison table

| | PostgreSQL | MySQL | SQLite |
|---|---|---|---|
| Placeholder | `$1`, `$2`, … | `?` | `?` |
| Identifier quote | `"col"` | `` `col` `` | `"col"` |
| Upsert style | `ON CONFLICT (…) DO NOTHING / DO UPDATE SET …` | `INSERT IGNORE` / `ON DUPLICATE KEY UPDATE …` | `ON CONFLICT (…) DO NOTHING / DO UPDATE SET …` |
| `RETURNING` | ✓ | **silently dropped** | ✓ |
| `DISTINCT ON` | ✓ | `BuildError` | `BuildError` |
| Row locking (`FOR UPDATE` / `FOR SHARE`) | ✓ | ✓ | **silent no-op** |
| `ILIKE` | native `ILIKE` | `LOWER(col) LIKE LOWER(?)` | `LOWER(col) LIKE LOWER(?)` |

## Placeholders

Postgres uses numbered placeholders (`$1`, `$2`, …, numbered in bind order
and kept contiguous across CTEs, subqueries and UNION arms); MySQL and SQLite
use positional `?`. You never write a placeholder yourself for bound values —
but raw fragments (`select_raw`, `having_raw`, …) must use the target
dialect's syntax, see the placeholder contract in
[SELECT](query/select.md). Bind ordering details:
[Binds & Values](binds.md).

## Identifier quoting

Every identifier the builder emits is quoted and escaped: double quotes on
Postgres and SQLite, backticks on MySQL. Quote characters embedded in a name
are doubled (`"` → `""`, `` ` `` → ` `` `), so identifiers can never break
out of their quoting. See the [Security Model](security.md).

## Upsert style

Postgres and SQLite share the `ON CONFLICT` grammar; MySQL has its own.
`on_conflict_do_nothing(targets)` renders `ON CONFLICT (…) DO NOTHING` on
Postgres/SQLite and `INSERT IGNORE` on MySQL (which ignores the conflict
targets — MySQL decides conflicts by whatever unique key collides).
`on_conflict_merge(targets)` renders `ON CONFLICT (…) DO UPDATE SET …` vs
`ON DUPLICATE KEY UPDATE …`. Full semantics and caveats:
[Upsert & RETURNING](query/upsert-returning.md).

## `RETURNING`

Supported on Postgres and SQLite. MySQL has no `RETURNING` clause, and the
builder **silently drops** it there rather than emitting invalid SQL — a
`returning(["id"])` chain still compiles and executes on MySQL, it just
returns no rows. If you need the inserted id on MySQL, use
[`execute`](sqlx.md)'s `QueryResult` (`last_insert_id`). Details and the
reasoning: [Upsert & RETURNING](query/upsert-returning.md).

## `DISTINCT ON`

A Postgres-only feature. Compiling a `distinct_on(...)` builder for MySQL or
SQLite is `BuildError::DistinctOnRequiresPostgres` from `try_to_sql()` (or a
panic with `DISTINCT ON requires PostgreSQL` from `to_sql()`) — there is no
silent fallback, because plain `DISTINCT` has different semantics. See
[SELECT](query/select.md) and [Error Handling](error-handling.md).

## Row locking

`for_update()` / `for_share()` (plus `skip_locked()` / `no_wait()`) render on
Postgres and MySQL. SQLite has no row-level locks — it locks the whole
database per transaction — so the entire lock clause is a **silent no-op**
there. The structural guards (`LockRequiresSelect`, and `LockWithUnion` on
locking dialects) still apply; the full matrix, including why SQLite's
lock-with-UNION is a no-op rather than an error, is in
[Row Locking](query/locking.md).

## `ILIKE`

`where_ilike` is portable case-insensitive matching: Postgres compiles it to
its native `col ILIKE $n`; MySQL and SQLite lower it to
`LOWER(col) LIKE LOWER(?)`. Semantics are equivalent for ASCII; for
collation- and Unicode-edge details (and the LIKE-wildcard caveat), see
[WHERE](query/where.md) and
[Case-insensitive Search](cookbook/search.md).

## Related pages

- [Getting Started](getting-started.md) — selecting the dialect via feature flags
- [Upsert & RETURNING](query/upsert-returning.md) — both upsert grammars side by side
- [Row Locking](query/locking.md) — the SQLite no-op in detail
- [SELECT](query/select.md) — `distinct_on` and raw-fragment placeholder contracts
- [Error Handling](error-handling.md) — the `BuildError` variants behind the table
- [Internals](internals.md) — how the `Dialect` trait drives compilation
