#![cfg(feature = "v2")]

use chain_builder::v2::{MySql, Postgres, QueryBuilder, Sqlite, Value};

#[test]
fn postgres_cte_main_union_placeholder_ordering() {
    // THE CRUX: $1 (cte body) -> $2 (main where) -> $3 (union arm).
    let cte = QueryBuilder::<Postgres>::table("logs")
        .select(["n"])
        .where_gt("n", 1i64);
    let arm = QueryBuilder::<Postgres>::table("recent")
        .select(["n"])
        .where_lt("n", 99i64);

    let (sql, binds) = QueryBuilder::<Postgres>::table("recent")
        .with("recent", cte)
        .select(["*"])
        .where_gt("n", 5i64)
        .union(arm)
        .to_sql();

    assert_eq!(
        sql,
        r#"WITH "recent" AS (SELECT "n" FROM "logs" WHERE "n" > $1) SELECT * FROM "recent" WHERE "n" > $2 UNION SELECT "n" FROM "recent" WHERE "n" < $3"#
    );
    assert_eq!(binds, vec![Value::I64(1), Value::I64(5), Value::I64(99)]);
}

#[test]
fn postgres_with_recursive() {
    let cte = QueryBuilder::<Postgres>::table("t").select(["n"]);
    let (sql, _binds) = QueryBuilder::<Postgres>::table("t")
        .with_recursive("t", cte)
        .select(["*"])
        .to_sql();

    assert_eq!(
        sql,
        r#"WITH RECURSIVE "t" AS (SELECT "n" FROM "t") SELECT * FROM "t""#
    );
}

#[test]
fn postgres_union_all() {
    let arm = QueryBuilder::<Postgres>::table("b").select(["id"]);
    let (sql, _binds) = QueryBuilder::<Postgres>::table("a")
        .select(["id"])
        .union_all(arm)
        .to_sql();

    assert_eq!(
        sql,
        r#"SELECT "id" FROM "a" UNION ALL SELECT "id" FROM "b""#
    );
}

#[test]
fn postgres_two_binds_in_cte_and_main() {
    let cte = QueryBuilder::<Postgres>::table("logs")
        .select(["n"])
        .where_gt("n", 1i64)
        .where_lt("n", 10i64);

    let (sql, binds) = QueryBuilder::<Postgres>::table("recent")
        .with("recent", cte)
        .select(["*"])
        .where_gte("n", 5i64)
        .where_lte("n", 8i64)
        .to_sql();

    assert_eq!(
        sql,
        r#"WITH "recent" AS (SELECT "n" FROM "logs" WHERE "n" > $1 AND "n" < $2) SELECT * FROM "recent" WHERE "n" >= $3 AND "n" <= $4"#
    );
    assert_eq!(
        binds,
        vec![Value::I64(1), Value::I64(10), Value::I64(5), Value::I64(8)]
    );
}

#[test]
fn mysql_cte_union() {
    let cte = QueryBuilder::<MySql>::table("logs")
        .select(["n"])
        .where_gt("n", 1i64);
    let arm = QueryBuilder::<MySql>::table("recent")
        .select(["n"])
        .where_lt("n", 99i64);

    let (sql, binds) = QueryBuilder::<MySql>::table("recent")
        .with("recent", cte)
        .select(["*"])
        .where_gt("n", 5i64)
        .union(arm)
        .to_sql();

    assert_eq!(
        sql,
        "WITH `recent` AS (SELECT `n` FROM `logs` WHERE `n` > ?) SELECT * FROM `recent` WHERE `n` > ? UNION SELECT `n` FROM `recent` WHERE `n` < ?"
    );
    assert_eq!(binds, vec![Value::I64(1), Value::I64(5), Value::I64(99)]);
}

#[test]
fn sqlite_cte_union() {
    let cte = QueryBuilder::<Sqlite>::table("logs")
        .select(["n"])
        .where_gt("n", 1i64);
    let arm = QueryBuilder::<Sqlite>::table("recent")
        .select(["n"])
        .where_lt("n", 99i64);

    let (sql, binds) = QueryBuilder::<Sqlite>::table("recent")
        .with("recent", cte)
        .select(["*"])
        .where_gt("n", 5i64)
        .union(arm)
        .to_sql();

    assert_eq!(
        sql,
        r#"WITH "recent" AS (SELECT "n" FROM "logs" WHERE "n" > ?) SELECT * FROM "recent" WHERE "n" > ? UNION SELECT "n" FROM "recent" WHERE "n" < ?"#
    );
    assert_eq!(binds, vec![Value::I64(1), Value::I64(5), Value::I64(99)]);
}
