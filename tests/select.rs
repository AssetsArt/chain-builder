use chain_builder::{MySql, Postgres, QueryBuilder, Sqlite, Value};

#[test]
fn postgres_select_with_wheres() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id", "name"])
        .where_eq("status", "active")
        .where_in("role", ["admin", "staff"])
        .where_gt("age", 18i64)
        .to_sql();

    assert_eq!(
        sql,
        r#"SELECT "id", "name" FROM "users" WHERE "status" = $1 AND "role" IN ($2, $3) AND "age" > $4"#
    );
    assert_eq!(
        binds,
        vec![
            Value::Text("active".into()),
            Value::Text("admin".into()),
            Value::Text("staff".into()),
            Value::I64(18),
        ]
    );
}

#[test]
fn mysql_uses_backticks_and_question_marks() {
    let (sql, binds) = QueryBuilder::<MySql>::table("users")
        .select(["id"])
        .where_eq("status", "active")
        .to_sql();

    assert_eq!(sql, "SELECT `id` FROM `users` WHERE `status` = ?");
    assert_eq!(binds, vec![Value::Text("active".into())]);
}

#[test]
fn sqlite_uses_double_quotes_and_question_marks() {
    let (sql, binds) = QueryBuilder::<Sqlite>::table("users")
        .select(["id"])
        .where_eq("status", "active")
        .to_sql();

    assert_eq!(sql, r#"SELECT "id" FROM "users" WHERE "status" = ?"#);
    assert_eq!(binds, vec![Value::Text("active".into())]);
}

#[test]
fn empty_in_yields_false_predicate() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .where_in("x", Vec::<i64>::new())
        .to_sql();
    assert_eq!(sql, r#"SELECT * FROM "users" WHERE 1 = 0"#);
    assert!(binds.is_empty());
}

#[test]
fn empty_not_in_yields_true_predicate() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .where_not_in("x", Vec::<i64>::new())
        .to_sql();
    assert_eq!(sql, r#"SELECT * FROM "users" WHERE 1 = 1"#);
    assert!(binds.is_empty());
}

#[test]
fn or_group_crosses_placeholder_ordering() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .where_eq("active", true)
        .or_where(|w| w.where_eq("role", "admin").where_gt("age", 40i64))
        .to_sql();

    assert_eq!(
        sql,
        r#"SELECT * FROM "users" WHERE "active" = $1 OR ("role" = $2 AND "age" > $3)"#
    );
    assert_eq!(
        binds,
        vec![
            Value::Bool(true),
            Value::Text("admin".into()),
            Value::I64(40),
        ]
    );
}

#[test]
fn group_as_first_predicate_suppresses_leading_or() {
    // TG1: a group is the FIRST predicate; the outer OR has nothing to attach
    // to, so no leading `OR` is emitted.
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .or_where(|w| w.where_eq("x", 1i64))
        .to_sql();
    assert_eq!(sql, r#"SELECT * FROM "t" WHERE ("x" = $1)"#);
    assert_eq!(binds, vec![Value::I64(1)]);
}

#[test]
fn empty_group_is_omitted() {
    // F4: an empty group must not emit invalid `()` and must not leave a
    // dangling separator; here it is dropped entirely.
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .where_eq("a", 1i64)
        .and_where(|w| w)
        .to_sql();
    assert_eq!(sql, r#"SELECT * FROM "t" WHERE "a" = $1"#);
    assert_eq!(binds, vec![Value::I64(1)]);
}

#[test]
fn only_empty_group_yields_no_where() {
    // F4: when the only predicate is an empty group, no WHERE is emitted.
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .and_where(|w| w)
        .to_sql();
    assert_eq!(sql, r#"SELECT * FROM "t""#);
    assert!(binds.is_empty());
}

#[test]
fn between_isolated() {
    // TG2: isolated BETWEEN — exact SQL and a 2-element bind vec.
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .where_between("age", 18i64, 65i64)
        .to_sql();
    assert_eq!(sql, r#"SELECT * FROM "t" WHERE "age" BETWEEN $1 AND $2"#);
    assert_eq!(binds, vec![Value::I64(18), Value::I64(65)]);
}

#[test]
fn null_predicates_have_no_binds() {
    // TG3: IS NULL / IS NOT NULL bind nothing.
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .where_null("a")
        .where_not_null("b")
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT * FROM "t" WHERE "a" IS NULL AND "b" IS NOT NULL"#
    );
    assert!(binds.is_empty());
}

#[test]
fn postgres_delete_with_where() {
    // TG6: Postgres DELETE with a WHERE.
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .delete()
        .where_eq("id", 1i64)
        .to_sql();
    assert_eq!(sql, r#"DELETE FROM "t" WHERE "id" = $1"#);
    assert_eq!(binds, vec![Value::I64(1)]);
}

#[test]
fn and_group_works() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .where_eq("active", true)
        .and_where(|w| w.where_eq("role", "admin").where_gt("age", 40i64))
        .to_sql();

    assert_eq!(
        sql,
        r#"SELECT * FROM "users" WHERE "active" = $1 AND ("role" = $2 AND "age" > $3)"#
    );
    assert_eq!(
        binds,
        vec![
            Value::Bool(true),
            Value::Text("admin".into()),
            Value::I64(40),
        ]
    );
}

#[test]
fn dotted_identifier_is_quoted_per_segment() {
    let (sql, _) = QueryBuilder::<Postgres>::table("users")
        .select(["users.id"])
        .to_sql();
    assert_eq!(sql, r#"SELECT "users"."id" FROM "users""#);
}

#[test]
fn star_select_passes_through() {
    let (sql, _) = QueryBuilder::<Postgres>::table("users")
        .select(["*"])
        .to_sql();
    assert_eq!(sql, r#"SELECT * FROM "users""#);
}

#[test]
fn injection_column_is_neutralized() {
    let (sql, _) = QueryBuilder::<Postgres>::table("users")
        .select([r#"id" ; DROP TABLE users; --"#])
        .to_sql();
    assert_eq!(sql, r#"SELECT "id"" ; DROP TABLE users; --" FROM "users""#);
}

#[test]
fn additional_where_predicates() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .where_ne("a", 1i64)
        .where_gte("b", 2i64)
        .where_lt("c", 3i64)
        .where_lte("d", 4i64)
        .where_like("e", "%x%")
        .where_null("f")
        .where_not_null("g")
        .where_between("h", 5i64, 6i64)
        // where_raw is the verbatim escape hatch: the caller must hand-write the
        // `$N` matching the actual bind position. Seven binds precede this raw
        // predicate (a, b, c, d, e, and h's two BETWEEN bounds), so its single
        // bind is the 8th → `$8`. No renumbering is performed.
        .where_raw("j @> $8", vec![Value::Text("raw".into())])
        .to_sql();

    assert_eq!(
        sql,
        r#"SELECT * FROM "t" WHERE "a" != $1 AND "b" >= $2 AND "c" < $3 AND "d" <= $4 AND "e" LIKE $5 AND "f" IS NULL AND "g" IS NOT NULL AND "h" BETWEEN $6 AND $7 AND j @> $8"#
    );
    assert_eq!(
        binds,
        vec![
            Value::I64(1),
            Value::I64(2),
            Value::I64(3),
            Value::I64(4),
            Value::Text("%x%".into()),
            Value::I64(5),
            Value::I64(6),
            Value::Text("raw".into()),
        ]
    );
}
