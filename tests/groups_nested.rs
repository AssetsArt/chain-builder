//! M11 — nested groups + OR-inside-group.
//!
//! Verifies that `WhereBuilder` can nest further groups (via `and_where`/
//! `or_where`) and that a `Predicate::Group` renders its inner predicates with
//! each inner pred's own attach-conj (so a nested `or_where` group is joined
//! with ` OR `). Existing flat-group output stays byte-identical.

use chain_builder::{MySql, Postgres, QueryBuilder, Value};

#[test]
fn pg_or_inside_group() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("t")
        .select(["*"])
        .and_where(|g| g.where_eq("a", 1i64).or_where(|h| h.where_eq("b", 2i64)))
        .to_sql();

    assert_eq!(sql, r#"SELECT * FROM "t" WHERE ("a" = $1 OR ("b" = $2))"#);
    assert_eq!(binds, vec![Value::I64(1), Value::I64(2)]);
}

#[test]
fn mysql_or_inside_group() {
    let (sql, binds) = QueryBuilder::<MySql>::table("t")
        .select(["*"])
        .and_where(|g| g.where_eq("a", 1i64).or_where(|h| h.where_eq("b", 2i64)))
        .to_sql();

    assert_eq!(sql, "SELECT * FROM `t` WHERE (`a` = ? OR (`b` = ?))");
    assert_eq!(binds, vec![Value::I64(1), Value::I64(2)]);
}

#[test]
fn pg_nested_groups_property_pattern() {
    let props = [("k1", "v1"), ("k2", "v2")];
    let mut qb = QueryBuilder::<Postgres>::table("items").select(["id"]);
    qb = qb.and_where(|outer| {
        let mut w = outer;
        for (k, v) in props {
            w = w
                .or_where(|g| {
                    g.where_eq("properties_desc", k)
                        .where_in("property_info", [v])
                })
                .or_where(|g| {
                    g.where_eq("properties_desc2", k)
                        .where_in("property_info2", [v])
                });
        }
        w
    });
    let (sql, binds) = qb.to_sql();

    assert_eq!(
        sql,
        r#"SELECT "id" FROM "items" WHERE (("properties_desc" = $1 AND "property_info" IN ($2)) OR ("properties_desc2" = $3 AND "property_info2" IN ($4)) OR ("properties_desc" = $5 AND "property_info" IN ($6)) OR ("properties_desc2" = $7 AND "property_info2" IN ($8)))"#
    );
    assert_eq!(binds.len(), 8);
    assert_eq!(
        binds,
        vec![
            Value::Text("k1".into()),
            Value::Text("v1".into()),
            Value::Text("k1".into()),
            Value::Text("v1".into()),
            Value::Text("k2".into()),
            Value::Text("v2".into()),
            Value::Text("k2".into()),
            Value::Text("v2".into()),
        ]
    );
}

/// Byte-identity regression: a flat `or_where`/`and_where` group (no nesting)
/// must render exactly as it did pre-M11.
#[test]
fn pg_flat_groups_byte_identical() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select(["id"])
        .where_eq("status", "active")
        .or_where(|g| g.where_eq("a", 1i64).where_eq("b", 2i64))
        .and_where(|g| g.where_gt("c", 3i64).where_lt("d", 4i64))
        .to_sql();

    assert_eq!(
        sql,
        r#"SELECT "id" FROM "users" WHERE "status" = $1 OR ("a" = $2 AND "b" = $3) AND ("c" > $4 AND "d" < $5)"#
    );
    assert_eq!(
        binds,
        vec![
            Value::Text("active".into()),
            Value::I64(1),
            Value::I64(2),
            Value::I64(3),
            Value::I64(4),
        ]
    );
}
