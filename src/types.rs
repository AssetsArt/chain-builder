//! Core types and enums for the Chain Builder library

use crate::query::QueryBuilder;
use serde_json::Value;

/// Supported database clients
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Client {
    /// MySQL database
    Mysql,
    /// PostgreSQL database (not yet implemented)
    Postgres,
    /// SQLite database
    Sqlite,
}

/// SQL statement types for WHERE clauses
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Statement {
    /// Simple value comparison: column, operator, value
    Value(String, crate::query::Operator, Value),
    /// Subquery with AND logic
    SubChain(Box<QueryBuilder>),
    /// Subquery with OR logic
    OrChain(Box<QueryBuilder>),
    /// Raw SQL statement with optional bind parameters
    Raw((String, Option<Vec<Value>>)),
}

impl Statement {
    /// Convert statement to a mutable query builder reference
    pub fn to_query_builder(&mut self) -> &mut QueryBuilder {
        match self {
            Statement::OrChain(query) => query,
            Statement::SubChain(query) => query,
            _ => panic!("Statement::to_query_builder() called on non-chain statement"),
        }
    }
}

/// SQL operation methods
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum Method {
    /// SELECT operation
    Select,
    /// INSERT operation
    Insert,
    /// INSERT multiple rows
    InsertMany,
    /// UPDATE operation
    Update,
    /// DELETE operation
    Delete,
}

/// SELECT clause types
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Select {
    /// Column names to select
    Columns(Vec<String>),
    /// Raw SQL with optional bind parameters
    Raw(String, Option<Vec<Value>>),
    /// Subquery as a column
    Builder(String, crate::builder::ChainBuilder),
}

/// Common SQL clauses (WITH, UNION, LIMIT, etc.)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Common {
    /// WITH clause (CTE)
    With(String, bool, crate::builder::ChainBuilder),
    /// UNION clause
    Union(bool, crate::builder::ChainBuilder),
    /// LIMIT clause
    Limit(usize),
    /// OFFSET clause
    Offset(usize),
    /// GROUP BY clause
    GroupBy(Vec<String>),
    /// Raw GROUP BY clause
    GroupByRaw(String, Option<Vec<Value>>),
    /// HAVING clause
    Having(String, Option<Vec<Value>>),
    /// ORDER BY clause
    OrderBy(String, String),
    /// Raw ORDER BY clause
    OrderByRaw(String, Option<Vec<Value>>),
}
