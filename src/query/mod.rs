//! Query building functionality

pub mod common;
pub mod join;

use crate::types::{Client, Common, Statement};
use serde_json::Value;

/// Main query builder for constructing WHERE clauses and other query parts
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueryBuilder {
    /// WHERE clause statements
    pub(crate) statement: Vec<Statement>,
    /// Raw SQL statements
    pub(crate) raw: Vec<(String, Option<Vec<Value>>)>,
    /// JOIN clauses
    pub(crate) join: Vec<join::JoinBuilder>,
    /// Common clauses (WITH, UNION, LIMIT, etc.)
    pub(crate) query_common: Vec<Common>,
    /// Database client type used for nested builders
    pub(crate) client: Client,
}

impl QueryBuilder {
    /// Create a new QueryBuilder for the specified client
    pub fn new(client: Client) -> Self {
        Self {
            statement: Vec::new(),
            raw: Vec::new(),
            join: Vec::new(),
            query_common: Vec::new(),
            client,
        }
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new(Client::Mysql)
    }
}

/// SQL comparison operators
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum Operator {
    /// Equal (=)
    Equal,
    /// Not equal (!=)
    NotEqual,
    /// IN operator
    In,
    /// NOT IN operator
    NotIn,
    /// IS NULL
    IsNull,
    /// IS NOT NULL
    IsNotNull,
    /// EXISTS
    Exists,
    /// NOT EXISTS
    NotExists,
    /// BETWEEN
    Between,
    /// NOT BETWEEN
    NotBetween,
    /// LIKE
    Like,
    /// NOT LIKE
    NotLike,
    /// Greater than (>)
    GreaterThan,
    /// Greater than or equal (>=)
    GreaterThanOrEqual,
    /// Less than (<)
    LessThan,
    /// Less than or equal (<=)
    LessThanOrEqual,
    /// Greater or less than (<>)
    GreaterORLessThan,
}
