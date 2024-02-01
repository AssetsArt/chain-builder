#[cfg(feature = "mysql")]
mod mysql;
// mods
mod join;
mod operator;
mod where_clauses;

// export
pub use join::{JoinBuilder, JoinMethods};
pub use operator::Operator;
pub use where_clauses::WhereClauses;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Client {
    Mysql,
    Postgres,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Statement {
    Value(String, Operator, serde_json::Value),
    SubChain(Box<QueryBuilder>),
    OrChain(Box<QueryBuilder>),
    Raw((String, Option<Vec<serde_json::Value>>)),
}

impl Statement {
    pub fn to_query_builder(&mut self) -> &mut QueryBuilder {
        match self {
            Statement::OrChain(query) => query,
            Statement::SubChain(query) => query,
            _ => panic!("Statement::to_chain_builder()"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum Method {
    Select,
    Insert,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChainBuilder {
    client: Client,
    db: Option<String>,
    table: String,
    as_name: Option<String>,
    select: Vec<Select>,
    query: QueryBuilder,
    method: Method,
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueryBuilder {
    statement: Vec<Statement>,
    raw: Vec<(String, Option<Vec<serde_json::Value>>)>,
    join: Vec<JoinBuilder>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Select {
    Columns(Vec<String>),
    Raw(String, Option<Vec<serde_json::Value>>),
    // column name, builder
    Builder(String, ChainBuilder),
}

impl ChainBuilder {
    pub fn new(client: Client) -> ChainBuilder {
        ChainBuilder {
            client,
            table: String::new(),
            select: Vec::new(),
            as_name: None,
            db: None,
            query: QueryBuilder::default(),
            method: Method::Select,
        }
    }

    pub fn db(&mut self, db: &str) -> &mut ChainBuilder {
        self.db = Some(db.to_string());
        self
    }

    pub fn table(&mut self, table: &str) -> &mut ChainBuilder {
        self.table = table.to_string();
        self
    }

    pub fn select(&mut self, select: Select) -> &mut ChainBuilder {
        self.method = Method::Select;
        self.select.push(select);
        self
    }

    pub fn as_name(&mut self, name: &str) -> &mut ChainBuilder {
        self.as_name = Some(name.to_string());
        self
    }

    pub fn to_sql(&self) -> (String, Vec<serde_json::Value>) {
        match self.client {
            #[cfg(feature = "mysql")]
            Client::Mysql => {
                let rs = mysql::to_sql(self);
                let mut sql: String = String::new();
                if !rs.method.0.is_empty() {
                    sql.push_str(rs.method.0.as_str());
                }
                if !rs.join.0.is_empty() {
                    sql.push(' ');
                    sql.push_str(rs.join.0.as_str());
                }
                if !rs.statement.0.is_empty() {
                    sql.push(' ');
                    sql.push_str(rs.statement.0.as_str());
                }
                let mut rs_binds: Vec<serde_json::Value> = vec![];
                rs_binds.extend(rs.method.1);
                rs_binds.extend(rs.join.1);
                rs_binds.extend(rs.statement.1);
                (sql, rs_binds)
            }
            #[cfg(feature = "postgres")]
            Client::Postgres => {
                panic!("not support client");
            }
            _ => {
                panic!("not support client");
            }
        }
    }

    #[cfg(all(feature = "mysql", feature = "sqlx_mysql"))]
    pub fn to_sqlx_query<'a>(
        &'a self,
        sql: &'a str,
        binds: Vec<serde_json::Value>,
    ) -> sqlx::query::Query<'_, sqlx::MySql, sqlx::mysql::MySqlArguments> {
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
                }
                _ => {}
            }
        }
        qb
    }

    #[cfg(all(feature = "mysql", feature = "sqlx_mysql"))]
    pub fn to_sqlx_query_as<'a, T>(
        &'a self,
        sql: &'a str,
        binds: Vec<serde_json::Value>,
    ) -> sqlx::query::QueryAs<'_, sqlx::MySql, T, sqlx::mysql::MySqlArguments>
    where
        T: for<'r> sqlx::FromRow<'r, sqlx::mysql::MySqlRow>,
    {
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
                }
                _ => {}
            }
        }
        qb
    }

    pub fn query(&mut self, mut query: impl FnMut(&mut QueryBuilder)) {
        query(&mut self.query);
    }

    pub fn add_raw(&mut self, sql: &str, val: Option<Vec<serde_json::Value>>) {
        self.query.raw.push((sql.to_string(), val));
    }
}
