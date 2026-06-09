//! Bound value model for v2.
//!
//! [`Value`] is the dialect-agnostic representation of a bound parameter. The
//! [`IntoBind`] trait converts common Rust types into a [`Value`] so they can be
//! used ergonomically as query bindings.

/// A dialect-agnostic bound parameter value.
///
/// This enum is `#[non_exhaustive]` so new variants can be added without a
/// breaking change; downstream `match`es must include a wildcard arm.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// SQL `NULL`.
    Null,
    /// Boolean.
    Bool(bool),
    /// Signed 64-bit integer (the canonical integer binding type).
    I64(i64),
    /// 64-bit floating point.
    F64(f64),
    /// UTF-8 text.
    Text(String),
    /// Raw bytes / BLOB.
    Bytes(Vec<u8>),
    /// JSON value (requires the `json` feature).
    #[cfg(feature = "json")]
    Json(serde_json::Value),
    /// UUID (requires the `uuid` feature).
    #[cfg(feature = "uuid")]
    Uuid(uuid::Uuid),
    /// Timezone-aware timestamp in UTC (requires the `chrono` feature).
    #[cfg(feature = "chrono")]
    DateTimeUtc(chrono::DateTime<chrono::Utc>),
    /// Naive (timezone-less) date and time (requires the `chrono` feature).
    #[cfg(feature = "chrono")]
    NaiveDateTime(chrono::NaiveDateTime),
    /// Naive date (requires the `chrono` feature).
    #[cfg(feature = "chrono")]
    NaiveDate(chrono::NaiveDate),
    /// Naive time of day (requires the `chrono` feature).
    #[cfg(feature = "chrono")]
    NaiveTime(chrono::NaiveTime),
    /// Fixed-point decimal for money/exact-numeric columns (requires the
    /// `decimal` feature). Bound natively on Postgres/MySQL (`NUMERIC`/`DECIMAL`)
    /// and as TEXT on SQLite (which has no native decimal type).
    #[cfg(feature = "decimal")]
    Decimal(rust_decimal::Decimal),
}

/// Conversion of a Rust value into a bound [`Value`].
pub trait IntoBind {
    /// Convert `self` into a [`Value`].
    fn into_bind(self) -> Value;
}

macro_rules! impl_into_bind_i64 {
    ($($t:ty),* $(,)?) => {
        $(
            impl IntoBind for $t {
                fn into_bind(self) -> Value {
                    Value::I64(self as i64)
                }
            }
        )*
    };
}

// Integers that always fit losslessly in i64.
impl_into_bind_i64!(i8, i16, i32, i64, u8, u16, u32);

// Wider/unsigned integers: narrowed via `as i64`.
//
// Values above `i64::MAX` wrap per Rust's `as` truncation semantics rather
// than erroring or saturating. For example `u64::MAX` becomes `-1`, and
// `(i64::MAX as u64) + 1` becomes `i64::MIN`. This is intentional: the
// canonical integer binding type is `i64`, and these conversions are provided
// for ergonomics. Callers binding values above `i64::MAX` must account for the
// wrap (or bind as `Value::Text` / a wider numeric type explicitly).
//
// (A `//` comment rather than `///` because rustdoc cannot attach a doc comment
// to a macro invocation; the wrap is also asserted in the test module below.)
impl_into_bind_i64!(u64, usize, isize);

impl IntoBind for f32 {
    fn into_bind(self) -> Value {
        Value::F64(self as f64)
    }
}

impl IntoBind for f64 {
    fn into_bind(self) -> Value {
        Value::F64(self)
    }
}

impl IntoBind for bool {
    fn into_bind(self) -> Value {
        Value::Bool(self)
    }
}

impl IntoBind for &str {
    fn into_bind(self) -> Value {
        Value::Text(self.to_owned())
    }
}

impl IntoBind for String {
    fn into_bind(self) -> Value {
        Value::Text(self)
    }
}

impl IntoBind for &String {
    fn into_bind(self) -> Value {
        Value::Text(self.clone())
    }
}

impl IntoBind for Vec<u8> {
    fn into_bind(self) -> Value {
        Value::Bytes(self)
    }
}

impl IntoBind for &[u8] {
    fn into_bind(self) -> Value {
        Value::Bytes(self.to_vec())
    }
}

impl IntoBind for Value {
    fn into_bind(self) -> Value {
        self
    }
}

impl<T: IntoBind> IntoBind for Option<T> {
    fn into_bind(self) -> Value {
        match self {
            None => Value::Null,
            Some(inner) => inner.into_bind(),
        }
    }
}

#[cfg(feature = "json")]
impl IntoBind for serde_json::Value {
    fn into_bind(self) -> Value {
        Value::Json(self)
    }
}

#[cfg(feature = "uuid")]
impl IntoBind for uuid::Uuid {
    fn into_bind(self) -> Value {
        Value::Uuid(self)
    }
}

#[cfg(feature = "chrono")]
impl IntoBind for chrono::DateTime<chrono::Utc> {
    fn into_bind(self) -> Value {
        Value::DateTimeUtc(self)
    }
}

#[cfg(feature = "chrono")]
impl IntoBind for chrono::NaiveDateTime {
    fn into_bind(self) -> Value {
        Value::NaiveDateTime(self)
    }
}

#[cfg(feature = "chrono")]
impl IntoBind for chrono::NaiveDate {
    fn into_bind(self) -> Value {
        Value::NaiveDate(self)
    }
}

#[cfg(feature = "chrono")]
impl IntoBind for chrono::NaiveTime {
    fn into_bind(self) -> Value {
        Value::NaiveTime(self)
    }
}

#[cfg(feature = "decimal")]
impl IntoBind for rust_decimal::Decimal {
    fn into_bind(self) -> Value {
        Value::Decimal(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_become_i64() {
        assert_eq!(7i8.into_bind(), Value::I64(7));
        assert_eq!(7i16.into_bind(), Value::I64(7));
        assert_eq!(7i32.into_bind(), Value::I64(7));
        assert_eq!(7i64.into_bind(), Value::I64(7));
        assert_eq!(7u8.into_bind(), Value::I64(7));
        assert_eq!(7u16.into_bind(), Value::I64(7));
        assert_eq!(7u32.into_bind(), Value::I64(7));
        assert_eq!(7u64.into_bind(), Value::I64(7));
        assert_eq!(7usize.into_bind(), Value::I64(7));
        assert_eq!(7isize.into_bind(), Value::I64(7));
    }

    #[test]
    fn u64_above_i64_max_wraps_intentionally() {
        // Values above i64::MAX truncate via `as i64` (documented behaviour).
        assert_eq!(((i64::MAX as u64) + 1).into_bind(), Value::I64(i64::MIN));
        assert_eq!(u64::MAX.into_bind(), Value::I64(-1));
    }

    #[test]
    fn floats_become_f64() {
        assert_eq!(1.5f32.into_bind(), Value::F64(1.5));
        assert_eq!(1.5f64.into_bind(), Value::F64(1.5));
    }

    #[test]
    fn bool_becomes_bool() {
        assert_eq!(true.into_bind(), Value::Bool(true));
        assert_eq!(false.into_bind(), Value::Bool(false));
    }

    #[test]
    fn strings_become_text() {
        assert_eq!("hi".into_bind(), Value::Text("hi".to_string()));
        assert_eq!(
            String::from("hi").into_bind(),
            Value::Text("hi".to_string())
        );
        let owned = String::from("hi");
        assert_eq!((&owned).into_bind(), Value::Text("hi".to_string()));
    }

    #[test]
    fn bytes_become_bytes() {
        assert_eq!(vec![1u8, 2, 3].into_bind(), Value::Bytes(vec![1, 2, 3]));
        let slice: &[u8] = &[1, 2, 3];
        assert_eq!(slice.into_bind(), Value::Bytes(vec![1, 2, 3]));
    }

    #[test]
    fn option_none_becomes_null() {
        assert_eq!(Option::<i64>::None.into_bind(), Value::Null);
    }

    #[test]
    fn option_some_becomes_inner() {
        assert_eq!(Some(5i64).into_bind(), Value::I64(5));
    }

    #[test]
    fn value_into_bind_is_identity() {
        assert_eq!(Value::I64(1).into_bind(), Value::I64(1));
    }
}
