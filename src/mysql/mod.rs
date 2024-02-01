mod join_compiler;
mod method_compiler;
mod operator_to_sql;
mod statement_compiler;

use serde_json::Value;

// inner
use crate::{
    mysql::{method_compiler::method_compiler, statement_compiler::statement_compiler},
    ChainBuilder,
};

use self::join_compiler::join_compiler;

#[derive(Debug, Clone, Default)]
pub struct ToSql {
    pub statement: (String, Vec<Value>),
    pub method: (String, Vec<Value>),
    pub join: (String, Vec<Value>),
}

pub fn to_sql(chain_builder: &ChainBuilder) -> ToSql {
    // statement compiler
    let mut statement = statement_compiler(chain_builder);
    if !statement.0.is_empty() {
        statement.0 = format!("WHERE {}", statement.0);
    }
    // compiler method
    let method = method_compiler(chain_builder);
    // join compiler
    let join = join_compiler(chain_builder, true);

    ToSql {
        statement,
        method,
        join,
    }
}
