use chain_builder::{ChainBuilder, Client, JoinMethods, QueryCommon, Select, WhereClauses};
use serde_json::{self, Value};
use sqlx::Execute;

#[test]
fn test_chain_builder() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .db("mydb") // For dynamic db
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
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
                "(latitude BETWEEN ? AND ?) AND (longitude BETWEEN ? AND ?)",
                Some(vec![
                    Value::Number(serde_json::Number::from_f64(40.0).unwrap()),
                    Value::Number(serde_json::Number::from_f64(41.0).unwrap()),
                    Value::Number(serde_json::Number::from_f64(70.0).unwrap()),
                    Value::Number(serde_json::Number::from_f64(71.0).unwrap()),
                ]),
            );
        });

    builder.add_raw("LIMIT ?", Some(vec![10.into()]));

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
    builder
        .db("mydb") // For dynamic db
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
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
    builder
        .db("mydb") // For dynamic db
        .table("users")
        .insert(serde_json::json!({
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

#[test]
fn test_insert_many() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .db("mydb") // For dynamic db
        .table("users")
        .insert_many(vec![
            serde_json::json!({
                "name": "John",
                "`city`": "New York",
                "department": "IT",
            }),
            serde_json::json!({
                "name": "Jane",
                "`city`": "New York",
                "department": "HR",
            }),
        ]);
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    // println!("final sql: {}", sql.0);
    // println!("final binds: {:?}", sql.1);
    let true_sql = "INSERT INTO mydb.users (`city`, department, name) VALUES (?, ?, ?), (?, ?, ?)";
    assert_eq!(sql.0, true_sql);
    assert_eq!(
        sql.1,
        vec![
            Value::String("New York".to_string()),
            Value::String("IT".to_string()),
            Value::String("John".to_string()),
            Value::String("New York".to_string()),
            Value::String("HR".to_string()),
            Value::String("Jane".to_string())
        ]
    );
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_update() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .db("mydb") // For dynamic db
        .table("users")
        .update(serde_json::json!({
            "name": "John",
            "`city`": "New York",
            "department": "IT",
        }))
        .query(|qb| {
            qb.where_eq("id", Value::Number(1.into()));
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    // println!("final sql: {}", sql.0);
    // println!("final binds: {:?}", sql.1);
    let true_sql = "UPDATE mydb.users SET `city` = ?, department = ?, name = ? WHERE id = ?";
    assert_eq!(sql.0, true_sql);
    assert_eq!(
        sql.1,
        vec![
            Value::String("New York".to_string()),
            Value::String("IT".to_string()),
            Value::String("John".to_string()),
            Value::Number(1.into())
        ]
    );
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_delete() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .db("mydb") // For dynamic db
        .table("users")
        .delete()
        .query(|qb| {
            qb.where_eq("id", Value::Number(1.into()));
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    // println!("final sql: {}", sql.0);
    // println!("final binds: {:?}", sql.1);
    let true_sql = "DELETE FROM mydb.users WHERE id = ?";
    assert_eq!(sql.0, true_sql);
    assert_eq!(sql.1, vec![Value::Number(1.into())]);
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_with() {
    let mut slct = ChainBuilder::new(Client::Mysql);
    slct.db("mydb") // For dynamic db
        .table("address")
        .select(Select::Columns(vec!["*".into()]))
        .query(|qb| {
            qb.where_eq("city", Value::String("New York".to_string()));
        });

    let mut active_users = ChainBuilder::new(Client::Mysql);
    active_users
        .db("mydb") // For dynamic db
        .table("users")
        .select(Select::Columns(vec!["*".into()]))
        .select(Select::Builder("address".to_string(), slct))
        .query(|qb| {
            qb.where_eq("status", Value::String("active".to_string()));
        });

    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .with("active_users", active_users)
        .select(Select::Columns(vec!["*".into()]))
        .table("active_users")
        .query(|qb| {
            qb.where_eq("name", Value::String("John".to_string()));
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    // println!("final sql: {}", sql.0);
    // println!("final binds: {:?}", sql.1);
    let true_sql = "WITH active_users AS (SELECT *, (SELECT * FROM mydb.address WHERE city = ?) AS address FROM mydb.users WHERE status = ?) SELECT * FROM active_users WHERE name = ?";
    assert_eq!(sql.0, true_sql);
    assert_eq!(
        sql.1,
        vec![
            Value::String("New York".to_string()),
            Value::String("active".to_string()),
            Value::String("John".to_string())
        ]
    );
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_with_recursive() {
    let mut slct = ChainBuilder::new(Client::Mysql);
    slct.db("mydb") // For dynamic db
        .table("address")
        .select(Select::Columns(vec!["*".into()]))
        .query(|qb| {
            qb.where_eq("city", Value::String("New York".to_string()));
        });

    let mut active_users = ChainBuilder::new(Client::Mysql);
    active_users
        .db("mydb") // For dynamic db
        .table("users")
        .select(Select::Columns(vec!["*".into()]))
        .select(Select::Builder("address".to_string(), slct))
        .query(|qb| {
            qb.where_eq("status", Value::String("active".to_string()));
        });

    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .with_recursive("active_users", active_users)
        .select(Select::Columns(vec!["*".into()]))
        .table("active_users")
        .query(|qb| {
            qb.where_eq("name", Value::String("John".to_string()));
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    // println!("final sql: {}", sql.0);
    // println!("final binds: {:?}", sql.1);
    let true_sql = "WITH RECURSIVE active_users AS (SELECT *, (SELECT * FROM mydb.address WHERE city = ?) AS address FROM mydb.users WHERE status = ?) SELECT * FROM active_users WHERE name = ?";
    assert_eq!(sql.0, true_sql);
    assert_eq!(
        sql.1,
        vec![
            Value::String("New York".to_string()),
            Value::String("active".to_string()),
            Value::String("John".to_string())
        ]
    );
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_union() {
    let mut pending_users = ChainBuilder::new(Client::Mysql);
    pending_users
        .db("mydb") // For dynamic db
        .table("users")
        .select(Select::Columns(vec!["*".into()]))
        .query(|qb| {
            qb.where_eq("status", Value::String("pending".to_string()));
        });

    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .union(pending_users)
        .db("mydb") // For dynamic db
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.where_eq("name", Value::String("John".to_string()));
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    let true_sql =
        "SELECT * FROM mydb.users WHERE name = ? UNION SELECT * FROM mydb.users WHERE status = ?";
    assert_eq!(sql.0, true_sql);
    assert_eq!(
        sql.1,
        vec![
            Value::String("John".to_string()),
            Value::String("pending".to_string())
        ]
    );
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_union_all() {
    let mut pending_users = ChainBuilder::new(Client::Mysql);
    pending_users
        .db("mydb") // For dynamic db
        .table("users")
        .select(Select::Columns(vec!["*".into()]))
        .query(|qb| {
            qb.where_eq("status", Value::String("pending".to_string()));
        });

    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .union_all(pending_users.clone())
        .union_all(pending_users)
        .db("mydb") // For dynamic db
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.where_eq("name", Value::String("John".to_string()));
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    let true_sql =
        "SELECT * FROM mydb.users WHERE name = ? UNION ALL SELECT * FROM mydb.users WHERE status = ? UNION ALL SELECT * FROM mydb.users WHERE status = ?";
    assert_eq!(sql.0, true_sql);
    assert_eq!(
        sql.1,
        vec![
            Value::String("John".to_string()),
            Value::String("pending".to_string()),
            Value::String("pending".to_string())
        ]
    );
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_limit_offset() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .db("mydb") // For dynamic db
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.where_eq("name", Value::String("John".to_string()));
            qb.limit(10).offset(5);
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    let true_sql = "SELECT * FROM mydb.users WHERE name = ? LIMIT ? OFFSET ?";
    assert_eq!(sql.0, true_sql);
    assert_eq!(
        sql.1,
        vec![
            Value::String("John".to_string()),
            Value::Number(10.into()),
            Value::Number(5.into())
        ]
    );
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_group_by() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .db("mydb") // For dynamic db
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.where_eq("name", Value::String("John".to_string()));
            qb.limit(10).offset(5);
            qb.group_by(vec!["name", "city"]);
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    let true_sql = "SELECT * FROM mydb.users WHERE name = ? GROUP BY name, city LIMIT ? OFFSET ?";
    assert_eq!(sql.0, true_sql);
    assert_eq!(
        sql.1,
        vec![
            Value::String("John".to_string()),
            Value::Number(10.into()),
            Value::Number(5.into())
        ]
    );
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_group_by_raw() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .db("mydb") // For dynamic db
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.where_eq("name", Value::String("John".to_string()));
            qb.limit(10).offset(5);
            qb.group_by_raw("name, city", None);
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    let true_sql = "SELECT * FROM mydb.users WHERE name = ? GROUP BY name, city LIMIT ? OFFSET ?";
    assert_eq!(sql.0, true_sql);
    assert_eq!(
        sql.1,
        vec![
            Value::String("John".to_string()),
            Value::Number(10.into()),
            Value::Number(5.into())
        ]
    );
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_order_by() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .db("mydb") // For dynamic db
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.where_eq("name", Value::String("John".to_string()));
            qb.limit(10).offset(5);
            qb.order_by("name", "ASC");
            qb.order_by("city", "DESC");
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    let true_sql =
        "SELECT * FROM mydb.users WHERE name = ? ORDER BY name ASC, city DESC LIMIT ? OFFSET ?";
    assert_eq!(sql.0, true_sql);
    assert_eq!(
        sql.1,
        vec![
            Value::String("John".to_string()),
            Value::Number(10.into()),
            Value::Number(5.into())
        ]
    );
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_order_by_raw() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .db("mydb") // For dynamic db
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.where_eq("name", Value::String("John".to_string()));
            qb.limit(10).offset(5);
            qb.order_by_raw("`count`, `name` order by (`name` is not null) desc", None);
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    let true_sql = "SELECT * FROM mydb.users WHERE name = ? ORDER BY `count`, `name` order by (`name` is not null) desc LIMIT ? OFFSET ?";
    assert_eq!(sql.0, true_sql);
    assert_eq!(
        sql.1,
        vec![
            Value::String("John".to_string()),
            Value::Number(10.into()),
            Value::Number(5.into())
        ]
    );
    assert_eq!(to_sqlx.sql(), true_sql);
}
