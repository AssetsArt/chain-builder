#[cfg(feature = "mysql")]
mod mysql;
#[cfg(all(feature = "mysql", feature = "sqlx_mysql"))]
mod sqlx_mysql;

// mods
mod join;
mod operator;
mod query_builder;

// use
use serde_json::Value;

// export
pub use join::{JoinBuilder, JoinMethods};
pub use operator::Operator;
pub use query_builder::{QueryCommon, WhereClauses};

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
    InsertMany,
    Update,
    Delete,
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
    insert_update: Value,
    sql_str: String,
    is_distinct: bool,
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueryBuilder {
    statement: Vec<Statement>,
    raw: Vec<(String, Option<Vec<serde_json::Value>>)>,
    join: Vec<JoinBuilder>,
    query_common: Vec<Common>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Select {
    Columns(Vec<String>),
    Raw(String, Option<Vec<serde_json::Value>>),
    // column name, builder
    Builder(String, ChainBuilder),
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Common {
    With(String, bool, ChainBuilder),
    Union(bool, ChainBuilder),
    Limit(usize),
    Offset(usize),
    GroupBy(Vec<String>),
    GroupByRaw(String, Option<Vec<serde_json::Value>>),
    OrderBy(String, String),
    OrderByRaw(String, Option<Vec<serde_json::Value>>),
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
            insert_update: Value::Null,
            sql_str: String::new(),
            is_distinct: false,
        }
    }

    #[cfg(feature = "mysql")]
    pub fn new_mysql() -> ChainBuilder {
        ChainBuilder::new(Client::Mysql)
    }

    pub fn db(&mut self, db: &str) -> &mut ChainBuilder {
        self.db = Some(db.to_string());
        self
    }

    pub fn table(&mut self, table: &str) -> &mut ChainBuilder {
        self.table = table.to_string();
        self
    }

    pub fn distinct(&mut self) -> &mut ChainBuilder {
        self.is_distinct = true;
        self
    }

    pub fn select(&mut self, select: Select) -> &mut ChainBuilder {
        self.method = Method::Select;
        self.select.push(select);
        self
    }

    pub fn insert(&mut self, data: Value) -> &mut ChainBuilder {
        self.method = Method::Insert;
        self.insert_update = data;
        self
    }

    pub fn insert_many(&mut self, data: Vec<Value>) -> &mut ChainBuilder {
        self.method = Method::InsertMany;
        self.insert_update = Value::Array(data);
        self
    }

    pub fn update(&mut self, data: Value) -> &mut ChainBuilder {
        self.method = Method::Update;
        self.insert_update = data;
        self
    }

    pub fn delete(&mut self) -> &mut ChainBuilder {
        self.method = Method::Delete;
        self
    }

    pub fn as_name(&mut self, name: &str) -> &mut ChainBuilder {
        self.as_name = Some(name.to_string());
        self
    }

    pub fn with(&mut self, alias: &str, chain_builder: ChainBuilder) -> &mut ChainBuilder {
        self.query
            .query_common
            .push(Common::With(alias.to_string(), false, chain_builder));
        self
    }

    pub fn with_recursive(
        &mut self,
        alias: &str,
        chain_builder: ChainBuilder,
    ) -> &mut ChainBuilder {
        self.query
            .query_common
            .push(Common::With(alias.to_string(), true, chain_builder));
        self
    }

    pub fn union(&mut self, chain_builder: ChainBuilder) -> &mut ChainBuilder {
        self.query
            .query_common
            .push(Common::Union(false, chain_builder));
        self
    }

    pub fn union_all(&mut self, chain_builder: ChainBuilder) -> &mut ChainBuilder {
        self.query
            .query_common
            .push(Common::Union(true, chain_builder));
        self
    }

    pub fn query(&mut self, query: impl FnOnce(&mut QueryBuilder)) {
        query(&mut self.query);
    }

    pub fn add_raw(&mut self, sql: &str, val: Option<Vec<serde_json::Value>>) {
        self.query.raw.push((sql.to_string(), val));
    }

    pub fn to_sql(&mut self) -> (String, Vec<serde_json::Value>) {
        match self.client {
            #[cfg(feature = "mysql")]
            Client::Mysql => {
                let rs = mysql::merge_to_sql(mysql::to_sql(self));
                self.sql_str = rs.0.clone();
                (self.sql_str.clone(), rs.1)
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
}
