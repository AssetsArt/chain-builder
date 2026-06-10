# Introduction

**chain-builder** is a typed, dialect-aware SQL query builder for Rust. One
builder API, generic over a `Dialect` marker type, targets **PostgreSQL**,
**MySQL**, and **SQLite** — mixing dialects is a compile error, not a runtime
surprise.

Two safety properties hold everywhere in the API, by construction:

- **Values are always bound parameters.** Anything you pass as a value
  (`where_eq("status", "active")`, `insert(...)`, `limit(...)`) becomes a
  placeholder (`$1` on Postgres, `?` on MySQL/SQLite) plus a typed bind — it is
  never interpolated into the SQL string.
- **Identifiers are always escaped.** Table, column, and database names are
  quoted with the dialect's quote character (`"users"` on Postgres/SQLite,
  `` `users` `` on MySQL), with embedded quote characters doubled.

The only places these guarantees do not apply are the explicitly named `*_raw`
escape hatches — see the [Security Model](security.md) for the complete
inventory.

## A taste

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .where_eq("status", "active")
    .to_sql();
// SELECT "id" FROM "users" WHERE "status" = $1
```

Swap `Postgres` for `MySql` and the same chain renders backticks and `?`
placeholders. See [Getting Started](getting-started.md) for installation and a
full walkthrough.

## Dialect feature matrix

| Feature | PostgreSQL | MySQL | SQLite |
|---|---|---|---|
| Placeholder style | `$N` (numbered) | `?` (positional) | `?` (positional) |
| Identifier quote char | `"` | `` ` `` | `"` |
| Upsert style | `ON CONFLICT (...) DO NOTHING / DO UPDATE SET …` | `INSERT IGNORE` / `ON DUPLICATE KEY UPDATE …` | `ON CONFLICT (...) DO NOTHING / DO UPDATE SET …` |
| `RETURNING` | ✅ supported | ❌ not supported (clause is a no-op) | ✅ supported |
| `DISTINCT ON` | ✅ supported | ❌ build error | ❌ build error |
| Row locking (`FOR UPDATE` / `FOR SHARE`) | ✅ supported | ✅ supported | ❌ silent no-op (SQLite locks the whole database) |
| Native `ILIKE` | ✅ native operator | lowered to `LOWER(col) LIKE LOWER(?)` | lowered to `LOWER(col) LIKE LOWER(?)` |

The [Dialect Differences](dialects.md) page expands each row with prose notes
and examples.

## Where to find things

- **crates.io** — <https://crates.io/crates/chain-builder>
- **API reference (docs.rs)** — <https://docs.rs/chain-builder>
- **Source repository** — <https://github.com/AssetsArt/chain-builder>
- **CHANGELOG** — <https://github.com/AssetsArt/chain-builder/blob/main/CHANGELOG.md>

This book is the long-form guide: query-building chapters, a reference section
(binds, errors, sqlx execution, dialects), a cookbook of realistic recipes, and
deep dives into the [security model](security.md) and
[compiler internals](internals.md).
