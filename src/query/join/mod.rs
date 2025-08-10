//! JOIN functionality for query building

mod join_methods;

use serde_json::Value;

// Re-export join methods
pub use join_methods::*;

/// JOIN statement types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum JoinStatement {
    /// Simple ON condition: left_column, operator, right_column
    On(String, String, String),
    /// OR chain for complex JOIN conditions
    OrChain(Box<JoinBuilder>),
    /// ON condition with value: column, operator, value
    OnVal(String, String, Value),
    /// Raw ON condition with optional bind parameters
    OnRaw(String, Option<Vec<Value>>),
}

impl JoinStatement {
    /// Convert statement to a mutable join builder reference
    pub fn as_mut_join_builder(&mut self) -> &mut JoinBuilder {
        match self {
            JoinStatement::OrChain(query) => query,
            _ => panic!("JoinStatement::as_mut_join_builder() called on non-chain statement"),
        }
    }
}

/// JOIN builder for constructing JOIN clauses
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct JoinBuilder {
    /// Table name to join
    pub(crate) table: String,
    /// JOIN type (JOIN, LEFT JOIN, RIGHT JOIN, etc.)
    pub(crate) join_type: String,
    /// JOIN conditions
    pub(crate) statement: Vec<JoinStatement>,
    /// Raw JOIN SQL with optional bind parameters
    pub(crate) raw: Option<(String, Option<Vec<Value>>)>,
    /// Table alias
    pub(crate) as_name: Option<String>,
}
