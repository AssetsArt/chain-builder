use crate::{
    builder::ChainBuilder,
    dialect::escape_identifier,
    types::{Method, Select},
};
use serde_json::Value;

pub trait ToSqlProvider {
    fn to_sql(&self, chain_builder: &ChainBuilder) -> (String, Vec<Value>);
}

/// Compile the target table reference (`[db.]table` or a raw expression),
/// escaping the `db`/`table` identifiers and collecting any raw binds.
fn compile_table(chain_builder: &ChainBuilder, binds: &mut Vec<Value>) -> String {
    let client = &chain_builder.query.client;
    if let Some((table, val)) = &chain_builder.table_raw {
        if let Some(val) = val {
            binds.extend(val.clone());
        }
        table.clone()
    } else if let Some(table) = &chain_builder.table {
        let mut sql = String::new();
        if let Some(db) = &chain_builder.db {
            sql.push_str(&escape_identifier(db, client));
            sql.push('.');
        }
        sql.push_str(&escape_identifier(table, client));
        sql
    } else {
        String::new()
    }
}

pub fn method_compiler_with_provider<T: ToSqlProvider>(
    chain_builder: &ChainBuilder,
    to_sql_provider: &T,
) -> (String, Vec<Value>) {
    match chain_builder.method {
        Method::Select => select_compiler(chain_builder, to_sql_provider),
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
    let client = &chain_builder.query.client;

    let table_sql = compile_table(chain_builder, &mut insert_binds);
    insert_sql.push_str(&table_sql);

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
        insert_sql.push_str(&escape_identifier(key, client));
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
        // Keys come from `data`, so this lookup always succeeds; bind NULL as a
        // defensive fallback instead of panicking on untrusted input.
        let value = data.get(key.as_str()).cloned().unwrap_or(Value::Null);
        insert_binds.push(value);
    }

    insert_sql.push(')');

    (insert_sql, insert_binds)
}

// InsertMany
fn insert_many_compiler(chain_builder: &ChainBuilder) -> (String, Vec<Value>) {
    let mut insert_sql = String::new();
    let mut insert_binds: Vec<serde_json::Value> = vec![];

    insert_sql.push_str("INSERT INTO ");
    let client = &chain_builder.query.client;

    let table_sql = compile_table(chain_builder, &mut insert_binds);
    insert_sql.push_str(&table_sql);

    insert_sql.push_str(" (");
    let mut is_first = true;
    let map_default = serde_json::Map::new();
    let vec_default = vec![];
    let data = chain_builder
        .insert_update
        .as_array()
        .unwrap_or(&vec_default);
    // Derive the column set from the first row; bail out gracefully on empty input
    // instead of indexing `data[0]` (which would panic).
    let mut keys = data
        .first()
        .and_then(|row| row.as_object())
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
        insert_sql.push_str(&escape_identifier(key, client));
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
            // Rows may be heterogeneous; bind NULL for a missing column instead
            // of panicking so untrusted data can't crash the process.
            let value = row.get(key.as_str()).cloned().unwrap_or(Value::Null);
            insert_binds.push(value);
        }
        insert_sql.push(')');
    }

    (insert_sql, insert_binds)
}

// Select with provider
fn select_compiler<T: ToSqlProvider>(
    chain_builder: &ChainBuilder,
    to_sql_provider: &T,
) -> (String, Vec<Value>) {
    let mut select_sql = String::new();
    let mut select_binds: Vec<serde_json::Value> = vec![];
    if chain_builder.is_distinct {
        select_sql.push_str("SELECT DISTINCT ");
    } else {
        select_sql.push_str("SELECT ");
    }

    let client = &chain_builder.query.client;
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
                    select_sql.push_str(&crate::dialect::escape_identifier_list(columns, client));
                }
                Select::Raw(sql, binds) => {
                    select_sql.push_str(sql.as_str());
                    if let Some(binds) = binds {
                        select_binds.extend(binds.clone());
                    }
                }
                Select::Builder(as_name, c2) => {
                    let (sub_sql, sub_binds) = to_sql_provider.to_sql(c2);
                    select_sql.push_str("(");
                    select_sql.push_str(&sub_sql);
                    select_sql.push_str(") AS ");
                    select_sql.push_str(&escape_identifier(as_name, client));
                    select_binds.extend(sub_binds);
                }
            }
        }
    }

    select_sql.push_str(" FROM ");
    let table_sql = compile_table(chain_builder, &mut select_binds);
    select_sql.push_str(&table_sql);
    if let Some(as_name) = &chain_builder.as_name {
        select_sql.push_str(" AS ");
        select_sql.push_str(&escape_identifier(as_name, client));
    }
    (select_sql, select_binds)
}

// Update
fn update_compiler(chain_builder: &ChainBuilder) -> (String, Vec<Value>) {
    let mut update_sql = String::new();
    let mut update_binds: Vec<serde_json::Value> = vec![];

    update_sql.push_str("UPDATE ");
    let client = &chain_builder.query.client;
    let table_sql = compile_table(chain_builder, &mut update_binds);
    update_sql.push_str(&table_sql);
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
        update_sql.push_str(&escape_identifier(key, client));
        update_sql.push_str(" = ?");
        // Keys come from `data`; bind NULL defensively rather than panicking.
        let value = data.get(key.as_str()).cloned().unwrap_or(Value::Null);
        update_binds.push(value);
    }

    (update_sql, update_binds)
}

// Delete
fn delete_compiler(chain_builder: &ChainBuilder) -> (String, Vec<Value>) {
    let mut delete_sql = String::new();
    let mut delete_binds: Vec<serde_json::Value> = vec![];
    delete_sql.push_str("DELETE FROM ");
    let table_sql = compile_table(chain_builder, &mut delete_binds);
    delete_sql.push_str(&table_sql);

    (delete_sql, delete_binds)
}
