//! 3.1.0: UPDATE expressions — `set_raw`, `increment`, `decrement`.

use chain_builder::{MySql, Postgres, QueryBuilder, Sqlite, Value};

#[test]
fn pg_update_with_increment_and_raw() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .update([("name", "x")])
        .increment("views", 1i64)
        .set_raw("updated_at", "NOW()", vec![])
        .where_eq("id", 9i64)
        .to_sql();
    assert_eq!(
        sql,
        r#"UPDATE "t" SET "name" = $1, "views" = "views" + $2, "updated_at" = NOW() WHERE "id" = $3"#
    );
    assert_eq!(
        binds,
        vec![Value::Text("x".into()), Value::I64(1), Value::I64(9)]
    );
}

#[test]
fn pg_increment_alone_is_a_valid_update() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .increment("views", 1i64)
        .where_eq("id", 9i64)
        .to_sql();
    assert_eq!(
        sql,
        r#"UPDATE "t" SET "views" = "views" + $1 WHERE "id" = $2"#
    );
    assert_eq!(binds, vec![Value::I64(1), Value::I64(9)]);
}

#[test]
fn pg_decrement_emits_minus() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .decrement("stock", 3i64)
        .to_sql();
    assert_eq!(sql, r#"UPDATE "t" SET "stock" = "stock" - $1"#);
    assert_eq!(binds, vec![Value::I64(3)]);
}

#[test]
fn pg_set_raw_own_binds_positions() {
    // One structured SET bind ($1) precedes the raw expr, so its hand-written
    // placeholder is $2; WHERE continues at $3.
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .update([("a", 1i64)])
        .set_raw("total", "price * $2", vec![Value::I64(3)])
        .where_eq("id", 9i64)
        .to_sql();
    assert_eq!(
        sql,
        r#"UPDATE "t" SET "a" = $1, "total" = price * $2 WHERE "id" = $3"#
    );
    assert_eq!(binds, vec![Value::I64(1), Value::I64(3), Value::I64(9)]);
}

#[test]
fn sqlite_update_exprs_use_question_marks() {
    let (sql, binds) = QueryBuilder::<Sqlite>::table("t")
        .update([("name", "x")])
        .increment("views", 1i64)
        .where_eq("id", 9i64)
        .to_sql();
    assert_eq!(
        sql,
        r#"UPDATE "t" SET "name" = ?, "views" = "views" + ? WHERE "id" = ?"#
    );
    assert_eq!(
        binds,
        vec![Value::Text("x".into()), Value::I64(1), Value::I64(9)]
    );
}

#[test]
fn exprs_only_update_does_not_err_empty() {
    let res = QueryBuilder::<Postgres>::table("t")
        .increment("v", 1i64)
        .try_to_sql();
    assert!(res.is_ok());
}

#[test]
fn empty_update_still_errs() {
    // Both `set` and `set_exprs` empty → EmptyUpdate, same Display text.
    let err = QueryBuilder::<Postgres>::table("t")
        .update(std::iter::empty::<(&str, Value)>())
        .try_to_sql()
        .unwrap_err();
    assert_eq!(err.to_string(), "update() requires at least one column");
}

#[test]
fn method_switch_away_from_update_ignores_exprs() {
    // Last-call-wins: insert() after set_raw() — exprs are ignored, like
    // leftover `set` state is today.
    let (sql, binds) = QueryBuilder::<Sqlite>::table("t")
        .set_raw("x", "1", vec![])
        .insert([("a", 1i64)])
        .to_sql();
    assert_eq!(sql, r#"INSERT INTO "t" ("a") VALUES (?)"#);
    assert_eq!(binds, vec![Value::I64(1)]);
}

#[test]
fn pg_step_bind_advances_counter_before_raw() {
    // The increment's bind goes through the placeholder counter ($1), so a
    // following set_raw with its own bind must hand-write $2; WHERE is $3.
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .increment("views", 1i64)
        .set_raw("total", "price * $2", vec![Value::I64(4)])
        .where_eq("id", 9i64)
        .to_sql();
    assert_eq!(
        sql,
        r#"UPDATE "t" SET "views" = "views" + $1, "total" = price * $2 WHERE "id" = $3"#
    );
    assert_eq!(binds, vec![Value::I64(1), Value::I64(4), Value::I64(9)]);
}

#[test]
fn mysql_increment_backtick_quotes_both_sides() {
    let (sql, binds) = QueryBuilder::<MySql>::table("t")
        .increment("views", 1i64)
        .where_eq("id", 9i64)
        .to_sql();
    assert_eq!(sql, "UPDATE `t` SET `views` = `views` + ? WHERE `id` = ?");
    assert_eq!(binds, vec![Value::I64(1), Value::I64(9)]);
}
