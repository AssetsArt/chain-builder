use super::{merge_to_sql, to_sql};
use crate::{builder::ChainBuilder, types::{Method, Select}};
use serde_json::Value;

pub fn method_compiler(chain_builder: &ChainBuilder) -> (String, Vec<Value>) {
    match chain_builder.method {
        Method::Select => select_compiler(chain_builder),
        Method::Insert => insert_into_compiler(chain_builder),
        Method::InsertMany => insert_many_compiler(chain_builder),
        Method::Update => update_compiler(chain_builder),
        Method::Delete => delete_compiler(chain_builder),
    }
}

// Insert
fn insert_into_compiler(chain_builder: &ChainBuilder) -> (String, Vec<Value>) {
    let mut insert_sql = String::new();
    let mut insert_binds: Vec<serde_json::Value> = vec![];
    insert_sql.push_str("INSERT INTO ");

    if let Some((table, val)) = &chain_builder.table_raw {
        insert_sql.push_str(table);
        if let Some(val) = val {
            insert_binds.extend(val.clone());
        }
    } else if let Some(table) = &chain_builder.table {
        if let Some(db) = &chain_builder.db {
            insert_sql.push_str(format!("{}.", db).as_str());
        }
        insert_sql.push_str(table.as_str());
    }

    insert_sql.push_str(" (");
    let mut is_first = true;
    let map_default = serde_json::Map::new();
    let data = chain_builder
        .insert_update
        .as_object()
        .unwrap_or(&map_default);
    let mut keys = data.keys().collect::<Vec<&String>>();
    keys.sort();
    let len = keys.len();
    for key in keys.iter().take(len) {
        if is_first {
            is_first = false;
        } else {
            insert_sql.push_str(", ");
        }
        insert_sql.push_str(key.as_str());
    }
    insert_sql.push_str(") VALUES (");
    is_first = true;
    for key in keys.iter().take(len) {
        if is_first {
            is_first = false;
        } else {
            insert_sql.push_str(", ");
        }
        insert_sql.push('?');
        // insert_binds.push(data.get(key.as_str()).unwrap().clone());
        match data.get(key.as_str()) {
            Some(value) => {
                insert_binds.push(value.clone());
            }
            None => {
                println!("[Err] key: {:?}", key);
                println!("[Err] data: {:?}", data);
                // Should not happen
                panic!("[Err] insert_into_compiler: data.get(key.as_str()) is None");
            }
        }
    }

    insert_sql.push(')');

    (insert_sql, insert_binds)
}

// InsertMany
fn insert_many_compiler(chain_builder: &ChainBuilder) -> (String, Vec<Value>) {
    let mut insert_sql = String::new();
    let mut insert_binds: Vec<serde_json::Value> = vec![];

    insert_sql.push_str("INSERT INTO ");
    if let Some((table, val)) = &chain_builder.table_raw {
        insert_sql.push_str(table);
        if let Some(val) = val {
            insert_binds.extend(val.clone());
        }
    } else if let Some(table) = &chain_builder.table {
        if let Some(db) = &chain_builder.db {
            insert_sql.push_str(format!("{}.", db).as_str());
        }
        insert_sql.push_str(table.as_str());
    }

    insert_sql.push_str(" (");
    let mut is_first = true;
    let map_default = serde_json::Map::new();
    let vec_default = vec![];
    let data = chain_builder
        .insert_update
        .as_array()
        .unwrap_or(&vec_default);
    let mut keys = data[0]
        .as_object()
        .unwrap_or(&map_default)
        .keys()
        .collect::<Vec<&String>>();
    keys.sort();
    let len = keys.len();
    for key in keys.iter().take(len) {
        if is_first {
            is_first = false;
        } else {
            insert_sql.push_str(", ");
        }
        insert_sql.push_str(key.as_str());
    }
    insert_sql.push_str(") VALUES ");
    is_first = true;
    for row in data.iter() {
        if is_first {
            is_first = false;
        } else {
            insert_sql.push_str(", ");
        }
        insert_sql.push('(');
        let mut is_first = true;
        let row = row.as_object().unwrap_or(&map_default);
        for key in keys.iter().take(len) {
            if is_first {
                is_first = false;
            } else {
                insert_sql.push_str(", ");
            }
            insert_sql.push('?');
            match row.get(key.as_str()) {
                Some(value) => {
                    insert_binds.push(value.clone());
                }
                None => {
                    println!("[Err] key: {:?}", key);
                    println!("[Err] row: {:?}", row);
                    // Should not happen
                    panic!("[Err] insert_many_compiler: row.get(key.as_str()) is None");
                }
            }
        }
        insert_sql.push(')');
    }

    (insert_sql, insert_binds)
}

// Select
fn select_compiler(chain_builder: &ChainBuilder) -> (String, Vec<Value>) {
    let mut select_sql = String::new();
    let mut select_binds: Vec<serde_json::Value> = vec![];
    if chain_builder.is_distinct {
        select_sql.push_str("SELECT DISTINCT ");
    } else {
        select_sql.push_str("SELECT ");
    }

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
                    let rs_sql = merge_to_sql(rs_tosql);
                    select_sql.push('(');
                    select_sql.push_str(rs_sql.0.as_str());
                    select_sql.push_str(") AS ");
                    select_sql.push_str(as_name.as_str());
                    select_binds.extend(rs_sql.1);
                }
            }
        }
    }

    select_sql.push_str(" FROM ");
    if let Some((table, val)) = &chain_builder.table_raw {
        select_sql.push_str(table);
        if let Some(val) = val {
            select_binds.extend(val.clone());
        }
    } else if let Some(table) = &chain_builder.table {
        if let Some(db) = &chain_builder.db {
            select_sql.push_str(format!("{}.", db).as_str());
        }
        select_sql.push_str(table.as_str());
    }
    if let Some(as_name) = &chain_builder.as_name {
        select_sql.push_str(" AS ");
        select_sql.push_str(as_name);
    }
    (select_sql, select_binds)
}

// Update
fn update_compiler(chain_builder: &ChainBuilder) -> (String, Vec<Value>) {
    let mut update_sql = String::new();
    let mut update_binds: Vec<serde_json::Value> = vec![];

    update_sql.push_str("UPDATE ");
    if let Some((table, val)) = &chain_builder.table_raw {
        update_sql.push_str(table);
        if let Some(val) = val {
            update_binds.extend(val.clone());
        }
    } else if let Some(table) = &chain_builder.table {
        if let Some(db) = &chain_builder.db {
            update_sql.push_str(format!("{}.", db).as_str());
        }
        update_sql.push_str(table.as_str());
    }
    update_sql.push_str(" SET ");
    let map_default = serde_json::Map::new();
    let data = chain_builder
        .insert_update
        .as_object()
        .unwrap_or(&map_default);
    let mut keys = data.keys().collect::<Vec<&String>>();
    keys.sort();
    let len = keys.len();
    let mut is_first = true;
    for key in keys.iter().take(len) {
        if is_first {
            is_first = false;
        } else {
            update_sql.push_str(", ");
        }
        update_sql.push_str(key.as_str());
        update_sql.push_str(" = ?");
        match data.get(key.as_str()) {
            Some(value) => {
                update_binds.push(value.clone());
            }
            None => {
                println!("[Err] key: {:?}", key);
                println!("[Err] data: {:?}", data);
                // Should not happen
                panic!("[Err] update_compiler: data.get(key.as_str()) is None");
            }
        }
    }

    (update_sql, update_binds)
}

// Delete
fn delete_compiler(chain_builder: &ChainBuilder) -> (String, Vec<Value>) {
    let mut delete_sql = String::new();
    let mut delete_binds: Vec<serde_json::Value> = vec![];
    delete_sql.push_str("DELETE FROM ");
    if let Some((table, val)) = &chain_builder.table_raw {
        delete_sql.push_str(table);
        if let Some(val) = val {
            delete_binds.extend(val.clone());
        }
    } else if let Some(table) = &chain_builder.table {
        if let Some(db) = &chain_builder.db {
            delete_sql.push_str(format!("{}.", db).as_str());
        }
        delete_sql.push_str(table.as_str());
    }

    (delete_sql, delete_binds)
}
