use crate::{operator::Operator, QueryBuilder, Statement};

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
    fn where_subquery(&mut self, value: impl FnOnce(&mut QueryBuilder));
    fn or(&mut self) -> &mut QueryBuilder;
    fn where_raw(&mut self, sql: &str, val: Option<Vec<serde_json::Value>>) -> &mut Self;
    fn where_gt(&mut self, column: &str, value: serde_json::Value) -> &mut Self;
    fn where_gte(&mut self, column: &str, value: serde_json::Value) -> &mut Self;
    fn where_lt(&mut self, column: &str, value: serde_json::Value) -> &mut Self;
    fn where_lte(&mut self, column: &str, value: serde_json::Value) -> &mut Self;
    fn where_gtlt(&mut self, column: &str, value: serde_json::Value) -> &mut Self;
}

impl WhereClauses for QueryBuilder {
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

    fn where_subquery(&mut self, value: impl FnOnce(&mut QueryBuilder)) {
        let mut query = self.clone();
        query.statement = vec![];
        query.raw = vec![];
        value(&mut query);
        self.statement.push(Statement::SubChain(Box::new(query)));
    }

    fn or(&mut self) -> &mut QueryBuilder {
        let mut chain = self.clone();
        chain.statement = vec![];
        chain.raw = vec![];
        self.statement.push(Statement::OrChain(Box::new(chain)));
        // SAFETY: unwrap() is safe because we just pushed an OrChain
        self.statement.last_mut().unwrap().to_query_builder()
    }

    fn where_raw(&mut self, sql: &str, val: Option<Vec<serde_json::Value>>) -> &mut Self {
        self.statement.push(Statement::Raw((sql.to_string(), val)));
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

    fn where_gt(&mut self, column: &str, value: serde_json::Value) -> &mut Self {
        self.where_clause(column, Operator::GreaterThan, value)
    }

    fn where_gte(&mut self, column: &str, value: serde_json::Value) -> &mut Self {
        self.where_clause(column, Operator::GreaterThanOrEqual, value)
    }

    fn where_lt(&mut self, column: &str, value: serde_json::Value) -> &mut Self {
        self.where_clause(column, Operator::LessThan, value)
    }

    fn where_lte(&mut self, column: &str, value: serde_json::Value) -> &mut Self {
        self.where_clause(column, Operator::LessThanOrEqual, value)
    }

    fn where_gtlt(&mut self, column: &str, value: serde_json::Value) -> &mut Self {
        self.where_clause(column, Operator::GreaterORLessThan, value)
    }
}
