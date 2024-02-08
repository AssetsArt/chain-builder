use super::{JoinBuilder, JoinStatement};
use crate::QueryBuilder;

pub trait JoinMethods {
    fn join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder));
    fn inner_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder));
    fn left_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder));
    fn right_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder));
    fn left_outer_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder));
    fn right_outer_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder));
    fn full_outer_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder));
    fn cross_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder));
    fn raw_join(&mut self, raw: &str, val: Option<Vec<serde_json::Value>>);
}

impl JoinMethods for QueryBuilder {
    fn join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder)) {
        let mut join = JoinBuilder {
            table: table.to_string(),
            statement: vec![],
            join_type: "JOIN".into(),
            raw: None,
            as_name: None,
        };
        on(&mut join);
        self.join.push(join);
    }

    fn inner_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder)) {
        let mut join = JoinBuilder {
            table: table.to_string(),
            statement: vec![],
            join_type: "INNER JOIN".into(),
            raw: None,
            as_name: None,
        };
        on(&mut join);
        self.join.push(join);
    }

    fn left_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder)) {
        let mut join = JoinBuilder {
            table: table.to_string(),
            statement: vec![],
            join_type: "LEFT JOIN".into(),
            raw: None,
            as_name: None,
        };
        on(&mut join);
        self.join.push(join);
    }

    fn right_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder)) {
        let mut join = JoinBuilder {
            table: table.to_string(),
            statement: vec![],
            join_type: "RIGHT JOIN".into(),
            raw: None,
            as_name: None,
        };
        on(&mut join);
        self.join.push(join);
    }

    fn left_outer_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder)) {
        let mut join = JoinBuilder {
            table: table.to_string(),
            statement: vec![],
            join_type: "LEFT OUTER JOIN".into(),
            raw: None,
            as_name: None,
        };
        on(&mut join);
        self.join.push(join);
    }

    fn right_outer_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder)) {
        let mut join = JoinBuilder {
            table: table.to_string(),
            statement: vec![],
            join_type: "RIGHT OUTER JOIN".into(),
            raw: None,
            as_name: None,
        };
        on(&mut join);
        self.join.push(join);
    }

    fn full_outer_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder)) {
        let mut join = JoinBuilder {
            table: table.to_string(),
            statement: vec![],
            join_type: "FULL OUTER JOIN".into(),
            raw: None,
            as_name: None,
        };
        on(&mut join);
        self.join.push(join);
    }

    fn cross_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder)) {
        let mut join = JoinBuilder {
            table: table.to_string(),
            statement: vec![],
            join_type: "CROSS JOIN".into(),
            raw: None,
            as_name: None,
        };
        on(&mut join);
        self.join.push(join);
    }

    fn raw_join(&mut self, raw: &str, val: Option<Vec<serde_json::Value>>) {
        self.join.push(JoinBuilder {
            table: raw.to_string(),
            statement: vec![],
            join_type: "".into(),
            raw: Some((raw.to_string(), val)),
            as_name: None,
        });
    }
}

impl JoinBuilder {
    pub fn as_name(&mut self, name: &str) -> &mut Self {
        self.as_name = Some(name.to_string());
        self
    }

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

    pub fn on_raw(&mut self, raw: &str, val: Option<Vec<serde_json::Value>>) -> &mut Self {
        self.statement
            .push(JoinStatement::OnRaw(raw.to_string(), val));
        self
    }
}
