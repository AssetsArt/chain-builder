use serde_json::Value;
use crate::ChainBuilder;

impl ChainBuilder {
    pub fn to_sqlx_query(
        &mut self,
    ) -> sqlx::query::Query<'_, sqlx::MySql, sqlx::mysql::MySqlArguments> {
        let (_, binds) = self.to_sql();
        let sql = self.sql_str.as_str();
        let mut qb = sqlx::query::<sqlx::MySql>(sql);
        for bind in binds {
            match bind {
                serde_json::Value::String(v) => {
                    qb = qb.bind(v);
                }
                serde_json::Value::Number(v) => {
                    if v.is_f64() {
                        qb = qb.bind(v.as_f64().unwrap_or(0.0));
                    } else if v.is_u64() {
                        qb = qb.bind(v.as_u64().unwrap_or(0));
                    } else if v.is_i64() {
                        qb = qb.bind(v.as_i64().unwrap_or(0));
                    } else {
                        qb = qb.bind(v.to_string());
                    }
                },
                serde_json::Value::Null => {
                    let null_data: Option<Value> = None;
                    qb = qb.bind(null_data);
                }
                _ => {
                    qb = qb.bind(bind);   
                }
            }
        }
        qb
    }

    pub fn to_sqlx_query_as<T>(
        &mut self,
    ) -> sqlx::query::QueryAs<'_, sqlx::MySql, T, sqlx::mysql::MySqlArguments>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow>,
    {
        let (_, binds) = self.to_sql();
        let sql = self.sql_str.as_str();
        let mut qb = sqlx::query_as::<_, T>(sql);
        for bind in binds {
            match bind {
                serde_json::Value::String(v) => {
                    qb = qb.bind(v);
                }
                serde_json::Value::Number(v) => {
                    if v.is_f64() {
                        qb = qb.bind(v.as_f64().unwrap_or(0.0));
                    } else if v.is_u64() {
                        qb = qb.bind(v.as_u64().unwrap_or(0));
                    } else if v.is_i64() {
                        qb = qb.bind(v.as_i64().unwrap_or(0));
                    } else {
                        qb = qb.bind(v.to_string());
                    }
                },
                serde_json::Value::Null => {
                    let null_data: Option<Value> = None;
                    qb = qb.bind(null_data);
                }
                _ => {
                    qb = qb.bind(bind);   
                }
            }
        }
        qb
    }
}

