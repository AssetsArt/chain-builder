use super::to_sql;
use crate::{ChainBuilder, Method, Select};
use serde_json::Value;

pub fn method_compiler(chain_builder: &ChainBuilder) -> (String, Vec<Value>) {
    match chain_builder.method {
        Method::Select => select_compiler(chain_builder),
        Method::Insert => insert_into_compiler(chain_builder),
    }
}

fn insert_into_compiler(chain_builder: &ChainBuilder) -> (String, Vec<Value>) {
    let mut insert_sql = String::new();
    let mut insert_binds: Vec<serde_json::Value> = vec![];

    insert_sql.push_str("INSERT INTO ");
    if let Some(db) = &chain_builder.db {
        insert_sql.push_str(format!("`{}`.", db).as_str());
    }
    insert_sql.push_str(format!("`{}`", chain_builder.table).as_str());

    insert_sql.push_str(" (");
    let mut is_first = true;
    let map_default = serde_json::Map::new();
    let data = chain_builder.inner.as_object().unwrap_or(&map_default);
    let mut keys = data.keys().collect::<Vec<&String>>();
    keys.sort_by(|a, b| a.cmp(b));
    let len = keys.len();
    for x in 0..len {
        if is_first {
            is_first = false;
        } else {
            insert_sql.push_str(", ");
        }
        insert_sql.push_str(keys[x]);
    }
    insert_sql.push_str(") VALUES (");
    is_first = true;
    for x in 0..len {
        if is_first {
            is_first = false;
        } else {
            insert_sql.push_str(", ");
        }
        insert_sql.push_str("?");
        insert_binds.push(data.get(keys[x].as_str()).unwrap().clone());
    }

    insert_sql.push_str(")");

    (insert_sql, insert_binds)
}

fn select_compiler(chain_builder: &ChainBuilder) -> (String, Vec<Value>) {
    let mut select_sql = String::new();
    let mut select_binds: Vec<serde_json::Value> = vec![];

    select_sql.push_str("SELECT ");

    if chain_builder.select.is_empty() {
        select_sql.push('*');
    } else {
        let mut is_first = true;
        for select in &chain_builder.select {
            if is_first {
                is_first = false;
            } else {
                select_sql.push_str(", ");
            }
            match select {
                Select::Columns(columns) => {
                    select_sql.push_str(&columns.join(", "));
                }
                Select::Raw(sql, binds) => {
                    select_sql.push_str(sql.as_str());
                    if let Some(binds) = binds {
                        select_binds.extend(binds.clone());
                    }
                }
                Select::Builder(as_name, c2) => {
                    let rs_tosql = to_sql(c2);
                    select_sql.push('(');
                    if !rs_tosql.method.0.is_empty() {
                        select_sql.push_str(rs_tosql.method.0.as_str());
                    }
                    if !rs_tosql.join.0.is_empty() {
                        select_sql.push(' ');
                        select_sql.push_str(rs_tosql.join.0.as_str());
                    }
                    if !rs_tosql.statement.0.is_empty() {
                        select_sql.push(' ');
                        select_sql.push_str(rs_tosql.statement.0.as_str());
                    }
                    if !rs_tosql.raw.0.is_empty() {
                        select_sql.push(' ');
                        select_sql.push_str(rs_tosql.raw.0.as_str());
                    }
                    select_sql.push_str(") AS ");
                    select_sql.push_str(as_name.as_str());
                    // Add all binds to select_binds order by select_binds, join_binds, binds, raw_binds
                    // 1. add select_binds
                    select_binds.extend(rs_tosql.method.1);
                    // 2. add join_binds
                    select_binds.extend(rs_tosql.join.1);
                    // 3. add binds
                    select_binds.extend(rs_tosql.statement.1);
                    // 4. add raw_binds
                    select_binds.extend(rs_tosql.raw.1);
                }
            }
        }
    }

    select_sql.push_str(" FROM ");
    if let Some(db) = &chain_builder.db {
        select_sql.push_str(format!("`{}`.", db).as_str());
    }
    select_sql.push_str(format!("`{}`", chain_builder.table).as_str());
    (select_sql, select_binds)
}
