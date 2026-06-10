# Multi-tenant with `.db()`

**Problem:** one application serves many tenants, each isolated in its own
database/schema on the same server — `tenant_acme.users`,
`tenant_globex.users`, … You want **one connection pool** and one set of
query-building code, with the tenant decided per request.

`.db(name)` is exactly that switch: it sets a database/schema qualifier on
the builder, and the compiler prefixes the **main table and every join
table** with it, escaped per dialect. The rest of the chain is untouched.

## One pool, many schemas

```rust
use chain_builder::{Postgres, QueryBuilder};

let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .db("tenant_acme")
    .select(["id"])
    .where_eq("status", "active")
    .to_sql();
// SELECT "id" FROM "tenant_acme"."users" WHERE "status" = $1
```

The qualifier applies to every statement kind, so the same tenant routing
covers reads and writes:

```rust
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .db("tenant_acme")
    .update([("name", "a")])
    .where_eq("id", 1i64)
    .to_sql();
// UPDATE "tenant_acme"."users" SET "name" = $1 WHERE "id" = $2
```

On MySQL the qualifier is a database name with backtick quoting
(``FROM `tenant_acme`.`users` ``); on SQLite it addresses an
[`ATTACH`ed database](https://sqlite.org/lang_attach.html)
(`FROM "tenant_acme"."users"`). The chain is identical on all three.

## Joins stay inside the tenant

`.db()` qualifies join tables too — the multi-tenant assumption is that all
tables of a query live in the same tenant database:

```rust
let (sql, _) = QueryBuilder::<Postgres>::table("users")
    .db("tenant_acme")
    .select(["users.id"])
    .left_join("profiles", |j| j.on("users.id", "=", "profiles.uid"))
    .to_sql();
// SELECT "users"."id" FROM "tenant_acme"."users" LEFT JOIN "tenant_acme"."profiles" ON "users"."id" = "profiles"."uid"
```

You cannot point one join at a *different* database through `.db()` — it is a
per-builder setting, by design. Column references in `select`/`on` are not
rewritten; qualify them with the table name as usual
(see [JOIN](../query/join.md)).

## A request-scoped helper

Centralize the tenant decision in one constructor and the rest of your data
layer never mentions tenancy again:

```rust
use chain_builder::{Postgres, QueryBuilder};

struct Tenant {
    schema: &'static str, // resolved from YOUR tenant registry — see below
}

impl Tenant {
    fn query(&self, table: &str) -> QueryBuilder<Postgres> {
        QueryBuilder::<Postgres>::table(table).db(self.schema)
    }
}

// in a handler:
let active = tenant
    .query("users")
    .select(["id", "name"])
    .where_eq("status", "active")
    .order_by_asc("name");
```

## Tenant names: allowlist, don't pass through

The tenant name is an **identifier**, not a bound value — so where it comes
from matters. It should come from your own system (a tenant registry, a row
in a control-plane table, a value baked into the session at login), **not**
raw user input like a host header or URL segment passed through verbatim.

The builder does escape it. A hostile name cannot break out of the quoting:

```rust
let (sql, _) = QueryBuilder::<Postgres>::table("t")
    .db(r#"ev"il"#)
    .select(["x"])
    .to_sql();
// SELECT "x" FROM "ev""il"."t"   — the quote is doubled, not an escape
```

So injection through `.db()` is neutralized — but escaping is not
*authorization*. If request input picks the schema directly, a client who
sends `tenant_globex` instead of `tenant_acme` gets a perfectly valid,
perfectly escaped query against **someone else's data**. Resolve the
request's tenant claim against an allowlist you control:

```rust
fn resolve_tenant(claim: &str) -> Option<&'static str> {
    // your registry: subdomain/JWT claim → schema name you created
    match claim {
        "acme" => Some("tenant_acme"),
        "globex" => Some("tenant_globex"),
        _ => None, // unknown tenant → 404/403, never a schema name
    }
}
```

This is the identifier policy from the [Security Model](../security.md):
identifier escaping makes names *syntactically* safe; *which* name is allowed
is your application's decision.

## Notes & caveats

- **Connection-level state still wins.** `.db()` qualifies table names in
  SQL; it does not change the pool's default database, `search_path`, or
  `USE` state. Unqualified SQL elsewhere in your app (raw `sqlx::query`,
  migrations) still hits the connection default.
- **CTEs and subqueries are separate builders** with their own `.db()` (or
  none). If a subquery should target the tenant schema, set `.db(tenant)` on
  it too — the helper-constructor pattern above makes that automatic.
- **SQLite:** the qualifier is only meaningful for databases attached with
  `ATTACH DATABASE … AS name` on the connection in use.

## Related pages

- [JOIN](../query/join.md) — `.db()` and join qualification
- [Security Model](../security.md) — identifier escaping vs identifier *policy*
- [Dialect Differences](../dialects.md) — quoting per backend
