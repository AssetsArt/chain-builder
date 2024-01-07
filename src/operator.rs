#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum Operator {
    Equal,
    NotEqual,
    In,
    NotIn,
    IsNull,
    IsNotNull,
    Exists,
    NotExists,
    Between,
    NotBetween,
    Like,
    NotLike,
}
