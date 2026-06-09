//! v2: `db("name")` database-qualified tables (multi-tenant: one connection,
//! many databases — like 1.x `db()`). The db identifier prefixes the main table
//! AND join tables, escaped per dialect.
#![cfg(feature = "v2")]

use chain_builder::v2::{MySql, Postgres, Sqlite};
type Pg = chain_builder::v2::QueryBuilder<Postgres>;
type My = chain_builder::v2::QueryBuilder<MySql>;
type Lite = chain_builder::v2::QueryBuilder<Sqlite>;

#[test]
fn postgres_db_qualifies_select_table() {
    let (sql, binds) = Pg::table("users")
        .db("mydb")
        .select(["id"])
        .where_eq("status", "active")
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "id" FROM "mydb"."users" WHERE "status" = $1"#
    );
    assert_eq!(binds.len(), 1);
}

#[test]
fn mysql_db_qualifies_select_table() {
    let (sql, _) = My::table("users").db("mydb").select(["id"]).to_sql();
    assert_eq!(sql, "SELECT `id` FROM `mydb`.`users`");
}

#[test]
fn sqlite_db_qualifies_select_table() {
    let (sql, _) = Lite::table("users").db("mydb").select(["id"]).to_sql();
    assert_eq!(sql, r#"SELECT "id" FROM "mydb"."users""#);
}

#[test]
fn db_qualifies_insert_update_delete() {
    let (ins, _) = Pg::table("users").db("t1").insert([("name", "a")]).to_sql();
    assert_eq!(ins, r#"INSERT INTO "t1"."users" ("name") VALUES ($1)"#);

    let (upd, _) = Pg::table("users")
        .db("t1")
        .update([("name", "a")])
        .where_eq("id", 1i64)
        .to_sql();
    assert_eq!(
        upd,
        r#"UPDATE "t1"."users" SET "name" = $1 WHERE "id" = $2"#
    );

    let (del, _) = Pg::table("users").db("t1").delete().where_eq("id", 1i64).to_sql();
    assert_eq!(del, r#"DELETE FROM "t1"."users" WHERE "id" = $1"#);
}

#[test]
fn db_also_prefixes_join_tables() {
    // Matches 1.x: the builder's db prefixes join tables too (same tenant db).
    let (sql, _) = Pg::table("users")
        .db("mydb")
        .select(["users.id"])
        .left_join("profiles", |j| j.on("users.id", "=", "profiles.uid"))
        .to_sql();
    assert_eq!(
        sql,
        r#"SELECT "users"."id" FROM "mydb"."users" LEFT JOIN "mydb"."profiles" ON "users"."id" = "profiles"."uid""#
    );
}

#[test]
fn no_db_is_unchanged() {
    let (sql, _) = Pg::table("users").select(["id"]).to_sql();
    assert_eq!(sql, r#"SELECT "id" FROM "users""#);
}

#[test]
fn db_identifier_is_escaped() {
    // Injection through the db name is neutralized (quote doubled).
    let (sql, _) = Pg::table("t").db(r#"ev"il"#).select(["x"]).to_sql();
    assert_eq!(sql, r#"SELECT "x" FROM "ev""il"."t""#);
}
