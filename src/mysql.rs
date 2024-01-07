use crate::{operator::Operator, ChainBuilder, Method, Select, Statement};

// operator and is_bind
pub fn operator_to_sql(operator: &Operator) -> (&str, bool) {
    match operator {
        Operator::Equal => ("=", true),
        Operator::NotEqual => ("!=", true),
        Operator::In => ("IN", true),
        Operator::NotIn => ("NOT IN", true),
        Operator::IsNull => ("IS NULL", false),
        Operator::IsNotNull => ("IS NOT NULL", false),
        Operator::Exists => ("EXISTS", false),
        Operator::NotExists => ("NOT EXISTS", false),
        Operator::Between => ("BETWEEN", true),
        Operator::NotBetween => ("NOT BETWEEN", true),
        Operator::Like => ("LIKE", true),
        Operator::NotLike => ("NOT LIKE", true),
    }
}

pub fn to_sql(c: &ChainBuilder, is_statement: bool) -> (String, Option<Vec<serde_json::Value>>) {
    let mut statement_sql = String::new();
    let mut to_binds: Vec<serde_json::Value> = vec![];
    for (i, statement) in c.statement.iter().enumerate() {
        match statement {
            Statement::Value(field, operator, value) => {
                if i > 0 {
                    if let Statement::OrChain(_) = c.statement[i - 1] {
                    } else {
                        statement_sql.push_str(" AND ");
                    }
                }
                statement_sql.push_str(field);
                statement_sql.push(' ');
                let (operator_str, is_bind) = operator_to_sql(operator);
                if *operator == Operator::Between || *operator == Operator::NotBetween {
                    statement_sql.push_str(operator_str);
                    statement_sql.push_str(" ? AND ?");
                    for v in value.as_array().unwrap() {
                        to_binds.push(v.clone());
                    }
                } else {
                    statement_sql.push_str(operator_str);
                    if is_bind {
                        if let serde_json::Value::Array(_) = value {
                            statement_sql.push_str(" (");
                            let mut is_first = true;
                            for v in value.as_array().unwrap_or(&vec![]) {
                                if is_first {
                                    is_first = false;
                                } else {
                                    statement_sql.push(',');
                                }
                                statement_sql.push('?');
                                to_binds.push(v.clone());
                            }
                            statement_sql.push(')');
                        } else {
                            statement_sql.push_str(" ?");
                            to_binds.push(value.clone());
                        }
                    }
                }
            }
            Statement::AndChain(chain) => {
                if i > 0 {
                    statement_sql.push_str(" AND ");
                }
                let (sql, binds) = chain.delegate_to_sql(true);
                statement_sql.push_str(&sql);
                if let Some(binds) = binds {
                    to_binds.extend(binds);
                }
            }
            Statement::OrChain(chain) => {
                if i > 0 {
                    statement_sql.push_str(" OR ");
                }
                let (sql, binds) = chain.delegate_to_sql(true);
                statement_sql.push_str(&sql);
                if let Some(binds) = binds {
                    to_binds.extend(binds);
                }
            }
            Statement::SubChain(chain) => {
                if i > 0 {
                    statement_sql.push_str(" AND ");
                }
                statement_sql.push('(');
                let (sql, binds) = chain.delegate_to_sql(true);
                statement_sql.push_str(&sql);
                if let Some(binds) = binds {
                    to_binds.extend(binds);
                }
                statement_sql.push(')');
            }
            Statement::Raw((sql, binds)) => {
                if i > 0 {
                    statement_sql.push_str(" AND ");
                }
                statement_sql.push_str(sql);
                if let Some(binds) = binds {
                    to_binds.extend(binds.clone());
                }
            }
        }
    }

    if is_statement {
        return (statement_sql, Some(to_binds));
    }

    let mut to_sql = String::new();
    if c.method == Method::Select {
        to_sql.push_str("SELECT ");
    }
    if let Some(select) = &c.select {
        match select {
            Select::Columns(columns) => {
                to_sql.push_str(&columns.join(", "));
            }
            Select::Raw((sql, binds)) => {
                to_sql.push_str(sql);
                if let Some(binds) = binds {
                    to_binds.extend(binds.clone());
                }
            }
            Select::Builder(builder) => {
                let (sql, binds) = builder.delegate_to_sql(true);
                to_sql.push_str(&sql);
                if let Some(binds) = binds {
                    to_binds.extend(binds.clone());
                }
            }
        }
    } else {
        to_sql.push('*');
    }
    to_sql.push_str(" FROM ");
    if let Some(db) = &c.db {
        to_sql.push_str(db);
        to_sql.push('.');
    }
    to_sql.push_str(&c.table);
    if let Some(as_name) = &c.as_name {
        to_sql.push_str(" AS ");
        to_sql.push_str(as_name);
    }
    if !statement_sql.is_empty() {
        to_sql.push_str(" WHERE ");
        to_sql.push_str(&statement_sql);
    }
    if let Some(raw) = &c.raw {
        to_sql.push(' ');
        to_sql.push_str(&raw.0);
        if let Some(binds) = &raw.1 {
            to_binds.extend(binds.clone());
        }
    }
    (to_sql, Some(to_binds))
}
