// mod
#[cfg(feature = "mysql")]
mod mysql;

mod operator;
mod where_clauses;

// export
pub use operator::Operator;
pub use where_clauses::WhereClauses;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Client {
    Mysql,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Statement {
    Value(String, Operator, serde_json::Value),
    AndChain(Box<ChainBuilder>),
    SubChain(Box<ChainBuilder>),
    OrChain(Box<ChainBuilder>),
    Raw((String, Option<Vec<serde_json::Value>>)),
}

impl Statement {
    pub fn to_chain_builder(&mut self) -> &mut ChainBuilder {
        match self {
            Statement::AndChain(chain) => chain,
            Statement::OrChain(chain) => chain,
            Statement::SubChain(chain) => chain,
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
    select: Option<Select>,
    statement: Vec<Statement>,
    raw: Option<(String, Option<Vec<serde_json::Value>>)>,
    method: Method,
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
            select: None,
            as_name: None,
            statement: vec![],
            raw: None,
            method: Method::None,
            db: None,
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
        self.select = Some(select);
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
}

#[cfg(test)]
mod tests {
    use super::{ChainBuilder, Client, Select, WhereClauses};

    #[test]
    fn test_chain_builder() {
        let mut builder = ChainBuilder::new(Client::Mysql);
        builder.db("mydb"); // For dynamic db
        builder.select(Select::Columns(vec!["*".into()]));
        builder.from("users");
        builder.where_eq("name", serde_json::Value::String("John".to_string()));
        builder.where_eq("city", serde_json::Value::String("New York".to_string()));
        builder.where_in(
            "department",
            vec![
                serde_json::Value::String("IT".to_string()),
                serde_json::Value::String("HR".to_string()),
            ],
        );

        builder.where_subquery(|sub| {
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
            sub
        });

        builder.where_raw((
            "(latitude BETWEEN ? AND ?) AND (longitude BETWEEN ? AND ?)".into(),
            Some(vec![
                serde_json::Value::Number(serde_json::Number::from_f64(40.0).unwrap()),
                serde_json::Value::Number(serde_json::Number::from_f64(41.0).unwrap()),
                serde_json::Value::Number(serde_json::Number::from_f64(70.0).unwrap()),
                serde_json::Value::Number(serde_json::Number::from_f64(71.0).unwrap()),
            ]),
        ));

        let sql = builder.to_sql();
        println!("final sql: {}", sql.0);
        println!("final binds: {:?}", sql.1);
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
}
