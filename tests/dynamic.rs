use chain_builder::{MySql, Postgres, QueryBuilder, Sqlite, Value};

#[test]
fn when_true_adds_predicate() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .when(true, |q| q.where_eq("a", 1i64))
        .to_sql();

    assert_eq!(sql, r#"SELECT "id" FROM "users" WHERE "a" = $1"#);
    assert_eq!(binds, vec![Value::I64(1)]);
}

#[test]
fn when_false_skips_predicate() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .when(false, |q| q.where_eq("a", 1i64))
        .to_sql();

    // No WHERE clause at all when the condition is false.
    assert_eq!(sql, r#"SELECT "id" FROM "users""#);
    assert!(!sql.contains("WHERE"));
    assert!(binds.is_empty());
}

#[test]
fn when_true_and_false_differ() {
    let (sql_true, _) = QueryBuilder::<Postgres>::table("users")
        .when(true, |q| q.where_eq("a", 1i64))
        .to_sql();
    let (sql_false, _) = QueryBuilder::<Postgres>::table("users")
        .when(false, |q| q.where_eq("a", 1i64))
        .to_sql();
    assert_ne!(sql_true, sql_false);
}

#[test]
fn when_else_false_applies_false_branch() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .when_else(false, |q| q.where_eq("a", 1i64), |q| q.where_eq("b", 2i64))
        .to_sql();

    assert_eq!(sql, r#"SELECT "id" FROM "users" WHERE "b" = $1"#);
    assert_eq!(binds, vec![Value::I64(2)]);
}

#[test]
fn when_else_true_applies_true_branch() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .when_else(true, |q| q.where_eq("a", 1i64), |q| q.where_eq("b", 2i64))
        .to_sql();

    assert_eq!(sql, r#"SELECT "id" FROM "users" WHERE "a" = $1"#);
    assert_eq!(binds, vec![Value::I64(1)]);
}

#[test]
fn when_mid_chain_preserves_rest_of_builder() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .where_eq("status", "active")
        .when(true, |q| q.where_eq("a", 1i64))
        .order_by_desc("created")
        .limit(5)
        .to_sql();

    assert_eq!(
        sql,
        r#"SELECT "id" FROM "users" WHERE "status" = $1 AND "a" = $2 ORDER BY "created" DESC LIMIT $3"#
    );
    assert_eq!(
        binds,
        vec![Value::Text("active".into()), Value::I64(1), Value::I64(5)]
    );
}

#[test]
fn when_false_mid_chain_preserves_rest_of_builder() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .where_eq("status", "active")
        .when(false, |q| q.where_eq("a", 1i64))
        .order_by_desc("created")
        .limit(5)
        .to_sql();

    assert_eq!(
        sql,
        r#"SELECT "id" FROM "users" WHERE "status" = $1 ORDER BY "created" DESC LIMIT $2"#
    );
    assert_eq!(binds, vec![Value::Text("active".into()), Value::I64(5)]);
}

#[test]
fn paginate_page_2_pg() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .paginate(2, 10)
        .to_sql();

    assert_eq!(sql, r#"SELECT "id" FROM "users" LIMIT $1 OFFSET $2"#);
    assert_eq!(binds, vec![Value::I64(10), Value::I64(10)]);
}

#[test]
fn paginate_page_1_offset_zero_pg() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .paginate(1, 10)
        .to_sql();

    assert_eq!(sql, r#"SELECT "id" FROM "users" LIMIT $1 OFFSET $2"#);
    assert_eq!(binds, vec![Value::I64(10), Value::I64(0)]);
}

#[test]
fn paginate_page_zero_treated_as_page_one_pg() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .paginate(0, 10)
        .to_sql();

    assert_eq!(sql, r#"SELECT "id" FROM "users" LIMIT $1 OFFSET $2"#);
    assert_eq!(binds, vec![Value::I64(10), Value::I64(0)]);
}

#[test]
fn paginate_negative_page_treated_as_page_one_pg() {
    let (_, binds) = QueryBuilder::<Postgres>::table("users")
        .paginate(-5, 10)
        .to_sql();

    assert_eq!(binds, vec![Value::I64(10), Value::I64(0)]);
}

#[test]
fn paginate_mysql_placeholder() {
    let (sql, binds) = QueryBuilder::<MySql>::table("users")
        .select(["id"])
        .paginate(2, 10)
        .to_sql();

    assert_eq!(sql, "SELECT `id` FROM `users` LIMIT ? OFFSET ?");
    assert_eq!(binds, vec![Value::I64(10), Value::I64(10)]);
}

#[test]
fn paginate_sqlite_placeholder() {
    let (sql, binds) = QueryBuilder::<Sqlite>::table("users")
        .select(["id"])
        .paginate(3, 5)
        .to_sql();

    assert_eq!(sql, r#"SELECT "id" FROM "users" LIMIT ? OFFSET ?"#);
    assert_eq!(binds, vec![Value::I64(5), Value::I64(10)]);
}
