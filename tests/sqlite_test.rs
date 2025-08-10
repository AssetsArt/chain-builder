use chain_builder::{ChainBuilder, Client, Select, WhereClauses, HavingClauses, JoinMethods, QueryCommon};
use serde_json::Value;

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
fn test_sqlite_insert() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .table("users")
        .insert(serde_json::json!({
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
    builder
        .table("users")
        .delete()
        .query(|qb| {
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
fn test_sqlite_joins() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .select(Select::Columns(vec!["users.name".into(), "profiles.bio".into()]))
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
    builder
        .table("orders")
        .query(|qb| {
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
fn test_sqlite_limit_offset() {
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
fn test_sqlite_union() {
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
