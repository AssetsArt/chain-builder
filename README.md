# Chain builder
A query builder for MySQL for Rust is designed to be flexible and easy to use.

## Installation
```bash
cargo add chain-builder
```

## Usage
```rust
use chain_builder::{ChainBuilder, Select, WhereClauses, Client};
let mut builder = ChainBuilder::new(Client::Mysql);
builder.db("mydb"); // For dynamic db
builder.select(Select::Columns(vec!["*".into()]));
builder.from("users");
builder.where_eq("name", serde_json::Value::String("John".to_string()));
builder.where_eq("city", serde_json::Value::String("New York".to_string()));
builder.where_in(
    "department",
    vec![
        serde_json::Value::String("IT".to_string()),
        serde_json::Value::String("HR".to_string()),
    ],
);
builder.where_subquery(|sub| {
    sub.where_eq("status", serde_json::Value::String("active".to_string()));
    sub.or()
        .where_eq("status", serde_json::Value::String("pending".to_string()))
        .where_between("registered_at", vec![
            serde_json::Value::String("2024-01-01".to_string()),
            serde_json::Value::String("2024-01-31".to_string()),
        ]);
    sub
});
builder.where_raw((
    "(latitude BETWEEN ? AND ?) AND (longitude BETWEEN ? AND ?)".into(),
    Some(vec![
      serde_json::Value::Number(serde_json::Number::from_f64(40.0).unwrap()),
      serde_json::Value::Number(serde_json::Number::from_f64(41.0).unwrap()),
      serde_json::Value::Number(serde_json::Number::from_f64(70.0).unwrap()),
      serde_json::Value::Number(serde_json::Number::from_f64(71.0).unwrap()),
    ]),
));
let sql = builder.to_sql();
println!("final sql: {}", sql.0);
// SELECT * FROM mydb.users WHERE name = ? AND city = ? AND department IN (?,?) AND (status = ? OR status = ? AND registered_at BETWEEN ? AND ?) AND (latitude BETWEEN ? AND ?) AND (longitude BETWEEN ? AND ?)
println!("final binds: {:?}", sql.1);
// Some([String("John"), String("New York"), String("IT"), String("HR"), String("active"), String("pending"), String("2024-01-01"), String("2024-01-31"), Number(40.0), Number(41.0), Number(70.0), Number(71.0)])
```