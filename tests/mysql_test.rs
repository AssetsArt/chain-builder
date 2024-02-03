use chain_builder::{ChainBuilder, Client, JoinMethods, Select, WhereClauses};
use serde_json::{self, Value};
use sqlx::Execute;

#[test]
fn test_chain_builder() {
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

    builder.add_raw("LIMIT ?".into(), Some(vec![10.into()]));

    let sql = builder.to_sql();
    // println!("final sql: {}", sql.0);
    // println!("final binds: {:?}", sql.1);
    assert_eq!(
            sql.0,
            "SELECT * FROM mydb.users WHERE name = ? AND city = ? AND department IN (?,?) AND (status = ? OR (status = ? AND registered_at BETWEEN ? AND ?)) AND (latitude BETWEEN ? AND ?) AND (longitude BETWEEN ? AND ?) LIMIT ?"
        );
    assert_eq!(
        sql.1,
        vec![
            Value::String("John".to_string()),
            Value::String("New York".to_string()),
            Value::String("IT".to_string()),
            Value::String("HR".to_string()),
            Value::String("active".to_string()),
            Value::String("pending".to_string()),
            Value::String("2024-01-01".to_string()),
            Value::String("2024-01-31".to_string()),
            Value::Number(serde_json::Number::from_f64(40.0).unwrap()),
            Value::Number(serde_json::Number::from_f64(41.0).unwrap()),
            Value::Number(serde_json::Number::from_f64(70.0).unwrap()),
            Value::Number(serde_json::Number::from_f64(71.0).unwrap()),
            Value::Number(10.into()),
        ]
    );
}

#[test]
fn test_join() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder.db("mydb"); // For dynamic db
    builder.select(Select::Columns(vec!["*".into()]));
    builder.table("users");
    builder.query(|qb| {
        qb.join("details", |join| {
            join.on("details.id", "=", "users.d_id");
            join.on("details.id_w", "=", "users.d_id_w");
            join.or().on("details.id_s", "=", "users.d_id_s").on(
                "details.id_w",
                "=",
                "users.d_id_w",
            );
        });
        qb.where_eq("name", Value::String("John".to_string()));
    });
    builder.select(Select::Raw(
        "(SELECT COUNT(*) FROM `mydb`.`users` WHERE users.id = ?) AS count".into(),
        Some(vec![Value::Number(1.into())]),
    ));
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    // println!("final sql: {:?}", sql.0);
    // println!("final binds: {:?}", sql.1);
    let true_sql = "SELECT *, (SELECT COUNT(*) FROM `mydb`.`users` WHERE users.id = ?) AS count FROM mydb.users JOIN mydb.details ON details.id = users.d_id AND details.id_w = users.d_id_w OR (details.id_s = users.d_id_s AND details.id_w = users.d_id_w) WHERE name = ?";
    assert_eq!(sql.0, true_sql);
    assert_eq!(
        sql.1,
        vec![Value::Number(1.into()), Value::String("John".to_string()),]
    );
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_insert() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder.db("mydb"); // For dynamic db
    builder.table("users");
    builder.insert(serde_json::json!({
        "name": "John",
        "`city`": "New York",
        "department": "IT",
    }));
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    // println!("final sql: {}", sql.0);
    // println!("final binds: {:?}", sql.1);
    let true_sql = "INSERT INTO mydb.users (`city`, department, name) VALUES (?, ?, ?)";
    assert_eq!(sql.0, true_sql);
    assert_eq!(
        sql.1,
        vec![
            Value::String("New York".to_string()),
            Value::String("IT".to_string()),
            Value::String("John".to_string())
        ]
    );
    assert_eq!(to_sqlx.sql(), true_sql);
}
