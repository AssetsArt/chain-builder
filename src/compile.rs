//! SQL compilation: turn a [`QueryBuilder`] into `(sql, binds)`.
//!
//! Values are never inlined — each value pushes onto a running `binds` vector
//! and emits a placeholder via [`Dialect::write_placeholder`] with a 1-based
//! counter, so Postgres yields `$1..$n` in first-appearance order (including
//! across nested groups) while MySQL/SQLite yield `?`.

use crate::builder::{
    ConflictAction, Cte, Having, Join, JoinCond, JoinKind, Lock, LockStrength, LockWait, Method,
    Order, QueryBuilder, SelectExpr,
};
use crate::dialect::{Dialect, UpsertStyle};
use crate::ident::escape_identifier;
use crate::value::Value;
use crate::where_::{Conj, Predicate};

/// Accumulates the generated SQL and the ordered bind values.
///
/// A single `Ctx` is threaded through the whole compilation (see
/// [`compile_into`]): SQL, binds, and the placeholder counter all continue
/// across every clause and across nested builders, which is what guarantees
/// Postgres placeholder continuity (e.g. WHERE `$1` → LIMIT `$2` → OFFSET `$3`).
struct Ctx {
    sql: String,
    binds: Vec<Value>,
    quote: char,
}

impl Ctx {
    /// Push a value and emit its placeholder (1-based = len after push).
    fn placeholder<D: Dialect>(&mut self, val: Value) {
        self.binds.push(val);
        D::write_placeholder(&mut self.sql, self.binds.len());
    }

    /// Escape a single identifier for SQL output.
    ///
    /// This is the ONLY place identifiers are turned into SQL in v2. Every
    /// identifier→SQL site in this module routes through `ctx.esc`, so
    /// `grep 'esc(' src/v2/compile.rs` is the complete inventory of identifier
    /// writes. (The sole exception is [`Predicate::Raw`], which is the
    /// documented verbatim escape hatch and is emitted unescaped.)
    fn esc(&self, ident: &str) -> String {
        escape_identifier(ident, self.quote)
    }

    /// Escape a table, optionally prefixed by a database qualifier:
    /// `Some("db")`, `"t"` → `"db"."t"`; `None` → `"t"`.
    fn qualify(&self, db: Option<&str>, table: &str) -> String {
        match db {
            Some(d) => format!("{}.{}", self.esc(d), self.esc(table)),
            None => self.esc(table),
        }
    }
}

/// Compile a [`QueryBuilder`] into `(sql, binds)`.
pub fn compile<D: Dialect>(qb: &QueryBuilder<D>) -> (String, Vec<Value>) {
    let mut ctx = Ctx {
        sql: String::new(),
        binds: Vec::new(),
        quote: D::quote_char(),
    };
    compile_into::<D>(&mut ctx, qb);
    (ctx.sql, ctx.binds)
}

/// Write `qb`'s SQL into the existing `ctx`, continuing its binds and
/// placeholder counter. This is the single-pass core used by [`compile`] (and,
/// in later M2 tasks, by nested builders such as CTEs/UNION arms).
fn compile_into<D: Dialect>(ctx: &mut Ctx, qb: &QueryBuilder<D>) {
    let table = ctx.qualify(qb.db.as_deref(), &qb.table);

    // A row lock is only meaningful on SELECT. Attaching one to INSERT/UPDATE/
    // DELETE would otherwise be silently dropped — a dangerous no-op for a lock
    // (the caller believes rows are locked when they are not). Fail loud,
    // dialect-independent, like the `offset`/`distinct_on` guards.
    if qb.lock.is_some() && qb.method != Method::Select {
        panic!("for_update()/for_share() is only valid on SELECT");
    }

    match qb.method {
        Method::Select => {
            // CTEs are emitted first so their binds (and pg `$N`) come first.
            write_ctes::<D>(ctx, &qb.ctes);
            if !qb.distinct_on.is_empty() {
                if !D::supports_distinct_on() {
                    panic!("DISTINCT ON requires PostgreSQL");
                }
                ctx.sql.push_str("SELECT DISTINCT ON (");
                let cols: Vec<String> = qb.distinct_on.iter().map(|c| ctx.esc(c)).collect();
                ctx.sql.push_str(&cols.join(", "));
                ctx.sql.push_str(") ");
            } else if qb.distinct {
                ctx.sql.push_str("SELECT DISTINCT ");
            } else {
                ctx.sql.push_str("SELECT ");
            }
            write_select_list::<D>(ctx, qb);
            ctx.sql.push_str(" FROM ");
            ctx.sql.push_str(&table);
            write_joins::<D>(ctx, &qb.joins, qb.db.as_deref());
            write_wheres::<D>(ctx, &qb.wheres);
            write_group_by(ctx, &qb.groups, qb.group_by_raw.as_ref());
            write_having::<D>(ctx, &qb.havings);
            write_order_by(ctx, &qb.orders, qb.order_by_raw.as_ref());
            write_limit_offset::<D>(ctx, qb.limit, qb.offset);
            write_unions::<D>(ctx, &qb.unions);
            write_lock::<D>(ctx, qb.lock.as_ref(), !qb.unions.is_empty());
        }
        Method::Insert => {
            if qb.set.is_empty() && qb.insert_rows.is_empty() {
                panic!("insert() requires at least one column");
            }

            // Single-row sorted pairs (preserves the M-prev path byte-for-byte,
            // including duplicate keys). For multi-row, columns come from the
            // FIRST row's sorted keys.
            let mut single_rows: Vec<&(String, Value)> = qb.set.iter().collect();
            single_rows.sort_by(|a, b| a.0.cmp(&b.0));
            let sorted_cols: Vec<&str> = if !qb.insert_rows.is_empty() {
                let mut cols: Vec<&str> =
                    qb.insert_rows[0].iter().map(|(k, _)| k.as_str()).collect();
                cols.sort_unstable();
                cols
            } else {
                single_rows.iter().map(|(k, _)| k.as_str()).collect()
            };

            // Decide the INSERT keyword up front: MySQL `DO NOTHING` becomes
            // `INSERT IGNORE INTO …` with no trailing conflict clause.
            let mysql_ignore = D::upsert_style() == UpsertStyle::OnDuplicateKey
                && matches!(
                    qb.on_conflict.as_ref().map(|c| c.action),
                    Some(ConflictAction::DoNothing)
                );
            if mysql_ignore {
                ctx.sql.push_str("INSERT IGNORE INTO ");
            } else {
                ctx.sql.push_str("INSERT INTO ");
            }
            ctx.sql.push_str(&table);
            ctx.sql.push_str(" (");
            let cols: Vec<String> = sorted_cols.iter().map(|k| ctx.esc(k)).collect();
            ctx.sql.push_str(&cols.join(", "));
            ctx.sql.push_str(") VALUES ");

            if !qb.insert_rows.is_empty() {
                // Multi-row: one `(…)` tuple per row. A key missing in a row binds
                // `Value::Null` (ragged rows are NULL-padded, never a panic).
                for (ri, row) in qb.insert_rows.iter().enumerate() {
                    if ri > 0 {
                        ctx.sql.push_str(", ");
                    }
                    ctx.sql.push('(');
                    for (ci, col) in sorted_cols.iter().enumerate() {
                        if ci > 0 {
                            ctx.sql.push_str(", ");
                        }
                        let v = row
                            .iter()
                            .find(|(k, _)| k == col)
                            .map(|(_, v)| v.clone())
                            .unwrap_or(Value::Null);
                        ctx.placeholder::<D>(v);
                    }
                    ctx.sql.push(')');
                }
            } else {
                // Single-row: byte-identical to the M-prev path (iterate the
                // sorted (key, value) pairs directly, duplicates and all).
                ctx.sql.push('(');
                for (i, (_, v)) in single_rows.iter().enumerate() {
                    if i > 0 {
                        ctx.sql.push_str(", ");
                    }
                    ctx.placeholder::<D>(v.clone());
                }
                ctx.sql.push(')');
            }

            if !mysql_ignore {
                if let Some(oc) = &qb.on_conflict {
                    write_on_conflict::<D>(ctx, oc, &sorted_cols);
                }
            }
            write_returning::<D>(ctx, &qb.returning);
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
                let col = ctx.esc(k);
                ctx.sql.push_str(&col);
                ctx.sql.push_str(" = ");
                ctx.placeholder::<D>(v.clone());
            }
            write_wheres::<D>(ctx, &qb.wheres);
            write_returning::<D>(ctx, &qb.returning);
        }
        Method::Delete => {
            ctx.sql.push_str("DELETE FROM ");
            ctx.sql.push_str(&table);
            write_wheres::<D>(ctx, &qb.wheres);
            write_returning::<D>(ctx, &qb.returning);
        }
    }
}

/// Render the upsert conflict clause for an `INSERT` (never called for the
/// MySQL `INSERT IGNORE` path, which is handled at the keyword). `inserted` is
/// the sorted-key list of inserted columns.
fn write_on_conflict<D: Dialect>(
    ctx: &mut Ctx,
    oc: &crate::builder::OnConflict,
    inserted: &[&str],
) {
    match D::upsert_style() {
        UpsertStyle::OnDuplicateKey => {
            // MySQL merge: `ON DUPLICATE KEY UPDATE c = VALUES(c), …` for ALL
            // inserted columns (explicit targets are ignored).
            ctx.sql.push_str(" ON DUPLICATE KEY UPDATE ");
            let sets: Vec<String> = inserted
                .iter()
                .map(|c| {
                    let e = ctx.esc(c);
                    format!("{e} = VALUES({e})")
                })
                .collect();
            ctx.sql.push_str(&sets.join(", "));
        }
        UpsertStyle::OnConflict => {
            let targets = &oc.targets;
            // SET list = inserted columns minus the conflict targets.
            let set_cols: Vec<&&str> = inserted
                .iter()
                .filter(|c| !targets.iter().any(|t| t == **c))
                .collect();
            let do_update = matches!(oc.action, ConflictAction::Merge)
                && !targets.is_empty()
                && !set_cols.is_empty();

            ctx.sql.push_str(" ON CONFLICT");
            if !targets.is_empty() {
                ctx.sql.push_str(" (");
                let cols: Vec<String> = targets.iter().map(|t| ctx.esc(t)).collect();
                ctx.sql.push_str(&cols.join(", "));
                ctx.sql.push(')');
            }
            if do_update {
                ctx.sql.push_str(" DO UPDATE SET ");
                let sets: Vec<String> = set_cols
                    .iter()
                    .map(|c| {
                        let e = ctx.esc(c);
                        // `EXCLUDED` is an unquoted, case-insensitive identifier
                        // accepted by both pg and sqlite — emitted literally.
                        format!("{e} = EXCLUDED.{e}")
                    })
                    .collect();
                ctx.sql.push_str(&sets.join(", "));
            } else {
                ctx.sql.push_str(" DO NOTHING");
            }
        }
    }
}

/// Render ` RETURNING col, …` when the dialect supports it and the list is
/// non-empty. A `"*"` column is emitted unescaped. No-op otherwise (e.g. MySQL).
fn write_returning<D: Dialect>(ctx: &mut Ctx, cols: &[String]) {
    if !D::supports_returning() || cols.is_empty() {
        return;
    }
    ctx.sql.push_str(" RETURNING ");
    let parts: Vec<String> = cols
        .iter()
        .map(|c| if c == "*" { "*".to_owned() } else { ctx.esc(c) })
        .collect();
    ctx.sql.push_str(&parts.join(", "));
}

/// Render `GROUP BY a, b, …` (SELECT only). No-op when there are no columns and
/// no raw fragment. A raw fragment is appended (comma-joined) after the
/// structured columns; if only raw is present it becomes the whole clause.
fn write_group_by(ctx: &mut Ctx, groups: &[String], raw: Option<&(String, Vec<Value>)>) {
    if groups.is_empty() && raw.is_none() {
        return;
    }
    ctx.sql.push_str(" GROUP BY ");
    let cols: Vec<String> = groups.iter().map(|c| ctx.esc(c)).collect();
    ctx.sql.push_str(&cols.join(", "));
    if let Some((sql, binds)) = raw {
        if !groups.is_empty() {
            ctx.sql.push_str(", ");
        }
        // Verbatim escape hatch (see `group_by_raw` docs).
        ctx.sql.push_str(sql);
        ctx.binds.extend(binds.iter().cloned());
    }
}

/// Render the SELECT column list: escaped `select_cols`, then verbatim
/// `select_raw` expressions, then `(<subquery>) AS {alias}` columns — in that
/// order. An empty list (no cols, no raw, no subqueries) yields `*`.
///
/// Written directly into `ctx` (not pre-joined) so subquery binds continue the
/// placeholder counter in emission order.
fn write_select_list<D: Dialect>(ctx: &mut Ctx, qb: &QueryBuilder<D>) {
    if qb.select_cols.is_empty()
        && qb.select_exprs.is_empty()
        && qb.select_raw.is_empty()
        && qb.select_subqueries.is_empty()
    {
        ctx.sql.push('*');
        return;
    }
    let mut wrote_any = false;
    for c in &qb.select_cols {
        if wrote_any {
            ctx.sql.push_str(", ");
        }
        let e = ctx.esc(c);
        ctx.sql.push_str(&e);
        wrote_any = true;
    }
    for expr in &qb.select_exprs {
        if wrote_any {
            ctx.sql.push_str(", ");
        }
        match expr {
            SelectExpr::Agg { func, col, alias } => {
                ctx.sql.push_str(func.as_str());
                ctx.sql.push('(');
                // A `*` column is emitted unescaped (`COUNT(*)`).
                if col == "*" {
                    ctx.sql.push('*');
                } else {
                    let c = ctx.esc(col);
                    ctx.sql.push_str(&c);
                }
                ctx.sql.push(')');
                if let Some(a) = alias {
                    let a = ctx.esc(a);
                    ctx.sql.push_str(" AS ");
                    ctx.sql.push_str(&a);
                }
            }
            SelectExpr::ColAs { col, alias } => {
                let c = ctx.esc(col);
                let a = ctx.esc(alias);
                ctx.sql.push_str(&c);
                ctx.sql.push_str(" AS ");
                ctx.sql.push_str(&a);
            }
        }
        wrote_any = true;
    }
    for (sql, binds) in &qb.select_raw {
        if wrote_any {
            ctx.sql.push_str(", ");
        }
        // Verbatim escape hatch (see `select_raw` docs).
        ctx.sql.push_str(sql);
        ctx.binds.extend(binds.iter().cloned());
        wrote_any = true;
    }
    for (alias, sub) in &qb.select_subqueries {
        if wrote_any {
            ctx.sql.push_str(", ");
        }
        ctx.sql.push('(');
        compile_into::<D>(ctx, sub);
        ctx.sql.push_str(") AS ");
        let a = ctx.esc(alias);
        ctx.sql.push_str(&a);
        wrote_any = true;
    }
}

/// Render each `JOIN` (SELECT only): ` {KIND} {esc table}[ ON cond AND …]`.
/// `CROSS JOIN` emits no `ON`. Placeholders from `OnVal`/`OnRaw` continue the
/// running counter.
fn write_joins<D: Dialect>(ctx: &mut Ctx, joins: &[Join], db: Option<&str>) {
    for j in joins {
        let kw = match j.kind {
            JoinKind::Inner => " INNER JOIN ",
            JoinKind::Left => " LEFT JOIN ",
            JoinKind::Right => " RIGHT JOIN ",
            JoinKind::FullOuter => " FULL OUTER JOIN ",
            JoinKind::Cross => " CROSS JOIN ",
        };
        ctx.sql.push_str(kw);
        let table = ctx.qualify(db, &j.table);
        ctx.sql.push_str(&table);
        if j.on.is_empty() {
            continue;
        }
        ctx.sql.push_str(" ON ");
        for (i, cond) in j.on.iter().enumerate() {
            if i > 0 {
                ctx.sql.push_str(" AND ");
            }
            match cond {
                JoinCond::On(c, op, c2) => {
                    let l = ctx.esc(c);
                    let r = ctx.esc(c2);
                    ctx.sql.push_str(&l);
                    ctx.sql.push(' ');
                    ctx.sql.push_str(op);
                    ctx.sql.push(' ');
                    ctx.sql.push_str(&r);
                }
                JoinCond::OnVal(c, op, v) => {
                    let l = ctx.esc(c);
                    ctx.sql.push_str(&l);
                    ctx.sql.push(' ');
                    ctx.sql.push_str(op);
                    ctx.sql.push(' ');
                    ctx.placeholder::<D>(v.clone());
                }
                JoinCond::OnRaw(sql, binds) => {
                    // Verbatim escape hatch (see `JoinClause::on_raw` docs).
                    ctx.sql.push_str(sql);
                    ctx.binds.extend(binds.iter().cloned());
                }
            }
        }
    }
}

/// Render ` HAVING cond AND …` (SELECT only) after GROUP BY. No-op when empty.
fn write_having<D: Dialect>(ctx: &mut Ctx, havings: &[Having]) {
    if havings.is_empty() {
        return;
    }
    ctx.sql.push_str(" HAVING ");
    for (i, h) in havings.iter().enumerate() {
        if i > 0 {
            ctx.sql.push_str(" AND ");
        }
        match h {
            Having::Col { col, op, val } => {
                let c = ctx.esc(col);
                ctx.sql.push_str(&c);
                ctx.sql.push(' ');
                ctx.sql.push_str(op);
                ctx.sql.push(' ');
                ctx.placeholder::<D>(val.clone());
            }
            Having::Raw { sql, binds } => {
                // Verbatim escape hatch (see `having_raw` docs).
                ctx.sql.push_str(sql);
                ctx.binds.extend(binds.iter().cloned());
            }
        }
    }
}

/// Render `WITH [RECURSIVE] name AS (body), … ` BEFORE the main SELECT.
///
/// Single-pass per CTE: the name header is written and the body compiled in one
/// go, so SQL text order equals bind-push order (placeholder/bind never desync).
fn write_ctes<D: Dialect>(ctx: &mut Ctx, ctes: &[Cte<D>]) {
    if ctes.is_empty() {
        return;
    }
    ctx.sql.push_str("WITH ");
    if ctes.iter().any(|c| c.recursive) {
        ctx.sql.push_str("RECURSIVE ");
    }
    for (i, cte) in ctes.iter().enumerate() {
        if i > 0 {
            ctx.sql.push_str(", ");
        }
        let name = ctx.esc(&cte.name);
        ctx.sql.push_str(&name);
        ctx.sql.push_str(" AS (");
        compile_into::<D>(ctx, &cte.query);
        ctx.sql.push(')');
    }
    ctx.sql.push(' ');
}

/// Render ` UNION [ALL] body` per arm, AFTER the main query (SELECT only).
fn write_unions<D: Dialect>(ctx: &mut Ctx, unions: &[(bool, QueryBuilder<D>)]) {
    for (all, arm) in unions {
        ctx.sql
            .push_str(if *all { " UNION ALL " } else { " UNION " });
        compile_into::<D>(ctx, arm);
    }
}

/// Render `ORDER BY a ASC, b DESC, …` (SELECT only). No-op when empty and no raw
/// fragment. A raw fragment is appended (comma-joined) after the structured
/// terms; if only raw is present it becomes the whole clause.
fn write_order_by(ctx: &mut Ctx, orders: &[(String, Order)], raw: Option<&(String, Vec<Value>)>) {
    if orders.is_empty() && raw.is_none() {
        return;
    }
    ctx.sql.push_str(" ORDER BY ");
    let cols: Vec<String> = orders
        .iter()
        .map(|(c, o)| {
            let dir = match o {
                Order::Asc => "ASC",
                Order::Desc => "DESC",
            };
            format!("{} {}", ctx.esc(c), dir)
        })
        .collect();
    ctx.sql.push_str(&cols.join(", "));
    if let Some((sql, binds)) = raw {
        if !orders.is_empty() {
            ctx.sql.push_str(", ");
        }
        // Verbatim escape hatch (see `order_by_raw` docs).
        ctx.sql.push_str(sql);
        ctx.binds.extend(binds.iter().cloned());
    }
}

/// Render `LIMIT $n [OFFSET $m]` (SELECT only), binding both values.
///
/// Panics if `offset` is set without `limit` (uniform across dialects; MySQL
/// rejects bare `OFFSET`).
fn write_limit_offset<D: Dialect>(ctx: &mut Ctx, limit: Option<i64>, offset: Option<i64>) {
    if offset.is_some() && limit.is_none() {
        panic!("offset(...) requires limit(...)");
    }
    if let Some(n) = limit {
        ctx.sql.push_str(" LIMIT ");
        ctx.placeholder::<D>(Value::I64(n));
    }
    if let Some(n) = offset {
        ctx.sql.push_str(" OFFSET ");
        ctx.placeholder::<D>(Value::I64(n));
    }
}

/// Render a row-locking clause (` FOR UPDATE`/` FOR SHARE` [+ ` SKIP LOCKED`/
/// ` NOWAIT`]) at the end of a `SELECT`. A **no-op on dialects without row
/// locking** (SQLite), so the lock is silently dropped there rather than
/// producing invalid SQL. Panics if a lock is combined with `UNION` on a
/// locking dialect (Postgres/MySQL reject that combination).
fn write_lock<D: Dialect>(ctx: &mut Ctx, lock: Option<&Lock>, has_unions: bool) {
    let Some(lock) = lock else {
        return;
    };
    if !D::supports_row_locking() {
        return;
    }
    // Postgres/MySQL reject `FOR UPDATE`/`FOR SHARE` on a `UNION` result; emitting
    // it would produce invalid SQL, so fail loud here. (No-op dialects returned
    // above never reach this, so a SQLite lock+UNION stays a harmless no-op.)
    if has_unions {
        panic!("for_update()/for_share() cannot be combined with UNION");
    }
    ctx.sql.push_str(match lock.strength {
        LockStrength::Update => " FOR UPDATE",
        LockStrength::Share => " FOR SHARE",
    });
    if let Some(wait) = lock.wait {
        ctx.sql.push_str(match wait {
            LockWait::SkipLocked => " SKIP LOCKED",
            LockWait::NoWait => " NOWAIT",
        });
    }
}

/// A predicate produces no SQL if it is an empty group (F4): an empty group
/// would emit invalid `()`, so it is skipped entirely (and must not leave a
/// dangling `AND`/`OR` separator behind it).
fn is_omitted<D: Dialect>(p: &Predicate<D>) -> bool {
    matches!(p, Predicate::Group { preds, .. } if preds.is_empty())
}

fn write_wheres<D: Dialect>(ctx: &mut Ctx, wheres: &[Predicate<D>]) {
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
fn write_clause_list<D: Dialect>(ctx: &mut Ctx, preds: &[Predicate<D>]) {
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

fn write_pred<D: Dialect>(ctx: &mut Ctx, pred: &Predicate<D>) {
    match pred {
        Predicate::Binary { col, op, val } => {
            let col = ctx.esc(col);
            ctx.sql.push_str(&col);
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
            let col = ctx.esc(col);
            ctx.sql.push_str(&col);
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
            let col = ctx.esc(col);
            ctx.sql.push_str(&col);
            ctx.sql
                .push_str(if *neg { " IS NOT NULL" } else { " IS NULL" });
        }
        Predicate::Between { col, lo, hi } => {
            let col = ctx.esc(col);
            ctx.sql.push_str(&col);
            ctx.sql.push_str(" BETWEEN ");
            ctx.placeholder::<D>(lo.clone());
            ctx.sql.push_str(" AND ");
            ctx.placeholder::<D>(hi.clone());
        }
        Predicate::ILike { col, val } => {
            let col = ctx.esc(col);
            if D::ilike_is_native() {
                // Postgres: native `col ILIKE $n`.
                ctx.sql.push_str(&col);
                ctx.sql.push_str(" ILIKE ");
                ctx.placeholder::<D>(val.clone());
            } else {
                // MySQL/SQLite: `LOWER(col) LIKE LOWER(?)`.
                ctx.sql.push_str("LOWER(");
                ctx.sql.push_str(&col);
                ctx.sql.push_str(") LIKE LOWER(");
                ctx.placeholder::<D>(val.clone());
                ctx.sql.push(')');
            }
        }
        Predicate::JsonContains { col, val } => {
            // Postgres-oriented `@>` (jsonb contains); emitted verbatim.
            let col = ctx.esc(col);
            ctx.sql.push_str(&col);
            ctx.sql.push_str(" @> ");
            ctx.placeholder::<D>(val.clone());
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
            // `outer_conj` controls how this group attaches to the *preceding*
            // clause (handled in `write_clause_list`). The inner predicates are
            // rendered with the SAME attach-conj logic as the top level
            // (`write_clause_list`): each inner pred is joined with ` AND `
            // unless it is itself a `Group` with `outer_conj == Conj::Or`, in
            // which case it is joined with ` OR `. This enables M11 nested
            // groups and inner-OR while staying byte-identical for the pre-M11
            // case (a group whose preds are all non-`Group` predicates joins
            // them all with ` AND `, exactly as the old hardcoded `Conj::And`).
            //
            // Empty groups never reach here: `write_clause_list` /
            // `write_wheres` filter them via `is_omitted` (F4), so we never
            // emit invalid `()`.
            ctx.sql.push('(');
            write_clause_list::<D>(ctx, preds);
            ctx.sql.push(')');
        }
        Predicate::Column { lhs, op, rhs } => {
            let l = ctx.esc(lhs);
            let r = ctx.esc(rhs);
            ctx.sql.push_str(&l);
            ctx.sql.push(' ');
            ctx.sql.push_str(op);
            ctx.sql.push(' ');
            ctx.sql.push_str(&r);
        }
        Predicate::Exists { neg, sub } => {
            ctx.sql
                .push_str(if *neg { "NOT EXISTS (" } else { "EXISTS (" });
            compile_into::<D>(ctx, sub);
            ctx.sql.push(')');
        }
        Predicate::InSubquery { col, neg, sub } => {
            let col = ctx.esc(col);
            ctx.sql.push_str(&col);
            ctx.sql.push_str(if *neg { " NOT IN (" } else { " IN (" });
            compile_into::<D>(ctx, sub);
            ctx.sql.push(')');
        }
    }
}
