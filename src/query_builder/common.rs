use crate::ChainBuilder;

pub trait QueryCommon {
    fn with(&mut self, alias: &str, chain_builder: ChainBuilder) -> &mut ChainBuilder;
    fn with_recursive(&mut self, alias: &str, chain_builder: ChainBuilder) -> &mut ChainBuilder;
}

impl QueryCommon for ChainBuilder {
    fn with(&mut self, alias: &str, chain_builder: ChainBuilder) -> &mut ChainBuilder {
        self.with.push((alias.to_string(), false, chain_builder));
        self
    }

    fn with_recursive(&mut self, alias: &str, chain_builder: ChainBuilder) -> &mut ChainBuilder {
        self.with.push((alias.to_string(), true, chain_builder));
        self
    }
}
