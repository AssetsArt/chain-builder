use crate::{join::JoinStatement, ChainBuilder};
use serde_json::Value;

pub fn join_compiler(chain_builder: &ChainBuilder, prefix: bool) -> (String, Vec<Value>) {
    let mut to_sql_str = String::new();
    let mut to_binds: Vec<serde_json::Value> = vec![];
    for (i, join) in chain_builder.query.join.iter().enumerate() {
        if i > 0 {
            to_sql_str.push(' ');
        }
        if prefix {
            let table = if let Some(db) = &chain_builder.db {
                format!("`{}`.`{}`", db, join.table)
            } else {
                join.table.clone()
            };
            to_sql_str.push_str(&format!("{} {} ON ", join.join_type, table));
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
                    let mut c = chain_builder.clone();
                    c.query.join = vec![*qb.clone()];
                    let sql = join_compiler(&c, false);
                    to_sql_str.push_str(&format!("({})", sql.0));
                    to_binds.extend(sql.1);
                }
                JoinStatement::OnVal(column, operator, value) => {
                    if j > 0 {
                        to_sql_str.push_str(" AND ");
                    }
                    to_sql_str.push_str(format!("{} {} ?", column, operator).as_str());
                    to_binds.push(value.clone());
                }
            }
        }
    }
    (to_sql_str, to_binds)
}
