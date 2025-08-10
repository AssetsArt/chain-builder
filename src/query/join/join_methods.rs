//! JOIN methods for building JOIN clauses

use super::{JoinBuilder, JoinStatement};
use crate::query::QueryBuilder;
use serde_json::Value;

/// Trait for JOIN operations
pub trait JoinMethods {
    /// Add a JOIN clause
    fn join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder));

    /// Add an INNER JOIN clause
    fn inner_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder));

    /// Add a LEFT JOIN clause
    fn left_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder));

    /// Add a RIGHT JOIN clause
    fn right_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder));

    /// Add a LEFT OUTER JOIN clause
    fn left_outer_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder));

    /// Add a RIGHT OUTER JOIN clause
    fn right_outer_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder));

    /// Add a FULL OUTER JOIN clause
    fn full_outer_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder));

    /// Add a CROSS JOIN clause
    fn cross_join(&mut self, table: &str, on: impl FnOnce(&mut JoinBuilder));

    /// Add a JOIN USING clause
    fn join_using(&mut self, table: &str, columns: Vec<String>);

    /// Add a raw JOIN clause
    fn raw_join(&mut self, raw: &str, val: Option<Vec<Value>>);
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

    fn join_using(&mut self, table: &str, columns: Vec<String>) {
        let columns_str = columns.join(", ");
        let sql = format!("JOIN {} USING ({})", table, columns_str);
        self.raw_join(&sql, None);
    }

    fn raw_join(&mut self, raw: &str, val: Option<Vec<Value>>) {
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
    /// Set table alias
    pub fn as_name(&mut self, name: &str) -> &mut Self {
        self.as_name = Some(name.to_string());
        self
    }

    /// Add ON condition with column comparison
    pub fn on(&mut self, column: &str, operator: &str, column2: &str) -> &mut Self {
        self.statement.push(JoinStatement::On(
            column.to_string(),
            operator.to_string(),
            column2.to_string(),
        ));
        self
    }

    /// Start an OR chain for complex JOIN conditions
    pub fn or(&mut self) -> &mut JoinBuilder {
        let mut chain = self.clone();
        chain.statement = vec![];
        self.statement.push(JoinStatement::OrChain(Box::new(chain)));
        // SAFETY: unwrap() is safe because we just pushed an OrChain
        self.statement.last_mut().unwrap().as_mut_join_builder()
    }

    /// Add ON condition with value comparison
    pub fn on_val(&mut self, column: &str, operator: &str, value: Value) -> &mut Self {
        self.statement.push(JoinStatement::OnVal(
            column.to_string(),
            operator.to_string(),
            value,
        ));
        self
    }

    /// Add raw ON condition
    pub fn on_raw(&mut self, raw: &str, val: Option<Vec<Value>>) -> &mut Self {
        self.statement
            .push(JoinStatement::OnRaw(raw.to_string(), val));
        self
    }
}
