use chain_builder::{MySql, Postgres, QueryBuilder, Sqlite, Value};

#[test]
fn postgres_inner_and_left_join_with_on() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["users.id"])
        .join("orders", |j| j.on("orders.user_id", "=", "users.id"))
        .left_join("profiles", |j| j.on("profiles.user_id", "=", "users.id"))
        .to_sql();

    assert_eq!(
        sql,
        r#"SELECT "users"."id" FROM "users" INNER JOIN "orders" ON "orders"."user_id" = "users"."id" LEFT JOIN "profiles" ON "profiles"."user_id" = "users"."id""#
    );
    assert!(binds.is_empty());
}

#[test]
fn postgres_on_val_placeholder_before_where() {
    // joins emit before where: ON $1 then WHERE $2
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .join("orders", |j| {
            j.on("orders.user_id", "=", "users.id")
                .on_val("orders.status", "=", "paid")
        })
        .where_eq("users.active", true)
        .to_sql();

    assert_eq!(
        sql,
        r#"SELECT "id" FROM "users" INNER JOIN "orders" ON "orders"."user_id" = "users"."id" AND "orders"."status" = $1 WHERE "users"."active" = $2"#
    );
    assert_eq!(binds, vec![Value::Text("paid".into()), Value::Bool(true)]);
}

#[test]
fn mysql_inner_join() {
    let (sql, binds) = QueryBuilder::<MySql>::table("users")
        .select(["id"])
        .join("orders", |j| j.on("orders.user_id", "=", "users.id"))
        .to_sql();

    assert_eq!(
        sql,
        "SELECT `id` FROM `users` INNER JOIN `orders` ON `orders`.`user_id` = `users`.`id`"
    );
    assert!(binds.is_empty());
}

#[test]
fn sqlite_left_join_with_on_val() {
    let (sql, binds) = QueryBuilder::<Sqlite>::table("users")
        .select(["id"])
        .left_join("orders", |j| j.on_val("orders.status", "=", "paid"))
        .to_sql();

    assert_eq!(
        sql,
        r#"SELECT "id" FROM "users" LEFT JOIN "orders" ON "orders"."status" = ?"#
    );
    assert_eq!(binds, vec![Value::Text("paid".into())]);
}

#[test]
fn right_and_full_outer_join() {
    let (sql, _binds) = QueryBuilder::<Postgres>::table("a")
        .select(["id"])
        .right_join("b", |j| j.on("b.a_id", "=", "a.id"))
        .full_outer_join("c", |j| j.on("c.a_id", "=", "a.id"))
        .to_sql();

    assert_eq!(
        sql,
        r#"SELECT "id" FROM "a" RIGHT JOIN "b" ON "b"."a_id" = "a"."id" FULL OUTER JOIN "c" ON "c"."a_id" = "a"."id""#
    );
}

#[test]
fn cross_join_has_no_on() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("a")
        .select(["id"])
        .cross_join("b")
        .to_sql();

    assert_eq!(sql, r#"SELECT "id" FROM "a" CROSS JOIN "b""#);
    assert!(binds.is_empty());
}

#[test]
fn on_raw_is_verbatim() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("a")
        .select(["id"])
        .join("b", |j| {
            j.on_raw(
                r#""b"."a_id" = "a"."id" AND "b"."n" > $1"#,
                vec![Value::I64(5)],
            )
        })
        .to_sql();

    assert_eq!(
        sql,
        r#"SELECT "id" FROM "a" INNER JOIN "b" ON "b"."a_id" = "a"."id" AND "b"."n" > $1"#
    );
    assert_eq!(binds, vec![Value::I64(5)]);
}

#[test]
fn join_with_having() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("orders")
        .select(["user_id"])
        .group_by(["user_id"])
        .having("user_id", ">", 10i64)
        .having_raw("COUNT(*) > ?", vec![Value::I64(2)])
        .to_sql();

    assert_eq!(
        sql,
        r#"SELECT "user_id" FROM "orders" GROUP BY "user_id" HAVING "user_id" > $1 AND COUNT(*) > ?"#
    );
    assert_eq!(binds, vec![Value::I64(10), Value::I64(2)]);
}

#[test]
fn having_placeholder_continues_from_where() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("orders")
        .select(["user_id"])
        .where_eq("active", true)
        .group_by(["user_id"])
        .having("total", ">", 100i64)
        .to_sql();

    assert_eq!(
        sql,
        r#"SELECT "user_id" FROM "orders" WHERE "active" = $1 GROUP BY "user_id" HAVING "total" > $2"#
    );
    assert_eq!(binds, vec![Value::Bool(true), Value::I64(100)]);
}
