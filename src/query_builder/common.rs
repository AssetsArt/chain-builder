use crate::{Common, QueryBuilder};

pub trait QueryCommon {
    fn limit(&mut self, limit: usize) -> &mut QueryBuilder;
    fn offset(&mut self, offset: usize) -> &mut QueryBuilder;
    fn group_by(&mut self, column: Vec<&str>) -> &mut QueryBuilder;
    fn group_by_raw(&mut self, sql: &str, val: Option<Vec<serde_json::Value>>)
        -> &mut QueryBuilder;
    fn order_by(&mut self, column: &str, order: &str) -> &mut QueryBuilder;
    fn order_by_raw(&mut self, sql: &str, val: Option<Vec<serde_json::Value>>)
        -> &mut QueryBuilder;
}

impl QueryCommon for QueryBuilder {
    fn limit(&mut self, limit: usize) -> &mut QueryBuilder {
        self.query_common.push(Common::Limit(limit));
        self
    }

    fn offset(&mut self, offset: usize) -> &mut QueryBuilder {
        self.query_common.push(Common::Offset(offset));
        self
    }

    fn group_by(&mut self, column: Vec<&str>) -> &mut QueryBuilder {
        self.query_common.push(Common::GroupBy(
            column.iter().map(|s| s.to_string()).collect(),
        ));
        self
    }

    fn group_by_raw(
        &mut self,
        sql: &str,
        val: Option<Vec<serde_json::Value>>,
    ) -> &mut QueryBuilder {
        self.query_common
            .push(Common::GroupByRaw(sql.to_string(), val));
        self
    }

    fn order_by(&mut self, column: &str, order: &str) -> &mut QueryBuilder {
        const ALLOWED: [&str; 2] = ["ASC", "DESC"];
        let mut order = order.to_uppercase();
        if !ALLOWED.contains(&order.as_str()) {
            // panic!("order must be either ASC or DESC");
            println!("order must be either ASC or DESC");
            order = "ASC".to_string();
        }
        self.query_common
            .push(Common::OrderBy(column.to_string(), order.to_string()));
        self
    }

    fn order_by_raw(
        &mut self,
        sql: &str,
        val: Option<Vec<serde_json::Value>>,
    ) -> &mut QueryBuilder {
        self.query_common
            .push(Common::OrderByRaw(sql.to_string(), val));
        self
    }
}
