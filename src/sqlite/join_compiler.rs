use crate::{query::join::JoinStatement, builder::ChainBuilder};
use serde_json::Value;

pub fn join_compiler(chain_builder: &ChainBuilder, _prefix: bool) -> (String, Vec<Value>) {
    let mut join_sql = String::new();
    let mut join_binds: Vec<Value> = vec![];

    for join in chain_builder.query.join.iter() {
        if !join_sql.is_empty() {
            join_sql.push(' ');
        }

        if let Some((raw, binds)) = &join.raw {
            join_sql.push_str(raw);
            if let Some(binds) = binds {
                join_binds.extend(binds.clone());
            }
        } else {
            join_sql.push_str(&join.join_type);
            join_sql.push(' ');
            
            if let Some(db) = &chain_builder.db {
                join_sql.push_str(&format!("{}.{}", db, join.table));
            } else {
                join_sql.push_str(&join.table);
            }

            if let Some(alias) = &join.as_name {
                join_sql.push_str(&format!(" AS {}", alias));
            }

            if !join.statement.is_empty() {
                join_sql.push_str(" ON ");
                let mut is_first = true;
                for statement in join.statement.iter() {
                    if !is_first {
                        join_sql.push_str(" AND ");
                    }
                    is_first = false;

                    match statement {
                        JoinStatement::On(column1, operator, column2) => {
                            join_sql.push_str(&format!("{} {} {}", column1, operator, column2));
                        }
                        JoinStatement::OrChain(join_builder) => {
                            // For now, just add the OR chain as raw SQL
                            join_sql.push_str("(OR condition)");
                        }
                        JoinStatement::OnVal(column, operator, value) => {
                            join_sql.push_str(&format!("{} {} ?", column, operator));
                            join_binds.push(value.clone());
                        }
                        JoinStatement::OnRaw(raw, binds) => {
                            join_sql.push_str(raw);
                            if let Some(binds) = binds {
                                join_binds.extend(binds.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    (join_sql, join_binds)
}
