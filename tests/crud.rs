use chain_builder::{IntoBind, Postgres, QueryBuilder, Sqlite, Value};

#[test]
fn sqlite_insert_sorts_keys() {
    let (sql, binds) = QueryBuilder::<Sqlite>::table("users")
        .insert([("name", "John"), ("age", "30")])
        .to_sql();
    // sorted keys: age, name
    assert_eq!(sql, r#"INSERT INTO "users" ("age", "name") VALUES (?, ?)"#);
    assert_eq!(
        binds,
        vec![Value::Text("30".into()), Value::Text("John".into())]
    );
}

#[test]
fn sqlite_insert_mixed_value_types() {
    let (sql, binds) = QueryBuilder::<Sqlite>::table("users")
        .insert([
            ("name", Value::Text("John".into())),
            ("age", Value::I64(30)),
        ])
        .to_sql();
    assert_eq!(sql, r#"INSERT INTO "users" ("age", "name") VALUES (?, ?)"#);
    assert_eq!(binds, vec![Value::I64(30), Value::Text("John".into())]);
}

#[test]
fn postgres_update_with_where() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .update([("age", 31i64)])
        .where_eq("id", 1i64)
        .to_sql();
    assert_eq!(sql, r#"UPDATE "users" SET "age" = $1 WHERE "id" = $2"#);
    assert_eq!(binds, vec![Value::I64(31), Value::I64(1)]);
}

#[test]
fn postgres_update_sorts_keys() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .update([("name", Value::Text("a".into())), ("age", Value::I64(2))])
        .where_eq("id", 1i64)
        .to_sql();
    assert_eq!(
        sql,
        r#"UPDATE "users" SET "age" = $1, "name" = $2 WHERE "id" = $3"#
    );
    assert_eq!(
        binds,
        vec![Value::I64(2), Value::Text("a".into()), Value::I64(1)]
    );
}

#[test]
fn sqlite_delete_with_where() {
    let (sql, binds) = QueryBuilder::<Sqlite>::table("users")
        .delete()
        .where_eq("id", 1i64)
        .to_sql();
    assert_eq!(sql, r#"DELETE FROM "users" WHERE "id" = ?"#);
    assert_eq!(binds, vec![Value::I64(1)]);
}

#[test]
fn delete_without_where() {
    let (sql, binds) = QueryBuilder::<Sqlite>::table("users").delete().to_sql();
    assert_eq!(sql, r#"DELETE FROM "users""#);
    assert!(binds.is_empty());
}

#[test]
fn insert_primitive_drives_into_bind_i64() {
    // TG7: a plain i64 primitive flows through IntoBind to Value::I64.
    let (sql, binds) = QueryBuilder::<Sqlite>::table("u")
        .insert([("age", 30i64)])
        .to_sql();
    assert_eq!(sql, r#"INSERT INTO "u" ("age") VALUES (?)"#);
    assert_eq!(binds, vec![Value::I64(30)]);
}

#[test]
fn insert_bool_and_option_drive_into_bind() {
    // TG7: bool -> Value::Bool, Option::None -> Value::Null. Keys sort:
    // active, nickname.
    let (sql, binds) = QueryBuilder::<Sqlite>::table("u")
        .insert([
            ("active", true.into_bind()),
            ("nickname", Option::<&str>::None.into_bind()),
        ])
        .to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "u" ("active", "nickname") VALUES (?, ?)"#
    );
    assert_eq!(binds, vec![Value::Bool(true), Value::Null]);
}

#[test]
#[should_panic(expected = "insert() requires at least one column")]
fn empty_insert_panics() {
    // F5: an INSERT with no columns is invalid SQL — panic with a clear message.
    let _ = QueryBuilder::<Sqlite>::table("u")
        .insert(Vec::<(&str, i64)>::new())
        .to_sql();
}

#[test]
#[should_panic(expected = "update() requires at least one column")]
fn empty_update_panics() {
    // F5: an UPDATE with no SET columns is invalid SQL — panic clearly.
    let _ = QueryBuilder::<Sqlite>::table("u")
        .update(Vec::<(&str, i64)>::new())
        .to_sql();
}
