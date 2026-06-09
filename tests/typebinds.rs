//! M9: feature-gated `uuid` / `chrono` bind types.
//!
//! Each module is gated by its feature so the default test build stays green and
//! byte-identical. Tests assert both that `IntoBind` produces the right `Value`
//! variant (no DB needed) and that the value flows through `to_sql` as a bind.

#[cfg(feature = "uuid")]
mod uuid_tests {
    use chain_builder::{IntoBind, Postgres, QueryBuilder, Value};

    #[test]
    fn uuid_into_bind_is_uuid_variant() {
        let id = uuid::Uuid::nil();
        assert_eq!(id.into_bind(), Value::Uuid(id));
    }

    #[test]
    fn uuid_binds() {
        let id = uuid::Uuid::nil();
        let (sql, binds) = QueryBuilder::<Postgres>::table("t")
            .select(["id"])
            .where_eq("id", id)
            .to_sql();
        assert_eq!(sql, r#"SELECT "id" FROM "t" WHERE "id" = $1"#);
        assert_eq!(binds, vec![Value::Uuid(id)]);
    }

    #[test]
    fn option_uuid_some_and_none() {
        let id = uuid::Uuid::nil();
        assert_eq!(Some(id).into_bind(), Value::Uuid(id));
        assert_eq!(Option::<uuid::Uuid>::None.into_bind(), Value::Null);
    }
}

#[cfg(feature = "chrono")]
mod chrono_tests {
    use chain_builder::{IntoBind, Postgres, QueryBuilder, Value};
    use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};

    fn sample_dt_utc() -> DateTime<Utc> {
        DateTime::<Utc>::from_timestamp(1_700_000_000, 0).unwrap()
    }

    fn sample_naive_date() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 6, 9).unwrap()
    }

    fn sample_naive_time() -> NaiveTime {
        NaiveTime::from_hms_opt(13, 30, 0).unwrap()
    }

    fn sample_naive_datetime() -> NaiveDateTime {
        NaiveDateTime::new(sample_naive_date(), sample_naive_time())
    }

    #[test]
    fn datetime_utc_into_bind() {
        let dt = sample_dt_utc();
        assert_eq!(dt.into_bind(), Value::DateTimeUtc(dt));
    }

    #[test]
    fn naive_datetime_into_bind() {
        let ndt = sample_naive_datetime();
        assert_eq!(ndt.into_bind(), Value::NaiveDateTime(ndt));
    }

    #[test]
    fn naive_date_into_bind() {
        let nd = sample_naive_date();
        assert_eq!(nd.into_bind(), Value::NaiveDate(nd));
    }

    #[test]
    fn naive_time_into_bind() {
        let nt = sample_naive_time();
        assert_eq!(nt.into_bind(), Value::NaiveTime(nt));
    }

    #[test]
    fn datetime_utc_binds() {
        let dt = sample_dt_utc();
        let (sql, binds) = QueryBuilder::<Postgres>::table("t")
            .select(["id"])
            .where_eq("created_at", dt)
            .to_sql();
        assert_eq!(sql, r#"SELECT "id" FROM "t" WHERE "created_at" = $1"#);
        assert_eq!(binds, vec![Value::DateTimeUtc(dt)]);
    }

    #[test]
    fn naive_date_binds() {
        let nd = sample_naive_date();
        let (sql, binds) = QueryBuilder::<Postgres>::table("t")
            .select(["id"])
            .where_eq("day", nd)
            .to_sql();
        assert_eq!(sql, r#"SELECT "id" FROM "t" WHERE "day" = $1"#);
        assert_eq!(binds, vec![Value::NaiveDate(nd)]);
    }

    #[test]
    fn option_chrono_none_becomes_null() {
        assert_eq!(Option::<DateTime<Utc>>::None.into_bind(), Value::Null);
        assert_eq!(Option::<NaiveDate>::None.into_bind(), Value::Null);
    }
}
