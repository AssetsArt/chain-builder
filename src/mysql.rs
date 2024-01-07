use crate::{operator::Operator, ChainBuilder, JoinStatement, Method, Select, Statement};

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
    for (i, statement) in c.query.statement.iter().enumerate() {
        match statement {
            Statement::Value(field, operator, value) => {
                if i > 0 {
                    if let Some(s) = c.query.statement.get(i - 1) {
                        match s {
                            Statement::OrChain(_) => {}
                            _ => {
                                statement_sql.push_str(" AND ");
                            }
                        }
                    }
                }
                statement_sql.push_str(&format!("{} ", field));
                let (operator_str, is_bind) = operator_to_sql(operator);
                if *operator == Operator::Between || *operator == Operator::NotBetween {
                    statement_sql.push_str(&format!("{} ? AND ?", operator_str));
                    for v in value.as_array().unwrap() {
                        to_binds.push(v.clone());
                    }
                } else {
                    statement_sql.push_str(operator_str);
                    if is_bind {
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
                                to_binds.push(v.clone());
                            });
                            statement_sql.push(')');
                        } else {
                            statement_sql.push_str(" ?");
                            to_binds.push(value.clone());
                        }
                    }
                }
            }
            Statement::OrChain(qb) => {
                if i > 0 {
                    statement_sql.push_str(" OR ");
                }
                let mut c = c.clone();
                c.query = *qb.clone();
                let (sql, binds) = to_sql(&c, true);
                if qb.statement.len() > 1 {
                    statement_sql.push_str(&format!("({})", sql));
                } else {
                    statement_sql.push_str(&sql);
                }
                if let Some(binds) = binds {
                    to_binds.extend(binds);
                }
            }
            Statement::SubChain(qb) => {
                if i > 0 {
                    statement_sql.push_str(" AND ");
                }
                let mut c = c.clone();
                c.query = *qb.clone();
                let (sql, binds) = to_sql(&c, true);
                statement_sql.push_str(&format!("({})", sql));
                if let Some(binds) = binds {
                    to_binds.extend(binds);
                }
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

    let mut to_sql_str = String::new();
    if c.method == Method::Select {
        to_sql_str.push_str("SELECT ");
    }
    if let Some(select) = &c.select {
        match select {
            Select::Columns(columns) => {
                to_sql_str.push_str(&columns.join(", "));
            }
            Select::Raw((sql, binds)) => {
                to_sql_str.push_str(sql);
                if let Some(binds) = binds {
                    to_binds.extend(binds.clone());
                }
            }
            Select::Builder(subc) => {
                let (sql, binds) = to_sql(subc, true);
                to_sql_str.push_str(&sql);
                if let Some(binds) = binds {
                    to_binds.extend(binds.clone());
                }
            }
        }
    } else {
        to_sql_str.push('*');
    }
    to_sql_str.push_str(" FROM ");
    if let Some(db) = &c.db {
        to_sql_str.push_str(db);
        to_sql_str.push('.');
    }
    to_sql_str.push_str(&c.table);
    if let Some(as_name) = &c.as_name {
        to_sql_str.push_str(" AS ");
        to_sql_str.push_str(as_name);
    }
    if !c.query.join.is_empty() {
        let (sql, binds) = join_to_sql(c, true);
        to_sql_str.push_str(&format!(" {}", sql));
        if let Some(binds) = binds {
            to_binds.extend(binds.clone());
        }
    }
    if !statement_sql.is_empty() {
        to_sql_str.push_str(" WHERE ");
        to_sql_str.push_str(&statement_sql);
    }
    if let Some(raw) = &c.query.raw {
        to_sql_str.push(' ');
        to_sql_str.push_str(&raw.0);
        if let Some(binds) = &raw.1 {
            to_binds.extend(binds.clone());
        }
    }
    (to_sql_str, Some(to_binds))
}

fn join_to_sql(c: &ChainBuilder, prefix: bool) -> (String, Option<Vec<serde_json::Value>>) {
    let mut to_sql_str = String::new();
    let mut to_binds: Vec<serde_json::Value> = vec![];
    for (i, join) in c.query.join.iter().enumerate() {
        if i > 0 {
            to_sql_str.push(' ');
        } else if prefix {
            to_sql_str.push_str(&format!("{} {} ON ", join.join_type, join.table));
        }
        for (j, statement) in join.statement.iter().enumerate() {
            match statement {
                JoinStatement::On(column, operator, column2) => {
                    if j > 0 {
                        to_sql_str.push_str(" AND ");
                    }
                    to_sql_str.push_str(format!("{} {} {}", column, operator, column2).as_str());
                }
                JoinStatement::OrChain(qb) => {
                    if j > 0 {
                        to_sql_str.push_str(" OR ");
                    }
                    let mut c = c.clone();
                    c.query.join = vec![*qb.clone()];
                    let (sql, binds) = join_to_sql(&c, false);
                    to_sql_str.push_str(&format!("({})", sql));
                    if let Some(binds) = binds {
                        to_binds.extend(binds);
                    }
                }
            }
        }
    }
    (to_sql_str, Some(to_binds))
}
