use super::operator_to_sql::operator_to_sql;
use crate::{builder::ChainBuilder, query::Operator, types::Statement};
use serde_json::Value;

pub fn statement_compiler(chain_builder: &ChainBuilder) -> (String, Vec<Value>) {
    let mut statement_sql = String::new();
    let mut statement_binds: Vec<Value> = vec![];

    for (i, statement) in chain_builder.query.statement.iter().enumerate() {
        if i > 0 {
            statement_sql.push_str(" AND ");
        }

        match statement {
            Statement::Value(column, operator, value) => {
                statement_sql.push_str(column);
                statement_sql.push(' ');
                let (operator_str, is_bind) = operator_to_sql(operator);
                statement_sql.push_str(operator_str);
                if is_bind {
                    statement_sql.push(' ');
                }
                
                if *operator == Operator::Between || *operator == Operator::NotBetween {
                    statement_sql.push_str("? AND ?");
                    if let Some(array) = value.as_array() {
                        for v in array {
                            statement_binds.push(v.clone());
                        }
                    }
                                } else {
                    if is_bind {
                        if *operator == Operator::In || *operator == Operator::NotIn {
                            if let serde_json::Value::Array(value) = value {
                                statement_sql.push_str("(");
                                let mut is_first = true;
                                value.iter().for_each(|v| {
                                    if is_first {
                                        is_first = false;
                                    } else {
                                        statement_sql.push(',');
                                    }
                                    statement_sql.push('?');
                                    statement_binds.push(v.clone());
                                });
                                statement_sql.push(')');
                            } else {
                                statement_sql.push('?');
                                statement_binds.push(value.clone());
                            }
                        } else {
                            statement_sql.push('?');
                            statement_binds.push(value.clone());
                        }
                    }
                }
            }
            Statement::SubChain(query) => {
                // For now, just add the subquery as raw SQL
                statement_sql.push_str("(SELECT * FROM subquery)");
            }
            Statement::OrChain(query) => {
                // For now, just add the OR chain as raw SQL
                statement_sql.push_str("(SELECT * FROM or_chain)");
            }
            Statement::Raw((sql, binds)) => {
                statement_sql.push_str(sql);
                if let Some(binds) = binds {
                    statement_binds.extend(binds.clone());
                }
            }
        }
    }

    (statement_sql, statement_binds)
}
