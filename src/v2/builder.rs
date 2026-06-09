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

/// Sort direction for an `ORDER BY` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// Ascending (`ASC`).
    Asc,
    /// Descending (`DESC`).
    Desc,
}

/// The kind of SQL `JOIN`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinKind {
    /// `INNER JOIN`.
    Inner,
    /// `LEFT JOIN`.
    Left,
    /// `RIGHT JOIN`.
    Right,
    /// `FULL OUTER JOIN`.
    FullOuter,
    /// `CROSS JOIN` (no `ON`).
    Cross,
}

/// A single `ON` condition of a [`Join`].
///
/// Columns in `On`/`OnVal` are stored raw and escaped at compile time. `OnRaw`
/// is the verbatim escape hatch (see [`JoinClause::on_raw`]).
#[derive(Debug, Clone, PartialEq)]
pub enum JoinCond {
    /// `lhs op rhs` — both sides are columns (escaped at compile time).
    On(String, &'static str, String),
    /// `col op ?` — `col` escaped, the value is bound.
    OnVal(String, &'static str, Value),
    /// Verbatim SQL with its own binds.
    OnRaw(String, Vec<Value>),
}

/// A `JOIN` clause: a kind, a target table, and zero or more `ON` conditions.
#[derive(Debug, Clone, PartialEq)]
pub struct Join {
    /// The join kind (`INNER`, `LEFT`, …).
    pub kind: JoinKind,
    /// Raw target table identifier (escaped at compile time).
    pub table: String,
    /// `ON` conditions, joined by `AND`. Empty for `CROSS JOIN`.
    pub on: Vec<JoinCond>,
}

/// A `HAVING` condition (SELECT-only, rendered after `GROUP BY`).
#[derive(Debug, Clone, PartialEq)]
pub enum Having {
    /// `col op ?` — `col` is a real column/alias (escaped); value bound.
    Col {
        /// Raw column identifier (escaped at compile time).
        col: String,
        /// SQL operator token (`>`, `=`, …).
        op: String,
        /// Bound value.
        val: Value,
    },
    /// Verbatim aggregate expression with its own binds (e.g. `COUNT(*) > ?`).
    Raw {
        /// Verbatim SQL.
        sql: String,
        /// Bound values appended in order.
        binds: Vec<Value>,
    },
}

/// A common table expression (`WITH` / `WITH RECURSIVE`).
pub struct Cte<D: Dialect> {
    /// Raw CTE name (escaped at compile time).
    pub name: String,
    /// Whether this CTE forces the single `WITH` to carry `RECURSIVE`.
    pub recursive: bool,
    /// The sub-query compiled into the CTE body.
    pub query: QueryBuilder<D>,
}

/// Accumulator passed to `join`/`left_join`/… closures to build `ON` conditions.
///
/// The closure receives an empty `JoinClause`, chains `on`/`on_val`/`on_raw`
/// calls, and returns it; the builder stores the collected conditions.
pub struct JoinClause<D: Dialect> {
    conds: Vec<JoinCond>,
    _marker: PhantomData<D>,
}

impl<D: Dialect> Default for JoinClause<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D: Dialect> JoinClause<D> {
    /// Create an empty accumulator.
    pub fn new() -> Self {
        Self {
            conds: Vec::new(),
            _marker: PhantomData,
        }
    }

    fn into_conds(self) -> Vec<JoinCond> {
        self.conds
    }

    /// `lhs op rhs` — both sides are columns (each escaped at compile time).
    pub fn on(mut self, col: &str, op: &'static str, col2: &str) -> Self {
        self.conds
            .push(JoinCond::On(col.to_owned(), op, col2.to_owned()));
        self
    }

    /// `col op ?` — `col` escaped, the value bound as a placeholder.
    pub fn on_val(mut self, col: &str, op: &'static str, val: impl IntoBind) -> Self {
        self.conds
            .push(JoinCond::OnVal(col.to_owned(), op, val.into_bind()));
        self
    }

    /// Raw `ON` SQL fragment with its own binds — the verbatim escape hatch.
    ///
    /// # Warning: positional placeholder contract
    ///
    /// `sql` is emitted **verbatim** (it is NOT escaped or renumbered) and
    /// `binds` are appended to the running bind list in order. For
    /// **Postgres**, the caller MUST write `$N` numbers matching the actual
    /// bind position — that is, `number of binds already accumulated + 1`, `+2`,
    /// … For MySQL/SQLite use `?`. No renumbering is performed, so a wrong `$N`
    /// produces a malformed query.
    pub fn on_raw(mut self, sql: &str, binds: Vec<Value>) -> Self {
        self.conds.push(JoinCond::OnRaw(sql.to_owned(), binds));
        self
    }
}

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
    pub(crate) joins: Vec<Join>,
    pub(crate) groups: Vec<String>,
    pub(crate) havings: Vec<Having>,
    pub(crate) orders: Vec<(String, Order)>,
    pub(crate) limit: Option<i64>,
    pub(crate) offset: Option<i64>,
    pub(crate) ctes: Vec<Cte<D>>,
    pub(crate) unions: Vec<(bool, QueryBuilder<D>)>,
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
            joins: Vec::new(),
            groups: Vec::new(),
            havings: Vec::new(),
            orders: Vec::new(),
            limit: None,
            offset: None,
            ctes: Vec::new(),
            unions: Vec::new(),
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

    /// Add `GROUP BY` columns (raw owned identifiers, escaped at compile time).
    ///
    /// SELECT-only: ignored for INSERT/UPDATE/DELETE.
    pub fn group_by<I, S>(mut self, cols: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.groups
            .extend(cols.into_iter().map(|c| c.as_ref().to_owned()));
        self
    }

    /// Add an `ORDER BY col <ord>` term. SELECT-only.
    pub fn order_by(mut self, col: &str, ord: Order) -> Self {
        self.orders.push((col.to_owned(), ord));
        self
    }

    /// Add an `ORDER BY col ASC` term. SELECT-only.
    pub fn order_by_asc(self, col: &str) -> Self {
        self.order_by(col, Order::Asc)
    }

    /// Add an `ORDER BY col DESC` term. SELECT-only.
    pub fn order_by_desc(self, col: &str) -> Self {
        self.order_by(col, Order::Desc)
    }

    /// Set `LIMIT n` (bound as a placeholder). SELECT-only.
    pub fn limit(mut self, n: i64) -> Self {
        self.limit = Some(n);
        self
    }

    /// Set `OFFSET n` (bound as a placeholder). SELECT-only.
    ///
    /// `offset` requires `limit`: compiling an offset without a limit panics
    /// (`offset(...) requires limit(...)`), uniform across dialects since MySQL
    /// rejects a bare `OFFSET`.
    pub fn offset(mut self, n: i64) -> Self {
        self.offset = Some(n);
        self
    }

    fn push_join(
        mut self,
        kind: JoinKind,
        table: &str,
        f: impl FnOnce(JoinClause<D>) -> JoinClause<D>,
    ) -> Self {
        let on = f(JoinClause::new()).into_conds();
        self.joins.push(Join {
            kind,
            table: table.to_owned(),
            on,
        });
        self
    }

    /// `INNER JOIN table ON …` — conditions built by the closure.
    ///
    /// SELECT-only: ignored for INSERT/UPDATE/DELETE.
    pub fn join(self, table: &str, f: impl FnOnce(JoinClause<D>) -> JoinClause<D>) -> Self {
        self.push_join(JoinKind::Inner, table, f)
    }

    /// `LEFT JOIN table ON …`. SELECT-only.
    pub fn left_join(self, table: &str, f: impl FnOnce(JoinClause<D>) -> JoinClause<D>) -> Self {
        self.push_join(JoinKind::Left, table, f)
    }

    /// `RIGHT JOIN table ON …`. SELECT-only.
    pub fn right_join(self, table: &str, f: impl FnOnce(JoinClause<D>) -> JoinClause<D>) -> Self {
        self.push_join(JoinKind::Right, table, f)
    }

    /// `FULL OUTER JOIN table ON …`. SELECT-only.
    pub fn full_outer_join(
        self,
        table: &str,
        f: impl FnOnce(JoinClause<D>) -> JoinClause<D>,
    ) -> Self {
        self.push_join(JoinKind::FullOuter, table, f)
    }

    /// `CROSS JOIN table` — takes **no** `ON` closure (a cross join has no
    /// condition). SELECT-only.
    pub fn cross_join(mut self, table: &str) -> Self {
        self.joins.push(Join {
            kind: JoinKind::Cross,
            table: table.to_owned(),
            on: Vec::new(),
        });
        self
    }

    /// `HAVING col op ?` — `col` is a real column/alias (escaped); value bound.
    ///
    /// For aggregate expressions like `COUNT(*) > ?`, use [`Self::having_raw`].
    /// SELECT-only: ignored for INSERT/UPDATE/DELETE. Multiple HAVING terms are
    /// joined by `AND`.
    pub fn having(mut self, col: &str, op: &str, val: impl IntoBind) -> Self {
        self.havings.push(Having::Col {
            col: col.to_owned(),
            op: op.to_owned(),
            val: val.into_bind(),
        });
        self
    }

    /// Raw `HAVING` expression with its own binds — the verbatim escape hatch
    /// for aggregates (e.g. `having_raw("COUNT(*) > ?", …)`).
    ///
    /// # Warning: positional placeholder contract
    ///
    /// `sql` is emitted **verbatim** (it is NOT escaped or renumbered) and
    /// `binds` are appended to the running bind list in order. For
    /// **Postgres**, the caller MUST write `$N` numbers matching the actual
    /// bind position — that is, `number of binds already accumulated + 1`, `+2`,
    /// … For MySQL/SQLite use `?`. No renumbering is performed, so a wrong `$N`
    /// produces a malformed query.
    pub fn having_raw(mut self, sql: &str, binds: Vec<Value>) -> Self {
        self.havings.push(Having::Raw {
            sql: sql.to_owned(),
            binds,
        });
        self
    }

    /// Add a `WITH name AS (query)` common table expression. SELECT-only.
    ///
    /// CTE bodies are compiled before the main query, so their binds (and pg
    /// `$N` numbers) appear first.
    pub fn with(mut self, name: &str, query: QueryBuilder<D>) -> Self {
        self.ctes.push(Cte {
            name: name.to_owned(),
            recursive: false,
            query,
        });
        self
    }

    /// Add a recursive CTE. If any CTE is recursive, the single `WITH` carries
    /// `RECURSIVE`. SELECT-only.
    pub fn with_recursive(mut self, name: &str, query: QueryBuilder<D>) -> Self {
        self.ctes.push(Cte {
            name: name.to_owned(),
            recursive: true,
            query,
        });
        self
    }

    /// Append a `UNION query` arm. SELECT-only.
    pub fn union(mut self, query: QueryBuilder<D>) -> Self {
        self.unions.push((false, query));
        self
    }

    /// Append a `UNION ALL query` arm. SELECT-only.
    pub fn union_all(mut self, query: QueryBuilder<D>) -> Self {
        self.unions.push((true, query));
        self
    }

    /// Compile to `(sql, binds)`.
    pub fn to_sql(&self) -> (String, Vec<Value>) {
        compile(self)
    }
}
