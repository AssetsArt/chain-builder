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
    pub sql_with: (String, Vec<Value>),
    pub sql_union: (String, Vec<Value>),
    pub limit: Option<usize>,
    pub offset: Option<usize>,
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
    let mut is_first_with = true;
    //  - union
    let mut sql_union = String::new();
    let mut sql_union_binds: Vec<serde_json::Value> = vec![];
    let mut is_first_union = true;
    // - limit
    let mut limit = None;
    // - offset
    let mut offset = None;
    for common in chain_builder.query_common.iter() {
        match common {
            crate::Common::With(alias, recursive, chain_builder) => {
                with.push_str("WITH");
                with.push(' ');
                if !is_first_with {
                    with.push_str(", ");
                }
                is_first_with = false;
                if *recursive {
                    with.push_str("RECURSIVE");
                    with.push(' ');
                }
                with.push_str(alias.as_str());
                with.push_str(" AS (");
                let sql = merge_to_sql(to_sql(chain_builder));
                with.push_str(sql.0.as_str());
                with.push(')');
                with_binds.extend(sql.1);
                with.push(' ');
            }
            crate::Common::Union(is_all, chain_builder) => {
                if !is_first_union {
                    sql_union.push(' ');
                }
                is_first_union = false;
                if *is_all {
                    sql_union.push_str("UNION ALL");
                } else {
                    sql_union.push_str("UNION");
                }
                sql_union.push(' ');
                let sql = merge_to_sql(to_sql(chain_builder));
                sql_union.push_str(sql.0.as_str());
                sql_union_binds.extend(sql.1);
            }
            crate::Common::Limit(l) => {
                limit = Some(*l);
            }
            crate::Common::Offset(o) => {
                offset = Some(*o);
            }
        }
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
        sql_with: (with, with_binds),
        sql_union: (sql_union, sql_union_binds),
        limit,
        offset,
    }
}

pub fn merge_to_sql(to_sql: ToSql) -> (String, Vec<Value>) {
    let mut select_sql = String::new();
    let mut select_binds: Vec<serde_json::Value> = vec![];
    if !to_sql.sql_with.0.is_empty() {
        select_sql.push_str(to_sql.sql_with.0.as_str());
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

    if !to_sql.sql_union.0.is_empty() {
        select_sql.push(' ');
        select_sql.push_str(to_sql.sql_union.0.as_str());
    }

    if !to_sql.raw.0.is_empty() {
        select_sql.push(' ');
        select_sql.push_str(to_sql.raw.0.as_str());
    }
    // Add all binds order by
    // - with,
    // - method
    // - join
    // - statement
    // - limit
    // - offset
    // - union
    // - raw
    select_binds.extend(to_sql.sql_with.1);
    select_binds.extend(to_sql.method.1);
    select_binds.extend(to_sql.join.1);
    select_binds.extend(to_sql.statement.1);
    if let Some(limit) = to_sql.limit {
        select_sql.push(' ');
        select_sql.push_str("LIMIT ?");
        select_binds.push(serde_json::Value::Number(serde_json::Number::from(limit)));
    }
    if let Some(offset) = to_sql.offset {
        select_sql.push(' ');
        select_sql.push_str("OFFSET ?");
        select_binds.push(serde_json::Value::Number(serde_json::Number::from(offset)));
    }
    select_binds.extend(to_sql.sql_union.1);
    select_binds.extend(to_sql.raw.1);

    (select_sql, select_binds)
}
