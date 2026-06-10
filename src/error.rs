//! Typed errors for fallible compilation ([`QueryBuilder::try_to_sql`]).
//!
//! Every fail-loud guard in the compiler maps to one [`BuildError`] variant.
//! [`QueryBuilder::to_sql`] panics with exactly the [`Display`] message of the
//! corresponding variant, so the panicking and fallible paths stay in sync.
//!
//! [`QueryBuilder::try_to_sql`]: crate::QueryBuilder::try_to_sql
//! [`QueryBuilder::to_sql`]: crate::QueryBuilder::to_sql
//! [`Display`]: core::fmt::Display

use core::fmt;

/// Invalid query construction, detected when compiling a
/// [`QueryBuilder`](crate::QueryBuilder).
///
/// Returned by [`try_to_sql`](crate::QueryBuilder::try_to_sql) /
/// [`try_compile`](crate::try_compile) (and the sqlx handoff twins
/// `try_to_sqlx_query` / `try_to_sqlx_query_as`). The panicking
/// [`to_sql`](crate::QueryBuilder::to_sql) path panics with this type's
/// [`Display`](core::fmt::Display) message.
///
/// All variants describe *caller* mistakes (a query that cannot be rendered as
/// valid SQL), never internal compiler state — a web app can therefore map
/// errors caused by end-user input (e.g. an operator or pagination parameter
/// taken from a request) to HTTP 4XX and the rest to 5XX.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum BuildError {
    /// `for_update()`/`for_share()` on a non-SELECT query.
    LockRequiresSelect,
    /// `distinct_on(...)` compiled for a dialect without `DISTINCT ON`.
    DistinctOnRequiresPostgres,
    /// `insert()` with no columns.
    EmptyInsert,
    /// `update()` with no columns and no `SET` expressions.
    EmptyUpdate,
    /// `offset(...)` without `limit(...)`.
    OffsetWithoutLimit,
    /// `for_update()`/`for_share()` combined with `UNION`.
    LockWithUnion,
    /// `having()` received an operator outside the fixed allowlist. Carries the
    /// rejected operator as passed by the caller.
    InvalidHavingOperator(String),
}

impl fmt::Display for BuildError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LockRequiresSelect => {
                f.write_str("for_update()/for_share() is only valid on SELECT")
            }
            Self::DistinctOnRequiresPostgres => f.write_str("DISTINCT ON requires PostgreSQL"),
            Self::EmptyInsert => f.write_str("insert() requires at least one column"),
            Self::EmptyUpdate => f.write_str("update() requires at least one column"),
            Self::OffsetWithoutLimit => f.write_str("offset(...) requires limit(...)"),
            Self::LockWithUnion => {
                f.write_str("for_update()/for_share() cannot be combined with UNION")
            }
            Self::InvalidHavingOperator(op) => write!(
                f,
                "having() operator {op:?} is not an allowed comparison operator \
                 (use having_raw() for arbitrary aggregate expressions)"
            ),
        }
    }
}

impl std::error::Error for BuildError {}

/// Unified error for the execution helpers (`fetch_*`, `execute`, `count`):
/// the query either failed to **build** (invalid construction, see
/// [`BuildError`]) or failed to **execute** (database/driver error from sqlx).
///
/// `From` impls for both sides make `?` work directly in handlers, and
/// matching on the variant separates caller mistakes (often HTTP 4XX when the
/// offending input came from a request) from runtime database failures (5XX):
///
/// ```ignore
/// match qb.fetch_all::<Row, _>(&pool).await {
///     Ok(rows) => ...,
///     Err(Error::Build(e)) => ...,  // invalid query construction
///     Err(Error::Sqlx(e)) => ...,   // connection/query/decode failure
///     Err(e) => ...,                // #[non_exhaustive]: wildcard required downstream
/// }
/// ```
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// The builder could not be compiled into SQL.
    Build(BuildError),
    /// sqlx failed to execute the compiled query.
    Sqlx(sqlx::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Build(e) => e.fmt(f),
            Self::Sqlx(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Build(e) => Some(e),
            Self::Sqlx(e) => Some(e),
        }
    }
}

impl From<BuildError> for Error {
    fn from(e: BuildError) -> Self {
        Self::Build(e)
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::Sqlx(e)
    }
}
