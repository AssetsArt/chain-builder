//! Security regression tests: identifier interpolation must not allow SQL
//! injection. Values are always bound; identifiers (columns/tables/aliases)
//! are dialect-escaped so attacker-controlled names cannot break out.

use chain_builder::{ChainBuilder, Client, JoinMethods, QueryCommon, Select, WhereClauses};
use serde_json::Value;

#[test]
fn mysql_where_column_is_escaped() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            // A malicious "column" coming from untrusted input.
            qb.where_eq("name`; DROP TABLE users; --", Value::String("x".to_string()));
        });
    let (sql, binds) = builder.to_sql();

    // The injection payload is contained inside a single backtick-quoted
    // identifier (backticks doubled); it is NOT executable SQL.
    assert_eq!(
        sql,
        "SELECT * FROM `users` WHERE `name``; DROP TABLE users; --` = ?"
    );
    // The value is still bound, not inlined.
    assert_eq!(binds, vec![Value::String("x".to_string())]);
    // No unescaped statement terminator leaked into the SQL.
    assert!(!sql.contains("; DROP TABLE users; --` = ? ;"));
}

#[test]
fn mysql_order_by_column_is_escaped() {
    // Classic dynamic-sort injection vector.
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .select(Select::Columns(vec!["*".into()]))
        .table("users")
        .query(|qb| {
            qb.order_by("id`; DROP TABLE users; --", "ASC");
        });
    let (sql, _) = builder.to_sql();
    assert_eq!(
        sql,
        "SELECT * FROM `users` ORDER BY `id``; DROP TABLE users; --` ASC"
    );
}

#[test]
fn sqlite_identifiers_use_double_quotes() {
    let mut builder = ChainBuilder::new(Client::Sqlite);
    builder
        .select(Select::Columns(vec!["id".into(), "name".into()]))
        .table("users")
        .query(|qb| {
            qb.where_eq("name\" OR \"1\"=\"1", Value::String("x".to_string()));
        });
    let (sql, _) = builder.to_sql();
    assert_eq!(
        sql,
        "SELECT \"id\", \"name\" FROM \"users\" WHERE \"name\"\" OR \"\"1\"\"=\"\"1\" = ?"
    );
}

#[test]
fn mysql_qualified_and_wildcard_identifiers() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder
        .db("mydb")
        .select(Select::Columns(vec!["users.*".into()]))
        .table("users")
        .query(|qb| {
            qb.left_join("profiles", |j| {
                j.on("users.id", "=", "profiles.user_id");
            });
            qb.where_eq("users.status", Value::String("active".to_string()));
        });
    let (sql, _) = builder.to_sql();
    assert_eq!(
        sql,
        "SELECT `users`.* FROM `mydb`.`users` \
         LEFT JOIN `mydb`.`profiles` ON `users`.`id` = `profiles`.`user_id` \
         WHERE `users`.`status` = ?"
    );
}

#[test]
fn mysql_insert_keys_are_escaped() {
    let mut builder = ChainBuilder::new(Client::Mysql);
    builder.table("users").insert(serde_json::json!({
        "name": "John",
        "email": "john@example.com",
    }));
    let (sql, _) = builder.to_sql();
    assert_eq!(
        sql,
        "INSERT INTO `users` (`email`, `name`) VALUES (?, ?)"
    );
}
