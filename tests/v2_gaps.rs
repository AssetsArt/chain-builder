#![cfg(feature = "v2")]

use chain_builder::v2::{MySql, Postgres, QueryBuilder, Sqlite, Value};

#[test]
fn pg_insert_many_basic() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("u")
        .insert_many([
            [("a", 1i64), ("b", 2i64)],
            [("a", 3i64), ("b", 4i64)],
        ])
        .to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "u" ("a", "b") VALUES ($1, $2), ($3, $4)"#
    );
    assert_eq!(
        binds,
        vec![Value::I64(1), Value::I64(2), Value::I64(3), Value::I64(4)]
    );
}

#[test]
fn pg_insert_many_ragged_binds_null() {
    // Second row missing "b" → that slot binds Null.
    let (sql, binds) = QueryBuilder::<Postgres>::table("u")
        .insert_many([
            vec![("a", 1i64), ("b", 2i64)],
            vec![("a", 3i64)],
        ])
        .to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "u" ("a", "b") VALUES ($1, $2), ($3, $4)"#
    );
    assert_eq!(
        binds,
        vec![Value::I64(1), Value::I64(2), Value::I64(3), Value::Null]
    );
}

#[test]
fn mysql_insert_many() {
    let (sql, binds) = QueryBuilder::<MySql>::table("u")
        .insert_many([
            [("a", 1i64), ("b", 2i64)],
            [("a", 3i64), ("b", 4i64)],
        ])
        .to_sql();
    assert_eq!(sql, "INSERT INTO `u` (`a`, `b`) VALUES (?, ?), (?, ?)");
    assert_eq!(
        binds,
        vec![Value::I64(1), Value::I64(2), Value::I64(3), Value::I64(4)]
    );
}

#[test]
fn sqlite_insert_many() {
    let (sql, binds) = QueryBuilder::<Sqlite>::table("u")
        .insert_many([
            [("a", 1i64), ("b", 2i64)],
            [("a", 3i64), ("b", 4i64)],
        ])
        .to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "u" ("a", "b") VALUES (?, ?), (?, ?)"#
    );
    assert_eq!(
        binds,
        vec![Value::I64(1), Value::I64(2), Value::I64(3), Value::I64(4)]
    );
}

#[test]
fn pg_insert_many_with_on_conflict_and_returning() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("u")
        .insert_many([
            [("a", 1i64), ("b", 2i64)],
            [("a", 3i64), ("b", 4i64)],
        ])
        .on_conflict_do_nothing(["a"])
        .returning(["a"])
        .to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "u" ("a", "b") VALUES ($1, $2), ($3, $4) ON CONFLICT ("a") DO NOTHING RETURNING "a""#
    );
    assert_eq!(
        binds,
        vec![Value::I64(1), Value::I64(2), Value::I64(3), Value::I64(4)]
    );
}

#[test]
fn pg_group_by_raw_verbatim() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .select(["a"])
        .group_by_raw("date_trunc('day', created_at)", vec![])
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "a" FROM "t" GROUP BY date_trunc('day', created_at)"#
    );
    assert!(binds.is_empty());
}

#[test]
fn pg_order_by_raw_verbatim_with_bind() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .select(["a"])
        .order_by_raw("CASE WHEN a = $1 THEN 0 ELSE 1 END", vec![Value::I64(5)])
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "a" FROM "t" ORDER BY CASE WHEN a = $1 THEN 0 ELSE 1 END"#
    );
    assert_eq!(binds, vec![Value::I64(5)]);
}

#[test]
fn sqlite_group_order_by_raw() {
    let (sql, binds) = QueryBuilder::<Sqlite>::table("t")
        .select(["a"])
        .group_by_raw("a, b", vec![])
        .order_by_raw("a DESC", vec![])
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "a" FROM "t" GROUP BY a, b ORDER BY a DESC"#
    );
    assert!(binds.is_empty());
}

#[test]
fn pg_group_order_by_raw_appends_to_structured() {
    let (sql, _) = QueryBuilder::<Postgres>::table("t")
        .select(["a"])
        .group_by(["a"])
        .group_by_raw("LOWER(b)", vec![])
        .order_by_asc("a")
        .order_by_raw("LOWER(b)", vec![])
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "a" FROM "t" GROUP BY "a", LOWER(b) ORDER BY "a" ASC, LOWER(b)"#
    );
}

// --- Regression: single-row insert and structured group/order unchanged ---

#[test]
fn pg_single_row_insert_unchanged() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("u")
        .insert([("a", 1i64), ("b", 2i64)])
        .to_sql();
    assert_eq!(sql, r#"INSERT INTO "u" ("a", "b") VALUES ($1, $2)"#);
    assert_eq!(binds, vec![Value::I64(1), Value::I64(2)]);
}

#[test]
fn pg_structured_group_order_unchanged() {
    let (sql, _) = QueryBuilder::<Postgres>::table("t")
        .select(["a"])
        .group_by(["a", "b"])
        .order_by_asc("a")
        .order_by_desc("b")
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "a" FROM "t" GROUP BY "a", "b" ORDER BY "a" ASC, "b" DESC"#
    );
}
