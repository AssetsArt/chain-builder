//! Typed, dialect-aware query builder.
//!
//! [`QueryBuilder`] is parameterized over a [`Dialect`] marker and uses
//! by-value (`self`) chaining: every mutator takes and returns `Self`. The
//! terminal [`QueryBuilder::to_sql`] compiles to `(sql, binds)`.

use core::marker::PhantomData;

use crate::compile::{compile, try_compile};
use crate::dialect::Dialect;
use crate::error::BuildError;
use crate::value::{IntoBind, Value};
use crate::where_::{Conj, Predicate, WhereBuilder};

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
#[derive(Debug, Clone, PartialEq)]
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

/// What to do when an `INSERT` hits a conflict (see [`OnConflict`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictAction {
    /// Skip the conflicting row (`DO NOTHING` / `INSERT IGNORE`).
    DoNothing,
    /// Update the non-target inserted columns from the proposed row
    /// (`DO UPDATE SET … = EXCLUDED.…` / `ON DUPLICATE KEY UPDATE …`).
    Merge,
}

/// An `ON CONFLICT` specification attached to an `INSERT`.
///
/// `targets` are the raw conflict-target column identifiers (escaped at compile
/// time). They are honored by Postgres / SQLite (`OnConflict` style) and
/// **ignored** by MySQL (`OnDuplicateKey` style), which relies on its own
/// unique/primary keys.
#[derive(Debug, Clone, PartialEq)]
pub struct OnConflict {
    /// Raw conflict-target column identifiers.
    pub targets: Vec<String>,
    /// What to do on conflict.
    pub action: ConflictAction,
}

/// A SQL aggregate function for the `select_*` aggregate helpers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggFn {
    /// `COUNT(...)`.
    Count,
    /// `SUM(...)`.
    Sum,
    /// `AVG(...)`.
    Avg,
    /// `MIN(...)`.
    Min,
    /// `MAX(...)`.
    Max,
}

impl AggFn {
    /// The uppercase SQL keyword for this function.
    pub fn as_str(&self) -> &'static str {
        match self {
            AggFn::Count => "COUNT",
            AggFn::Sum => "SUM",
            AggFn::Avg => "AVG",
            AggFn::Min => "MIN",
            AggFn::Max => "MAX",
        }
    }
}

/// A structured `SELECT`-list expression (aggregate or aliased column).
///
/// The `col`/`alias` identifiers are stored raw and escaped at compile time
/// (a `*` column is emitted unescaped, e.g. `COUNT(*)`). Backs the `select_count`
/// / `select_sum` / … and `select_as` helpers.
#[derive(Debug, Clone, PartialEq)]
pub enum SelectExpr {
    /// `FUNC(col)` with an optional `AS alias` — e.g. `COUNT(*) AS "total"`.
    Agg {
        /// The aggregate function.
        func: AggFn,
        /// Raw column identifier (escaped at compile time; `*` passed through).
        col: String,
        /// Optional alias (escaped at compile time).
        alias: Option<String>,
    },
    /// `col AS alias` — both identifiers escaped at compile time.
    ColAs {
        /// Raw column identifier (escaped at compile time).
        col: String,
        /// Alias (escaped at compile time).
        alias: String,
    },
}

/// A structured or raw `SET` expression for UPDATE. Backs
/// [`QueryBuilder::set_raw`] / [`QueryBuilder::increment`] /
/// [`QueryBuilder::decrement`]. Rendered after the (sorted) structured
/// `update()` columns, in call order.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SetExpr {
    /// `{esc col} = {expr verbatim}` with its own binds (NOT escaped or
    /// renumbered — the `where_raw` contract).
    Raw {
        /// Column identifier; escaped in compile.rs.
        col: String,
        /// Verbatim RHS expression.
        expr: String,
        /// Binds appended to the running bind list in order.
        binds: Vec<Value>,
    },
    /// `{esc col} = {esc col} {+|-} {placeholder}` — structured step.
    Step {
        /// Column identifier; escaped in compile.rs.
        col: String,
        /// The step amount, bound via the normal placeholder machinery.
        by: Value,
        /// `false` → `+` (increment); `true` → `-` (decrement).
        neg: bool,
    },
}

/// The strength of a row-locking clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockStrength {
    /// `FOR UPDATE` — exclusive lock.
    Update,
    /// `FOR SHARE` — shared lock.
    Share,
}

/// The optional wait behavior of a row-locking clause.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockWait {
    /// `SKIP LOCKED` — skip rows already locked.
    SkipLocked,
    /// `NOWAIT` — error immediately if a row is already locked.
    NoWait,
}

/// A row-locking clause appended to a `SELECT` (`FOR UPDATE` / `FOR SHARE`,
/// optionally `SKIP LOCKED` / `NOWAIT`).
///
/// Honored by Postgres / MySQL; a **silent no-op on SQLite** (see
/// [`Dialect::supports_row_locking`](crate::Dialect::supports_row_locking)).
/// Compiling panics if a lock is attached to a non-`SELECT` statement (a
/// dangerous silent no-op otherwise) or combined with `UNION` on a locking
/// dialect (invalid SQL).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lock {
    /// `FOR UPDATE` vs `FOR SHARE`.
    pub strength: LockStrength,
    /// Optional `SKIP LOCKED` / `NOWAIT` modifier.
    pub wait: Option<LockWait>,
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
#[derive(Debug, Clone, PartialEq)]
pub struct QueryBuilder<D: Dialect> {
    pub(crate) table: String,
    /// Optional database/schema qualifier (multi-tenant: one connection, many DBs).
    /// When set, prefixes the main table and join tables: `"db"."table"`.
    pub(crate) db: Option<String>,
    pub(crate) select_cols: Vec<String>,
    /// Structured `SELECT` expressions (aggregates / aliased columns), escaped at
    /// compile time. Rendered after `select_cols`. Backs `select_count`/… /
    /// `select_as`.
    pub(crate) select_exprs: Vec<SelectExpr>,
    /// Raw `SELECT` expressions (verbatim, NOT escaped) with their own binds,
    /// appended after `select_cols`. Backs `select_raw`.
    pub(crate) select_raw: Vec<(String, Vec<Value>)>,
    /// Subquery `SELECT` columns: `(alias, sub)` → `(<sub>) AS {esc alias}`,
    /// appended after `select_cols` / `select_raw`. Backs `select_subquery`.
    pub(crate) select_subqueries: Vec<(String, Box<QueryBuilder<D>>)>,
    /// `SELECT DISTINCT` flag (raw; off by default for M1 byte-identity).
    pub(crate) distinct: bool,
    /// `SELECT DISTINCT ON (cols)` columns (raw; Postgres-only).
    pub(crate) distinct_on: Vec<String>,
    pub(crate) wheres: Vec<Predicate<D>>,
    pub(crate) method: Method,
    pub(crate) set: Vec<(String, Value)>,
    /// UPDATE `SET` expressions, rendered after the sorted `set` pairs in
    /// call order. Backs `set_raw`/`increment`/`decrement`.
    pub(crate) set_exprs: Vec<SetExpr>,
    /// Multi-row `INSERT` rows (empty unless `insert_many` was used). Each row is
    /// a `(column, value)` list; columns come from the first row's sorted keys.
    pub(crate) insert_rows: Vec<Vec<(String, Value)>>,
    pub(crate) joins: Vec<Join>,
    pub(crate) groups: Vec<String>,
    /// Raw `GROUP BY` fragment (verbatim) with its own binds, appended after any
    /// structured `groups`.
    pub(crate) group_by_raw: Option<(String, Vec<Value>)>,
    pub(crate) havings: Vec<Having>,
    pub(crate) orders: Vec<(String, Order)>,
    /// Raw `ORDER BY` fragment (verbatim) with its own binds, appended after any
    /// structured `orders`.
    pub(crate) order_by_raw: Option<(String, Vec<Value>)>,
    pub(crate) limit: Option<i64>,
    pub(crate) offset: Option<i64>,
    pub(crate) ctes: Vec<Cte<D>>,
    pub(crate) unions: Vec<(bool, QueryBuilder<D>)>,
    /// `ON CONFLICT` spec for `INSERT` (ignored on UPDATE/DELETE).
    pub(crate) on_conflict: Option<OnConflict>,
    /// `RETURNING` column list (raw; `"*"` emitted unescaped).
    pub(crate) returning: Vec<String>,
    /// Row-locking clause (`FOR UPDATE`/`FOR SHARE`); SELECT-only, no-op on SQLite.
    pub(crate) lock: Option<Lock>,
    /// First misuse detected by a builder method (e.g. a disallowed `having()`
    /// operator). Deferred instead of panicking mid-chain; surfaced by
    /// [`Self::try_to_sql`] as `Err` (and by [`Self::to_sql`] as a panic).
    pub(crate) error: Option<BuildError>,
    _marker: PhantomData<D>,
}

impl<D: Dialect> QueryBuilder<D> {
    /// Start a query against `name`.
    pub fn table(name: &str) -> Self {
        Self {
            table: name.to_owned(),
            db: None,
            select_cols: Vec::new(),
            select_exprs: Vec::new(),
            select_raw: Vec::new(),
            select_subqueries: Vec::new(),
            distinct: false,
            distinct_on: Vec::new(),
            wheres: Vec::new(),
            method: Method::Select,
            set: Vec::new(),
            set_exprs: Vec::new(),
            insert_rows: Vec::new(),
            joins: Vec::new(),
            groups: Vec::new(),
            group_by_raw: None,
            havings: Vec::new(),
            orders: Vec::new(),
            order_by_raw: None,
            limit: None,
            offset: None,
            ctes: Vec::new(),
            unions: Vec::new(),
            on_conflict: None,
            returning: Vec::new(),
            lock: None,
            error: None,
            _marker: PhantomData,
        }
    }

    /// Set the database/schema qualifier (multi-tenant: one connection, many DBs).
    ///
    /// The name prefixes the main table and every join table, escaped per dialect:
    /// `QueryBuilder::<Postgres>::table("users").db("mydb")` →
    /// `… FROM "mydb"."users"`. Matches 1.x `db()`.
    pub fn db(mut self, name: &str) -> Self {
        self.db = Some(name.to_owned());
        self
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

    /// Add a raw `SELECT` expression (verbatim, NOT escaped) with optional binds
    /// — the escape hatch for aggregates/functions like `COUNT(*)`.
    ///
    /// Appended to the column list after any [`Self::select`] columns. Multiple
    /// calls accumulate.
    ///
    /// # Warning: positional placeholder contract
    ///
    /// `sql` is emitted **verbatim** (it is NOT escaped or renumbered) and any
    /// `binds` are appended to the running bind list in order. For **Postgres**,
    /// the caller MUST write `$N` numbers matching the actual bind position. For
    /// MySQL/SQLite use `?`.
    pub fn select_raw(mut self, sql: &str, binds: Option<Vec<Value>>) -> Self {
        self.select_raw
            .push((sql.to_owned(), binds.unwrap_or_default()));
        self
    }

    /// Add a subquery `SELECT` column: emits `(<sub>) AS {alias}` after the
    /// regular columns and any [`Self::select_raw`] expressions.
    ///
    /// The subquery is compiled with placeholder continuity (its binds appear in
    /// `$N` order at the point it is emitted — before the `WHERE` clause, since
    /// the SELECT list is rendered first). SELECT-only.
    pub fn select_subquery(mut self, alias: &str, sub: QueryBuilder<D>) -> Self {
        self.select_subqueries
            .push((alias.to_owned(), Box::new(sub)));
        self
    }

    fn push_agg(mut self, func: AggFn, col: &str, alias: Option<&str>) -> Self {
        self.select_exprs.push(SelectExpr::Agg {
            func,
            col: col.to_owned(),
            alias: alias.map(|a| a.to_owned()),
        });
        self
    }

    /// Add `COUNT(col)` to the SELECT list (`col == "*"` → `COUNT(*)`).
    pub fn select_count(self, col: &str) -> Self {
        self.push_agg(AggFn::Count, col, None)
    }

    /// Add `COUNT(col) AS alias` (both identifiers escaped; `*` passed through).
    pub fn select_count_as(self, col: &str, alias: &str) -> Self {
        self.push_agg(AggFn::Count, col, Some(alias))
    }

    /// Add `SUM(col)` to the SELECT list.
    pub fn select_sum(self, col: &str) -> Self {
        self.push_agg(AggFn::Sum, col, None)
    }

    /// Add `SUM(col) AS alias`.
    pub fn select_sum_as(self, col: &str, alias: &str) -> Self {
        self.push_agg(AggFn::Sum, col, Some(alias))
    }

    /// Add `AVG(col)` to the SELECT list.
    pub fn select_avg(self, col: &str) -> Self {
        self.push_agg(AggFn::Avg, col, None)
    }

    /// Add `AVG(col) AS alias`.
    pub fn select_avg_as(self, col: &str, alias: &str) -> Self {
        self.push_agg(AggFn::Avg, col, Some(alias))
    }

    /// Add `MIN(col)` to the SELECT list.
    pub fn select_min(self, col: &str) -> Self {
        self.push_agg(AggFn::Min, col, None)
    }

    /// Add `MIN(col) AS alias`.
    pub fn select_min_as(self, col: &str, alias: &str) -> Self {
        self.push_agg(AggFn::Min, col, Some(alias))
    }

    /// Add `MAX(col)` to the SELECT list.
    pub fn select_max(self, col: &str) -> Self {
        self.push_agg(AggFn::Max, col, None)
    }

    /// Add `MAX(col) AS alias`.
    pub fn select_max_as(self, col: &str, alias: &str) -> Self {
        self.push_agg(AggFn::Max, col, Some(alias))
    }

    /// Add `col AS alias` to the SELECT list (both identifiers escaped).
    pub fn select_as(mut self, col: &str, alias: &str) -> Self {
        self.select_exprs.push(SelectExpr::ColAs {
            col: col.to_owned(),
            alias: alias.to_owned(),
        });
        self
    }

    /// Emit `SELECT DISTINCT …` (all dialects).
    pub fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }

    /// Emit `SELECT DISTINCT ON (cols) …` — **Postgres only**.
    ///
    /// `cols` are raw identifiers (escaped at compile time). Compiling against a
    /// dialect without `DISTINCT ON` support panics
    /// (`DISTINCT ON requires PostgreSQL`).
    pub fn distinct_on<I, S>(mut self, cols: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.distinct_on = cols.into_iter().map(|c| c.as_ref().to_owned()).collect();
        self.distinct = true;
        self
    }

    /// `col ILIKE val` — dialect-aware case-insensitive match.
    ///
    /// On **Postgres** this compiles to the native `{col} ILIKE {ph}`. On
    /// MySQL/SQLite (no native `ILIKE`) it compiles to
    /// `LOWER({col}) LIKE LOWER({ph})`.
    pub fn where_ilike(mut self, col: &str, val: impl IntoBind) -> Self {
        self.wheres.push(Predicate::ILike {
            col: col.to_owned(),
            val: val.into_bind(),
        });
        self
    }

    /// `col @> val` — JSONB containment.
    ///
    /// **Postgres-specific:** the `@>` operator is emitted verbatim for all
    /// dialects, but is only meaningful on Postgres `jsonb` columns. `val` is
    /// typically a JSON text string (or `Value::Json` behind the `json`
    /// feature).
    pub fn where_jsonb_contains(mut self, col: &str, val: impl IntoBind) -> Self {
        self.wheres.push(Predicate::JsonContains {
            col: col.to_owned(),
            val: val.into_bind(),
        });
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

    /// `lhs op rhs` — compare two column identifiers (both escaped at compile
    /// time), no bind. e.g. `where_column("orders.user_id", "=", "users.id")`.
    pub fn where_column(mut self, lhs: &str, op: &'static str, rhs: &str) -> Self {
        self.wheres.push(Predicate::Column {
            lhs: lhs.to_owned(),
            op,
            rhs: rhs.to_owned(),
        });
        self
    }

    /// `EXISTS (subquery)` — takes an already-built sub-builder by value
    /// (mirrors [`Self::union`] / [`Self::with`]). The sub-query is compiled
    /// with placeholder continuity.
    pub fn where_exists(mut self, sub: QueryBuilder<D>) -> Self {
        self.wheres.push(Predicate::Exists {
            neg: false,
            sub: Box::new(sub),
        });
        self
    }

    /// `NOT EXISTS (subquery)`. See [`Self::where_exists`].
    pub fn where_not_exists(mut self, sub: QueryBuilder<D>) -> Self {
        self.wheres.push(Predicate::Exists {
            neg: true,
            sub: Box::new(sub),
        });
        self
    }

    /// `col IN (subquery)` — takes an already-built sub-builder by value. The
    /// sub-query is compiled with placeholder continuity.
    pub fn where_in_subquery(mut self, col: &str, sub: QueryBuilder<D>) -> Self {
        self.wheres.push(Predicate::InSubquery {
            col: col.to_owned(),
            neg: false,
            sub: Box::new(sub),
        });
        self
    }

    /// `col NOT IN (subquery)`. See [`Self::where_in_subquery`].
    pub fn where_not_in_subquery(mut self, col: &str, sub: QueryBuilder<D>) -> Self {
        self.wheres.push(Predicate::InSubquery {
            col: col.to_owned(),
            neg: true,
            sub: Box::new(sub),
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

    /// Build a multi-row `INSERT` from an iterator of rows, each a sequence of
    /// `(column, value)` pairs.
    ///
    /// The inserted column set is taken from the **first** row's keys (sorted, as
    /// with [`Self::insert`]). For each subsequent row, a value is bound for every
    /// column in that set; a key **missing** in a later row binds `Value::Null`
    /// rather than panicking (DoS-safe, matching the 1.x hardening). Composes with
    /// `on_conflict_*` and `returning`.
    pub fn insert_many<K, V, R, Rows>(mut self, rows: Rows) -> Self
    where
        K: AsRef<str>,
        V: IntoBind,
        R: IntoIterator<Item = (K, V)>,
        Rows: IntoIterator<Item = R>,
    {
        self.method = Method::Insert;
        self.insert_rows = rows
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|(k, v)| (k.as_ref().to_owned(), v.into_bind()))
                    .collect()
            })
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

    /// Set a column to a **verbatim** SQL expression — the UPDATE escape
    /// hatch. Switches the builder to UPDATE, like [`Self::update`] (and
    /// like every method-selecting call, last one wins: switching away from
    /// UPDATE afterwards leaves the expressions ignored).
    ///
    /// `col` is identifier-escaped; `expr` is emitted verbatim with `binds`
    /// appended. Expressions render after the (sorted) structured
    /// [`Self::update`] columns, in call order. Duplicate target columns are
    /// not detected — the database reports them.
    ///
    /// # Warning: positional placeholder contract
    ///
    /// `expr` is NOT escaped or renumbered. For **Postgres**, hand-write
    /// `$N` matching the actual bind position at the point this expression
    /// is reached — `SET` precedes `WHERE`, so that is `(number of
    /// structured SET binds) + (binds of all earlier expressions, counting
    /// 1 per preceding `increment`/`decrement`) + 1`, `+2`, … For
    /// MySQL/SQLite use `?`. A wrong `$N` produces a malformed query.
    ///
    /// ```
    /// use chain_builder::{Postgres, QueryBuilder, Value};
    /// let (sql, binds) = QueryBuilder::<Postgres>::table("t")
    ///     .update([("name", "x")])
    ///     .set_raw("updated_at", "NOW()", vec![])
    ///     .to_sql();
    /// assert_eq!(sql, r#"UPDATE "t" SET "name" = $1, "updated_at" = NOW()"#);
    /// assert_eq!(binds, vec![Value::Text("x".into())]);
    /// ```
    pub fn set_raw(mut self, col: &str, expr: &str, binds: Vec<Value>) -> Self {
        self.method = Method::Update;
        self.set_exprs.push(SetExpr::Raw {
            col: col.to_owned(),
            expr: expr.to_owned(),
            binds,
        });
        self
    }

    /// `col = col + value` — atomic counter increment. Structured (column
    /// escaped, value bound); switches the builder to UPDATE, so
    /// `qb.increment("views", 1)` alone is a valid UPDATE.
    ///
    /// ```
    /// use chain_builder::{Postgres, QueryBuilder, Value};
    /// let (sql, binds) = QueryBuilder::<Postgres>::table("t")
    ///     .increment("views", 1i64)
    ///     .to_sql();
    /// assert_eq!(sql, r#"UPDATE "t" SET "views" = "views" + $1"#);
    /// assert_eq!(binds, vec![Value::I64(1)]);
    /// ```
    pub fn increment(mut self, col: &str, by: impl IntoBind) -> Self {
        self.method = Method::Update;
        self.set_exprs.push(SetExpr::Step {
            col: col.to_owned(),
            by: by.into_bind(),
            neg: false,
        });
        self
    }

    /// `col = col - value` — atomic counter decrement. See
    /// [`Self::increment`].
    pub fn decrement(mut self, col: &str, by: impl IntoBind) -> Self {
        self.method = Method::Update;
        self.set_exprs.push(SetExpr::Step {
            col: col.to_owned(),
            by: by.into_bind(),
            neg: true,
        });
        self
    }

    /// Build a `DELETE`. WHERE still applies.
    pub fn delete(mut self) -> Self {
        self.method = Method::Delete;
        self
    }

    /// On conflict, skip the row (`INSERT`-only; ignored on UPDATE/DELETE).
    ///
    /// `targets` are the conflict-target columns (may be empty).
    ///
    /// - **Postgres / SQLite:** emits `ON CONFLICT ({targets}) DO NOTHING`, or
    ///   bare `ON CONFLICT DO NOTHING` when `targets` is empty.
    /// - **MySQL:** emits `INSERT IGNORE INTO …` (no trailing clause). Note that
    ///   `IGNORE` suppresses *more* than duplicate-key errors (also truncation
    ///   and bad-value coercion) — broader than pg/sqlite `DO NOTHING`.
    pub fn on_conflict_do_nothing<I, S>(mut self, targets: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.on_conflict = Some(OnConflict {
            targets: targets.into_iter().map(|c| c.as_ref().to_owned()).collect(),
            action: ConflictAction::DoNothing,
        });
        self
    }

    /// On conflict, update the non-target inserted columns from the proposed row
    /// (`INSERT`-only; ignored on UPDATE/DELETE).
    ///
    /// - **Postgres / SQLite:** emits
    ///   `ON CONFLICT ({targets}) DO UPDATE SET {c} = EXCLUDED.{c}, …` for every
    ///   inserted column *except* the conflict targets. If `targets` is empty or
    ///   covers all inserted columns (empty SET list), falls back to the
    ///   `DO NOTHING` rendering (pg/sqlite require a target for `DO UPDATE`).
    /// - **MySQL:** the explicit `targets` are **ignored** (MySQL uses its own
    ///   unique/primary keys); emits
    ///   `ON DUPLICATE KEY UPDATE {c} = VALUES({c}), …` for *all* inserted
    ///   columns. `VALUES()` is used for MySQL 5.7/8.x compatibility. Including a
    ///   PK column in the insert set yields a redundant-but-harmless
    ///   `pk = VALUES(pk)`.
    pub fn on_conflict_merge<I, S>(mut self, targets: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.on_conflict = Some(OnConflict {
            targets: targets.into_iter().map(|c| c.as_ref().to_owned()).collect(),
            action: ConflictAction::Merge,
        });
        self
    }

    /// Add a `RETURNING` column list. Works on INSERT / UPDATE / DELETE for
    /// Postgres and SQLite; a `"*"` column is emitted unescaped (`RETURNING *`).
    ///
    /// On **MySQL** this is a silent no-op (MySQL has no `RETURNING`). On
    /// **SQLite** `RETURNING` requires SQLite ≥ 3.35.0 (2021); `supports_returning()`
    /// is a compile-time dialect flag, not a runtime version check.
    pub fn returning<I, S>(mut self, cols: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        self.returning = cols.into_iter().map(|c| c.as_ref().to_owned()).collect();
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

    /// Add a raw `GROUP BY` fragment with its own binds — the verbatim escape
    /// hatch. SELECT-only.
    ///
    /// The fragment is appended after any structured [`Self::group_by`] columns
    /// within the same `GROUP BY` clause (e.g. `GROUP BY "a", <raw>`); if no
    /// structured columns are present it becomes the whole `GROUP BY <raw>`.
    ///
    /// # Warning: positional placeholder contract
    ///
    /// `sql` is emitted **verbatim** (it is NOT escaped or renumbered) and
    /// `binds` are appended to the running bind list in order. For
    /// **Postgres**, the caller MUST write `$N` numbers matching the actual
    /// bind position — that is, `number of binds already accumulated + 1`, `+2`,
    /// … For MySQL/SQLite use `?`. No renumbering is performed, so a wrong `$N`
    /// produces a malformed query.
    pub fn group_by_raw(mut self, sql: &str, binds: Vec<Value>) -> Self {
        self.group_by_raw = Some((sql.to_owned(), binds));
        self
    }

    /// Add a raw `ORDER BY` fragment with its own binds — the verbatim escape
    /// hatch. SELECT-only.
    ///
    /// The fragment is appended after any structured [`Self::order_by`] terms
    /// within the same `ORDER BY` clause (e.g. `ORDER BY "a" ASC, <raw>`); if no
    /// structured terms are present it becomes the whole `ORDER BY <raw>`.
    ///
    /// # Warning: positional placeholder contract
    ///
    /// `sql` is emitted **verbatim** (it is NOT escaped or renumbered) and
    /// `binds` are appended to the running bind list in order. For
    /// **Postgres**, the caller MUST write `$N` numbers matching the actual
    /// bind position — that is, `number of binds already accumulated + 1`, `+2`,
    /// … For MySQL/SQLite use `?`. No renumbering is performed, so a wrong `$N`
    /// produces a malformed query.
    pub fn order_by_raw(mut self, sql: &str, binds: Vec<Value>) -> Self {
        self.order_by_raw = Some((sql.to_owned(), binds));
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

    /// Lock selected rows with `FOR UPDATE`.
    ///
    /// Honored by Postgres / MySQL; a **silent no-op on SQLite**. Preserves any
    /// `SKIP LOCKED` / `NOWAIT` modifier already set. **SELECT-only:** compiling
    /// panics if attached to INSERT/UPDATE/DELETE or combined with `UNION`.
    pub fn for_update(mut self) -> Self {
        let wait = self.lock.and_then(|l| l.wait);
        self.lock = Some(Lock {
            strength: LockStrength::Update,
            wait,
        });
        self
    }

    /// Lock selected rows with `FOR SHARE`.
    ///
    /// Honored by Postgres / MySQL; a **silent no-op on SQLite**. Preserves any
    /// `SKIP LOCKED` / `NOWAIT` modifier already set. **SELECT-only:** compiling
    /// panics if attached to INSERT/UPDATE/DELETE or combined with `UNION`.
    pub fn for_share(mut self) -> Self {
        let wait = self.lock.and_then(|l| l.wait);
        self.lock = Some(Lock {
            strength: LockStrength::Share,
            wait,
        });
        self
    }

    /// Add `SKIP LOCKED` to the row-locking clause (skip already-locked rows).
    ///
    /// If no lock strength was set yet, defaults to `FOR UPDATE`. SELECT-only;
    /// no-op on SQLite.
    pub fn skip_locked(mut self) -> Self {
        let strength = self
            .lock
            .map(|l| l.strength)
            .unwrap_or(LockStrength::Update);
        self.lock = Some(Lock {
            strength,
            wait: Some(LockWait::SkipLocked),
        });
        self
    }

    /// Add `NOWAIT` to the row-locking clause (error if a row is already locked).
    ///
    /// If no lock strength was set yet, defaults to `FOR UPDATE`. SELECT-only;
    /// no-op on SQLite.
    pub fn no_wait(mut self) -> Self {
        let strength = self
            .lock
            .map(|l| l.strength)
            .unwrap_or(LockStrength::Update);
        self.lock = Some(Lock {
            strength,
            wait: Some(LockWait::NoWait),
        });
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
    ///
    /// # Operator allowlist (injection guard)
    ///
    /// Unlike `where_eq`/`where_column`/`JoinClause::on`, which take
    /// `op: &'static str` (so only compile-time literals are accepted), this
    /// method takes `op: &str` for ergonomics. Because `op` is emitted
    /// **verbatim** into the SQL (it is not a bound value and cannot be escaped
    /// without changing its meaning), an attacker-controlled operator would be a
    /// SQL-injection vector. To prevent that, `op` is validated against a fixed
    /// set of comparison operators. A disallowed operator records a deferred
    /// [`BuildError::InvalidHavingOperator`] (the chain stays intact):
    /// [`Self::try_to_sql`] returns it as `Err`, while [`Self::to_sql`] panics
    /// with the same message (fail-loud, like the `offset`/`distinct_on`/lock
    /// guards). The operator is matched case-insensitively and stored trimmed.
    /// For anything outside this set — arbitrary aggregate expressions, custom
    /// operators — use [`Self::having_raw`], the documented verbatim escape
    /// hatch.
    ///
    /// Allowed: `=`, `!=`, `<>`, `>`, `>=`, `<`, `<=`, `LIKE`, `NOT LIKE`.
    pub fn having(mut self, col: &str, op: &str, val: impl IntoBind) -> Self {
        const ALLOWED_HAVING_OPS: &[&str] =
            &["=", "!=", "<>", ">", ">=", "<", "<=", "LIKE", "NOT LIKE"];
        let normalized = op.trim();
        if !ALLOWED_HAVING_OPS
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(normalized))
        {
            // Keep the FIRST error: it points at the original misuse, and a
            // later mistake must not mask it.
            if self.error.is_none() {
                self.error = Some(BuildError::InvalidHavingOperator(op.to_owned()));
            }
            return self;
        }
        self.havings.push(Having::Col {
            col: col.to_owned(),
            op: normalized.to_owned(),
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

    /// Conditionally apply `f` to the builder, keeping the chain intact.
    ///
    /// Returns `f(self)` when `cond` is true, otherwise `self` unchanged. This
    /// lets you add clauses based on a runtime flag without breaking the
    /// by-value chain.
    ///
    /// ```
    /// use chain_builder::{Postgres, QueryBuilder};
    /// let only_active = true;
    /// let (sql, _) = QueryBuilder::<Postgres>::table("users")
    ///     .select(["id"])
    ///     .when(only_active, |q| q.where_eq("status", "active"))
    ///     .to_sql();
    /// assert_eq!(sql, r#"SELECT "id" FROM "users" WHERE "status" = $1"#);
    /// ```
    pub fn when(self, cond: bool, f: impl FnOnce(Self) -> Self) -> Self {
        if cond {
            f(self)
        } else {
            self
        }
    }

    /// Apply `if_true` when `cond` holds, otherwise `if_false`, keeping the
    /// chain intact.
    ///
    /// ```
    /// use chain_builder::{Postgres, QueryBuilder};
    /// let active = false;
    /// let (sql, _) = QueryBuilder::<Postgres>::table("users")
    ///     .select(["id"])
    ///     .when_else(
    ///         active,
    ///         |q| q.where_eq("status", "active"),
    ///         |q| q.where_eq("status", "inactive"),
    ///     )
    ///     .to_sql();
    /// assert_eq!(sql, r#"SELECT "id" FROM "users" WHERE "status" = $1"#);
    /// ```
    pub fn when_else(
        self,
        cond: bool,
        if_true: impl FnOnce(Self) -> Self,
        if_false: impl FnOnce(Self) -> Self,
    ) -> Self {
        if cond {
            if_true(self)
        } else {
            if_false(self)
        }
    }

    /// Apply `LIMIT`/`OFFSET` for a **1-based** page: row window
    /// `[(page-1) * per_page, page * per_page)`.
    ///
    /// Equivalent to `self.limit(per_page).offset((page - 1).max(0) * per_page)`.
    /// A `page < 1` is treated as page 1 (offset 0), so callers never get a
    /// negative offset. SELECT-only, like [`Self::limit`] / [`Self::offset`].
    ///
    /// ```
    /// use chain_builder::{Postgres, QueryBuilder, Value};
    /// let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    ///     .select(["id"])
    ///     .paginate(2, 10)
    ///     .to_sql();
    /// assert_eq!(sql, r#"SELECT "id" FROM "users" LIMIT $1 OFFSET $2"#);
    /// assert_eq!(binds, vec![Value::I64(10), Value::I64(10)]);
    /// ```
    pub fn paginate(self, page: i64, per_page: i64) -> Self {
        self.limit(per_page).offset((page - 1).max(0) * per_page)
    }

    /// Compile to `(sql, binds)`, panicking on an invalid builder.
    ///
    /// Panicking twin of [`Self::try_to_sql`]; the panic message is the
    /// [`BuildError`]'s `Display` text. Prefer [`Self::try_to_sql`] when any
    /// part of the query is driven by runtime input (e.g. an HTTP request), so
    /// misuse maps to an error response instead of a crash.
    pub fn to_sql(&self) -> (String, Vec<Value>) {
        compile(self)
    }

    /// Compile to `(sql, binds)`, or return the [`BuildError`] describing why
    /// the query cannot be rendered (invalid construction such as `offset()`
    /// without `limit()`, an empty `insert()`/`update()`, a row lock outside
    /// SELECT, or a disallowed `having()` operator).
    ///
    /// Errors recorded by builder methods (deferred, chain-preserving) and
    /// errors detected during compilation — including inside nested builders
    /// (CTEs, UNION arms, subqueries) — all surface here.
    pub fn try_to_sql(&self) -> Result<(String, Vec<Value>), BuildError> {
        try_compile(self)
    }

    /// Render the compiled query as a human-readable string for logs and
    /// debugging: the SQL, then one indented line per bind.
    ///
    /// ```
    /// use chain_builder::{Postgres, QueryBuilder};
    /// let qb = QueryBuilder::<Postgres>::table("users")
    ///     .select(["id"])
    ///     .where_eq("status", "active");
    /// assert_eq!(
    ///     qb.to_sql_pretty(),
    ///     "SELECT \"id\" FROM \"users\" WHERE \"status\" = $1\nbinds:\n  $1 = Text(\"active\")"
    /// );
    /// ```
    ///
    /// Bind labels are the dialect placeholder (`$1`, `$2`, … on Postgres);
    /// dialects whose placeholder is a bare `?` get a 1-based ordinal
    /// appended for readability (`?1`, `?2`, …) — the SQL itself still uses
    /// `?`. With zero binds the output is the SQL line only. The output
    /// format is for humans and is **not** a stability contract.
    ///
    /// # Warning
    ///
    /// The output includes the **literal bind values**. Do not log it if
    /// any bind may contain sensitive data (passwords, tokens, PII).
    ///
    /// Panicking twin of [`Self::try_to_sql_pretty`]; the panic message is
    /// the [`BuildError`]'s `Display` text (same policy as
    /// [`Self::to_sql`]).
    pub fn to_sql_pretty(&self) -> String {
        self.try_to_sql_pretty().unwrap_or_else(|e| panic!("{e}"))
    }

    /// Fallible twin of [`Self::to_sql_pretty`]; surfaces the same
    /// [`BuildError`] as [`Self::try_to_sql`]. The same sensitive-data
    /// warning applies.
    pub fn try_to_sql_pretty(&self) -> Result<String, BuildError> {
        use std::fmt::Write as _;
        let (sql, binds) = try_compile(self)?;
        let mut out = sql;
        if !binds.is_empty() {
            out.push_str("\nbinds:");
            let mut label = String::with_capacity(4);
            for (i, b) in binds.iter().enumerate() {
                label.clear();
                D::write_placeholder(&mut label, i + 1);
                if label == "?" {
                    // Infallible for String.
                    let _ = write!(label, "{}", i + 1);
                }
                out.push_str("\n  ");
                out.push_str(&label);
                out.push_str(" = ");
                let _ = write!(out, "{b:?}");
            }
        }
        Ok(out)
    }
}
