use chain_builder::{ChainBuilder, Client, Select, WhereClauses, HavingClauses, JoinMethods, QueryCommon};
use serde_json::Value;

#[test]
fn test_advanced_where_clauses() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .db("mydb")
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            // Test where_ilike (case-insensitive LIKE)
            qb.where_ilike("name", Value::String("john".to_string()));
            
            // Test where_column (column-to-column comparison)
            qb.where_column("users.age", ">", "profiles.min_age");
            
            // Test where_exists
            qb.where_exists(|sub| {
                sub.db("mydb")
                    .table("orders")
                    .select(Select::Columns(vec!["id".into()]))
                    .query(|sub_qb| {
                        sub_qb.where_column("orders.user_id", "=", "users.id");
                        sub_qb.where_eq("status", Value::String("completed".to_string()));
                    });
            });
            
            // Test where_json_contains
            qb.where_json_contains("metadata", Value::String("premium".to_string()));
        });

    let (sql, binds) = builder.to_sql();
    println!("Advanced WHERE SQL: {}", sql);
    println!("Binds: {:?}", binds);
    
    // Basic assertions
    assert!(sql.contains("LOWER(name) LIKE LOWER(?)"));
    assert!(sql.contains("users.age > profiles.min_age"));
    assert!(sql.contains("EXISTS ("));
    assert!(sql.contains("JSON_CONTAINS(metadata, ?)"));
}

#[test]
fn test_aggregate_functions() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .db("mydb")
        .table("orders")
        .query(|qb| {
            qb.group_by(vec!["user_id".to_string()]);
            qb.having("COUNT(*)", ">", Value::Number(5.into()));
        });
    
    // Test aggregate functions
    builder
        .select_count("id")
        .select_sum("amount")
        .select_avg("amount")
        .select_max("created_at")
        .select_min("created_at")
        .select_alias("user_id", "uid");

    let (sql, binds) = builder.to_sql();
    println!("Aggregate SQL: {}", sql);
    println!("Binds: {:?}", binds);
    
    // Basic assertions
    assert!(sql.contains("COUNT(id)"));
    assert!(sql.contains("SUM(amount)"));
    assert!(sql.contains("AVG(amount)"));
    assert!(sql.contains("MAX(created_at)"));
    assert!(sql.contains("MIN(created_at)"));
    assert!(sql.contains("user_id AS uid"));
}

#[test]
fn test_advanced_joins() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .db("mydb")
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            // Test full outer join
            qb.full_outer_join("profiles", |join| {
                join.on("users.id", "=", "profiles.user_id");
            });
            
            // Test cross join
            qb.cross_join("departments", |join| {
                join.on("users.department_id", "=", "departments.id");
            });
            
            // Test join using
            qb.join_using("roles", vec!["user_id".to_string()]);
        });

    let (sql, binds) = builder.to_sql();
    println!("Advanced JOIN SQL: {}", sql);
    println!("Binds: {:?}", binds);
    
    // Basic assertions
    assert!(sql.contains("FULL OUTER JOIN"));
    assert!(sql.contains("CROSS JOIN"));
    assert!(sql.contains("JOIN roles USING (user_id)"));
}

#[test]
fn test_having_clauses() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .db("mydb")
        .select(Select::Columns(vec!["user_id".to_string(), "COUNT(*)".to_string()]))
        .table("orders")
        .query(|qb| {
            qb.group_by(vec!["user_id".to_string()]);
            
            // Test having conditions
            qb.having("COUNT(*)", ">", Value::Number(5.into()));
            qb.having_between("SUM(amount)", [Value::Number(100.into()), Value::Number(1000.into())]);
            qb.having_in("user_id", vec![Value::Number(1.into()), Value::Number(2.into())]);
            qb.having_raw("AVG(amount) > ?", Some(vec![Value::Number(50.into())]));
        });

    let (sql, binds) = builder.to_sql();
    println!("HAVING SQL: {}", sql);
    println!("Binds: {:?}", binds);
    
    // Basic assertions
    assert!(sql.contains("GROUP BY"));
    assert!(sql.contains("HAVING"));
}

#[test]
fn test_select_methods() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .db("mydb")
        .table("users")
        .query(|qb| {
            qb.where_eq("status", Value::String("active".to_string()));
        });
    
    // Test various select methods
    builder
        .select_distinct(vec!["name".to_string(), "email".to_string()])
        .select_raw("CONCAT(first_name, ' ', last_name) AS full_name", None)
        .select_count("id")
        .select_sum("points")
        .select_alias("created_at", "joined_at");

    let (sql, binds) = builder.to_sql();
    println!("Advanced SELECT SQL: {}", sql);
    println!("Binds: {:?}", binds);
    
    // Basic assertions
    assert!(sql.contains("SELECT DISTINCT"));
    assert!(sql.contains("CONCAT(first_name, ' ', last_name) AS full_name"));
    assert!(sql.contains("COUNT(id)"));
    assert!(sql.contains("SUM(points)"));
    assert!(sql.contains("created_at AS joined_at"));
}
