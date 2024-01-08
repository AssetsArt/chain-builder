// mod mysql;
mod join_methods;
#[cfg(feature = "mysql")]
mod mysql;
mod operator;
mod where_clauses;

// export
pub use join_methods::{JoinBuilderMethods, JoinMethods};
pub use operator::Operator;
pub use where_clauses::WhereClauses;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Client {
    Mysql,
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
    None,
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
    raw: Option<(String, Option<Vec<serde_json::Value>>)>,
    join: Vec<JoinBuilder>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum JoinStatement {
    On(String, String, String),
    OrChain(Box<JoinBuilder>),
}

impl JoinStatement {
    pub fn to_join_builder(&mut self) -> &mut JoinBuilder {
        match self {
            JoinStatement::OrChain(query) => query,
            _ => panic!("JoinStatement::to_join_builder()"),
        }
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct JoinBuilder {
    table: String,
    join_type: String,
    statement: Vec<JoinStatement>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Select {
    Columns(Vec<String>),
    Raw((String, Option<Vec<serde_json::Value>>)),
    Builder(Box<ChainBuilder>),
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
            method: Method::None,
        }
    }

    pub fn db(&mut self, db: &str) -> &mut ChainBuilder {
        self.db = Some(db.to_string());
        self
    }

    pub fn from(&mut self, table: &str) -> &mut ChainBuilder {
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

    fn delegate_to_sql(&self, is_statement: bool) -> (String, Option<Vec<serde_json::Value>>) {
        match self.client {
            Client::Mysql => mysql::to_sql(self, is_statement),
        }
    }

    pub fn to_sql(&self) -> (String, Option<Vec<serde_json::Value>>) {
        self.delegate_to_sql(false)
    }

    pub fn query(&mut self, mut query: impl FnMut(&mut QueryBuilder)) {
        query(&mut self.query);
    }

    pub fn add_raw(&mut self, raw: (String, Option<Vec<serde_json::Value>>)) {
        self.query.raw = Some(raw);
    }
}

#[cfg(test)]
mod tests {
    use super::{ChainBuilder, Client, JoinBuilderMethods, JoinMethods, Select, WhereClauses};

    #[test]
    fn test_chain_builder() {
        let mut builder = ChainBuilder::new(Client::Mysql);
        builder.db("mydb"); // For dynamic db
        builder.select(Select::Columns(vec!["*".into()]));
        builder.from("users");
        builder.query(|qb| {
            qb.where_eq("name", serde_json::Value::String("John".to_string()));
            qb.where_eq("city", serde_json::Value::String("New York".to_string()));
            qb.where_in(
                "department",
                vec![
                    serde_json::Value::String("IT".to_string()),
                    serde_json::Value::String("HR".to_string()),
                ],
            );

            qb.where_subquery(|sub| {
                sub.where_eq("status", serde_json::Value::String("active".to_string()));
                sub.or()
                    .where_eq("status", serde_json::Value::String("pending".to_string()))
                    .where_between(
                        "registered_at",
                        [
                            serde_json::Value::String("2024-01-01".to_string()),
                            serde_json::Value::String("2024-01-31".to_string()),
                        ],
                    );
            });

            qb.where_raw((
                "(latitude BETWEEN ? AND ?) AND (longitude BETWEEN ? AND ?)".into(),
                Some(vec![
                    serde_json::Value::Number(serde_json::Number::from_f64(40.0).unwrap()),
                    serde_json::Value::Number(serde_json::Number::from_f64(41.0).unwrap()),
                    serde_json::Value::Number(serde_json::Number::from_f64(70.0).unwrap()),
                    serde_json::Value::Number(serde_json::Number::from_f64(71.0).unwrap()),
                ]),
            ));
        });

        let sql = builder.to_sql();
        // println!("final sql: {}", sql.0);
        // println!("final binds: {:?}", sql.1);
        assert_eq!(
            sql.0,
            "SELECT * FROM mydb.users WHERE name = ? AND city = ? AND department IN (?,?) AND (status = ? OR (status = ? AND registered_at BETWEEN ? AND ?)) AND (latitude BETWEEN ? AND ?) AND (longitude BETWEEN ? AND ?)"
        );
        assert_eq!(
            sql.1,
            Some(vec![
                serde_json::Value::String("John".to_string()),
                serde_json::Value::String("New York".to_string()),
                serde_json::Value::String("IT".to_string()),
                serde_json::Value::String("HR".to_string()),
                serde_json::Value::String("active".to_string()),
                serde_json::Value::String("pending".to_string()),
                serde_json::Value::String("2024-01-01".to_string()),
                serde_json::Value::String("2024-01-31".to_string()),
                serde_json::Value::Number(serde_json::Number::from_f64(40.0).unwrap()),
                serde_json::Value::Number(serde_json::Number::from_f64(41.0).unwrap()),
                serde_json::Value::Number(serde_json::Number::from_f64(70.0).unwrap()),
                serde_json::Value::Number(serde_json::Number::from_f64(71.0).unwrap()),
            ])
        );
    }

    #[test]
    fn test_join() {
        let mut builder = ChainBuilder::new(Client::Mysql);
        builder.db("mydb"); // For dynamic db
        builder.select(Select::Columns(vec!["*".into()]));
        builder.select(Select::Raw((
            "(SELECT COUNT(*) FROM mydb.users WHERE users.id = details.id) AS count".into(),
            Some(vec![]),
        )));
        builder.from("users");
        builder.query(|qb| {
            qb.join("details", |join| {
                join.on("details.id", "=", "users.d_id");
                join.on("details.id_w", "=", "users.d_id_w");
                join.or().on("details.id_s", "=", "users.d_id_s").on(
                    "details.id_w",
                    "=",
                    "users.d_id_w",
                );
            });
            qb.where_eq("name", serde_json::Value::String("John".to_string()));
        });
        let sql = builder.to_sql();
        println!("final sql: {}", sql.0);
        println!("final binds: {:?}", sql.1);
        assert_eq!(
            sql.0,
            "SELECT *, (SELECT COUNT(*) FROM mydb.users WHERE users.id = details.id) AS count FROM mydb.users JOIN mydb.details ON details.id = users.d_id AND details.id_w = users.d_id_w OR (details.id_s = users.d_id_s AND details.id_w = users.d_id_w) WHERE name = ?"
        );
    }
}
