// mod
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

    pub fn to_sql(&self, is_statement: bool) -> (String, Option<Vec<serde_json::Value>>) {
        match self.client {
            Client::Mysql => mysql::to_sql(self, is_statement),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_builder() {
        let mut builder = ChainBuilder::new(Client::Mysql);
        builder.db("test"); // For dynamic db
        builder.select(Select::Columns(vec!["*".into()]));
        builder.from("users");
        builder.where_eq("name", serde_json::Value::String("test".to_string()));
        builder.where_eq("age", serde_json::Value::Number(20.into()));
        builder.where_in(
            "id",
            vec![
                serde_json::Value::Number(1.into()),
                serde_json::Value::Number(2.into()),
                serde_json::Value::Number(3.into()),
            ],
        );

        builder.where_subquery(|sub| {
            sub.where_eq("name", serde_json::Value::String("test".to_string()));
            sub.or()
                .where_eq("age", serde_json::Value::Number(20.into()))
                .where_exists("id");
            sub
        });

        builder.where_raw((
            "name = ? AND age = ?".to_string(),
            Some(vec![
                serde_json::Value::String("test".to_string()),
                serde_json::Value::Number(20.into()),
            ]),
        ));

        let sql = builder.to_sql(false);
        println!("final sql: {}", sql.0);
        println!("final binds: {:?}", sql.1);
    }
}
