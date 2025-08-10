use chain_builder::{
    ChainBuilder, Client, HavingClauses, JoinMethods, QueryCommon, Select, WhereClauses,
};
use serde_json::Value;
use sqlx::Execute;

#[test]
fn test_sqlite_basic_select() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.where_eq("status", Value::String("active".to_string()));
        });

    let (sql, binds) = builder.to_sql();
    println!("SQLite SELECT SQL: {}", sql);
    println!("Binds: {:?}", binds);

    assert!(sql.contains("SELECT * FROM users"));
    assert!(sql.contains("WHERE status = ?"));
    assert_eq!(binds.len(), 1);
}

#[test]
fn test_sqlite_chain_builder() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
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
                let or_sub = sub.or();
                or_sub.where_eq("status", Value::String("pending".to_string()));
                or_sub.where_between(
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
fn test_sqlite_join() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
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
    let true_sql = "SELECT *, (SELECT COUNT(*) FROM `mydb`.`users` WHERE users.id = ?) AS count FROM mydb.users JOIN mydb.details ON details.id = users.d_id AND details.id_w = users.d_id_w OR (details.id_s = users.d_id_s AND details.id_w = users.d_id_w) WHERE name = ?";
    assert_eq!(sql.0, true_sql);
    assert_eq!(
        sql.1,
        vec![Value::Number(1.into()), Value::String("John".to_string()),]
    );
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_sqlite_tow_join() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .db("mydb") // For dynamic db
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.join("details", |join| {
                join.on("details.id", "=", "users.d_id");
            });
            qb.join("profiles", |join| {
                join.on("profiles.id", "=", "users.p_id");
            });
            qb.where_eq("name", Value::String("John".to_string()));
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    let true_sql = "SELECT * FROM mydb.users JOIN mydb.details ON details.id = users.d_id JOIN mydb.profiles ON profiles.id = users.p_id WHERE name = ?";
    assert_eq!(sql.0, true_sql);
    assert_eq!(sql.1, vec![Value::String("John".to_string())]);
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_sqlite_join_raw() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .db("mydb") // For dynamic db
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.raw_join(
                "JOIN details ON details.id = users.d_id AND details.status = ?",
                Some(vec![Value::String("active".to_string())]),
            );
            qb.where_eq("name", Value::String("John".to_string()));
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    let true_sql = "SELECT * FROM mydb.users JOIN details ON details.id = users.d_id AND details.status = ? WHERE name = ?";
    assert_eq!(sql.0, true_sql);
    assert_eq!(
        sql.1,
        vec![
            Value::String("active".to_string()),
            Value::String("John".to_string()),
        ]
    );
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_sqlite_insert() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder.table("users").insert(serde_json::json!({
        "name": "John Doe",
        "email": "john@example.com",
        "age": 30
    }));

    let (sql, binds) = builder.to_sql();
    println!("SQLite INSERT SQL: {}", sql);
    println!("Binds: {:?}", binds);

    assert!(sql.contains("INSERT INTO users"));
    assert!(sql.contains("VALUES (?, ?, ?)"));
    assert_eq!(binds.len(), 3);
}

#[test]
fn test_sqlite_insert_many() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder.table("users").insert_many(vec![
        serde_json::json!({
            "name": "John Doe",
            "email": "john@example.com",
            "age": 30
        }),
        serde_json::json!({
            "name": "Jane Smith",
            "email": "jane@example.com",
            "age": 25
        }),
    ]);

    let (sql, binds) = builder.to_sql();
    println!("SQLite INSERT MANY SQL: {}", sql);
    println!("Binds: {:?}", binds);

    assert!(sql.contains("INSERT INTO users"));
    assert!(sql.contains("VALUES (?, ?, ?), (?, ?, ?)"));
    assert_eq!(binds.len(), 6);
}

#[test]
fn test_sqlite_update() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .table("users")
        .update(serde_json::json!({
            "status": "inactive",
            "updated_at": "2024-01-15"
        }))
        .query(|qb| {
            qb.where_eq("id", Value::Number(1.into()));
        });

    let (sql, binds) = builder.to_sql();
    println!("SQLite UPDATE SQL: {}", sql);
    println!("Binds: {:?}", binds);

    assert!(sql.contains("UPDATE users SET"));
    assert!(sql.contains("status = ?"));
    assert!(sql.contains("updated_at = ?"));
    assert!(sql.contains("WHERE id = ?"));
    assert_eq!(binds.len(), 3);
}

#[test]
fn test_sqlite_delete() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder.table("users").delete().query(|qb| {
        qb.where_eq("status", Value::String("deleted".to_string()));
    });

    let (sql, binds) = builder.to_sql();
    println!("SQLite DELETE SQL: {}", sql);
    println!("Binds: {:?}", binds);

    assert!(sql.contains("DELETE FROM users"));
    assert!(sql.contains("WHERE status = ?"));
    assert_eq!(binds.len(), 1);
}

#[test]
fn test_sqlite_with() {
    let mut slct = ChainBuilder::new(Client::Sqlite);
    slct.db("mydb") // For dynamic db
        .table("address")
        .select(Select::Columns(vec!["*".into()]))
        .query(|qb| {
            qb.where_eq("city", Value::String("New York".to_string()));
        });

    let mut active_users = ChainBuilder::new(Client::Sqlite);
    active_users
        .db("mydb") // For dynamic db
        .table("users")
        .select(Select::Columns(vec!["*".into()]))
        .select(Select::Builder("address".to_string(), slct))
        .query(|qb| {
            qb.where_eq("status", Value::String("active".to_string()));
        });

    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .with("active_users", active_users)
        .select(Select::Columns(vec!["*".into()]))
        .table("active_users")
        .query(|qb| {
            qb.where_eq("name", Value::String("John".to_string()));
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    let true_sql = "WITH active_users AS (SELECT *, (SELECT * FROM mydb.address WHERE city = ?) AS address FROM mydb.users)SELECT * FROM active_users WHERE name = ?";
    assert_eq!(sql.0, true_sql);
    assert_eq!(
        sql.1,
        vec![
            Value::String("New York".to_string()),
            Value::String("John".to_string())
        ]
    );
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_sqlite_with_recursive() {
    let mut slct = ChainBuilder::new(Client::Sqlite);
    slct.db("mydb") // For dynamic db
        .table("address")
        .select(Select::Columns(vec!["*".into()]))
        .query(|qb| {
            qb.where_eq("city", Value::String("New York".to_string()));
        });

    let mut active_users = ChainBuilder::new(Client::Sqlite);
    active_users
        .db("mydb") // For dynamic db
        .table("users")
        .select(Select::Columns(vec!["*".into()]))
        .select(Select::Builder("address".to_string(), slct))
        .query(|qb| {
            qb.where_eq("status", Value::String("active".to_string()));
        });

    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .with_recursive("active_users", active_users)
        .select(Select::Columns(vec!["*".into()]))
        .table("active_users")
        .query(|qb| {
            qb.where_eq("name", Value::String("John".to_string()));
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    let true_sql = "WITH RECURSIVE active_users AS (SELECT *, (SELECT * FROM mydb.address WHERE city = ?) AS address FROM mydb.users)SELECT * FROM active_users WHERE name = ?";
    assert_eq!(sql.0, true_sql);
    assert_eq!(
        sql.1,
        vec![
            Value::String("New York".to_string()),
            Value::String("John".to_string())
        ]
    );
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_sqlite_union() {
    let mut pending_users = ChainBuilder::new(Client::Sqlite);
    pending_users
        .db("mydb") // For dynamic db
        .table("users")
        .select(Select::Columns(vec!["*".into()]))
        .query(|qb| {
            qb.where_eq("status", Value::String("pending".to_string()));
        });

    let mut builder = ChainBuilder::new(Client::Sqlite);
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
    let true_sql = "SELECT * FROM mydb.users WHERE name = ? UNION SELECT * FROM mydb.users";
    assert_eq!(sql.0, true_sql);
    assert_eq!(sql.1, vec![Value::String("John".to_string()),]);
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_sqlite_union_all() {
    let mut pending_users = ChainBuilder::new(Client::Sqlite);
    pending_users
        .db("mydb") // For dynamic db
        .table("users")
        .select(Select::Columns(vec!["*".into()]))
        .query(|qb| {
            qb.where_eq("status", Value::String("pending".to_string()));
        });

    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .union_all(pending_users)
        .db("mydb") // For dynamic db
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.where_eq("name", Value::String("John".to_string()));
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    let true_sql = "SELECT * FROM mydb.users WHERE name = ? UNION ALL SELECT * FROM mydb.users";
    assert_eq!(sql.0, true_sql);
    assert_eq!(sql.1, vec![Value::String("John".to_string()),]);
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_sqlite_limit_offset() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .db("mydb") // For dynamic db
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.limit(10);
            qb.offset(20);
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    let true_sql = "SELECT * FROM mydb.users LIMIT 20, 10";
    assert_eq!(sql.0, true_sql);
    assert_eq!(sql.1, Vec::<Value>::new());
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_sqlite_group_by() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .db("mydb") // For dynamic db
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.group_by(vec!["department".to_string(), "status".to_string()]);
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    let true_sql = "SELECT * FROM mydb.users GROUP BY department, status";
    assert_eq!(sql.0, true_sql);
    assert_eq!(sql.1, Vec::<Value>::new());
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_sqlite_group_by_raw() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .db("mydb") // For dynamic db
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.group_by_raw("department, status", None);
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    let true_sql = "SELECT * FROM mydb.users GROUP BY department, status";
    assert_eq!(sql.0, true_sql);
    assert_eq!(sql.1, Vec::<Value>::new());
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_sqlite_order_by() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .db("mydb") // For dynamic db
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.order_by("name", "ASC");
            qb.order_by("age", "DESC");
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    let true_sql = "SELECT * FROM mydb.users ORDER BY name ASC, age DESC";
    assert_eq!(sql.0, true_sql);
    assert_eq!(sql.1, Vec::<Value>::new());
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_sqlite_order_by_raw() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .db("mydb") // For dynamic db
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.order_by_raw("name ASC, age DESC", None);
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    let true_sql = "SELECT * FROM mydb.users ORDER BY name ASC, age DESC";
    assert_eq!(sql.0, true_sql);
    assert_eq!(sql.1, Vec::<Value>::new());
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_sqlite_table_raw() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .db("mydb") // For dynamic db
        .select(Select::Columns(vec!["*".into()]))
        .table_raw(
            "(SELECT * FROM users WHERE status = ?) as active_users",
            Some(vec![Value::String("active".to_string())]),
        )
        .query(|qb| {
            qb.where_eq("name", Value::String("John".to_string()));
        });
    let sql = builder.to_sql();
    let to_sqlx = builder.to_sqlx_query();
    let true_sql =
        "SELECT * FROM (SELECT * FROM users WHERE status = ?) as active_users WHERE name = ?";
    assert_eq!(sql.0, true_sql);
    assert_eq!(
        sql.1,
        vec![
            Value::String("active".to_string()),
            Value::String("John".to_string()),
        ]
    );
    assert_eq!(to_sqlx.sql(), true_sql);
}

#[test]
fn test_sqlite_joins() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .select(Select::Columns(vec![
            "users.name".into(),
            "profiles.bio".into(),
        ]))
        .table("users")
        .query(|qb| {
            qb.left_join("profiles", |join| {
                join.on("users.id", "=", "profiles.user_id");
            });
            qb.where_eq("users.status", Value::String("active".to_string()));
        });

    let (sql, binds) = builder.to_sql();
    println!("SQLite JOIN SQL: {}", sql);
    println!("Binds: {:?}", binds);

    assert!(sql.contains("SELECT users.name, profiles.bio FROM users"));
    assert!(sql.contains("LEFT JOIN profiles ON users.id = profiles.user_id"));
    assert!(sql.contains("WHERE users.status = ?"));
}

#[test]
fn test_sqlite_aggregate_functions() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder.table("orders").query(|qb| {
        qb.group_by(vec!["user_id".to_string()]);
        qb.having("COUNT(*)", ">", Value::Number(5.into()));
    });

    builder
        .select_count("id")
        .select_sum("amount")
        .select_avg("amount");

    let (sql, binds) = builder.to_sql();
    println!("SQLite Aggregate SQL: {}", sql);
    println!("Binds: {:?}", binds);

    assert!(sql.contains("SELECT COUNT(id), SUM(amount), AVG(amount) FROM orders"));
    assert!(sql.contains("GROUP BY user_id"));
    assert!(sql.contains("HAVING COUNT(*) > ?"));
}

#[test]
fn test_sqlite_limit_offset_old() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.limit(10);
            qb.offset(20);
        });

    let (sql, binds) = builder.to_sql();
    println!("SQLite LIMIT/OFFSET SQL: {}", sql);
    println!("Binds: {:?}", binds);

    // SQLite uses LIMIT offset, count syntax
    assert!(sql.contains("LIMIT 20, 10"));
}

#[test]
fn test_sqlite_with_cte() {
    let mut active_users = ChainBuilder::new(Client::Sqlite);
    active_users
        .table("users")
        .select(Select::Columns(vec!["*".into()]))
        .query(|qb| {
            qb.where_eq("status", Value::String("active".to_string()));
        });

    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .with("active_users", active_users)
        .select(Select::Columns(vec!["*".into()]))
        .table("active_users")
        .query(|qb| {
            qb.where_gt("age", Value::Number(25.into()));
        });

    let (sql, binds) = builder.to_sql();
    println!("SQLite CTE SQL: {}", sql);
    println!("Binds: {:?}", binds);

    assert!(sql.contains("WITH active_users AS ("));
    assert!(sql.contains("SELECT * FROM active_users"));
}

#[test]
fn test_sqlite_union_old() {
    let mut pending_users = ChainBuilder::new(Client::Sqlite);
    pending_users
        .table("users")
        .select(Select::Columns(vec!["*".into()]))
        .query(|qb| {
            qb.where_eq("status", Value::String("pending".to_string()));
        });

    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .union(pending_users)
        .table("users")
        .select(Select::Columns(vec!["*".into()]))
        .query(|qb| {
            qb.where_eq("status", Value::String("active".to_string()));
        });

    let (sql, binds) = builder.to_sql();
    println!("SQLite UNION SQL: {}", sql);
    println!("Binds: {:?}", binds);

    assert!(sql.contains("UNION"));
    assert!(sql.contains("SELECT * FROM users"));
}
