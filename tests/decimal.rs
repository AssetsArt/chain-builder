//! `Value::Decimal` binding (requires the `decimal` feature).
#![cfg(feature = "decimal")]

use std::str::FromStr;

use chain_builder::{IntoBind, Postgres, QueryBuilder, Value};
use rust_decimal::Decimal;

#[test]
fn decimal_into_bind() {
    let d = Decimal::from_str("19.99").unwrap();
    assert_eq!(d.into_bind(), Value::Decimal(d));
}

#[test]
fn decimal_binds_as_placeholder() {
    let price = Decimal::from_str("19.99").unwrap();
    let (sql, binds) = QueryBuilder::<Postgres>::table("products")
        .select(["id"])
        .where_eq("price", price)
        .to_sql();
    assert_eq!(sql, r#"SELECT "id" FROM "products" WHERE "price" = $1"#);
    assert_eq!(binds, vec![Value::Decimal(price)]);
}

#[test]
fn decimal_option_none_is_null() {
    assert_eq!(Option::<Decimal>::None.into_bind(), Value::Null);
}
