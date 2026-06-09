//! SQL compilation: turn a [`QueryBuilder`] into `(sql, binds)`.
//!
//! Values are never inlined — each value pushes onto a running `binds` vector
//! and emits a placeholder via [`Dialect::write_placeholder`] with a 1-based
//! counter, so Postgres yields `$1..$n` in first-appearance order (including
//! across nested groups) while MySQL/SQLite yield `?`.

use crate::v2::builder::{Method, QueryBuilder};
use crate::v2::dialect::Dialect;
use crate::v2::ident::escape_identifier;
use crate::v2::value::Value;
use crate::v2::where_::{Conj, Predicate};

/// Escape a single identifier for SQL output.
///
/// This is the ONLY place identifiers are turned into SQL in v2. Every
/// identifier→SQL site in this module routes through `esc`, so
/// `grep 'esc(' src/v2/compile.rs` is the complete inventory of identifier
/// writes. (The sole exception is [`Predicate::Raw`], which is the documented
/// verbatim escape hatch and is emitted unescaped.)
fn esc(ident: &str, quote: char) -> String {
    escape_identifier(ident, quote)
}

/// Accumulates the generated SQL and the ordered bind values.
struct Ctx {
    sql: String,
    binds: Vec<Value>,
    quote: char,
}

impl Ctx {
    fn new(quote: char) -> Self {
        Self {
            sql: String::new(),
            binds: Vec::new(),
            quote,
        }
    }

    /// Push a value and emit its placeholder (1-based = len after push).
    fn placeholder<D: Dialect>(&mut self, val: Value) {
        self.binds.push(val);
        D::write_placeholder(&mut self.sql, self.binds.len());
    }
}

/// Compile a [`QueryBuilder`] into `(sql, binds)`.
pub fn compile<D: Dialect>(qb: &QueryBuilder<D>) -> (String, Vec<Value>) {
    let quote = D::quote_char();
    let mut ctx = Ctx::new(quote);
    let table = esc(&qb.table, quote);

    match qb.method {
        Method::Select => {
            ctx.sql.push_str("SELECT ");
            if qb.select_cols.is_empty() {
                ctx.sql.push('*');
            } else {
                let cols: Vec<String> =
                    qb.select_cols.iter().map(|c| esc(c, quote)).collect();
                ctx.sql.push_str(&cols.join(", "));
            }
            ctx.sql.push_str(" FROM ");
            ctx.sql.push_str(&table);
            write_wheres::<D>(&mut ctx, &qb.wheres);
        }
        Method::Insert => {
            if qb.set.is_empty() {
                panic!("insert() requires at least one column");
            }
            let mut rows: Vec<&(String, Value)> = qb.set.iter().collect();
            rows.sort_by(|a, b| a.0.cmp(&b.0));
            ctx.sql.push_str("INSERT INTO ");
            ctx.sql.push_str(&table);
            ctx.sql.push_str(" (");
            let cols: Vec<String> = rows.iter().map(|(k, _)| esc(k, quote)).collect();
            ctx.sql.push_str(&cols.join(", "));
            ctx.sql.push_str(") VALUES (");
            for (i, (_, v)) in rows.iter().enumerate() {
                if i > 0 {
                    ctx.sql.push_str(", ");
                }
                ctx.placeholder::<D>(v.clone());
            }
            ctx.sql.push(')');
        }
        Method::Update => {
            if qb.set.is_empty() {
                panic!("update() requires at least one column");
            }
            let mut rows: Vec<&(String, Value)> = qb.set.iter().collect();
            rows.sort_by(|a, b| a.0.cmp(&b.0));
            ctx.sql.push_str("UPDATE ");
            ctx.sql.push_str(&table);
            ctx.sql.push_str(" SET ");
            for (i, (k, v)) in rows.iter().enumerate() {
                if i > 0 {
                    ctx.sql.push_str(", ");
                }
                ctx.sql.push_str(&esc(k, quote));
                ctx.sql.push_str(" = ");
                ctx.placeholder::<D>(v.clone());
            }
            write_wheres::<D>(&mut ctx, &qb.wheres);
        }
        Method::Delete => {
            ctx.sql.push_str("DELETE FROM ");
            ctx.sql.push_str(&table);
            write_wheres::<D>(&mut ctx, &qb.wheres);
        }
    }

    (ctx.sql, ctx.binds)
}

/// A predicate produces no SQL if it is an empty group (F4): an empty group
/// would emit invalid `()`, so it is skipped entirely (and must not leave a
/// dangling `AND`/`OR` separator behind it).
fn is_omitted(p: &Predicate) -> bool {
    matches!(p, Predicate::Group { preds, .. } if preds.is_empty())
}

fn write_wheres<D: Dialect>(ctx: &mut Ctx, wheres: &[Predicate]) {
    // Skip empty groups so they neither emit `()` nor force a `WHERE`.
    if wheres.iter().all(is_omitted) {
        return;
    }
    ctx.sql.push_str(" WHERE ");
    write_clause_list::<D>(ctx, wheres);
}

/// Render a top-level clause list. Predicates are joined by `AND` by default,
/// but a [`Predicate::Group`] attaches to the preceding clause using its own
/// outer conjunction (so `or_where` emits `... OR (...)`). Empty groups are
/// omitted and never contribute a separator.
fn write_clause_list<D: Dialect>(ctx: &mut Ctx, preds: &[Predicate]) {
    let mut wrote_any = false;
    for p in preds.iter() {
        if is_omitted(p) {
            continue;
        }
        if wrote_any {
            let sep = match p {
                Predicate::Group {
                    outer_conj: Conj::Or,
                    ..
                } => " OR ",
                _ => " AND ",
            };
            ctx.sql.push_str(sep);
        }
        write_pred::<D>(ctx, p);
        wrote_any = true;
    }
}

/// Render a list of predicates joined by `conj` (used inside groups).
fn write_preds<D: Dialect>(ctx: &mut Ctx, preds: &[Predicate], conj: Conj) {
    let sep = match conj {
        Conj::And => " AND ",
        Conj::Or => " OR ",
    };
    for (i, p) in preds.iter().enumerate() {
        if i > 0 {
            ctx.sql.push_str(sep);
        }
        write_pred::<D>(ctx, p);
    }
}

fn write_pred<D: Dialect>(ctx: &mut Ctx, pred: &Predicate) {
    match pred {
        Predicate::Binary { col, op, val } => {
            ctx.sql.push_str(&esc(col, ctx.quote));
            ctx.sql.push(' ');
            ctx.sql.push_str(op);
            ctx.sql.push(' ');
            ctx.placeholder::<D>(val.clone());
        }
        Predicate::In { col, neg, vals } => {
            if vals.is_empty() {
                // Empty IN is always false; empty NOT IN is always true.
                ctx.sql.push_str(if *neg { "1 = 1" } else { "1 = 0" });
                return;
            }
            ctx.sql.push_str(&esc(col, ctx.quote));
            ctx.sql.push_str(if *neg { " NOT IN (" } else { " IN (" });
            for (i, v) in vals.iter().enumerate() {
                if i > 0 {
                    ctx.sql.push_str(", ");
                }
                ctx.placeholder::<D>(v.clone());
            }
            ctx.sql.push(')');
        }
        Predicate::Null { col, neg } => {
            ctx.sql.push_str(&esc(col, ctx.quote));
            ctx.sql
                .push_str(if *neg { " IS NOT NULL" } else { " IS NULL" });
        }
        Predicate::Between { col, lo, hi } => {
            ctx.sql.push_str(&esc(col, ctx.quote));
            ctx.sql.push_str(" BETWEEN ");
            ctx.placeholder::<D>(lo.clone());
            ctx.sql.push_str(" AND ");
            ctx.placeholder::<D>(hi.clone());
        }
        Predicate::Raw { sql, binds } => {
            // Verbatim escape hatch: SQL is NOT escaped (see `where_raw` docs).
            ctx.sql.push_str(sql);
            ctx.binds.extend(binds.iter().cloned());
        }
        Predicate::Group {
            outer_conj: _,
            preds,
        } => {
            // `outer_conj` controls how the group attaches to the preceding
            // clause (handled in `write_clause_list`). Inner predicates are
            // ALWAYS joined by `Conj::And` here: this is intentional for M1 —
            // groups are flat AND-lists, and nested groups / inner-OR are a
            // documented M1 limitation (see `WhereBuilder` docs, TG4).
            //
            // Empty groups never reach here: `write_clause_list` /
            // `write_wheres` filter them via `is_omitted` (F4), so we never
            // emit invalid `()`.
            ctx.sql.push('(');
            write_preds::<D>(ctx, preds, Conj::And);
            ctx.sql.push(')');
        }
    }
}
