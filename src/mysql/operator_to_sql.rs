use crate::Operator;

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
