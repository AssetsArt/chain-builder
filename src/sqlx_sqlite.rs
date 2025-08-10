use crate::builder::ChainBuilder;
use serde_json::Value;
use sqlx::{self, sqlite::SqliteArguments, Arguments};

impl ChainBuilder {
    /// Build SQL + args for SQLite (use with sqlx::query_with(&sql, args))
    #[cfg(all(feature = "sqlite", feature = "sqlx_sqlite"))]
    pub fn to_sqlx_parts_sqlite(&mut self) -> (String, SqliteArguments<'_>) {
        let (sql, binds) = self.to_sql();
        let mut args = SqliteArguments::default();

        for bind in binds {
            push_sqlite_arg(&mut args, bind);
        }

        (sql, args)
    }
}

#[cfg(all(feature = "sqlite", feature = "sqlx_sqlite"))]
fn push_sqlite_arg(arguments: &mut SqliteArguments, v: Value) {
    match v {
        serde_json::Value::String(v) => {
            // qb = qb.bind(v);
            let _ = arguments.add(v);
        }
        serde_json::Value::Number(v) => {
            if v.is_f64() {
                // qb = qb.bind(v.as_f64().unwrap_or(0.0));
                let _ = arguments.add(v.as_f64().unwrap_or(0.0));
            } else if v.is_u64() {
                // qb = qb.bind(v.as_u64().unwrap_or(0));
                let _ = arguments.add(v.as_u64().unwrap_or(0) as i64);
            } else if v.is_i64() {
                // qb = qb.bind(v.as_i64().unwrap_or(0));
                let _ = arguments.add(v.as_i64().unwrap_or(0));
            } else {
                // qb = qb.bind(v.to_string());
                let _ = arguments.add(v.to_string());
            }
        }
        serde_json::Value::Bool(v) => {
            // qb = qb.bind(v);
            let _ = arguments.add(v);
        }
        serde_json::Value::Null => {
            let null_data: Option<Value> = None;
            // qb = qb.bind(null_data);
            let _ = arguments.add(null_data);
        }
        serde_json::Value::Object(v) => {
            let to_string = serde_json::to_string(&v).unwrap_or_default();
            // qb = qb.bind(to_string);
            let _ = arguments.add(to_string);
        }
        _ => {
            // qb = qb.bind(bind);
            let _ = arguments.add(v);
        }
    }
}
