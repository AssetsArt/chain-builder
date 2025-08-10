use crate::builder::ChainBuilder;
use serde_json::Value;
use sqlx::{self, mysql::MySqlArguments, Arguments, Row};

impl ChainBuilder {
    #[cfg(all(feature = "mysql", feature = "sqlx_mysql"))]
    fn value_to_arguments(&self, binds: &Vec<Value>) -> MySqlArguments {
        let mut arguments: MySqlArguments = MySqlArguments::default();
        for bind in binds {
            match bind {
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
        arguments
    }

    #[cfg(all(feature = "mysql", feature = "sqlx_mysql"))]
    pub fn to_sqlx_query(
        &mut self,
    ) -> sqlx::query::Query<'_, sqlx::MySql, sqlx::mysql::MySqlArguments> {
        let (_, binds) = self.to_sql();
        sqlx::query_with(self.sql_str.as_str(), self.value_to_arguments(&binds))
    }

    #[cfg(all(feature = "mysql", feature = "sqlx_mysql"))]
    pub fn to_sqlx_query_as<T>(
        &mut self,
    ) -> sqlx::query::QueryAs<'_, sqlx::MySql, T, sqlx::mysql::MySqlArguments>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow>,
    {
        let (_, binds) = self.to_sql();
        sqlx::query_as_with(self.sql_str.as_str(), self.value_to_arguments(&binds))
    }

    #[cfg(all(feature = "mysql", feature = "sqlx_mysql"))]
    pub async fn count(
        &mut self,
        column: &str,
        pool: &sqlx::Pool<sqlx::MySql>,
    ) -> Result<i64, sqlx::Error> {
        let (_, binds) = self.to_sql();
        let sql = self.sql_str.as_str();
        let sql = format!("SELECT COUNT({}) FROM ({}) as count", column, sql);
        let qb = sqlx::query_with(&sql, self.value_to_arguments(&binds));
        let query_count = qb.fetch_one(pool).await?;
        let count: i64 = query_count.try_get(0)?;
        Ok(count)
    }
}
