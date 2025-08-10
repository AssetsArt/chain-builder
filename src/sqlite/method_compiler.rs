use super::to_sql;
use crate::{builder::ChainBuilder, types::{Method, Select}};
use serde_json::Value;

pub fn method_compiler(chain_builder: &ChainBuilder) -> (String, Vec<Value>) {
    match chain_builder.method {
        Method::Select => {
            let mut select_sql = String::new();
            let mut select_binds: Vec<Value> = vec![];

            // Handle DISTINCT
            if chain_builder.is_distinct {
                select_sql.push_str("SELECT DISTINCT ");
            } else {
                select_sql.push_str("SELECT ");
            }

            // Handle SELECT clauses
            if chain_builder.select.is_empty() {
                select_sql.push_str("*");
            } else {
                let mut is_first = true;
                for select in chain_builder.select.iter() {
                    if !is_first {
                        select_sql.push_str(", ");
                    }
                    is_first = false;

                    match select {
                        Select::Columns(columns) => {
                            select_sql.push_str(&columns.join(", "));
                        }
                        Select::Raw(sql, binds) => {
                            select_sql.push_str(sql);
                            if let Some(binds) = binds {
                                select_binds.extend(binds.clone());
                            }
                        }
                        Select::Builder(alias, builder) => {
                            let to_sql_result = to_sql(builder);
                            select_sql.push_str(&format!("({}) AS {}", to_sql_result.method.0, alias));
                            select_binds.extend(to_sql_result.method.1);
                        }
                    }
                }
            }

            // Add FROM clause
            select_sql.push_str(" FROM ");

            // Handle table
            if let Some(table) = &chain_builder.table {
                if let Some(db) = &chain_builder.db {
                    select_sql.push_str(&format!("{}.{}", db, table));
                } else {
                    select_sql.push_str(table);
                }
            } else if let Some((table, binds)) = &chain_builder.table_raw {
                select_sql.push_str(table);
                if let Some(binds) = binds {
                    select_binds.extend(binds.clone());
                }
            }

            // Add table alias
            if let Some(alias) = &chain_builder.as_name {
                select_sql.push_str(&format!(" AS {}", alias));
            }

            (select_sql, select_binds)
        }
        Method::Insert => {
            let mut insert_sql = String::new();
            let mut insert_binds: Vec<Value> = vec![];

            insert_sql.push_str("INSERT INTO ");

            // Add table
            if let Some(table) = &chain_builder.table {
                if let Some(db) = &chain_builder.db {
                    insert_sql.push_str(&format!("{}.{}", db, table));
                } else {
                    insert_sql.push_str(table);
                }
            }

            // Handle INSERT data
            match &chain_builder.insert_update {
                Value::Object(obj) => {
                    let mut columns: Vec<&String> = obj.keys().collect();
                    let mut values: Vec<&Value> = obj.values().collect();
                    // Sort columns to ensure consistent order
                    columns.sort();
                    values.sort_by(|a, b| {
                        let a_key = obj.iter().find(|(_, v)| std::ptr::eq(*v, *a)).map(|(k, _)| k);
                        let b_key = obj.iter().find(|(_, v)| std::ptr::eq(*v, *b)).map(|(k, _)| k);
                        a_key.cmp(&b_key)
                    });
                    
                    insert_sql.push_str(" (");
                    insert_sql.push_str(&columns.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));
                    insert_sql.push_str(") VALUES (");
                    
                    let placeholders = vec!["?"; values.len()].join(", ");
                    insert_sql.push_str(&placeholders);
                    insert_sql.push_str(")");
                    
                    insert_binds.extend(values.iter().cloned().cloned());
                }
                Value::Array(arr) => {
                    // Multiple rows insert
                    if let Some(Value::Object(first_row)) = arr.first() {
                        let columns: Vec<&String> = first_row.keys().collect();
                        insert_sql.push_str(" (");
                        insert_sql.push_str(&columns.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", "));
                        insert_sql.push_str(") VALUES ");
                        
                        let mut is_first = true;
                        for row in arr {
                            if let Value::Object(obj) = row {
                                if !is_first {
                                    insert_sql.push_str(", ");
                                }
                                is_first = false;
                                
                                let placeholders = vec!["?"; columns.len()].join(", ");
                                insert_sql.push_str(&format!("({})", placeholders));
                                
                                for col in &columns {
                                    if let Some(val) = obj.get(*col) {
                                        insert_binds.push(val.clone());
                                    }
                                }
                            }
                        }
                    }
                }
                _ => {
                    // Raw insert
                    insert_sql.push_str(" DEFAULT VALUES");
                }
            }

            (insert_sql, insert_binds)
        }
        Method::Update => {
            let mut update_sql = String::new();
            let mut update_binds: Vec<Value> = vec![];

            update_sql.push_str("UPDATE ");

            // Add table
            if let Some(table) = &chain_builder.table {
                if let Some(db) = &chain_builder.db {
                    update_sql.push_str(&format!("{}.{}", db, table));
                } else {
                    update_sql.push_str(table);
                }
            }

            update_sql.push_str(" SET ");

            // Handle UPDATE data
            match &chain_builder.insert_update {
                Value::Object(obj) => {
                    let mut is_first = true;
                    for (column, value) in obj {
                        if !is_first {
                            update_sql.push_str(", ");
                        }
                        is_first = false;
                        
                        // Handle special increment/decrement cases
                        if let Value::String(val_str) = value {
                            if val_str.contains(" + ") || val_str.contains(" - ") {
                                update_sql.push_str(&format!("{} = {}", column, val_str));
                                continue;
                            }
                        }
                        
                        update_sql.push_str(&format!("{} = ?", column));
                        update_binds.push(value.clone());
                    }
                }
                _ => {
                    update_sql.push_str("column = ?");
                    update_binds.push(chain_builder.insert_update.clone());
                }
            }

            (update_sql, update_binds)
        }
        Method::Delete => {
            let mut delete_sql = String::new();
            let delete_binds: Vec<Value> = vec![];

            delete_sql.push_str("DELETE FROM ");

            // Add table
            if let Some(table) = &chain_builder.table {
                if let Some(db) = &chain_builder.db {
                    delete_sql.push_str(&format!("{}.{}", db, table));
                } else {
                    delete_sql.push_str(table);
                }
            }

            (delete_sql, delete_binds)
        }
        Method::InsertMany => {
            // Handle multiple inserts (same as Insert for now)
            // Just call the Insert case directly
            method_compiler(chain_builder)
        }
    }
}
