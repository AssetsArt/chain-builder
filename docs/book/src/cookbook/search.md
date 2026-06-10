# Case-insensitive Search

**Problem:** a search box — `?q=ali` should match "Alice", "ALI", "Salim" —
implemented portably across Postgres, MySQL, and SQLite, **without** letting
the user's input smuggle `LIKE` wildcards into your pattern.

Two distinct concerns, often conflated:

1. **Case-insensitivity** — solved by `where_ilike`, which the builder lowers
   per dialect.
2. **Wildcard hygiene** — `%` and `_` inside user input are *pattern
   metacharacters*, and the builder deliberately does **not** escape them.
   That part is yours.

## `where_ilike`: one chain, three dialects

`where_ilike` is the portable case-insensitive match. On **Postgres** it
compiles to the native `ILIKE` operator; **MySQL** and **SQLite** have no
`ILIKE`, so the builder lowers it to `LOWER(col) LIKE LOWER(?)` — same
semantics, same chain:

```rust,ignore
use chain_builder::{MySql, Postgres, QueryBuilder};

let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .select(["id", "name"])
    .where_ilike("name", "%ali%")
    .to_sql();
// SELECT "id", "name" FROM "users" WHERE "name" ILIKE $1

let (sql, _) = QueryBuilder::<MySql>::table("users")
    .select(["id", "name"])
    .where_ilike("name", "%ali%")
    .to_sql();
// SELECT `id`, `name` FROM `users` WHERE LOWER(`name`) LIKE LOWER(?)
```

The pattern is a bound value either way — no injection is possible through
it. SQLite renders like MySQL but with double-quote identifiers.

## The wildcard caveat

The builder binds your pattern verbatim. If you build it as
`format!("%{q}%")` from raw input, then a user who types `100%` matches
`100<anything>`, a lone `%` matches **every row**, and `a_c` matches `abc`.
That is not SQL injection — the value is still safely bound — but it *is* a
correctness and abuse problem: a filter that should narrow results can be
forced wide open, and pathological patterns (`%a%b%c%d%…`) make the database
scan hard.

Escape the three metacharacters before splicing input into a pattern —
backslash first, then `%` and `_` (order matters, or the backslashes
produced for `%`/`_` get double-escaped). The crate ships exactly this:

```rust,ignore
use chain_builder::escape_like; // in-crate since 3.1.0

assert_eq!(escape_like("100%_a\\b"), "100\\%\\_a\\\\b");
```

### Portable form: `where_raw` + `ESCAPE`

What `\` means inside a `LIKE` pattern is itself dialect-defined: Postgres
and MySQL treat backslash as the default escape character, but **SQLite has
no default escape character at all** — `\%` there is two literal characters.
The portable fix is an explicit `ESCAPE` clause, which the structured
`where_like`/`where_ilike` do not model, so this is a legitimate
[`where_raw`](../query/where.md) job:

```rust,ignore
use chain_builder::{Postgres, QueryBuilder, Value};

let q = "50%"; // raw user input
let pattern = format!("%{}%", escape_like(q)); // "%50\%%"

// One bind ($1) precedes the raw fragment, so its placeholder is $2 —
// where_raw is verbatim and does NOT renumber (use ? on MySQL/SQLite).
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["id", "name"])
    .where_eq("status", "active")
    .where_raw(
        r#"LOWER("name") LIKE LOWER($2) ESCAPE '\'"#,
        vec![Value::Text(pattern)],
    )
    .to_sql();
// SELECT "id", "name" FROM "users" WHERE "status" = $1 AND LOWER("name") LIKE LOWER($2) ESCAPE '\'
```

On MySQL, write the escape literal as `ESCAPE '\\'` — inside a MySQL string
literal a single backslash escapes the closing quote, so `'\'` is a syntax
error. (SQLite accepts `'\'` as-is.)

Now `50%` matches only names containing the literal text `50%`. Mind the
`where_raw` contract: the fragment is emitted verbatim — hand-write the
correct `$N` (binds already accumulated + 1) on Postgres, `?` elsewhere, and
quote any identifiers in the fragment yourself. Never build the *fragment*
from user input; only the bound pattern may carry it.

### Simpler: the prefix-match pattern

If prefix search is enough (`ali` → "Alice", "Ali"), you can stay on the
structured API. Escape the input, anchor it at the start, and skip the
trailing-`%`-only worry:

```rust,ignore
let q = "ali";
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .select(["id", "name"])
    .where_ilike("name", format!("{}%", escape_like(q)))
    .to_sql();
// SELECT "id", "name" FROM "users" WHERE "name" ILIKE $1
```

This is also the index-friendly shape: a leading wildcard (`%ali%`) defeats
ordinary B-tree indexes, while `ali%` can use one (on Postgres, an index with
`text_pattern_ops` / `varchar_pattern_ops`, or a functional index on
`LOWER(name)` for the lowered form). Caveat: without an `ESCAPE` clause the
escaped pattern relies on backslash being the default escape character —
true on Postgres and MySQL, **not** on SQLite, where you need the
`where_raw … ESCAPE` form above for literal matching.

## Notes & caveats

- **Escaping is per-pattern, not global.** Apply `escape_like` to the user's
  term only, *then* add your own `%` anchors — escaping after
  `format!("%{q}%")` would neutralize your anchors too.
- **`where_like` shares the caveat.** Everything here applies to the
  case-sensitive `where_like` as well; see [WHERE](../query/where.md).
- **Unicode case-folding differs per backend** (Postgres follows the
  database collation; MySQL depends on column collation — many `_ci`
  collations are case-insensitive for plain `LIKE` already; SQLite's
  built-in `LOWER` only folds ASCII). For ASCII search terms the lowered
  form behaves identically everywhere.
- **Heavy search workloads** outgrow `LIKE`: consider Postgres `pg_trgm` or
  full-text search, MySQL `FULLTEXT`, SQLite FTS5 — all reachable via
  `where_raw` with bound values.

## Related pages

- [WHERE](../query/where.md) — `where_like`, `where_ilike`, and the `where_raw` placeholder contract
- [Security Model](../security.md) — why bound wildcards are an abuse problem, not an injection problem
- [HTTP Filters & Pagination](http-filters-pagination.md) — wiring a search term into a filter chain
