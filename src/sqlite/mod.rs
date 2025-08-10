//! SQLite-specific compilation logic

mod method_compiler;
mod operator_to_sql;
mod statement_compiler;
mod join_compiler;

use crate::builder::ChainBuilder;
use serde_json::Value;

// Re-export compilation functions
pub use method_compiler::method_compiler;
pub use statement_compiler::statement_compiler;
pub use join_compiler::join_compiler;

/// SQLite compilation result structure
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
    pub group_by: Vec<String>,
    pub group_by_raw: (String, Vec<Value>),
    pub having: (String, Vec<Value>),
    pub order_by: Vec<String>,
    pub order_by_raw: (String, Vec<Value>),
}

/// Main SQLite compilation function
pub fn to_sql(chain_builder: &ChainBuilder) -> ToSql {
    // Compile different parts
    let statement = statement_compiler(chain_builder);
    let method = method_compiler(chain_builder);
    let join = join_compiler(chain_builder, true);
    let _raw = (String::new(), Vec::<Value>::new());

    // Process common clauses
    let mut with = String::new();
    let mut with_binds: Vec<serde_json::Value> = vec![];
    let mut sql_union = String::new();
    let mut sql_union_binds: Vec<serde_json::Value> = vec![];
    let mut limit: Option<usize> = None;
    let mut offset: Option<usize> = None;
    let mut group_by: Vec<String> = vec![];
    let mut group_by_raw = String::new();
    let mut group_by_raw_binds: Vec<serde_json::Value> = vec![];
    let mut having = String::new();
    let mut having_binds: Vec<serde_json::Value> = vec![];
    let mut order_by: Vec<String> = vec![];
    let mut order_by_raw = String::new();
    let mut order_by_raw_binds: Vec<serde_json::Value> = vec![];

    // Process raw statements
    let mut raw_sql = String::new();
    let mut raw_binds: Vec<serde_json::Value> = vec![];
    for (sql, binds) in chain_builder.query.raw.iter() {
        if !raw_sql.is_empty() {
            raw_sql.push(' ');
        }
        raw_sql.push_str(sql);
        if let Some(binds) = binds {
            raw_binds.extend(binds.clone());
        }
    }

    // Process common clauses
    for common in chain_builder.query.query_common.iter() {
        match common {
            crate::types::Common::With(alias, recursive, chain_builder) => {
                if with.is_empty() {
                    with.push_str("WITH ");
                } else {
                    with.push_str(", ");
                }
                if *recursive {
                    with.push_str("RECURSIVE ");
                }
                with.push_str(alias);
                with.push_str(" AS (");
                let sql = to_sql(chain_builder);
                with.push_str(&sql.method.0);
                with_binds.extend(sql.method.1);
                with.push_str(")");
            }
            crate::types::Common::Union(is_all, chain_builder) => {
                if !sql_union.is_empty() {
                    sql_union.push(' ');
                }
                if *is_all {
                    sql_union.push_str("UNION ALL ");
                } else {
                    sql_union.push_str("UNION ");
                }
                let sql = to_sql(chain_builder);
                sql_union.push_str(sql.method.0.as_str());
                sql_union_binds.extend(sql.method.1);
            }
            crate::types::Common::Limit(l) => {
                limit = Some(*l);
            }
            crate::types::Common::Offset(o) => {
                offset = Some(*o);
            }
            crate::types::Common::GroupBy(g) => {
                group_by.extend(g.clone());
            }
            crate::types::Common::GroupByRaw(g, b) => {
                group_by_raw.push_str(g.as_str());
                if let Some(b) = b {
                    group_by_raw_binds.extend(b.clone());
                }
            }
            crate::types::Common::Having(sql, val) => {
                if !having.is_empty() {
                    having.push_str(" AND ");
                }
                having.push_str(sql);
                if let Some(val) = val {
                    having_binds.extend(val.clone());
                }
            }
            crate::types::Common::OrderBy(column, order) => {
                order_by.push(format!("{} {}", column, order));
            }
            crate::types::Common::OrderByRaw(sql, val) => {
                order_by_raw.push_str(sql.as_str());
                if let Some(val) = val {
                    order_by_raw_binds.extend(val.clone());
                }
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
        group_by,
        group_by_raw: (group_by_raw, group_by_raw_binds),
        having: (having, having_binds),
        order_by,
        order_by_raw: (order_by_raw, order_by_raw_binds),
    }
}

/// Merge compilation results into final SQL
pub fn merge_to_sql(to_sql: ToSql) -> (String, Vec<Value>) {
    let mut select_sql = String::new();
    let mut select_binds: Vec<serde_json::Value> = vec![];

    // Add WITH clause
    if !to_sql.sql_with.0.is_empty() {
        select_sql.push_str(&to_sql.sql_with.0);
        select_binds.extend(to_sql.sql_with.1);
    }

    // Add main query
    select_sql.push_str(&to_sql.method.0);
    select_binds.extend(to_sql.method.1);

    // Add JOINs
    if !to_sql.join.0.is_empty() {
        select_sql.push(' ');
        select_sql.push_str(&to_sql.join.0);
        select_binds.extend(to_sql.join.1);
    }

    // Add WHERE clause
    if !to_sql.statement.0.is_empty() {
        select_sql.push_str(" WHERE ");
        select_sql.push_str(&to_sql.statement.0.trim());
        select_binds.extend(to_sql.statement.1);
    }

    // Add raw SQL
    if !to_sql.raw.0.is_empty() {
        select_sql.push(' ');
        select_sql.push_str(&to_sql.raw.0);
        select_binds.extend(to_sql.raw.1);
    }

    // Add GROUP BY
    if !to_sql.group_by.is_empty() {
        select_sql.push_str(" GROUP BY ");
        select_sql.push_str(&to_sql.group_by.join(", "));
    }
    if !to_sql.group_by_raw.0.is_empty() {
        select_sql.push_str(" GROUP BY ");
        select_sql.push_str(&to_sql.group_by_raw.0);
        select_binds.extend(to_sql.group_by_raw.1);
    }

    // Add HAVING
    if !to_sql.having.0.is_empty() {
        select_sql.push_str(" HAVING ");
        select_sql.push_str(&to_sql.having.0);
        select_binds.extend(to_sql.having.1);
    }

    // Add ORDER BY
    if !to_sql.order_by.is_empty() {
        select_sql.push_str(" ORDER BY ");
        select_sql.push_str(&to_sql.order_by.join(", "));
    }
    if !to_sql.order_by_raw.0.is_empty() {
        select_sql.push_str(" ORDER BY ");
        select_sql.push_str(&to_sql.order_by_raw.0);
        select_binds.extend(to_sql.order_by_raw.1);
    }

    // Add LIMIT and OFFSET (SQLite uses LIMIT offset, count)
    if let Some(limit_val) = to_sql.limit {
        if let Some(offset_val) = to_sql.offset {
            select_sql.push_str(&format!(" LIMIT {}, {}", offset_val, limit_val));
        } else {
            select_sql.push_str(&format!(" LIMIT {}", limit_val));
        }
    } else if let Some(offset_val) = to_sql.offset {
        select_sql.push_str(&format!(" LIMIT {}, -1", offset_val));
    }

    // Add UNION
    if !to_sql.sql_union.0.is_empty() {
        select_sql.push(' ');
        select_sql.push_str(&to_sql.sql_union.0);
        select_binds.extend(to_sql.sql_union.1);
    }

    (select_sql, select_binds)
}
