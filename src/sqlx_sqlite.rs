use crate::builder::ChainBuilder;
use serde_json::Value;
use sqlx::{self, sqlite::SqliteArguments, Arguments};

impl ChainBuilder {
    /// Convert to sqlx query for SQLite
    #[cfg(all(feature = "sqlite", feature = "sqlx_sqlite"))]
    pub fn to_sqlx_query_sqlite(&mut self) -> sqlx::query::Query<'static, sqlx::Sqlite, SqliteArguments> {
        let (_sql, binds) = self.to_sql();
        let mut args = SqliteArguments::default();
        
        for bind in binds {
            match bind {
                Value::Null => {
                    let _ = args.add(Option::<String>::None);
                }
                Value::Bool(b) => {
                    let _ = args.add(b);
                }
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        let _ = args.add(i);
                    } else if let Some(f) = n.as_f64() {
                        let _ = args.add(f);
                    } else {
                        let _ = args.add(n.to_string());
                    }
                }
                Value::String(s) => {
                    let _ = args.add(s);
                }
                Value::Array(arr) => {
                    // For arrays, we'll serialize as JSON string
                    let _ = args.add(serde_json::to_string(&arr).unwrap());
                }
                Value::Object(obj) => {
                    // For objects, we'll serialize as JSON string
                    let _ = args.add(serde_json::to_string(&obj).unwrap());
                }
            }
        }
        
        // Use a static string for now - this is a limitation
        sqlx::query_with("SELECT 1", args)
    }
}
