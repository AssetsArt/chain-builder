//! M8: subqueries + `where_column`.

use chain_builder::{MySql, Postgres, QueryBuilder, Sqlite, Value};

#[test]
fn pg_where_exists_placeholder_continuity() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .where_eq("active", true)
        .where_exists(
            QueryBuilder::<Postgres>::table("orders")
                .select(["1"])
                .where_column("orders.user_id", "=", "users.id")
                .where_gt("total", 100i64),
        )
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "id" FROM "users" WHERE "active" = $1 AND EXISTS (SELECT "1" FROM "orders" WHERE "orders"."user_id" = "users"."id" AND "total" > $2)"#
    );
    assert_eq!(binds, vec![Value::Bool(true), Value::I64(100)]);
}

#[test]
fn pg_where_not_exists() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .where_not_exists(
            QueryBuilder::<Postgres>::table("orders")
                .select(["1"])
                .where_column("orders.user_id", "=", "users.id"),
        )
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "id" FROM "users" WHERE NOT EXISTS (SELECT "1" FROM "orders" WHERE "orders"."user_id" = "users"."id")"#
    );
    assert!(binds.is_empty());
}

#[test]
fn pg_where_in_subquery() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .where_in_subquery(
            "id",
            QueryBuilder::<Postgres>::table("ban")
                .select(["user_id"])
                .where_eq("k", 7i64),
        )
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "id" FROM "users" WHERE "id" IN (SELECT "user_id" FROM "ban" WHERE "k" = $1)"#
    );
    assert_eq!(binds, vec![Value::I64(7)]);
}

#[test]
fn pg_where_not_in_subquery() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .where_not_in_subquery(
            "id",
            QueryBuilder::<Postgres>::table("ban")
                .select(["user_id"])
                .where_eq("k", 7i64),
        )
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "id" FROM "users" WHERE "id" NOT IN (SELECT "user_id" FROM "ban" WHERE "k" = $1)"#
    );
    assert_eq!(binds, vec![Value::I64(7)]);
}

#[test]
fn pg_where_column_standalone() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .select(["x"])
        .where_column("a.x", "=", "b.y")
        .to_sql();
    assert_eq!(sql, r#"SELECT "x" FROM "t" WHERE "a"."x" = "b"."y""#);
    assert!(binds.is_empty());
}

#[test]
fn pg_select_subquery() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .select_subquery(
            "cnt",
            QueryBuilder::<Postgres>::table("orders")
                .select_raw("COUNT(*)", None)
                .where_column("orders.uid", "=", "users.id"),
        )
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "id", (SELECT COUNT(*) FROM "orders" WHERE "orders"."uid" = "users"."id") AS "cnt" FROM "users""#
    );
    assert!(binds.is_empty());
}

#[test]
fn pg_select_subquery_bind_before_where_bind() {
    // The select-list subquery carries a bind ($1); the outer WHERE carries
    // another ($2). SELECT is emitted first, so the subquery bind must be $1.
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .select_subquery(
            "cnt",
            QueryBuilder::<Postgres>::table("orders")
                .select_raw("COUNT(*)", None)
                .where_eq("status", 1i64),
        )
        .where_eq("active", true)
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "id", (SELECT COUNT(*) FROM "orders" WHERE "status" = $1) AS "cnt" FROM "users" WHERE "active" = $2"#
    );
    assert_eq!(binds, vec![Value::I64(1), Value::Bool(true)]);
}

#[test]
fn mysql_where_exists() {
    let (sql, binds) = QueryBuilder::<MySql>::table("users")
        .select(["id"])
        .where_exists(
            QueryBuilder::<MySql>::table("orders")
                .select(["1"])
                .where_column("orders.user_id", "=", "users.id")
                .where_gt("total", 100i64),
        )
        .to_sql();
    assert_eq!(
        sql,
        "SELECT `id` FROM `users` WHERE EXISTS (SELECT `1` FROM `orders` WHERE `orders`.`user_id` = `users`.`id` AND `total` > ?)"
    );
    assert_eq!(binds, vec![Value::I64(100)]);
}

#[test]
fn sqlite_where_in_subquery() {
    let (sql, binds) = QueryBuilder::<Sqlite>::table("users")
        .select(["id"])
        .where_in_subquery(
            "id",
            QueryBuilder::<Sqlite>::table("ban")
                .select(["user_id"])
                .where_eq("k", 7i64),
        )
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "id" FROM "users" WHERE "id" IN (SELECT "user_id" FROM "ban" WHERE "k" = ?)"#
    );
    assert_eq!(binds, vec![Value::I64(7)]);
}

#[test]
fn regression_plain_builder_unchanged() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id", "name"])
        .where_eq("active", true)
        .where_gt("age", 18i64)
        .order_by_asc("name")
        .limit(10)
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "id", "name" FROM "users" WHERE "active" = $1 AND "age" > $2 ORDER BY "name" ASC LIMIT $3"#
    );
    assert_eq!(
        binds,
        vec![Value::Bool(true), Value::I64(18), Value::I64(10)]
    );
}

// --- 3.1.0: subquery predicates inside and_where / or_where groups ---

#[test]
fn pg_group_where_exists_continuity() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .where_eq("active", true)
        .and_where(|g| {
            g.where_exists(
                QueryBuilder::<Postgres>::table("orders")
                    .select(["1"])
                    .where_column("orders.user_id", "=", "users.id")
                    .where_gt("total", 100i64),
            )
            .or_where(|h| h.where_eq("vip", true))
        })
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "id" FROM "users" WHERE "active" = $1 AND (EXISTS (SELECT "1" FROM "orders" WHERE "orders"."user_id" = "users"."id" AND "total" > $2) OR ("vip" = $3))"#
    );
    assert_eq!(
        binds,
        vec![Value::Bool(true), Value::I64(100), Value::Bool(true)]
    );
}

#[test]
fn pg_group_in_subquery_and_not_exists() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .and_where(|g| {
            g.where_in_subquery(
                "id",
                QueryBuilder::<Postgres>::table("ban")
                    .select(["user_id"])
                    .where_eq("k", 7i64),
            )
            .where_not_exists(QueryBuilder::<Postgres>::table("audit").select(["1"]))
        })
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "id" FROM "users" WHERE ("id" IN (SELECT "user_id" FROM "ban" WHERE "k" = $1) AND NOT EXISTS (SELECT "1" FROM "audit"))"#
    );
    assert_eq!(binds, vec![Value::I64(7)]);
}

#[test]
fn mysql_group_not_in_subquery() {
    let (sql, binds) = QueryBuilder::<MySql>::table("users")
        .select(["id"])
        .and_where(|g| {
            g.where_not_in_subquery(
                "id",
                QueryBuilder::<MySql>::table("ban")
                    .select(["user_id"])
                    .where_eq("k", 7i64),
            )
        })
        .to_sql();
    assert_eq!(
        sql,
        "SELECT `id` FROM `users` WHERE (`id` NOT IN (SELECT `user_id` FROM `ban` WHERE `k` = ?))"
    );
    assert_eq!(binds, vec![Value::I64(7)]);
}
