use crate::builder::ChainBuilder;
use serde_json::Value;
use sqlx::{self, mysql::MySqlArguments, Arguments, Row};

impl ChainBuilder {
    fn value_to_arguments(&self, binds: &Vec<Value>) -> MySqlArguments {
        let mut arguments: MySqlArguments = MySqlArguments::default();
        for bind in binds {
            match bind {
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
                        let _ = arguments.add(v.as_u64().unwrap_or(0));
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
                    let _ = arguments.add(bind);
                }
            }
        }
        arguments
    }

    pub fn to_sqlx_query(
        &mut self,
    ) -> sqlx::query::Query<'_, sqlx::MySql, sqlx::mysql::MySqlArguments> {
        let (_, binds) = self.to_sql();
        sqlx::query_with(self.sql_str.as_str(), self.value_to_arguments(&binds))
    }

    pub fn to_sqlx_query_as<T>(
        &mut self,
    ) -> sqlx::query::QueryAs<'_, sqlx::MySql, T, sqlx::mysql::MySqlArguments>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow>,
    {
        let (_, binds) = self.to_sql();
        sqlx::query_as_with(self.sql_str.as_str(), self.value_to_arguments(&binds))
    }

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
