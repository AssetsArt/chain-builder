#![cfg(feature = "v2")]

use chain_builder::v2::{MySql, Order, Postgres, QueryBuilder, Sqlite, Value};

#[test]
fn postgres_group_order_limit_offset() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .where_eq("status", "active")
        .group_by(["dept"])
        .order_by_desc("created")
        .limit(10)
        .offset(20)
        .to_sql();

    assert_eq!(
        sql,
        r#"SELECT "id" FROM "users" WHERE "status" = $1 GROUP BY "dept" ORDER BY "created" DESC LIMIT $2 OFFSET $3"#
    );
    assert_eq!(
        binds,
        vec![Value::Text("active".into()), Value::I64(10), Value::I64(20)]
    );
}

#[test]
fn mysql_group_order_limit_offset() {
    let (sql, binds) = QueryBuilder::<MySql>::table("users")
        .select(["id"])
        .where_eq("status", "active")
        .group_by(["dept"])
        .order_by_desc("created")
        .limit(10)
        .offset(20)
        .to_sql();

    assert_eq!(
        sql,
        "SELECT `id` FROM `users` WHERE `status` = ? GROUP BY `dept` ORDER BY `created` DESC LIMIT ? OFFSET ?"
    );
    assert_eq!(
        binds,
        vec![Value::Text("active".into()), Value::I64(10), Value::I64(20)]
    );
}

#[test]
fn sqlite_group_order_limit_offset() {
    let (sql, binds) = QueryBuilder::<Sqlite>::table("users")
        .select(["id"])
        .where_eq("status", "active")
        .group_by(["dept"])
        .order_by_desc("created")
        .limit(10)
        .offset(20)
        .to_sql();

    assert_eq!(
        sql,
        r#"SELECT "id" FROM "users" WHERE "status" = ? GROUP BY "dept" ORDER BY "created" DESC LIMIT ? OFFSET ?"#
    );
    assert_eq!(
        binds,
        vec![Value::Text("active".into()), Value::I64(10), Value::I64(20)]
    );
}

#[test]
fn dotted_group_col_is_escaped() {
    let (sql, _binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .group_by(["t.col"])
        .to_sql();

    assert_eq!(sql, r#"SELECT "id" FROM "users" GROUP BY "t"."col""#);
}

#[test]
fn multi_col_order_asc_and_desc() {
    let (sql, _binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .order_by_asc("a")
        .order_by_desc("b")
        .to_sql();

    assert_eq!(
        sql,
        r#"SELECT "id" FROM "users" ORDER BY "a" ASC, "b" DESC"#
    );
}

#[test]
fn order_by_with_explicit_order_enum() {
    let (sql, _binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .order_by("a", Order::Asc)
        .order_by("b", Order::Desc)
        .to_sql();

    assert_eq!(
        sql,
        r#"SELECT "id" FROM "users" ORDER BY "a" ASC, "b" DESC"#
    );
}

#[test]
fn multi_col_group_by() {
    let (sql, _binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .group_by(["a", "b"])
        .to_sql();

    assert_eq!(sql, r#"SELECT "id" FROM "users" GROUP BY "a", "b""#);
}

#[test]
fn limit_without_offset() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .limit(5)
        .to_sql();

    assert_eq!(sql, r#"SELECT "id" FROM "users" LIMIT $1"#);
    assert_eq!(binds, vec![Value::I64(5)]);
}

#[test]
#[should_panic(expected = "offset(...) requires limit(...)")]
fn offset_without_limit_panics() {
    let _ = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .offset(5)
        .to_sql();
}
