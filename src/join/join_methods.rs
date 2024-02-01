use super::{JoinBuilder, JoinStatement};
use crate::QueryBuilder;

pub trait JoinMethods {
    fn join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder));
    fn left_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder));
}

impl JoinMethods for QueryBuilder {
    fn join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder)) {
        let mut join = JoinBuilder {
            table: table.to_string(),
            statement: vec![],
            join_type: "JOIN".into(),
        };
        on(&mut join);
        self.join.push(join);
    }

    fn left_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder)) {
        let mut join = JoinBuilder {
            table: table.to_string(),
            statement: vec![],
            join_type: "LEFT JOIN".into(),
        };
        on(&mut join);
        self.join.push(join);
    }
}

impl JoinBuilder {
    pub fn on(&mut self, column: &str, operator: &str, column2: &str) -> &mut Self {
        self.statement.push(JoinStatement::On(
            column.to_string(),
            operator.to_string(),
            column2.to_string(),
        ));
        self
    }

    pub fn or(&mut self) -> &mut JoinBuilder {
        let mut chain = self.clone();
        chain.statement = vec![];
        self.statement.push(JoinStatement::OrChain(Box::new(chain)));
        // SAFETY: unwrap() is safe because we just pushed an OrChain
        self.statement.last_mut().unwrap().as_mut_join_builder()
    }

    pub fn on_val(&mut self, column: &str, operator: &str, value: serde_json::Value) -> &mut Self {
        self.statement.push(JoinStatement::OnVal(
            column.to_string(),
            operator.to_string(),
            value,
        ));
        self
    }
}
