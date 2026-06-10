# JOIN

Joins combine the main table with others in a `SELECT`. Each join method takes
the table name plus a closure that builds the `ON` conditions on a
`JoinClause` accumulator — except `cross_join`, which by definition has no
condition. Joins are **SELECT-only**: they are ignored for
INSERT/UPDATE/DELETE. Join tables are escaped like every identifier, and if
the builder has a [`.db()` qualifier](#db-qualifies-join-tables-too) it
prefixes the join tables as well.

## Join kinds

| Method | Emits |
|---|---|
| `join(table, f)` | `INNER JOIN table ON …` |
| `left_join(table, f)` | `LEFT JOIN table ON …` |
| `right_join(table, f)` | `RIGHT JOIN table ON …` |
| `full_outer_join(table, f)` | `FULL OUTER JOIN table ON …` |
| `cross_join(table)` | `CROSS JOIN table` (no `ON`) |

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["users.id"])
    .join("orders", |j| j.on("orders.user_id", "=", "users.id"))
    .left_join("profiles", |j| j.on("profiles.user_id", "=", "users.id"))
    .to_sql();
// SELECT "users"."id" FROM "users" INNER JOIN "orders" ON "orders"."user_id" = "users"."id" LEFT JOIN "profiles" ON "profiles"."user_id" = "users"."id"
```

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("a")
    .select(["id"])
    .right_join("b", |j| j.on("b.a_id", "=", "a.id"))
    .full_outer_join("c", |j| j.on("c.a_id", "=", "a.id"))
    .to_sql();
// SELECT "id" FROM "a" RIGHT JOIN "b" ON "b"."a_id" = "a"."id" FULL OUTER JOIN "c" ON "c"."a_id" = "a"."id"
```

`cross_join` takes no closure — a cross join has no condition:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("a")
    .select(["id"])
    .cross_join("b")
    .to_sql();
// SELECT "id" FROM "a" CROSS JOIN "b"
```

> **Dialect note** — the builder renders all five kinds identically on every
> dialect; whether the server accepts them is up to the database. Notably
> MySQL has no `FULL OUTER JOIN`, and SQLite only gained `RIGHT`/`FULL OUTER
> JOIN` in 3.39 — the builder does not guard against this, the database
> rejects the query at execution time.

## Building `ON` conditions

The closure receives an empty `JoinClause` and chains conditions; multiple
conditions are joined with `AND`. There are three condition kinds:

### `on` — column to column

`on(col, op, col2)`: both sides are identifiers, escaped at compile time. The
operator is a `&'static str`, so it cannot come from runtime input:

```rust,ignore
let (sql, _) = QueryBuilder::<MySql>::table("users")
    .select(["id"])
    .join("orders", |j| j.on("orders.user_id", "=", "users.id"))
    .to_sql();
// SELECT `id` FROM `users` INNER JOIN `orders` ON `orders`.`user_id` = `users`.`id`
```

### `on_val` — column to bound value

`on_val(col, op, val)`: the column is escaped, the value becomes a
placeholder + bind — the same discipline as [WHERE](where.md). Join binds are
emitted **before** WHERE binds, because the `JOIN` clause renders first:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .join("orders", |j| {
        j.on("orders.user_id", "=", "users.id")
            .on_val("orders.status", "=", "paid")
    })
    .where_eq("users.active", true)
    .to_sql();
// SELECT "id" FROM "users" INNER JOIN "orders" ON "orders"."user_id" = "users"."id" AND "orders"."status" = $1 WHERE "users"."active" = $2
// binds == [Value::Text("paid"), Value::Bool(true)]
```

### `on_raw` — verbatim fragment

`on_raw(sql, binds)` is the escape hatch for `ON` conditions the structured
API cannot express:

> **⚠️ Positional placeholder contract**
>
> The fragment is emitted **verbatim** — it is NOT escaped and NOT renumbered;
> `binds` are appended to the running bind list in order. On **Postgres** you
> must hand-write `$N` numbers matching the actual bind position; on
> MySQL/SQLite use `?`. See the [Security Model](../security.md) for the full
> escape-hatch inventory.

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("a")
    .select(["id"])
    .join("b", |j| {
        j.on_raw(
            r#""b"."a_id" = "a"."id" AND "b"."n" > $1"#,
            vec![Value::I64(5)],
        )
    })
    .to_sql();
// SELECT "id" FROM "a" INNER JOIN "b" ON "b"."a_id" = "a"."id" AND "b"."n" > $1
```

Note that identifiers inside the raw fragment must be quoted by hand (the
example writes `"b"."a_id"` itself).

## `.db()` qualifies join tables too

A builder-level [`db("name")`](../cookbook/multi-tenant.md) qualifier prefixes
the main table **and every join table** — the multi-tenant assumption is that
all tables live in the same tenant database:

```rust,ignore
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .db("mydb")
    .select(["users.id"])
    .left_join("profiles", |j| j.on("users.id", "=", "profiles.uid"))
    .to_sql();
// SELECT "users"."id" FROM "mydb"."users" LEFT JOIN "mydb"."profiles" ON "users"."id" = "profiles"."uid"
```

Column references in `select`/`on` are not rewritten — qualify them with the
table name as usual.

## Related pages

- [SELECT](select.md) — choosing columns across joined tables
- [WHERE](where.md) — filtering after the join
- [Multi-tenant with `.db()`](../cookbook/multi-tenant.md) — one pool, many schemas
- [Security Model](../security.md) — `on_raw` and the other escape hatches
