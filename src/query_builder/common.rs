use crate::{ChainBuilder, Common};

pub trait QueryCommon {
    fn with(&mut self, alias: &str, chain_builder: ChainBuilder) -> &mut ChainBuilder;
    fn with_recursive(&mut self, alias: &str, chain_builder: ChainBuilder) -> &mut ChainBuilder;
    fn union(&mut self, chain_builder: ChainBuilder) -> &mut ChainBuilder;
    fn union_all(&mut self, chain_builder: ChainBuilder) -> &mut ChainBuilder;
    fn limit(&mut self, limit: usize) -> &mut ChainBuilder;
    fn offset(&mut self, offset: usize) -> &mut ChainBuilder;
    fn group_by(&mut self, column: Vec<&str>) -> &mut ChainBuilder;
    fn group_by_raw(&mut self, sql: &str, val: Option<Vec<serde_json::Value>>)
        -> &mut ChainBuilder;
}

impl QueryCommon for ChainBuilder {
    fn with(&mut self, alias: &str, chain_builder: ChainBuilder) -> &mut ChainBuilder {
        self.query_common
            .push(Common::With(alias.to_string(), false, chain_builder));
        self
    }

    fn with_recursive(&mut self, alias: &str, chain_builder: ChainBuilder) -> &mut ChainBuilder {
        self.query_common
            .push(Common::With(alias.to_string(), true, chain_builder));
        self
    }

    fn union(&mut self, chain_builder: ChainBuilder) -> &mut ChainBuilder {
        self.query_common.push(Common::Union(false, chain_builder));
        self
    }

    fn union_all(&mut self, chain_builder: ChainBuilder) -> &mut ChainBuilder {
        self.query_common.push(Common::Union(true, chain_builder));
        self
    }

    fn limit(&mut self, limit: usize) -> &mut ChainBuilder {
        self.query_common.push(Common::Limit(limit));
        self
    }

    fn offset(&mut self, offset: usize) -> &mut ChainBuilder {
        self.query_common.push(Common::Offset(offset));
        self
    }

    fn group_by(&mut self, column: Vec<&str>) -> &mut ChainBuilder {
        self.query_common.push(Common::GroupBy(
            column.iter().map(|s| s.to_string()).collect(),
        ));
        self
    }

    fn group_by_raw(
        &mut self,
        sql: &str,
        val: Option<Vec<serde_json::Value>>,
    ) -> &mut ChainBuilder {
        self.query_common
            .push(Common::GroupByRaw(sql.to_string(), val));
        self
    }
}
