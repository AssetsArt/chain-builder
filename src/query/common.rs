//! Common query functionality for WHERE clauses and other query parts

use serde_json::Value;
use crate::types::Common;
use crate::query::QueryBuilder;

/// Trait for common query operations
pub trait QueryCommon {
    /// Add a WITH clause
    fn with(&mut self, alias: &str, chain_builder: crate::builder::ChainBuilder);
    
    /// Add a recursive WITH clause
    fn with_recursive(&mut self, alias: &str, chain_builder: crate::builder::ChainBuilder);
    
    /// Add a UNION clause
    fn union(&mut self, chain_builder: crate::builder::ChainBuilder);
    
    /// Add a UNION ALL clause
    fn union_all(&mut self, chain_builder: crate::builder::ChainBuilder);
    
    /// Add a LIMIT clause
    fn limit(&mut self, limit: usize);
    
    /// Add an OFFSET clause
    fn offset(&mut self, offset: usize);
    
    /// Add a GROUP BY clause
    fn group_by(&mut self, columns: Vec<String>);
    
    /// Add a raw GROUP BY clause
    fn group_by_raw(&mut self, sql: &str, binds: Option<Vec<Value>>);
    
    /// Add an ORDER BY clause
    fn order_by(&mut self, column: &str, order: &str);
    
    /// Add a raw ORDER BY clause
    fn order_by_raw(&mut self, sql: &str, binds: Option<Vec<Value>>);
}

impl QueryCommon for QueryBuilder {
    fn with(&mut self, alias: &str, chain_builder: crate::builder::ChainBuilder) {
        self.query_common.push(Common::With(alias.to_string(), false, chain_builder));
    }
    
    fn with_recursive(&mut self, alias: &str, chain_builder: crate::builder::ChainBuilder) {
        self.query_common.push(Common::With(alias.to_string(), true, chain_builder));
    }
    
    fn union(&mut self, chain_builder: crate::builder::ChainBuilder) {
        self.query_common.push(Common::Union(false, chain_builder));
    }
    
    fn union_all(&mut self, chain_builder: crate::builder::ChainBuilder) {
        self.query_common.push(Common::Union(true, chain_builder));
    }
    
    fn limit(&mut self, limit: usize) {
        self.query_common.push(Common::Limit(limit));
    }
    
    fn offset(&mut self, offset: usize) {
        self.query_common.push(Common::Offset(offset));
    }
    
    fn group_by(&mut self, columns: Vec<String>) {
        self.query_common.push(Common::GroupBy(columns));
    }
    
    fn group_by_raw(&mut self, sql: &str, binds: Option<Vec<Value>>) {
        self.query_common.push(Common::GroupByRaw(sql.to_string(), binds));
    }
    
    fn order_by(&mut self, column: &str, order: &str) {
        self.query_common.push(Common::OrderBy(column.to_string(), order.to_string()));
    }
    
    fn order_by_raw(&mut self, sql: &str, binds: Option<Vec<Value>>) {
        self.query_common.push(Common::OrderByRaw(sql.to_string(), binds));
    }
}

/// Trait for HAVING clause operations
pub trait HavingClauses {
    /// Add a HAVING condition
    fn having(&mut self, column: &str, operator: &str, value: Value);
    
    /// Add a HAVING condition with raw SQL
    fn having_raw(&mut self, sql: &str, binds: Option<Vec<Value>>);
    
    /// Add a HAVING BETWEEN condition
    fn having_between(&mut self, column: &str, values: [Value; 2]);
    
    /// Add a HAVING IN condition
    fn having_in(&mut self, column: &str, values: Vec<Value>);
    
    /// Add a HAVING NOT IN condition
    fn having_not_in(&mut self, column: &str, values: Vec<Value>);
}

impl HavingClauses for QueryBuilder {
    fn having(&mut self, column: &str, operator: &str, value: Value) {
        let sql = format!("{} {} ?", column, operator);
        self.query_common.push(Common::Having(sql, Some(vec![value])));
    }
    
    fn having_raw(&mut self, sql: &str, binds: Option<Vec<Value>>) {
        self.query_common.push(Common::Having(sql.to_string(), binds));
    }
    
    fn having_between(&mut self, column: &str, values: [Value; 2]) {
        let sql = format!("{} BETWEEN ? AND ?", column);
        self.query_common.push(Common::Having(sql, Some(values.to_vec())));
    }
    
    fn having_in(&mut self, column: &str, values: Vec<Value>) {
        let placeholders = vec!["?"; values.len()].join(", ");
        let sql = format!("{} IN ({})", column, placeholders);
        self.query_common.push(Common::Having(sql, Some(values)));
    }
    
    fn having_not_in(&mut self, column: &str, values: Vec<Value>) {
        let placeholders = vec!["?"; values.len()].join(", ");
        let sql = format!("{} NOT IN ({})", column, placeholders);
        self.query_common.push(Common::Having(sql, Some(values)));
    }
}

/// Trait for WHERE clause operations
pub trait WhereClauses {
    /// Add an equality condition
    fn where_eq(&mut self, column: &str, value: Value);
    
    /// Add a not equality condition
    fn where_ne(&mut self, column: &str, value: Value);
    
    /// Add an IN condition
    fn where_in(&mut self, column: &str, values: Vec<Value>);
    
    /// Add a NOT IN condition
    fn where_not_in(&mut self, column: &str, values: Vec<Value>);
    
    /// Add an IS NULL condition
    fn where_null(&mut self, column: &str);
    
    /// Add an IS NOT NULL condition
    fn where_not_null(&mut self, column: &str);
    
    /// Add a BETWEEN condition
    fn where_between(&mut self, column: &str, values: [Value; 2]);
    
    /// Add a NOT BETWEEN condition
    fn where_not_between(&mut self, column: &str, values: [Value; 2]);
    
    /// Add a LIKE condition
    fn where_like(&mut self, column: &str, value: Value);
    
    /// Add a NOT LIKE condition
    fn where_not_like(&mut self, column: &str, value: Value);
    
    /// Add a case-insensitive LIKE condition (ILIKE for Postgres, LOWER() for MySQL)
    fn where_ilike(&mut self, column: &str, value: Value);
    
    /// Add a greater than condition
    fn where_gt(&mut self, column: &str, value: Value);
    
    /// Add a greater than or equal condition
    fn where_gte(&mut self, column: &str, value: Value);
    
    /// Add a less than condition
    fn where_lt(&mut self, column: &str, value: Value);
    
    /// Add a less than or equal condition
    fn where_lte(&mut self, column: &str, value: Value);
    
    /// Add a column-to-column comparison
    fn where_column(&mut self, lhs: &str, operator: &str, rhs: &str);
    
    /// Add an EXISTS condition
    fn where_exists(&mut self, query: impl FnOnce(&mut crate::builder::ChainBuilder));
    
    /// Add a NOT EXISTS condition
    fn where_not_exists(&mut self, query: impl FnOnce(&mut crate::builder::ChainBuilder));
    
    /// Add a JSON contains condition (MySQL JSON_CONTAINS)
    fn where_json_contains(&mut self, column: &str, value: Value);
    
    /// Add a subquery condition
    fn where_subquery(&mut self, query: impl FnOnce(&mut QueryBuilder));
    
    /// Add an OR condition
    fn or(&mut self) -> &mut QueryBuilder;
    
    /// Add a raw WHERE condition
    fn where_raw(&mut self, sql: &str, binds: Option<Vec<Value>>);
}

impl WhereClauses for QueryBuilder {
    fn where_eq(&mut self, column: &str, value: Value) {
        self.statement.push(crate::types::Statement::Value(
            column.to_string(),
            crate::query::Operator::Equal,
            value,
        ));
    }
    
    fn where_ne(&mut self, column: &str, value: Value) {
        self.statement.push(crate::types::Statement::Value(
            column.to_string(),
            crate::query::Operator::NotEqual,
            value,
        ));
    }
    
    fn where_in(&mut self, column: &str, values: Vec<Value>) {
        self.statement.push(crate::types::Statement::Value(
            column.to_string(),
            crate::query::Operator::In,
            Value::Array(values),
        ));
    }
    
    fn where_not_in(&mut self, column: &str, values: Vec<Value>) {
        self.statement.push(crate::types::Statement::Value(
            column.to_string(),
            crate::query::Operator::NotIn,
            Value::Array(values),
        ));
    }
    
    fn where_null(&mut self, column: &str) {
        self.statement.push(crate::types::Statement::Value(
            column.to_string(),
            crate::query::Operator::IsNull,
            Value::Null,
        ));
    }
    
    fn where_not_null(&mut self, column: &str) {
        self.statement.push(crate::types::Statement::Value(
            column.to_string(),
            crate::query::Operator::IsNotNull,
            Value::Null,
        ));
    }
    
    fn where_between(&mut self, column: &str, values: [Value; 2]) {
        self.statement.push(crate::types::Statement::Value(
            column.to_string(),
            crate::query::Operator::Between,
            Value::Array(values.to_vec()),
        ));
    }
    
    fn where_not_between(&mut self, column: &str, values: [Value; 2]) {
        self.statement.push(crate::types::Statement::Value(
            column.to_string(),
            crate::query::Operator::NotBetween,
            Value::Array(values.to_vec()),
        ));
    }
    
    fn where_like(&mut self, column: &str, value: Value) {
        self.statement.push(crate::types::Statement::Value(
            column.to_string(),
            crate::query::Operator::Like,
            value,
        ));
    }
    
    fn where_not_like(&mut self, column: &str, value: Value) {
        self.statement.push(crate::types::Statement::Value(
            column.to_string(),
            crate::query::Operator::NotLike,
            value,
        ));
    }
    
    fn where_gt(&mut self, column: &str, value: Value) {
        self.statement.push(crate::types::Statement::Value(
            column.to_string(),
            crate::query::Operator::GreaterThan,
            value,
        ));
    }
    
    fn where_gte(&mut self, column: &str, value: Value) {
        self.statement.push(crate::types::Statement::Value(
            column.to_string(),
            crate::query::Operator::GreaterThanOrEqual,
            value,
        ));
    }
    
    fn where_lt(&mut self, column: &str, value: Value) {
        self.statement.push(crate::types::Statement::Value(
            column.to_string(),
            crate::query::Operator::LessThan,
            value,
        ));
    }
    
    fn where_lte(&mut self, column: &str, value: Value) {
        self.statement.push(crate::types::Statement::Value(
            column.to_string(),
            crate::query::Operator::LessThanOrEqual,
            value,
        ));
    }
    
    fn where_subquery(&mut self, query: impl FnOnce(&mut QueryBuilder)) {
        let mut sub_query = QueryBuilder::default();
        query(&mut sub_query);
        self.statement.push(crate::types::Statement::SubChain(Box::new(sub_query)));
    }
    
    fn or(&mut self) -> &mut QueryBuilder {
        let or_query = QueryBuilder::default();
        self.statement.push(crate::types::Statement::OrChain(Box::new(or_query)));
        self.statement.last_mut().unwrap().to_query_builder()
    }
    
    fn where_raw(&mut self, sql: &str, binds: Option<Vec<Value>>) {
        self.statement.push(crate::types::Statement::Raw((sql.to_string(), binds)));
    }
    
    fn where_ilike(&mut self, column: &str, value: Value) {
        // For MySQL, use LOWER() function
        let sql = format!("LOWER({}) LIKE LOWER(?)", column);
        self.statement.push(crate::types::Statement::Raw((sql, Some(vec![value]))));
    }
    
    fn where_column(&mut self, lhs: &str, operator: &str, rhs: &str) {
        let sql = format!("{} {} {}", lhs, operator, rhs);
        self.statement.push(crate::types::Statement::Raw((sql, None)));
    }
    
    fn where_exists(&mut self, query: impl FnOnce(&mut crate::builder::ChainBuilder)) {
        let mut sub_builder = crate::builder::ChainBuilder::new(crate::types::Client::Mysql);
        query(&mut sub_builder);
        let sql = format!("EXISTS ({})", sub_builder.to_sql().0);
        self.statement.push(crate::types::Statement::Raw((sql, None)));
    }
    
    fn where_not_exists(&mut self, query: impl FnOnce(&mut crate::builder::ChainBuilder)) {
        let mut sub_builder = crate::builder::ChainBuilder::new(crate::types::Client::Mysql);
        query(&mut sub_builder);
        let sql = format!("NOT EXISTS ({})", sub_builder.to_sql().0);
        self.statement.push(crate::types::Statement::Raw((sql, None)));
    }
    
    fn where_json_contains(&mut self, column: &str, value: Value) {
        let sql = format!("JSON_CONTAINS({}, ?)", column);
        self.statement.push(crate::types::Statement::Raw((sql, Some(vec![value]))));
    }
}
