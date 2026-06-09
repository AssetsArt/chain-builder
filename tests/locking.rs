//! Row-locking clauses (`FOR UPDATE` / `FOR SHARE`, `SKIP LOCKED` / `NOWAIT`).

use chain_builder::{MySql, Postgres, QueryBuilder, Sqlite};

#[test]
fn for_update_pg() {
    let (sql, _) = QueryBuilder::<Postgres>::table("jobs")
        .select(["id"])
        .where_eq("status", "queued")
        .for_update()
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "id" FROM "jobs" WHERE "status" = $1 FOR UPDATE"#
    );
}

#[test]
fn for_share_pg() {
    let (sql, _) = QueryBuilder::<Postgres>::table("jobs")
        .select(["id"])
        .for_share()
        .to_sql();
    assert_eq!(sql, r#"SELECT "id" FROM "jobs" FOR SHARE"#);
}

#[test]
fn for_update_skip_locked() {
    let (sql, _) = QueryBuilder::<Postgres>::table("jobs")
        .select(["id"])
        .for_update()
        .skip_locked()
        .to_sql();
    assert_eq!(sql, r#"SELECT "id" FROM "jobs" FOR UPDATE SKIP LOCKED"#);
}

#[test]
fn for_update_nowait() {
    let (sql, _) = QueryBuilder::<Postgres>::table("jobs")
        .select(["id"])
        .for_update()
        .no_wait()
        .to_sql();
    assert_eq!(sql, r#"SELECT "id" FROM "jobs" FOR UPDATE NOWAIT"#);
}

#[test]
fn skip_locked_defaults_to_for_update() {
    // Calling skip_locked() without a prior for_update()/for_share() defaults the
    // strength to FOR UPDATE (the job-queue idiom).
    let (sql, _) = QueryBuilder::<Postgres>::table("jobs")
        .select(["id"])
        .skip_locked()
        .to_sql();
    assert_eq!(sql, r#"SELECT "id" FROM "jobs" FOR UPDATE SKIP LOCKED"#);
}

#[test]
fn lock_after_limit() {
    let (sql, binds) = QueryBuilder::<Postgres>::table("jobs")
        .select(["id"])
        .limit(1)
        .for_update()
        .skip_locked()
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "id" FROM "jobs" LIMIT $1 FOR UPDATE SKIP LOCKED"#
    );
    assert_eq!(binds.len(), 1);
}

#[test]
fn for_update_mysql() {
    let (sql, _) = QueryBuilder::<MySql>::table("jobs")
        .select(["id"])
        .for_update()
        .skip_locked()
        .to_sql();
    assert_eq!(sql, "SELECT `id` FROM `jobs` FOR UPDATE SKIP LOCKED");
}

#[test]
fn locking_is_noop_on_sqlite() {
    // SQLite locks the whole database, not rows: the clause is silently dropped.
    let (sql, _) = QueryBuilder::<Sqlite>::table("jobs")
        .select(["id"])
        .for_update()
        .skip_locked()
        .to_sql();
    assert_eq!(sql, r#"SELECT "id" FROM "jobs""#);
}

#[test]
fn for_share_then_skip_locked_preserves_strength() {
    let (sql, _) = QueryBuilder::<Postgres>::table("jobs")
        .select(["id"])
        .for_share()
        .skip_locked()
        .to_sql();
    assert_eq!(sql, r#"SELECT "id" FROM "jobs" FOR SHARE SKIP LOCKED"#);
}

#[test]
#[should_panic(expected = "only valid on SELECT")]
fn lock_on_delete_panics() {
    // A lock on a non-SELECT is a dangerous silent no-op — fail loud instead.
    let _ = QueryBuilder::<Postgres>::table("jobs")
        .delete()
        .for_update()
        .to_sql();
}

#[test]
#[should_panic(expected = "only valid on SELECT")]
fn lock_on_update_panics() {
    let _ = QueryBuilder::<Postgres>::table("jobs")
        .update([("status", "done")])
        .for_update()
        .to_sql();
}

#[test]
#[should_panic(expected = "only valid on SELECT")]
fn lock_on_non_select_panics_on_sqlite_too() {
    // Dialect-independent: the misuse is structural, not a dialect capability.
    let _ = QueryBuilder::<Sqlite>::table("jobs")
        .delete()
        .for_update()
        .to_sql();
}

#[test]
#[should_panic(expected = "cannot be combined with UNION")]
fn lock_with_union_panics() {
    // Postgres/MySQL reject FOR UPDATE on a UNION result — fail loud.
    let arm = QueryBuilder::<Postgres>::table("archived_jobs").select(["id"]);
    let _ = QueryBuilder::<Postgres>::table("jobs")
        .select(["id"])
        .for_update()
        .union(arm)
        .to_sql();
}

#[test]
fn lock_with_union_is_noop_on_sqlite() {
    // SQLite drops the lock entirely, so lock+UNION stays a harmless no-op there.
    let arm = QueryBuilder::<Sqlite>::table("archived_jobs").select(["id"]);
    let (sql, _) = QueryBuilder::<Sqlite>::table("jobs")
        .select(["id"])
        .for_update()
        .union(arm)
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "id" FROM "jobs" UNION SELECT "id" FROM "archived_jobs""#
    );
}
