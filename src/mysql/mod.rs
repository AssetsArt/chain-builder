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
    pub raw: (String, Vec<Value>),
    pub with: (String, Vec<Value>),
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

    // QueryCommon
    // - with
    let mut with = String::new();
    let mut with_binds: Vec<serde_json::Value> = vec![];
    if !chain_builder.with.is_empty() {
        with.push_str("WITH ");
        for (i, (alias, chain_builder)) in chain_builder.with.iter().enumerate() {
            if i > 0 {
                with.push_str(", ");
            }
            let sql = merge_to_sql(to_sql(chain_builder));
            with.push_str(&format!("{} AS ({})", alias, sql.0));
            with_binds.extend(sql.1);
        }
        with.push(' ');
    }

    // raw compiler
    let mut raw_sql = String::new();
    let mut raw_binds: Vec<serde_json::Value> = vec![];
    if !chain_builder.query.raw.is_empty() {
        for (i, raw) in chain_builder.query.raw.iter().enumerate() {
            if i > 0 {
                raw_sql.push(' ');
            }
            raw_sql.push_str(&raw.0);
            if let Some(binds) = &raw.1 {
                raw_binds.extend(binds.clone());
            }
        }
    }

    ToSql {
        statement,
        method,
        join,
        raw: (raw_sql, raw_binds),
        with: (with, with_binds),
    }
}

pub fn merge_to_sql(to_sql: ToSql) -> (String, Vec<Value>) {
    let mut select_sql = String::new();
    let mut select_binds: Vec<serde_json::Value> = vec![];
    if !to_sql.with.0.is_empty() {
        select_sql.push_str(to_sql.with.0.as_str());
    }
    if !to_sql.method.0.is_empty() {
        select_sql.push_str(to_sql.method.0.as_str());
    }
    if !to_sql.join.0.is_empty() {
        select_sql.push(' ');
        select_sql.push_str(to_sql.join.0.as_str());
    }
    if !to_sql.statement.0.is_empty() {
        select_sql.push(' ');
        select_sql.push_str(to_sql.statement.0.as_str());
    }
    if !to_sql.raw.0.is_empty() {
        select_sql.push(' ');
        select_sql.push_str(to_sql.raw.0.as_str());
    }
    // Add all binds order by with, method, join, statement, raw
    // 1. add with_binds
    select_binds.extend(to_sql.with.1);
    // 2 add select_binds
    select_binds.extend(to_sql.method.1);
    // 3. add join_binds
    select_binds.extend(to_sql.join.1);
    // 4. add binds
    select_binds.extend(to_sql.statement.1);
    // 5. add raw_binds
    select_binds.extend(to_sql.raw.1);

    (select_sql, select_binds)
}
