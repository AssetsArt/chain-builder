# chain-builder docs site — implementation plan

Spec: `docs/superpowers/specs/2026-06-10-docs-site-design.md` (read it first;
it is the contract. The spec's "Page content contracts" section is the
per-page outline — API names there were verified against 3.0.0 source).

Global rules for every task:
- English prose. Snippets per spec policy A (```rust,ignore``` for builder-only,
  ```rust,no_run``` for async/sqlx). Copy/adapt from rustdoc doctests and
  `tests/*.rs` where possible.
- NEVER modify `src/**` or `tests/**` (acceptance criterion 5).
- Verification after each task: `mdbook build docs/book` exits 0.
- Real content only — no TODO/placeholder text anywhere.
- Cross-link related pages with relative links (`../error-handling.md` style).

Spec coverage map: T1 → spec pages 1-2 + book.toml/.gitignore;
T2 → pages 3-5; T3 → pages 6-9; T4 → pages 10-12; T5 → pages 13-15;
T6 → pages 16-20; T7 → pages 21-22; T8 → CI/CD + README/guide.md + Pages
enablement. Acceptance criteria 1-2 enforced per-task, 3-5 in T8/Phase 6.

## Task 1 — Scaffold + Introduction + Getting Started

Create:
- `docs/book/book.toml`:
  ```toml
  [book]
  title = "chain-builder"
  authors = ["AssetsArt"]
  description = "A typed, dialect-aware SQL query builder for Rust"
  language = "en"
  src = "src"

  [build]
  create-missing = false

  [output.html]
  git-repository-url = "https://github.com/AssetsArt/chain-builder"
  edit-url-template = "https://github.com/AssetsArt/chain-builder/edit/main/docs/book/{path}"
  ```
- `docs/book/src/SUMMARY.md` listing ALL 22 pages exactly as the spec's File
  structure section (Introduction unnumbered prefix entry; numbered chapters
  for the rest; `Query Building` and `Cookbook` as section headers with
  nested entries).
- ALL 22 page files with REAL full content for `introduction.md` +
  `getting-started.md`; for the other 20 pages, because `create-missing =
  false` fails the build on missing files, create each file with its H1 title
  and a single italic line `*Coming in this PR — see SUMMARY for scope.*`
  (placeholder allowed ONLY transiently inside this PR branch; later tasks
  replace them — final acceptance forbids leftovers, T8 implementer must
  verify none remain).
- Append `docs/book/book/` to `.gitignore`.

Content contracts: spec pages 1 (Introduction: pitch, dialect feature matrix
table, links) and 2 (Getting Started: install table per feature flag, the
`default = ["sqlx_mysql"]` / `default-features = false` gotcha, first query
with to_sql, try_to_sql paragraph, sqlx teaser linking to sqlx.md).

Verify: `mdbook build docs/book` → exit 0; `ls docs/book/src/**/*.md | wc -l`
→ 23 (22 pages + SUMMARY).

BLOCKED fallback: if mdBook 0.5.x rejects a book.toml key, drop ONLY the
offending key and note it in the report.

## Task 2 — SELECT, WHERE, JOIN pages

Replace stub content of `query/select.md`, `query/where.md`, `query/join.md`
per spec contracts 3-5. Mandatory details: select_raw/$N warning;
distinct_on → `BuildError::DistinctOnRequiresPostgres` elsewhere; empty-IN
semantics; where_like wildcard caveat; where_ilike per-dialect lowering
(`LOWER(col) LIKE LOWER(?)` on MySQL/SQLite, native ILIKE on pg);
where_jsonb_contains `@>` verbatim-on-all-dialects note; and_where/or_where
nesting + empty-group omission; where_raw contract; join kinds incl.
cross_join (no ON); on/on_val/on_raw; `.db()` qualifies join tables too.
Source refs: `src/builder.rs`, `src/where_.rs`, `src/compile.rs`,
`tests/select.rs`, `tests/clauses.rs`, `tests/groups_nested.rs`,
`tests/join.rs`, `tests/dynamic.rs`.

Verify: `mdbook build docs/book` exit 0; grep each documented method name
against `src/builder.rs`/`src/where_.rs` to prove existence.

## Task 3 — GROUP/HAVING/ORDER/LIMIT, CTE & UNION, INSERT/UPDATE/DELETE, Upsert & RETURNING

Replace stubs of `query/group-having-order-limit.md`, `query/cte-union.md`,
`query/insert-update-delete.md`, `query/upsert-returning.md` per spec
contracts 6-9. Mandatory: having allowlist (case-insensitive, trimmed,
deferred BuildError, having_raw for aggregates); offset-requires-limit;
paginate 1-based; CTE binds first; insert_many NULL-padding + sorted columns;
EmptyInsert/EmptyUpdate; on_conflict_do_nothing/on_conflict_merge per-dialect
rendering incl. MySQL INSERT IGNORE and merge-ignores-targets note; returning
no-op on MySQL. Source refs: `tests/aggregates.rs`, `tests/having_guard.rs`,
`tests/cte_union.rs`, `tests/crud.rs`, `tests/upsert.rs`, `tests/pg.rs`.

Verify: same as T2.

## Task 4 — Locking, Dynamic, Binds & Values

Replace stubs of `query/locking.md`, `query/dynamic.md`, `binds.md` per spec
contracts 10-12. Mandatory: lock SELECT-only + UNION conflict (both
BuildError), SQLite silent no-op; when/when_else doctest-derived examples;
builder Clone/reuse; IntoBind table incl. u64 wrap (`u64::MAX → -1`),
Option None→Null/Some→inner; feature-gated Value variants + sqlite decimal
TEXT comparison caveat. Source refs: `tests/locking.rs`, `tests/dynamic.rs`,
`tests/typebinds.rs`, `tests/decimal.rs`, `src/value.rs`.

Verify: same as T2.

## Task 5 — Error Handling, Executing with sqlx, Dialect Differences

Replace stubs of `error-handling.md`, `sqlx.md`, `dialects.md` per spec
contracts 13-15. Mandatory: BuildError variants table with Display messages
(from `src/error.rs`); twin-API policy paragraph (panicking kept
deliberately); deferred having error incl. nested propagation; Error{Build,
Sqlx} + From impls + non_exhaustive wildcard requirement (match example MUST
include `Err(e) =>` arm); HTTP mapping snippet; "since 3.0" note; full fetch
API list incl. count wrapping SQL; SqlxQuery/SqlxQueryAs; dialect comparison
table (placeholder/quote/upsert/RETURNING/DISTINCT ON/locking/ILIKE). Source
refs: `src/error.rs`, `src/fetch.rs`, `src/sqlx_bind.rs`, `src/dialect/`,
`tests/build_error.rs`, `tests/fetch_error.rs`, `CHANGELOG.md` (3.0.0).

Verify: same as T2.

## Task 6 — Cookbook (5 recipes)

Replace stubs of `cookbook/*.md` per spec contracts 16-20. Each recipe:
problem statement → complete code (rust,no_run for axum/sqlx) → notes.
axum types may be sketched minimally (no axum dep exists in this repo —
recipes are illustrative; mark ```rust,ignore``` where the snippet references
axum). Mandatory: error-mapping recipe reuses Error/BuildError correctly with
wildcard arm; search recipe documents LIKE-wildcard non-escaping with a
sanitization snippet; bulk recipe shows insert_many + on_conflict_merge and a
chunking loop. Source refs: spec contracts + pages written in T5.

Verify: `mdbook build docs/book` exit 0.

## Task 7 — Security Model + Internals

Replace stubs of `security.md`, `internals.md` per spec contracts 21-22.
Mandatory: guarantees list; COMPLETE escape-hatch inventory (select_raw,
where_raw, group_by_raw, order_by_raw, having_raw, on_raw) each with its
contract; "not protected" list (raw fragments, LIKE wildcards, identifier
names from untrusted input); internals: single-pass Ctx, $N continuity,
ctx.esc chokepoint, deferred-error flow, sorted-column determinism, public
AST re-exports scope note pointing to docs.rs. Source refs: `src/compile.rs`
(module docs + Ctx), `src/ident.rs`, `tests/security_test.rs` if present,
spec.

Verify: `mdbook build docs/book` exit 0; grep confirms no `Coming in this PR`
stub line remains anywhere under `docs/book/src/`.

## Task 8 — CI/CD + README/guide.md + Pages enablement

- Create `.github/workflows/docs.yml`:
  - `on: push: branches: [main], paths: ['docs/book/**', '.github/workflows/docs.yml']`,
    `pull_request:` same paths, `workflow_dispatch:`.
  - Job `build`: ubuntu-latest; checkout@v4; install mdBook via
    `peaceiris/actions-mdbook@v2` with `mdbook-version: '0.4.40'`… **NO — use
    the version that matches local 0.5.3**: `mdbook-version: 'latest'` is not
    pinned; instead download the pinned binary:
    ```yaml
    - name: Install mdBook
      run: |
        mkdir -p "$HOME/.local/bin"
        curl -sSL https://github.com/rust-lang/mdBook/releases/download/v0.5.3/mdbook-v0.5.3-x86_64-unknown-linux-gnu.tar.gz \
          | tar -xz -C "$HOME/.local/bin"
        echo "$HOME/.local/bin" >> "$GITHUB_PATH"
    ```
  - `mdbook build docs/book`; `actions/upload-pages-artifact@v3` with
    `path: docs/book/book`.
  - Job `deploy`: `needs: build`, runs only when
    `github.event_name != 'pull_request'`, `permissions: pages: write,
    id-token: write`, `environment: github-pages` with url from step output,
    `actions/deploy-pages@v4`.
- README: add a `## Documentation` section right after badges/intro linking
  site (https://assetsart.github.io/chain-builder/), docs.rs, CHANGELOG.
- `docs/guide.md`: replace body with pointer (keep title): site link +
  docs.rs link + one-line note that the guide moved.
- Enable Pages: `gh api -X POST repos/AssetsArt/chain-builder/pages -f build_type=workflow`
  (idempotent-check: GET first; if 404 then POST).
- Final sweep: `grep -rn "Coming in this PR" docs/book/src/` → no hits.

Verify: `mdbook build docs/book` exit 0; `gh api repos/AssetsArt/chain-builder/pages --jq .build_type`
→ `workflow`.

BLOCKED fallback: if the v0.5.3 binary URL 404s (release asset naming), fall
back to `peaceiris/actions-mdbook@v2` with `mdbook-version: '0.5.3'`; if
Pages POST fails with 409 (already enabled), treat as success.

## Out of scope guard

No task may touch `src/**`, `tests/**`, `Cargo.*`. Workflow files other than
the new `docs.yml` are untouched.
