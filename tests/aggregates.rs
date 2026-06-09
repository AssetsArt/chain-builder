//! Aggregate SELECT helpers (`select_count`/`sum`/`avg`/`min`/`max` + `_as`)
//! and plain column aliasing (`select_as`).

use chain_builder::{MySql, Postgres, QueryBuilder};

#[test]
fn count_star_pg() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("users")
        .select_count("*")
        .to_sql();
    assert_eq!(sql, r#"SELECT COUNT(*) FROM "users""#);
    assert!(binds.is_empty());
}

#[test]
fn count_column_is_escaped() {
    let (sql, _) = QueryBuilder::<Postgres>::table("users")
        .select_count("id")
        .to_sql();
    assert_eq!(sql, r#"SELECT COUNT("id") FROM "users""#);
}

#[test]
fn count_as_alias() {
    let (sql, _) = QueryBuilder::<Postgres>::table("users")
        .select_count_as("*", "total")
        .to_sql();
    assert_eq!(sql, r#"SELECT COUNT(*) AS "total" FROM "users""#);
}

#[test]
fn all_aggregates_pg() {
    let (sql, _) = QueryBuilder::<Postgres>::table("orders")
        .select_sum("amount")
        .select_avg("amount")
        .select_min("amount")
        .select_max("amount")
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT SUM("amount"), AVG("amount"), MIN("amount"), MAX("amount") FROM "orders""#
    );
}

#[test]
fn aggregate_with_group_by_and_alias() {
    let (sql, _) = QueryBuilder::<Postgres>::table("orders")
        .select(["status"])
        .select_count_as("*", "cnt")
        .select_sum_as("amount", "total")
        .group_by(["status"])
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "status", COUNT(*) AS "cnt", SUM("amount") AS "total" FROM "orders" GROUP BY "status""#
    );
}

#[test]
fn select_as_plain_alias() {
    let (sql, _) = QueryBuilder::<Postgres>::table("users")
        .select_as("created_at", "joined")
        .to_sql();
    assert_eq!(sql, r#"SELECT "created_at" AS "joined" FROM "users""#);
}

#[test]
fn aggregates_use_backticks_on_mysql() {
    let (sql, _) = QueryBuilder::<MySql>::table("orders")
        .select_count_as("id", "cnt")
        .select_max("price")
        .to_sql();
    assert_eq!(
        sql,
        "SELECT COUNT(`id`) AS `cnt`, MAX(`price`) FROM `orders`"
    );
}

#[test]
fn qualified_column_in_aggregate() {
    let (sql, _) = QueryBuilder::<Postgres>::table("orders")
        .select_sum("orders.amount")
        .to_sql();
    assert_eq!(sql, r#"SELECT SUM("orders"."amount") FROM "orders""#);
}
