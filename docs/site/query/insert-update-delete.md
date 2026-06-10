# INSERT · UPDATE · DELETE

A builder starts life as a SELECT; calling `insert`, `insert_many`, `update`,
or `delete` switches it to a write statement. Values flow through the same
`IntoBind` machinery as WHERE predicates (see [Binds & Values](../binds.md)),
column names are escaped, and — the property worth remembering — **columns
are sorted alphabetically**, so the generated SQL is deterministic regardless
of the order you list the pairs in. Conflict handling and `RETURNING` live on
the next page: [Upsert & RETURNING](upsert-returning.md).

## `insert(pairs)` — single row

`insert` takes any iterable of `(column, value)` pairs. Columns are sorted
alphabetically before rendering; binds follow the sorted column order:

```rust
let (sql, binds) = QueryBuilder::<Sqlite>::table("users")
    .insert([("name", "John"), ("age", "30")])
    .to_sql();
// sorted keys: age, name
// INSERT INTO "users" ("age", "name") VALUES (?, ?)
// binds == [Value::Text("30".into()), Value::Text("John".into())]
```

Deterministic SQL is not cosmetic: byte-identical statements cache better
(prepared-statement caches key on SQL text) and diff cleanly in logs and
tests.

To mix value types in one row, pass `Value` directly (or call
`.into_bind()`):

```rust
let (sql, binds) = QueryBuilder::<Sqlite>::table("users")
    .insert([
        ("name", Value::Text("John".into())),
        ("age", Value::I64(30)),
    ])
    .to_sql();
// INSERT INTO "users" ("age", "name") VALUES (?, ?)
// binds == [Value::I64(30), Value::Text("John".into())]
```

`Option` binds NULL for `None` — handy for nullable columns:

```rust
let (sql, binds) = QueryBuilder::<Sqlite>::table("u")
    .insert([
        ("active", true.into_bind()),
        ("nickname", Option::<&str>::None.into_bind()),
    ])
    .to_sql();
// INSERT INTO "u" ("active", "nickname") VALUES (?, ?)
// binds == [Value::Bool(true), Value::Null]
```

## `insert_many(rows)` — multi-row insert

`insert_many` takes an iterator of rows, each itself a sequence of
`(column, value)` pairs. It renders one `(…)` tuple per row:

```rust
let (sql, binds) = QueryBuilder::<Postgres>::table("u")
    .insert_many([[("a", 1i64), ("b", 2i64)], [("a", 3i64), ("b", 4i64)]])
    .to_sql();
// INSERT INTO "u" ("a", "b") VALUES ($1, $2), ($3, $4)
// binds == [Value::I64(1), Value::I64(2), Value::I64(3), Value::I64(4)]
```

Two rules govern the column set:

1. **The inserted columns come from the FIRST row's keys**, sorted
   alphabetically (same determinism as `insert`).
2. **Ragged rows are NULL-padded, never an error**: for every column in that
   set, each subsequent row binds its value — or `Value::Null` if the key is
   missing from that row. A malformed later row can therefore never panic
   your handler (DoS-safe by design):

```rust
// Second row missing "b" → that slot binds Null.
let (sql, binds) = QueryBuilder::<Postgres>::table("u")
    .insert_many([vec![("a", 1i64), ("b", 2i64)], vec![("a", 3i64)]])
    .to_sql();
// INSERT INTO "u" ("a", "b") VALUES ($1, $2), ($3, $4)
// binds == [Value::I64(1), Value::I64(2), Value::I64(3), Value::Null]
```

The flip side: a key that appears only in a *later* row is silently dropped —
the first row defines the schema of the statement.

`insert_many` composes with `on_conflict_*` and `returning` exactly like
single-row `insert`:

```rust
let (sql, binds) = QueryBuilder::<Postgres>::table("u")
    .insert_many([[("a", 1i64), ("b", 2i64)], [("a", 3i64), ("b", 4i64)]])
    .on_conflict_do_nothing(["a"])
    .returning(["a"])
    .to_sql();
// INSERT INTO "u" ("a", "b") VALUES ($1, $2), ($3, $4) ON CONFLICT ("a") DO NOTHING RETURNING "a"
```

See [Bulk Insert & Upsert](../cookbook/bulk-insert-upsert.md) for batching
guidance.

## `update(pairs)` — with WHERE

`update` takes the same `(column, value)` pairs (also sorted alphabetically)
and renders a `SET` list. The full [WHERE](where.md) API still applies — its
binds come **after** the SET binds, because `SET` renders first:

```rust
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .update([("age", 31i64)])
    .where_eq("id", 1i64)
    .to_sql();
// UPDATE "users" SET "age" = $1 WHERE "id" = $2
// binds == [Value::I64(31), Value::I64(1)]
```

```rust
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .update([("name", Value::Text("a".into())), ("age", Value::I64(2))])
    .where_eq("id", 1i64)
    .to_sql();
// UPDATE "users" SET "age" = $1, "name" = $2 WHERE "id" = $3
// binds == [Value::I64(2), Value::Text("a".into()), Value::I64(1)]
```

## UPDATE expressions: `set_raw`, `increment`, `decrement`

Plain `update()` can only bind values (`SET "col" = $1`). Three companions
cover computed assignments (3.1.0+). All three switch the builder to UPDATE,
so `increment` alone is a valid statement:

```rust
use chain_builder::{Postgres, QueryBuilder, Value};

let (sql, binds) = QueryBuilder::<Postgres>::table("t")
    .update([("name", "x")])
    .increment("views", 1i64)
    .set_raw("updated_at", "NOW()", vec![])
    .where_eq("id", 9i64)
    .to_sql();
// UPDATE "t" SET "name" = $1, "views" = "views" + $2, "updated_at" = NOW() WHERE "id" = $3
```

Ordering: the (sorted) `update()` columns render first, then expressions in
call order. `increment`/`decrement` are fully structured — column escaped,
amount bound. `set_raw` is the verbatim escape hatch and follows the same
positional-placeholder contract as `where_raw`: on Postgres hand-write `$N`
counting all binds accumulated so far (`SET` precedes `WHERE`, and a
preceding `increment`/`decrement` counts as one bind); on MySQL/SQLite use
`?`. Duplicate target columns are not detected — the database reports them.

## `delete()`

`delete` takes no arguments; WHERE applies as usual:

```rust
let (sql, binds) = QueryBuilder::<Sqlite>::table("users")
    .delete()
    .where_eq("id", 1i64)
    .to_sql();
// DELETE FROM "users" WHERE "id" = ?
// binds == [Value::I64(1)]
```

A `delete()` **without** WHERE compiles happily — to a statement that deletes
every row. The builder does not second-guess you here; if that is not what
you meant, the bug is yours:

```rust
let (sql, binds) = QueryBuilder::<Sqlite>::table("users").delete().to_sql();
// DELETE FROM "users"
```

## Empty-set errors

An `INSERT` with no columns or an `UPDATE` with an empty `SET` list is not
valid SQL, so the compiler refuses to render it:

- empty `insert(…)` / `insert_many(…)` → `BuildError::EmptyInsert`
  (`insert() requires at least one column`)
- `update(…)` with no columns **and** no `SET` expressions →
  `BuildError::EmptyUpdate` (`update() requires at least one column`).
  An empty `update(…)` plus an `increment`/`decrement`/`set_raw` is a
  valid UPDATE (3.1.0+).

As always, [`try_to_sql()`](../error-handling.md) returns the error and
`to_sql()` panics with the same message:

```rust
let err = QueryBuilder::<Postgres>::table("users")
    .insert(std::iter::empty::<(&str, Value)>())
    .try_to_sql()
    .unwrap_err();
// err == BuildError::EmptyInsert
// err.to_string() == "insert() requires at least one column"

let err = QueryBuilder::<Postgres>::table("users")
    .update(std::iter::empty::<(&str, Value)>())
    .try_to_sql()
    .unwrap_err();
// err == BuildError::EmptyUpdate
// err.to_string() == "update() requires at least one column"
```

This bites in practice when the SET list is built from optional request
fields and they are all absent — use `try_to_sql()` and map the error to an
HTTP 4XX (see [Mapping Errors to HTTP Status](../cookbook/http-error-mapping.md)).

> **Dialect note** — writes differ across dialects only in quoting and
> placeholders, e.g. on MySQL:
> ``INSERT INTO `u` (`a`, `b`) VALUES (?, ?), (?, ?)``
> (see [Dialect Differences](../dialects.md)). The SELECT-only
> clauses (`group_by`, `order_by`, `limit`, CTEs, unions, locks) are ignored
> on writes — except a [row lock](locking.md), which fails loud with
> `BuildError::LockRequiresSelect` rather than silently not locking.

## Related pages

- [Upsert & RETURNING](upsert-returning.md) — `on_conflict_*` and `returning` for these statements
- [WHERE](where.md) — predicates for `update`/`delete`
- [Binds & Values](../binds.md) — `IntoBind`, `Value`, `Option` → NULL
- [Error Handling](../error-handling.md) — `EmptyInsert`/`EmptyUpdate` and the try/panic twins
- [Bulk Insert & Upsert](../cookbook/bulk-insert-upsert.md) — batching `insert_many`
