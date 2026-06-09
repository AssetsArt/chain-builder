//! Typed, dialect-aware query builder.
//!
//! [`QueryBuilder`] is parameterized over a [`Dialect`] marker and uses
//! by-value (`self`) chaining: every mutator takes and returns `Self`. The
//! terminal [`QueryBuilder::to_sql`] compiles to `(sql, binds)`.

use core::marker::PhantomData;

use crate::v2::compile::compile;
use crate::v2::dialect::Dialect;
use crate::v2::value::{IntoBind, Value};
use crate::v2::where_::{Conj, Predicate, WhereBuilder};

/// Which kind of statement is being built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Method {
    /// `SELECT`.
    #[default]
    Select,
    /// `INSERT`.
    Insert,
    /// `UPDATE`.
    Update,
    /// `DELETE`.
    Delete,
}

/// Typed, dialect-aware SQL query builder.
pub struct QueryBuilder<D: Dialect> {
    pub(crate) table: String,
    pub(crate) select_cols: Vec<String>,
    pub(crate) wheres: Vec<Predicate>,
    pub(crate) method: Method,
    pub(crate) set: Vec<(String, Value)>,
    _marker: PhantomData<D>,
}

impl<D: Dialect> QueryBuilder<D> {
    /// Start a query against `name`.
    pub fn table(name: &str) -> Self {
        Self {
            table: name.to_owned(),
            select_cols: Vec::new(),
            wheres: Vec::new(),
            method: Method::Select,
            set: Vec::new(),
            _marker: PhantomData,
        }
    }

    /// Restrict the selected columns. An empty list selects `*`.
    pub fn select<I, S>(mut self, cols: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.select_cols = cols.into_iter().map(|c| c.as_ref().to_owned()).collect();
        self
    }

    fn binary(mut self, col: &str, op: &'static str, val: impl IntoBind) -> Self {
        self.wheres.push(Predicate::Binary {
            col: col.to_owned(),
            op,
            val: val.into_bind(),
        });
        self
    }

    /// `col = val`.
    pub fn where_eq(self, col: &str, val: impl IntoBind) -> Self {
        self.binary(col, "=", val)
    }

    /// `col != val`.
    pub fn where_ne(self, col: &str, val: impl IntoBind) -> Self {
        self.binary(col, "!=", val)
    }

    /// `col > val`.
    pub fn where_gt(self, col: &str, val: impl IntoBind) -> Self {
        self.binary(col, ">", val)
    }

    /// `col >= val`.
    pub fn where_gte(self, col: &str, val: impl IntoBind) -> Self {
        self.binary(col, ">=", val)
    }

    /// `col < val`.
    pub fn where_lt(self, col: &str, val: impl IntoBind) -> Self {
        self.binary(col, "<", val)
    }

    /// `col <= val`.
    pub fn where_lte(self, col: &str, val: impl IntoBind) -> Self {
        self.binary(col, "<=", val)
    }

    /// `col LIKE val`.
    pub fn where_like(self, col: &str, val: impl IntoBind) -> Self {
        self.binary(col, "LIKE", val)
    }

    fn in_(mut self, col: &str, neg: bool, vals: impl IntoIterator<Item = impl IntoBind>) -> Self {
        self.wheres.push(Predicate::In {
            col: col.to_owned(),
            neg,
            vals: vals.into_iter().map(IntoBind::into_bind).collect(),
        });
        self
    }

    /// `col IN (...)`.
    pub fn where_in(self, col: &str, vals: impl IntoIterator<Item = impl IntoBind>) -> Self {
        self.in_(col, false, vals)
    }

    /// `col NOT IN (...)`.
    pub fn where_not_in(self, col: &str, vals: impl IntoIterator<Item = impl IntoBind>) -> Self {
        self.in_(col, true, vals)
    }

    fn null(mut self, col: &str, neg: bool) -> Self {
        self.wheres.push(Predicate::Null {
            col: col.to_owned(),
            neg,
        });
        self
    }

    /// `col IS NULL`.
    pub fn where_null(self, col: &str) -> Self {
        self.null(col, false)
    }

    /// `col IS NOT NULL`.
    pub fn where_not_null(self, col: &str) -> Self {
        self.null(col, true)
    }

    /// `col BETWEEN lo AND hi`.
    pub fn where_between(mut self, col: &str, lo: impl IntoBind, hi: impl IntoBind) -> Self {
        self.wheres.push(Predicate::Between {
            col: col.to_owned(),
            lo: lo.into_bind(),
            hi: hi.into_bind(),
        });
        self
    }

    /// Raw SQL predicate with its own binds — the verbatim escape hatch.
    ///
    /// # Warning: positional placeholder contract
    ///
    /// `sql` is emitted **verbatim** (it is NOT escaped or renumbered) and
    /// `binds` are appended to the running bind list in order. For
    /// **Postgres**, the caller MUST write `$N` numbers matching the actual
    /// bind position — that is, `number of binds already accumulated + 1`, `+2`,
    /// … For MySQL/SQLite use `?`. No renumbering is performed, so a wrong `$N`
    /// produces a malformed query.
    pub fn where_raw(mut self, sql: &str, binds: Vec<Value>) -> Self {
        self.wheres.push(Predicate::Raw {
            sql: sql.to_owned(),
            binds,
        });
        self
    }

    fn group(
        mut self,
        outer_conj: Conj,
        f: impl FnOnce(WhereBuilder<D>) -> WhereBuilder<D>,
    ) -> Self {
        let preds = f(WhereBuilder::new()).into_preds();
        self.wheres.push(Predicate::Group { outer_conj, preds });
        self
    }

    /// Add a parenthesized `AND (...)` group built by the closure.
    pub fn and_where(self, f: impl FnOnce(WhereBuilder<D>) -> WhereBuilder<D>) -> Self {
        self.group(Conj::And, f)
    }

    /// Add a parenthesized `OR (...)` group built by the closure.
    pub fn or_where(self, f: impl FnOnce(WhereBuilder<D>) -> WhereBuilder<D>) -> Self {
        self.group(Conj::Or, f)
    }

    /// Build an `INSERT` from a single row of `(column, value)` pairs.
    pub fn insert<K, V, I>(mut self, row: I) -> Self
    where
        K: AsRef<str>,
        V: IntoBind,
        I: IntoIterator<Item = (K, V)>,
    {
        self.method = Method::Insert;
        self.set = row
            .into_iter()
            .map(|(k, v)| (k.as_ref().to_owned(), v.into_bind()))
            .collect();
        self
    }

    /// Build an `UPDATE` from `(column, value)` pairs. WHERE still applies.
    pub fn update<K, V, I>(mut self, set: I) -> Self
    where
        K: AsRef<str>,
        V: IntoBind,
        I: IntoIterator<Item = (K, V)>,
    {
        self.method = Method::Update;
        self.set = set
            .into_iter()
            .map(|(k, v)| (k.as_ref().to_owned(), v.into_bind()))
            .collect();
        self
    }

    /// Build a `DELETE`. WHERE still applies.
    pub fn delete(mut self) -> Self {
        self.method = Method::Delete;
        self
    }

    /// Compile to `(sql, binds)`.
    pub fn to_sql(&self) -> (String, Vec<Value>) {
        compile(self)
    }
}
