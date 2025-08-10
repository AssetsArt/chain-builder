use crate::builder::ChainBuilder;
use serde_json::Value;
use sqlx::{self, sqlite::SqliteArguments, Arguments};

impl ChainBuilder {
    /// Build SQL + args for SQLite (use with sqlx::query_with(&sql, args))
    #[cfg(all(feature = "sqlite", feature = "sqlx_sqlite"))]
    pub fn to_sqlx_parts_sqlite(&mut self) -> (String, SqliteArguments<'static>) {
        let (sql, binds) = self.to_sql();
        // ระบุ lifetime ให้ชัดเจน
        let mut args: SqliteArguments<'static> = SqliteArguments::default();

        for bind in binds {
            push_sqlite_arg(&mut args, bind);
        }

        (sql, args)
    }

    #[cfg(all(feature = "sqlite", feature = "sqlx_sqlite"))]
    pub fn to_sqlx_query(
        &mut self,
    ) -> sqlx::query::Query<'_, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'_>> {
        let (_, binds) = self.to_sql();
        sqlx::query_with(self.sql_str.as_str(), self.value_to_arguments(&binds))
    }

    #[cfg(all(feature = "sqlite", feature = "sqlx_sqlite"))]
    pub fn to_sqlx_query_as<T>(
        &mut self,
    ) -> sqlx::query::QueryAs<'_, sqlx::Sqlite, T, sqlx::sqlite::SqliteArguments<'_>>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::sqlite::SqliteRow>,
    {
        let (_, binds) = self.to_sql();
        sqlx::query_as_with(self.sql_str.as_str(), self.value_to_arguments(&binds))
    }

    #[cfg(all(feature = "sqlite", feature = "sqlx_sqlite"))]
    fn value_to_arguments(&self, binds: &Vec<Value>) -> SqliteArguments<'static> {
        let mut arguments: SqliteArguments<'static> = SqliteArguments::default();
        for bind in binds {
            push_sqlite_arg(&mut arguments, bind.clone());
        }
        arguments
    }
}

#[cfg(all(feature = "sqlite", feature = "sqlx_sqlite"))]
fn push_sqlite_arg<'a>(arguments: &mut SqliteArguments<'a>, v: Value) {
    match v {
        Value::Null => {
            // bind NULL อย่างชัดเจน
            let _ = arguments.add(Option::<String>::None);
        }
        Value::Bool(b) => {
            let _ = arguments.add(b);
        }
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                let _ = arguments.add(i);
            } else if let Some(u) = n.as_u64() {
                if u <= i64::MAX as u64 {
                    let _ = arguments.add(u as i64);
                } else {
                    // ถ้าใหญ่เกิน เก็บเป็น string ปลอดภัยสุด
                    let _ = arguments.add(u.to_string());
                }
            } else if let Some(f) = n.as_f64() {
                let _ = arguments.add(f);
            } else {
                let _ = arguments.add(n.to_string());
            }
        }
        Value::String(s) => {
            let _ = arguments.add(s);
        }
        Value::Array(arr) => {
            // SQLite ไม่มี array type → เก็บเป็น JSON text
            let _ = arguments.add(serde_json::to_string(&arr).unwrap_or_default());
        }
        Value::Object(obj) => {
            // เก็บเป็น JSON text
            let _ = arguments.add(serde_json::to_string(&obj).unwrap_or_default());
        }
    }
}
