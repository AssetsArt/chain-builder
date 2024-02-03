# Chain builder

[![Documentation](https://img.shields.io/badge/docs.rs-chain--builder-66c2a5?style=for-the-badge&labelColor=555555&logoColor=white)](https://docs.rs/chain-builder)
[![Version](https://img.shields.io/crates/v/chain-builder?style=for-the-badge)](https://crates.io/crates/chain-builder)
[![License](https://img.shields.io/crates/l/chain-builder?style=for-the-badge)](https://crates.io/crates/chain-builder)

A query builder for MySQL for Rust is designed to be flexible and easy to use.

## Installation
```bash
cargo add chain-builder
```

## Usage
```rust
use chain_builder::{ChainBuilder, Select, WhereClauses, Client};
use serde_json::{self, Value};

let mut builder = ChainBuilder::new(Client::Mysql);
builder.db("mydb"); // For dynamic db
builder.select(Select::Columns(vec!["*".into()]));
builder.table("users");
builder.query(|qb| {
    qb.where_eq("name", Value::String("John".to_string()));
    qb.where_eq("city", Value::String("New York".to_string()));
    qb.where_in(
        "department",
        vec![
            Value::String("IT".to_string()),
            Value::String("HR".to_string()),
        ],
    );
    qb.where_subquery(|sub| {
        sub.where_eq("status", Value::String("active".to_string()));
        sub.or()
            .where_eq("status", Value::String("pending".to_string()))
            .where_between(
                "registered_at",
                [
                    Value::String("2024-01-01".to_string()),
                    Value::String("2024-01-31".to_string()),
                ],
            );
    });
    qb.where_raw(
        "(latitude BETWEEN ? AND ?) AND (longitude BETWEEN ? AND ?)".into(),
        Some(vec![
            Value::Number(serde_json::Number::from_f64(40.0).unwrap()),
            Value::Number(serde_json::Number::from_f64(41.0).unwrap()),
            Value::Number(serde_json::Number::from_f64(70.0).unwrap()),
            Value::Number(serde_json::Number::from_f64(71.0).unwrap()),
        ]),
    );
});
let sql = builder.to_sql();
println!("final sql: {}", sql.0);
// SELECT * FROM mydb.users WHERE name = ? AND city = ? AND department IN (?,?) AND (status = ? OR (status = ? AND registered_at BETWEEN ? AND ?)) AND (latitude BETWEEN ? AND ?) AND (longitude BETWEEN ? AND ?)
println!("final binds: {:?}", sql.1);
// Some([String("John"), String("New York"), String("IT"), String("HR"), String("active"), String("pending"), String("2024-01-01"), String("2024-01-31"), Number(40.0), Number(41.0), Number(70.0), Number(71.0)])
```