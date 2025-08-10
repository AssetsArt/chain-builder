use super::operator_to_sql::operator_to_sql;
use crate::{builder::ChainBuilder, query::Operator, types::Statement};
use serde_json::Value;

pub fn statement_compiler(chain_builder: &ChainBuilder) -> (String, Vec<Value>) {
    let mut statement_sql = String::new();
    let mut statement_binds: Vec<serde_json::Value> = vec![];
    let mut is_first = true;
    let mut build_statement = |statement: &Statement| match statement {
        Statement::Value(field, operator, value) => {
            let (operator_str, is_bind) = operator_to_sql(operator);
            if is_first {
                is_first = false;
            } else {
                statement_sql.push_str(" AND ");
            }
            statement_sql.push_str(field);
            statement_sql.push(' ');
            if *operator == Operator::Between || *operator == Operator::NotBetween {
                statement_sql.push_str(&format!("{} ? AND ?", operator_str));
                if let Some(array) = value.as_array() {
                    for v in array {
                        statement_binds.push(v.clone());
                    }
                }
            } else {
                statement_sql.push_str(operator_str);
                if is_bind {
                    if *operator == Operator::In || *operator == Operator::NotIn {
                        if let serde_json::Value::Array(value) = value {
                            statement_sql.push_str(" (");
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
                            statement_sql.push_str(" ?");
                            statement_binds.push(value.clone());
                        }
                    } else {
                        statement_sql.push_str(" ?");
                        statement_binds.push(value.clone());
                    }
                }
            }
        }
        Statement::OrChain(qb) => {
            if is_first {
                is_first = false;
            } else {
                statement_sql.push_str(" OR ");
            }
            let mut c = chain_builder.clone();
            c.query = *qb.clone();
            let (sql, binds) = statement_compiler(&c);
            if qb.statement.len() > 1 {
                statement_sql.push_str(&format!("({})", sql));
            } else {
                statement_sql.push_str(&sql);
            }
            statement_binds.extend(binds);
        }
        Statement::SubChain(qb) => {
            if is_first {
                is_first = false;
            } else {
                statement_sql.push_str(" AND ");
            }
            let mut c = chain_builder.clone();
            c.query = *qb.clone();
            let (sql, binds) = statement_compiler(&c);
            statement_sql.push_str(&format!("({})", sql));
            statement_binds.extend(binds);
        }
        Statement::Raw((sql, binds)) => {
            if is_first {
                is_first = false;
            } else {
                statement_sql.push_str(" AND ");
            }
            statement_sql.push_str(sql);
            if let Some(binds) = binds {
                statement_binds.extend(binds.clone());
            }
        }
    };

    for statement in chain_builder.query.statement.iter() {
        build_statement(statement);
    }

    (statement_sql, statement_binds)
}
