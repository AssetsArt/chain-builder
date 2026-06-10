# chain-builder docs site (mdBook) — design

Date: 2026-06-10
Status: approved (user picked: mdBook site, English, GitHub Pages auto-deploy,
content = core guide + cookbook + security/internals, snippet policy A)

## Goal

A full documentation site for chain-builder 3.0.0, built with mdBook, written
in English, deployed automatically to GitHub Pages. It replaces the
single-page `docs/guide.md` as the canonical long-form documentation.

## Non-goals

- **No migration guide page** (user explicitly deselected). The 2→3 breaking
  change is covered inside the Error Handling page as a short "since 3.0"
  note, not a dedicated page.
- **No Thai translation** (English only).
- **No `mdbook test` CI wiring** (snippet policy A, below).
- **No rustdoc expansion** — docs.rs stays as-is; the site links to it.
- **No changes to Rust source code.** This project is docs + CI only.

## Snippet policy (A)

Code blocks in the book are marked ```` ```rust,ignore ```` (builder-only
snippets) or ```` ```rust,no_run ```` (async/sqlx snippets). Snippets are
copied/adapted from real tested code (rustdoc doctests, `tests/*.rs`) wherever
one exists, so drift risk is bounded by the crate's own test suite. The book
is an explanation layer, not a test layer. `mdbook build` with `create-missing = false` (which fails on missing SUMMARY
targets; note it does NOT validate inter-page hyperlinks) is the only CI gate
for the book.

Convention for SQL output in examples: show the generated SQL as a comment
under the builder chain, Postgres dialect unless the section is
dialect-specific:

```rust,ignore
let (sql, binds) = QueryBuilder::<Postgres>::table("users")
    .select(["id"])
    .where_eq("status", "active")
    .to_sql();
// SELECT "id" FROM "users" WHERE "status" = $1
```

## File structure

```
docs/book/
  book.toml                 # title, authors, repo links, search, git-edit-url
  src/
    SUMMARY.md
    introduction.md
    getting-started.md
    query/
      select.md
      where.md
      join.md
      group-having-order-limit.md
      cte-union.md
      insert-update-delete.md
      upsert-returning.md
      locking.md
      dynamic.md
    binds.md
    error-handling.md
    sqlx.md
    dialects.md
    cookbook/
      http-filters-pagination.md
      http-error-mapping.md
      multi-tenant.md
      bulk-insert-upsert.md
      search.md
    security.md
    internals.md
.github/workflows/docs.yml  # build + deploy
.gitignore                  # add docs/book/book/ (build output)
```

`docs/guide.md` is reduced to a short pointer page (title + link to the site +
link to docs.rs). README gains a **Documentation** link near the top
(site + docs.rs + CHANGELOG).

## Page content contracts

Every page: intro paragraph (what/when-to-use), code examples per the snippet
policy, cross-links to related pages, and — where behavior differs by dialect
— an explicit dialect note. Source-of-truth references the authors must use:
`src/builder.rs`, `src/where_.rs`, `src/compile.rs`, `src/error.rs`,
`src/fetch.rs`, `src/sqlx_bind.rs`, `src/dialect/`, `src/value.rs`,
`tests/*.rs`, `docs/guide.md` (current content), `CHANGELOG.md`.

1. **Introduction** — what chain-builder is (typed, dialect-aware SQL builder;
   values always bound, identifiers always escaped), feature matrix per
   dialect (placeholders, upsert style, RETURNING, DISTINCT ON, row locking,
   native ILIKE), links: crates.io, docs.rs, GitHub, CHANGELOG.
2. **Getting Started** — install table (feature flags incl. json/uuid/chrono/
   decimal), **the `default = ["sqlx_mysql"]` gotcha** (Postgres/SQLite users
   must add `default-features = false` or they silently compile the MySQL
   driver), first query, `to_sql()` vs `try_to_sql()` in one paragraph,
   executing with sqlx teaser.
3. **SELECT** — `select`, `select_as`, aggregates (`select_count[_as]`/sum/avg/
   min/max), `select_raw` (+ placeholder contract warning), `select_subquery`,
   `distinct`, `distinct_on` (pg-only, BuildError elsewhere).
4. **WHERE** — all predicates (`where_eq/ne/gt/gte/lt/lte`, `where_in/not_in`
   + empty-IN semantics `1 = 0`/`1 = 1`, `where_null/not_null`,
   `where_between`, `where_like` (wildcards in user input are NOT escaped — caveat here, referenced by the search cookbook), `where_ilike` + per-dialect lowering, `where_jsonb_contains` (emits `@>` verbatim on ALL dialects — meaningful only on Postgres jsonb; page must carry this dialect note),
   `where_column`, `where_exists`/`where_not_exists`, `where_in_subquery`/`where_not_in_subquery`),
   groups `and_where`/`or_where` (+ nesting, empty-group omission),
   `where_raw` escape hatch.
5. **JOIN** — kinds (inner/left/right/full outer/cross), `on`/`on_val`/`on_raw`,
   `.db()` qualifier interaction.
6. **GROUP BY · HAVING · ORDER · LIMIT** — `group_by[_raw]`, `having`
   (operator allowlist — matched case-insensitively, stored trimmed → deferred
   BuildError), `having_raw` ($N contract),
   `order_by[_asc/_desc/_raw]`, `limit`/`offset` (offset-requires-limit),
   `paginate`.
7. **CTE & UNION** — `with`/`with_recursive`, `union`/`union_all`, bind
   ordering (CTEs first), SELECT-only note.
8. **INSERT · UPDATE · DELETE** — `insert`, `insert_many` (NULL-padding of
   ragged rows), sorted-column determinism, `update`, `delete`, empty-set
   errors.
9. **Upsert & RETURNING** — `on_conflict_do_nothing(targets)` / `on_conflict_merge(targets)` (the only two upsert entry points; `ConflictAction` is AST-only, never passed by users),
   per-dialect rendering (pg/sqlite `ON CONFLICT`, mysql
   `INSERT IGNORE`/`ON DUPLICATE KEY UPDATE`), `returning` (no-op on MySQL).
10. **Row Locking** — `for_update`/`for_share`, `skip_locked`/`no_wait`,
    SELECT-only (BuildError), UNION conflict (BuildError), SQLite no-op.
11. **Dynamic Building** — `when`/`when_else`, building from request params,
    builder reuse/clone, `paginate`.
12. **Binds & Values** — `IntoBind` impls table (ints→I64 incl. u64 wrap note,
    floats→F64, bool, strings, bytes, `Option<T>`: `None`→Null, `Some`→inner value), feature-gated values
    (json/uuid/chrono/decimal incl. sqlite-decimal-as-TEXT note).
13. **Error Handling** — `BuildError` variants table, `try_to_sql`/`try_compile`/
    `try_to_sqlx_query[_as]` vs panicking twins + the documented policy,
    deferred `having()` error + nested propagation, unified `Error{Build,Sqlx}`,
    `#[non_exhaustive]` wildcard-arm requirement, HTTP 4XX/5XX mapping,
    "since 3.0" note (fetch_* error type changed).
14. **Executing with sqlx** — feature flags, `to_sqlx_query[_as]` + try twins,
    `fetch_all/one/optional`, `execute`, `count` (wrapping), `fetch_scalar`/`fetch_optional_scalar`, `SqlxQuery`/`SqlxQueryAs` aliases.
15. **Dialect Differences** — one table: placeholder, quote char, upsert style,
    RETURNING, DISTINCT ON, row locking, ILIKE; prose notes per row.
16. **Cookbook: HTTP filters & pagination** — axum handler: query params →
    `when` chains + `paginate` + `try_to_sql`.
17. **Cookbook: Mapping errors to HTTP status** — `Error`/`BuildError` → 400 vs
    500, `IntoResponse` example.
18. **Cookbook: Multi-tenant `.db()`** — one pool, many schemas; join
    qualification.
19. **Cookbook: Bulk insert & upsert** — `insert_many` + `on_conflict_merge`;
    batching note.
20. **Cookbook: Case-insensitive search** — `where_ilike` portability,
    `LIKE`-escaping caveat (user input wildcards are NOT escaped — document).
21. **Security Model** — guarantees (values bound, identifiers escaped, having
    allowlist, `&'static str` operator params), complete escape-hatch
    inventory (`select_raw`, `where_raw`, `group_by_raw`, `order_by_raw`,
    `having_raw`, `on_raw` — verbatim, caller-audited), what is NOT protected
    (raw SQL fragments, LIKE wildcards, identifier *names* from untrusted
    input policy).
22. **Internals** — scope note: the publicly re-exported AST types (`Predicate`,
    `Having`, `OnConflict`, …) are advanced API and deliberately NOT documented
    page-by-page — one paragraph acknowledges them and points to docs.rs;
    single-pass `Ctx` compiler, placeholder numbering
    continuity ($N across CTE/where/limit), identifier escaping chokepoint
    (`ctx.esc`), deferred-error flow, byte-identity discipline & sorted
    columns.

## book.toml requirements

- title "chain-builder", authors AssetsArt
- `[output.html]`: `git-repository-url` → GitHub repo, `edit-url-template` →
  `.../edit/main/docs/book/{path}`, default theme, built-in search enabled
  (default)
- `[build] create-missing = false` — otherwise mdBook silently creates empty
  files for missing SUMMARY targets instead of failing
- build dir default (`book`) — covered by .gitignore entry `docs/book/book/`

## CI/CD (docs.yml)

- Triggers: `push` to `main` with paths `docs/book/**`,
  `.github/workflows/docs.yml`; `workflow_dispatch`; `pull_request` touching
  the same paths runs **build only** (no deploy).
- Jobs: `build` (install mdBook pinned version via `peaceiris/actions-mdbook`
  or direct binary download — pick one in plan; build; upload
  `actions/upload-pages-artifact` from `docs/book/book`), `deploy`
  (needs build, `actions/deploy-pages`, only on push/dispatch to main,
  permissions `pages: write`, `id-token: write`, environment `github-pages`).
- Repo Pages must be enabled with build type `workflow` — done via
  `gh api -X POST repos/AssetsArt/chain-builder/pages -f build_type=workflow`
  (404 currently confirms it is not yet enabled).

## Acceptance criteria

1. `mdbook build docs/book` succeeds locally with zero warnings about missing
   SUMMARY targets; every page in File Structure exists with real content (no
   placeholders/TODO).
2. Every API mentioned in the book exists in the 3.0.0 source (spot-checkable
   by grep against `src/`).
3. `docs.yml` deploys successfully and the site is reachable at
   `https://assetsart.github.io/chain-builder/`.
4. README links to the site; `docs/guide.md` is a pointer page; build output
   is gitignored.
5. No Rust source files modified (`git diff --stat` contains no `src/` or
   `tests/` changes).

## Known limitations

- Book snippets are not compile-tested (policy A); drift is mitigated by
  sourcing from tested code and by review.
- Site versioning: single version (latest main). Historical versions live in
  docs.rs per release.

## Open questions resolved at plan-time

- mdBook install method in CI (action vs binary download) — plan decides,
  pinned either way.
- Exact README wording for the Documentation section.
- Pages enablement permission: confirmed — `gh api` against the repo works with
  admin scope (404 on GET /pages simply means not yet enabled).
