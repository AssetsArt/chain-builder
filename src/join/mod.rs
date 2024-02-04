mod join_methods;

// export
pub use join_methods::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) enum JoinStatement {
    On(String, String, String),
    OrChain(Box<JoinBuilder>),
    OnVal(String, String, serde_json::Value),
}

impl JoinStatement {
    pub fn as_mut_join_builder(&mut self) -> &mut JoinBuilder {
        match self {
            JoinStatement::OrChain(query) => query,
            _ => panic!("JoinStatement::to_join_builder()"),
        }
    }
}

#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct JoinBuilder {
    pub(crate) table: String,
    pub(crate) join_type: String,
    pub(crate) statement: Vec<JoinStatement>,
    pub(crate) raw: Option<(String, Option<Vec<serde_json::Value>>)>,
}
