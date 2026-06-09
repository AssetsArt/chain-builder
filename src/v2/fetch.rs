//! Typed fetch / execution helpers for v2 (`feature = "sqlx_*"`).
//!
//! These are thin async delegations on top of the sqlx query objects already
//! produced by [`to_sqlx_query`](QueryBuilder::to_sqlx_query) and
//! [`to_sqlx_query_as`](QueryBuilder::to_sqlx_query_as) in
//! [`crate::v2::sqlx_bind`]. They let any [`QueryBuilder<D>`] whose
//! `D: SqlxDialect` run against a sqlx [`Executor`](sqlx::Executor) and decode
//! rows into a `T: FromRow`, or pull a single scalar column.
//!
//! [`count`](QueryBuilder::count) mirrors 1.x `ChainBuilder::count`: it wraps the
//! built SQL in `SELECT COUNT(*) FROM (<sql>) AS __cb_count`, binds the same
//! arguments, and fetches a single `i64`.

use crate::v2::builder::QueryBuilder;
use crate::v2::sqlx_bind::SqlxDialect;

impl<D: SqlxDialect> QueryBuilder<D> {
    /// Fetch all rows, decoding each into `T`.
    ///
    /// Delegates to `self.to_sqlx_query_as::<T>().fetch_all(executor)`.
    pub async fn fetch_all<'e, T, E>(&self, executor: E) -> Result<Vec<T>, sqlx::Error>
    where
        T: for<'r> sqlx::FromRow<'r, <D::Database as sqlx::Database>::Row> + Send + Unpin,
        E: sqlx::Executor<'e, Database = D::Database>,
    {
        self.to_sqlx_query_as::<T>().fetch_all(executor).await
    }

    /// Fetch exactly one row, decoding it into `T`.
    ///
    /// Errors with [`sqlx::Error::RowNotFound`] if no row is returned.
    pub async fn fetch_one<'e, T, E>(&self, executor: E) -> Result<T, sqlx::Error>
    where
        T: for<'r> sqlx::FromRow<'r, <D::Database as sqlx::Database>::Row> + Send + Unpin,
        E: sqlx::Executor<'e, Database = D::Database>,
    {
        self.to_sqlx_query_as::<T>().fetch_one(executor).await
    }

    /// Fetch at most one row, decoding it into `T`.
    pub async fn fetch_optional<'e, T, E>(
        &self,
        executor: E,
    ) -> Result<Option<T>, sqlx::Error>
    where
        T: for<'r> sqlx::FromRow<'r, <D::Database as sqlx::Database>::Row> + Send + Unpin,
        E: sqlx::Executor<'e, Database = D::Database>,
    {
        self.to_sqlx_query_as::<T>().fetch_optional(executor).await
    }

    /// Execute the query (INSERT / UPDATE / DELETE / DDL), returning the
    /// database's `QueryResult` (affected rows, last insert id, …).
    ///
    /// Delegates to `self.to_sqlx_query().execute(executor)`.
    pub async fn execute<'e, E>(
        &self,
        executor: E,
    ) -> Result<<D::Database as sqlx::Database>::QueryResult, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = D::Database>,
    {
        self.to_sqlx_query().execute(executor).await
    }

    /// Count the rows the query would return.
    ///
    /// Wraps the built SQL as `SELECT COUNT(*) FROM (<sql>) AS __cb_count`, binds
    /// the same arguments, and fetches a single `i64`. Mirrors 1.x
    /// `ChainBuilder::count`.
    pub async fn count<'e, E>(&self, executor: E) -> Result<i64, sqlx::Error>
    where
        E: sqlx::Executor<'e, Database = D::Database>,
        // `query_scalar_with` decodes `(i64,)` from the row, which needs `i64` to
        // be decodable for this database and a positional column index. These hold
        // for every sqlx built-in backend but are not implied by `SqlxDialect`.
        i64: for<'r> sqlx::Decode<'r, D::Database> + sqlx::Type<D::Database>,
        usize: sqlx::ColumnIndex<<D::Database as sqlx::Database>::Row>,
    {
        let (sql, binds) = self.to_sql();
        let wrapped = format!("SELECT COUNT(*) FROM ({sql}) AS __cb_count");
        let args = D::bind_arguments(&binds);
        sqlx::query_scalar_with::<D::Database, i64, _>(sqlx::AssertSqlSafe(wrapped), args)
            .fetch_one(executor)
            .await
    }

    /// Fetch the first column of the first row, decoded into `T`.
    ///
    /// Errors with [`sqlx::Error::RowNotFound`] if no row is returned. Useful for
    /// `pluck`/aggregate-style queries.
    pub async fn fetch_scalar<'e, T, E>(&self, executor: E) -> Result<T, sqlx::Error>
    where
        T: for<'r> sqlx::Decode<'r, D::Database> + sqlx::Type<D::Database> + Send + Unpin,
        E: sqlx::Executor<'e, Database = D::Database>,
        // `query_scalar_with` decodes `(T,)`, which requires a positional column
        // index on the backend's row type.
        usize: sqlx::ColumnIndex<<D::Database as sqlx::Database>::Row>,
    {
        let (sql, binds) = self.to_sql();
        let args = D::bind_arguments(&binds);
        sqlx::query_scalar_with::<D::Database, T, _>(sqlx::AssertSqlSafe(sql), args)
            .fetch_one(executor)
            .await
    }

    /// Fetch the first column of the first row (if any), decoded into `T`.
    pub async fn fetch_optional_scalar<'e, T, E>(
        &self,
        executor: E,
    ) -> Result<Option<T>, sqlx::Error>
    where
        T: for<'r> sqlx::Decode<'r, D::Database> + sqlx::Type<D::Database> + Send + Unpin,
        E: sqlx::Executor<'e, Database = D::Database>,
        usize: sqlx::ColumnIndex<<D::Database as sqlx::Database>::Row>,
    {
        let (sql, binds) = self.to_sql();
        let args = D::bind_arguments(&binds);
        sqlx::query_scalar_with::<D::Database, T, _>(sqlx::AssertSqlSafe(sql), args)
            .fetch_optional(executor)
            .await
    }
}
