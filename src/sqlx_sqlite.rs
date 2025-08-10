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
fn push_sqlite_arg(args: &mut SqliteArguments, v: Value) {
    match v {
        Value::Null => {
            let _ = args.add(Option::<i32>::None); // NULL ชนิดไหนก็ได้
        }
        Value::Bool(b) => {
            // SQLite รองรับ bool ผ่าน sqlx (encode เป็น INTEGER 0/1)
            let _ = args.add(b);
        }
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                let _ = args.add(i);
            } else if let Some(u) = n.as_u64() {
                // SQLite เป็น signed 64-bit; ระวัง overflow
                if u <= i64::MAX as u64 {
                    let _ = args.add(u as i64);
                } else {
                    // ใหญ่เกิน เก็บเป็น string ปลอดภัยกว่า
                    let _ = args.add(u.to_string());
                }
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
            // เก็บเป็น JSON text
            let _ = args.add(serde_json::to_string(&arr).unwrap());
        }
        Value::Object(obj) => {
            // เก็บเป็น JSON text
            let _ = args.add(serde_json::to_string(&obj).unwrap());
        }
    }
}