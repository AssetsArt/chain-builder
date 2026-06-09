//! WHERE-clause predicate model and the nested-group accumulator.
//!
//! [`Predicate`] is the dialect-agnostic AST for a single WHERE condition.
//! [`WhereBuilder`] is a thin accumulator used by `and_where`/`or_where` to
//! collect a nested group of predicates without threading the whole
//! [`QueryBuilder`](crate::QueryBuilder) into the closure.

use core::marker::PhantomData;

use crate::dialect::Dialect;
use crate::value::{IntoBind, Value};

/// How a [`Predicate::Group`] attaches to the clause that precedes it.
///
/// This is the *outer* conjunction (the separator written before the group:
/// `AND (...)` vs `OR (...)`). It does NOT control how predicates *inside* the
/// group are joined — in M1 inner group predicates are ALWAYS joined by `AND`
/// (see [`WhereBuilder`] and `compile::write_pred`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conj {
    /// Attach the group with `AND (...)`.
    And,
    /// Attach the group with `OR (...)`.
    Or,
}

/// A single WHERE-clause predicate.
#[derive(Debug, Clone, PartialEq)]
pub enum Predicate {
    /// `col op value`, e.g. `"age" > $1`.
    Binary {
        /// Raw identifier; escaped in compile.rs.
        col: String,
        /// SQL operator token (`=`, `!=`, `LIKE`, …).
        op: &'static str,
        /// Bound value.
        val: Value,
    },
    /// `col [NOT ]IN (...)`.
    In {
        /// Raw identifier; escaped in compile.rs.
        col: String,
        /// Whether this is a `NOT IN`.
        neg: bool,
        /// Bound values; empty yields a constant true/false predicate.
        vals: Vec<Value>,
    },
    /// `col IS [NOT ]NULL`.
    Null {
        /// Raw identifier; escaped in compile.rs.
        col: String,
        /// Whether this is `IS NOT NULL`.
        neg: bool,
    },
    /// `col BETWEEN lo AND hi`.
    Between {
        /// Raw identifier; escaped in compile.rs.
        col: String,
        /// Lower bound.
        lo: Value,
        /// Upper bound.
        hi: Value,
    },
    /// `col ILIKE val` — dialect-aware case-insensitive match.
    ///
    /// Postgres emits native `{col} ILIKE {ph}`; MySQL/SQLite emit
    /// `LOWER({col}) LIKE LOWER({ph})`. Dispatched in `compile::write_pred`.
    ILike {
        /// Raw identifier; escaped in compile.rs.
        col: String,
        /// Bound value.
        val: Value,
    },
    /// `col @> val` — JSONB containment (Postgres-oriented; `@>` emitted
    /// verbatim for all dialects).
    JsonContains {
        /// Raw identifier; escaped in compile.rs.
        col: String,
        /// Bound value (JSON text or `Value::Json`).
        val: Value,
    },
    /// Raw SQL fragment with its own binds, emitted verbatim.
    Raw {
        /// Verbatim SQL.
        sql: String,
        /// Bound values appended in order.
        binds: Vec<Value>,
    },
    /// A parenthesized group of predicates.
    ///
    /// `outer_conj` controls how the group attaches to the preceding clause
    /// (`AND (...)` vs `OR (...)`). The predicates *inside* the group are
    /// ALWAYS joined by `AND` in M1 — see the [`WhereBuilder`] limitation note.
    Group {
        /// How this group attaches to the preceding clause (outer separator).
        outer_conj: Conj,
        /// Inner predicates (always `AND`-joined in M1).
        preds: Vec<Predicate>,
    },
}

/// Accumulator passed to `and_where`/`or_where` closures.
///
/// It owns its own `Vec<Predicate>`; the closure consumes it and returns it,
/// and the caller wraps the collected predicates into a [`Predicate::Group`].
/// The `PhantomData<D>` keeps the dialect in scope without threading the full
/// builder through the closure.
///
/// # M1 limitation
///
/// Groups in M1 are **non-nestable** and **flat-`AND`**: every predicate added
/// inside the closure is joined to its siblings with `AND`, and there is no way
/// to nest a further group or to `OR` predicates *within* a group. The only
/// `OR` available is the *outer* attachment chosen by `or_where` (which emits
/// `... OR (...)`). Nested groups and inner-`OR` are a documented M1 limitation.
pub struct WhereBuilder<D: Dialect> {
    preds: Vec<Predicate>,
    _marker: PhantomData<D>,
}

impl<D: Dialect> Default for WhereBuilder<D> {
    fn default() -> Self {
        Self::new()
    }
}

impl<D: Dialect> WhereBuilder<D> {
    /// Create an empty accumulator.
    pub fn new() -> Self {
        Self {
            preds: Vec::new(),
            _marker: PhantomData,
        }
    }

    /// Consume the accumulator and return the collected predicates.
    pub(crate) fn into_preds(self) -> Vec<Predicate> {
        self.preds
    }

    fn binary(mut self, col: &str, op: &'static str, val: impl IntoBind) -> Self {
        self.preds.push(Predicate::Binary {
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

    /// `col ILIKE val` — dialect-aware case-insensitive match (see
    /// [`QueryBuilder::where_ilike`](crate::QueryBuilder::where_ilike)).
    pub fn where_ilike(mut self, col: &str, val: impl IntoBind) -> Self {
        self.preds.push(Predicate::ILike {
            col: col.to_owned(),
            val: val.into_bind(),
        });
        self
    }

    /// `col @> val` — JSONB containment (Postgres-oriented; see
    /// [`QueryBuilder::where_jsonb_contains`](crate::QueryBuilder::where_jsonb_contains)).
    pub fn where_jsonb_contains(mut self, col: &str, val: impl IntoBind) -> Self {
        self.preds.push(Predicate::JsonContains {
            col: col.to_owned(),
            val: val.into_bind(),
        });
        self
    }

    fn in_(mut self, col: &str, neg: bool, vals: impl IntoIterator<Item = impl IntoBind>) -> Self {
        self.preds.push(Predicate::In {
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
        self.preds.push(Predicate::Null {
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
        self.preds.push(Predicate::Between {
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
        self.preds.push(Predicate::Raw {
            sql: sql.to_owned(),
            binds,
        });
        self
    }
}
