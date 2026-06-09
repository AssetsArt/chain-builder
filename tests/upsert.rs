use chain_builder::{MySql, Postgres, QueryBuilder, Sqlite};

// ----- 1. pg merge -----

#[test]
fn pg_merge() {
    let (sql, _binds) = QueryBuilder::<Postgres>::table("users")
        .insert([("id", 1i64), ("email", 0), ("name", 0)]) // values irrelevant for SQL shape
        .on_conflict_merge(["id"])
        .to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("email", "id", "name") VALUES ($1, $2, $3) ON CONFLICT ("id") DO UPDATE SET "email" = EXCLUDED."email", "name" = EXCLUDED."name""#
    );
}

// ----- 2. pg ignore -----

#[test]
fn pg_ignore() {
    let (sql, _binds) = QueryBuilder::<Postgres>::table("users")
        .insert([("id", 1i64), ("email", 0), ("name", 0)])
        .on_conflict_do_nothing(["id"])
        .to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("email", "id", "name") VALUES ($1, $2, $3) ON CONFLICT ("id") DO NOTHING"#
    );
}

// ----- 3. sqlite merge + ignore -----

#[test]
fn sqlite_merge() {
    let (sql, _binds) = QueryBuilder::<Sqlite>::table("users")
        .insert([("id", 1i64), ("email", 0), ("name", 0)])
        .on_conflict_merge(["id"])
        .to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("email", "id", "name") VALUES (?, ?, ?) ON CONFLICT ("id") DO UPDATE SET "email" = EXCLUDED."email", "name" = EXCLUDED."name""#
    );
}

#[test]
fn sqlite_ignore() {
    let (sql, _binds) = QueryBuilder::<Sqlite>::table("users")
        .insert([("id", 1i64), ("email", 0), ("name", 0)])
        .on_conflict_do_nothing(["id"])
        .to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("email", "id", "name") VALUES (?, ?, ?) ON CONFLICT ("id") DO NOTHING"#
    );
}

// ----- 4. mysql merge -----

#[test]
fn mysql_merge() {
    let (sql, _binds) = QueryBuilder::<MySql>::table("users")
        .insert([("id", 1i64), ("email", 0), ("name", 0)])
        .on_conflict_merge(["id"])
        .to_sql();
    assert_eq!(
        sql,
        "INSERT INTO `users` (`email`, `id`, `name`) VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE `email` = VALUES(`email`), `id` = VALUES(`id`), `name` = VALUES(`name`)"
    );
}

// ----- 5. mysql ignore -----

#[test]
fn mysql_ignore() {
    let (sql, _binds) = QueryBuilder::<MySql>::table("users")
        .insert([("id", 1i64), ("email", 0), ("name", 0)])
        .on_conflict_do_nothing(["id"])
        .to_sql();
    assert_eq!(
        sql,
        "INSERT IGNORE INTO `users` (`email`, `id`, `name`) VALUES (?, ?, ?)"
    );
}

// ----- 6. pg/sqlite RETURNING on insert/update/delete -----

#[test]
fn pg_returning_insert() {
    let (sql, _binds) = QueryBuilder::<Postgres>::table("users")
        .insert([("name", "x")])
        .returning(["id"])
        .to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("name") VALUES ($1) RETURNING "id""#
    );
}

#[test]
fn pg_returning_star() {
    let (sql, _binds) = QueryBuilder::<Postgres>::table("users")
        .insert([("name", "x")])
        .returning(["*"])
        .to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("name") VALUES ($1) RETURNING *"#
    );
}

#[test]
fn pg_returning_update() {
    let (sql, _binds) = QueryBuilder::<Postgres>::table("users")
        .update([("name", "x")])
        .where_eq("id", 1i64)
        .returning(["id"])
        .to_sql();
    assert_eq!(
        sql,
        r#"UPDATE "users" SET "name" = $1 WHERE "id" = $2 RETURNING "id""#
    );
}

#[test]
fn pg_returning_delete() {
    let (sql, _binds) = QueryBuilder::<Postgres>::table("users")
        .delete()
        .where_eq("id", 1i64)
        .returning(["id"])
        .to_sql();
    assert_eq!(sql, r#"DELETE FROM "users" WHERE "id" = $1 RETURNING "id""#);
}

#[test]
fn sqlite_returning_insert() {
    let (sql, _binds) = QueryBuilder::<Sqlite>::table("users")
        .insert([("name", "x")])
        .returning(["id"])
        .to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("name") VALUES (?) RETURNING "id""#
    );
}

// ----- 7. mysql RETURNING omitted -----

#[test]
fn mysql_returning_omitted() {
    let (sql, _binds) = QueryBuilder::<MySql>::table("users")
        .insert([("name", "x")])
        .returning(["id"])
        .to_sql();
    assert_eq!(sql, "INSERT INTO `users` (`name`) VALUES (?)");
    assert!(!sql.contains("RETURNING"));
}

// ----- 8. pg merge where targets cover all cols -> DO NOTHING -----

#[test]
fn pg_merge_all_cols_targets() {
    let (sql, _binds) = QueryBuilder::<Postgres>::table("users")
        .insert([("id", 1i64)])
        .on_conflict_merge(["id"])
        .to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("id") VALUES ($1) ON CONFLICT ("id") DO NOTHING"#
    );
}

// ----- 9. empty-targets merge -> bare DO NOTHING -----

#[test]
fn pg_merge_empty_targets() {
    let (sql, _binds) = QueryBuilder::<Postgres>::table("users")
        .insert([("id", 1i64), ("name", 0)])
        .on_conflict_merge(Vec::<&str>::new())
        .to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("id", "name") VALUES ($1, $2) ON CONFLICT DO NOTHING"#
    );
}

#[test]
fn pg_ignore_empty_targets() {
    let (sql, _binds) = QueryBuilder::<Postgres>::table("users")
        .insert([("id", 1i64), ("name", 0)])
        .on_conflict_do_nothing(Vec::<&str>::new())
        .to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("id", "name") VALUES ($1, $2) ON CONFLICT DO NOTHING"#
    );
}

// ----- 10. regression: plain insert unchanged -----

#[test]
fn pg_plain_insert_unchanged() {
    let (sql, _binds) = QueryBuilder::<Postgres>::table("users")
        .insert([("id", 1i64), ("email", 0), ("name", 0)])
        .to_sql();
    assert_eq!(
        sql,
        r#"INSERT INTO "users" ("email", "id", "name") VALUES ($1, $2, $3)"#
    );
}
