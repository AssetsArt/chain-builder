use chain_builder::{MySql, Postgres, QueryBuilder, Sqlite, Value};

// ---------------------------------------------------------------------------
// DISTINCT / DISTINCT ON
// ---------------------------------------------------------------------------

#[test]
fn pg_distinct() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .distinct()
        .select(["a"])
        .to_sql();
    assert_eq!(sql, r#"SELECT DISTINCT "a" FROM "t""#);
    assert!(binds.is_empty());
}

#[test]
fn pg_distinct_on() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .distinct_on(["a", "b"])
        .select(["a"])
        .to_sql();
    assert_eq!(sql, r#"SELECT DISTINCT ON ("a", "b") "a" FROM "t""#);
    assert!(binds.is_empty());
}

#[test]
fn mysql_distinct() {
    let (sql, _binds) = QueryBuilder::<MySql>::table("t")
        .distinct()
        .select(["a"])
        .to_sql();
    assert_eq!(sql, "SELECT DISTINCT `a` FROM `t`");
}

#[test]
#[should_panic(expected = "DISTINCT ON requires PostgreSQL")]
fn mysql_distinct_on_panics() {
    let _ = QueryBuilder::<MySql>::table("t")
        .distinct_on(["a"])
        .select(["a"])
        .to_sql();
}

// ---------------------------------------------------------------------------
// ILIKE (dialect-aware)
// ---------------------------------------------------------------------------

#[test]
fn pg_ilike() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .select(["a"])
        .where_ilike("name", "%jo%")
        .to_sql();
    assert_eq!(sql, r#"SELECT "a" FROM "t" WHERE "name" ILIKE $1"#);
    assert_eq!(binds, vec![Value::Text("%jo%".to_string())]);
}

#[test]
fn mysql_ilike() {
    let (sql, binds) = QueryBuilder::<MySql>::table("t")
        .select(["a"])
        .where_ilike("name", "%jo%")
        .to_sql();
    assert_eq!(sql, "SELECT `a` FROM `t` WHERE LOWER(`name`) LIKE LOWER(?)");
    assert_eq!(binds, vec![Value::Text("%jo%".to_string())]);
}

#[test]
fn sqlite_ilike() {
    let (sql, binds) = QueryBuilder::<Sqlite>::table("t")
        .select(["a"])
        .where_ilike("name", "%jo%")
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "a" FROM "t" WHERE LOWER("name") LIKE LOWER(?)"#
    );
    assert_eq!(binds, vec![Value::Text("%jo%".to_string())]);
}

#[test]
fn pg_ilike_placeholder_ordering() {
    // A preceding WHERE so the ILIKE placeholder is $2.
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .select(["a"])
        .where_eq("status", "active")
        .where_ilike("name", "%jo%")
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "a" FROM "t" WHERE "status" = $1 AND "name" ILIKE $2"#
    );
    assert_eq!(
        binds,
        vec![
            Value::Text("active".to_string()),
            Value::Text("%jo%".to_string()),
        ]
    );
}

#[test]
fn pg_ilike_inside_or_where_group() {
    // ILIKE within an or_where group: placeholder ordering continues.
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .select(["a"])
        .where_eq("status", "active")
        .or_where(|w| w.where_ilike("name", "%jo%").where_eq("age", 30i64))
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "a" FROM "t" WHERE "status" = $1 OR ("name" ILIKE $2 AND "age" = $3)"#
    );
    assert_eq!(
        binds,
        vec![
            Value::Text("active".to_string()),
            Value::Text("%jo%".to_string()),
            Value::I64(30),
        ]
    );
}

// ---------------------------------------------------------------------------
// jsonb contains
// ---------------------------------------------------------------------------

#[test]
fn pg_jsonb_contains() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .select(["a"])
        .where_jsonb_contains("meta", "{\"a\":1}")
        .to_sql();
    assert_eq!(sql, r#"SELECT "a" FROM "t" WHERE "meta" @> $1"#);
    assert_eq!(binds, vec![Value::Text("{\"a\":1}".to_string())]);
}

#[test]
fn pg_jsonb_contains_in_group() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .select(["a"])
        .where_eq("status", "active")
        .and_where(|w| w.where_jsonb_contains("meta", "{\"a\":1}"))
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "a" FROM "t" WHERE "status" = $1 AND ("meta" @> $2)"#
    );
    assert_eq!(
        binds,
        vec![
            Value::Text("active".to_string()),
            Value::Text("{\"a\":1}".to_string()),
        ]
    );
}

// ---------------------------------------------------------------------------
// Regression: neither distinct nor ilike → byte-identical to M1.
// ---------------------------------------------------------------------------

#[test]
fn regression_plain_select_unchanged() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id", "name"])
        .where_eq("status", "active")
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "id", "name" FROM "users" WHERE "status" = $1"#
    );
    assert_eq!(binds, vec![Value::Text("active".to_string())]);
}
