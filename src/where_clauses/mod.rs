use crate::{operator::Operator, ChainBuilder, Method, Statement};

pub trait WhereClauses {
    fn where_clause(
        &mut self,
        column: &str,
        operator: Operator,
        value: serde_json::Value,
    ) -> &mut Self;
    fn where_eq(&mut self, column: &str, value: serde_json::Value) -> &mut Self;
    fn where_not_eq(&mut self, column: &str, value: serde_json::Value) -> &mut Self;
    fn where_in(&mut self, column: &str, value: Vec<serde_json::Value>) -> &mut Self;
    fn where_not_in(&mut self, column: &str, value: Vec<serde_json::Value>) -> &mut Self;
    fn where_null(&mut self, column: &str) -> &mut Self;
    fn where_not_null(&mut self, column: &str) -> &mut Self;
    fn where_exists(&mut self, column: &str) -> &mut Self;
    fn where_not_exists(&mut self, column: &str) -> &mut Self;
    fn where_between(&mut self, column: &str, value: [serde_json::Value; 2]) -> &mut Self;
    fn where_not_between(&mut self, column: &str, value: [serde_json::Value; 2]) -> &mut Self;
    fn where_like(&mut self, column: &str, value: serde_json::Value) -> &mut Self;
    fn where_not_like(&mut self, column: &str, value: serde_json::Value) -> &mut Self;
    fn where_subquery(
        &mut self,
        value: impl FnMut(&mut ChainBuilder) -> &mut ChainBuilder,
    ) -> &mut Self;
    fn or(&mut self) -> &mut Self;
    fn where_raw(&mut self, raw: (String, Option<Vec<serde_json::Value>>)) -> &mut Self;
}

impl WhereClauses for ChainBuilder {
    fn where_clause(
        &mut self,
        column: &str,
        operator: Operator,
        value: serde_json::Value,
    ) -> &mut Self {
        self.statement
            .push(Statement::Value(column.to_string(), operator, value));
        self
    }

    fn where_subquery(
        &mut self,
        mut value: impl FnMut(&mut ChainBuilder) -> &mut ChainBuilder,
    ) -> &mut Self {
        let mut chain = ChainBuilder::new(self.client.clone());
        value(&mut chain);
        self.statement.push(Statement::SubChain(Box::new(chain)));
        self
    }

    fn or(&mut self) -> &mut Self {
        self.statement
            .push(Statement::OrChain(Box::new(ChainBuilder {
                client: self.client.clone(),
                table: self.table.clone(),
                select: self.select.clone(),
                as_name: self.as_name.clone(),
                statement: vec![],
                raw: None,
                method: Method::None,
                db: self.db.clone(),
            })));
        self
    }

    fn where_raw(&mut self, raw: (String, Option<Vec<serde_json::Value>>)) -> &mut Self {
        self.statement.push(Statement::Raw(raw));
        self
    }

    fn where_eq(&mut self, column: &str, value: serde_json::Value) -> &mut Self {
        self.where_clause(column, Operator::Equal, value)
    }

    fn where_not_eq(&mut self, column: &str, value: serde_json::Value) -> &mut Self {
        self.where_clause(column, Operator::NotEqual, value)
    }

    fn where_in(&mut self, column: &str, value: Vec<serde_json::Value>) -> &mut Self {
        self.where_clause(column, Operator::In, serde_json::Value::Array(value))
    }

    fn where_not_in(&mut self, column: &str, value: Vec<serde_json::Value>) -> &mut Self {
        self.where_clause(column, Operator::NotIn, serde_json::Value::Array(value))
    }

    fn where_null(&mut self, column: &str) -> &mut Self {
        self.where_clause(column, Operator::IsNull, serde_json::Value::Null)
    }

    fn where_not_null(&mut self, column: &str) -> &mut Self {
        self.where_clause(column, Operator::IsNotNull, serde_json::Value::Null)
    }

    fn where_exists(&mut self, column: &str) -> &mut Self {
        self.where_clause(column, Operator::Exists, serde_json::Value::Null)
    }

    fn where_not_exists(&mut self, column: &str) -> &mut Self {
        self.where_clause(column, Operator::NotExists, serde_json::Value::Null)
    }

    fn where_between(&mut self, column: &str, value: [serde_json::Value; 2]) -> &mut Self {
        self.where_clause(
            column,
            Operator::Between,
            serde_json::Value::Array(vec![value[0].clone(), value[1].clone()]),
        )
    }

    fn where_not_between(&mut self, column: &str, value: [serde_json::Value; 2]) -> &mut Self {
        self.where_clause(
            column,
            Operator::NotBetween,
            serde_json::Value::Array(vec![value[0].clone(), value[1].clone()]),
        )
    }

    fn where_like(&mut self, column: &str, value: serde_json::Value) -> &mut Self {
        self.where_clause(column, Operator::Like, value)
    }

    fn where_not_like(&mut self, column: &str, value: serde_json::Value) -> &mut Self {
        self.where_clause(column, Operator::NotLike, value)
    }
}
