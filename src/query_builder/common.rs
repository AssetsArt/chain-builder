use crate::ChainBuilder;

pub trait QueryCommon {
    fn with(&mut self, alias: &str, chain_builder: ChainBuilder) -> &mut ChainBuilder;
    fn with_recursive(&mut self, alias: &str, chain_builder: ChainBuilder) -> &mut ChainBuilder;
    fn union(&mut self, chain_builder: ChainBuilder) -> &mut ChainBuilder;
    fn union_all(&mut self, chain_builder: ChainBuilder) -> &mut ChainBuilder;
}

impl QueryCommon for ChainBuilder {
    fn with(&mut self, alias: &str, chain_builder: ChainBuilder) -> &mut ChainBuilder {
        self.query_with
            .push((alias.to_string(), false, chain_builder));
        self
    }

    fn with_recursive(&mut self, alias: &str, chain_builder: ChainBuilder) -> &mut ChainBuilder {
        self.query_with
            .push((alias.to_string(), true, chain_builder));
        self
    }

    fn union(&mut self, chain_builder: ChainBuilder) -> &mut ChainBuilder {
        self.query_union.push((false, chain_builder));
        self
    }

    fn union_all(&mut self, chain_builder: ChainBuilder) -> &mut ChainBuilder {
        self.query_union.push((true, chain_builder));
        self
    }
}
