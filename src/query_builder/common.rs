use crate::ChainBuilder;

pub trait QueryCommon {
    fn with(&mut self, alias: &str, chain_builder: ChainBuilder) -> &mut ChainBuilder;
}

impl QueryCommon for ChainBuilder {
    fn with(&mut self, alias: &str, chain_builder: ChainBuilder) -> &mut ChainBuilder {
        self.with.push((alias.to_string(), chain_builder));
        self
    }
}
