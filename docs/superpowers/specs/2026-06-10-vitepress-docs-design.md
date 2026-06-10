# Docs site: mdBook → VitePress (design)

Date: 2026-06-10
Status: approved in-session (interactive brainstorm).
Current docs: mdBook at `docs/book/` (22 pages, ~5,200 lines), deployed to
GitHub Pages by `.github/workflows/docs.yml` (mdBook v0.5.3). Live at
https://assetsart.github.io/chain-builder/.

## Goal

Replace the mdBook documentation site with a [VitePress](https://vitepress.dev)
site styled after knexjs.org: a hero landing page, top nav, a sidebar mirroring
the current information architecture, local search, dark/light toggle, and a
custom **green** brand palette. Port the existing 22 pages verbatim (no prose
rewrites). Deploy to the same GitHub Pages target.

## Non-goals (explicitly out of scope)

- No prose rewriting. Content is ported as-is; only mechanical transforms
  (code-fence info strings, see below) are applied.
- No restructure of the IA — same sections, same page set as today's
  `SUMMARY.md`.
- No i18n, no versioned docs, no Algolia (local search only), no custom Vue
  components, no carbon ads, no edit-on-GitHub links.
- No change to the crate, its Rust build, or any non-docs workflow.
- No change to GitHub Pages settings (already `build_type = workflow`).

## Source of truth: content invariant

The mdBook snippets carry SQL-output comments that are **byte-matched to test
assertions** in `tests/` (snippet policy A from the 3.0/3.1 docs work). VitePress
does not compile or run Rust snippets either — so this invariant is preserved by
*not editing snippet bodies*. The port copies snippet text unchanged; only the
fenced-block info string changes (`rust,ignore`/`rust,no_run` → `rust`).

## Directory layout

```
docs/
  site/                          ← NEW: VitePress root
    package.json
    package-lock.json
    .gitignore                   ← node_modules/, .vitepress/dist/, .vitepress/cache/
    .vitepress/
      config.mts                 ← base, nav, sidebar, search, theme hookup
      theme/
        index.ts                 ← extends default theme, imports custom.css
        custom.css               ← green brand tokens (light + dark)
    index.md                     ← home (layout: home — hero + features)
    introduction.md
    getting-started.md
    query/
      select.md where.md join.md group-having-order-limit.md
      cte-union.md insert-update-delete.md upsert-returning.md
      locking.md dynamic.md
    binds.md
    error-handling.md
    sqlx.md
    dialects.md
    cookbook/
      http-filters-pagination.md http-error-mapping.md
      multi-tenant.md bulk-insert-upsert.md search.md
    security.md
    internals.md
  superpowers/                   ← UNCHANGED (specs/plans; not part of the site)
```

Remove `docs/book/` in its entirety (content lives in `docs/site/` after the
port; `book.toml` and `SUMMARY.md` are replaced by `config.mts`).

## Content port

For each of the 22 pages under `docs/book/src/`:

1. Copy to the mirror path under `docs/site/` (directory structure already
   matches — `query/`, `cookbook/` stay; top-level pages stay top-level).
2. Transform code-fence info strings: ` ```rust,ignore ` and ` ```rust,no_run `
   → ` ```rust `. Shiki (VitePress's highlighter) treats the whole token after
   the backticks as the language; `rust,ignore` is unrecognized and renders
   unhighlighted. There are ~21 such fences across the pages.
3. Leave internal `.md` relative links as-is. They are all sibling/`../`
   relative (verified: `(security.md)`, `(../query/where.md)`, …) and resolve
   identically under the preserved directory structure; VitePress rewrites them
   to clean URLs and **fails the build on any dead link** (the port's
   completeness gate).

`introduction.md` stays as the first guide page. `SUMMARY.md` and `book.toml`
are NOT ported — their roles move to `config.mts`.

### Home page (`index.md`)

VitePress `layout: home` frontmatter:

- **Hero:** name `chain-builder`, tagline "A typed, dialect-aware SQL query
  builder for Rust", actions: `Get Started` → `/introduction`,
  `View on GitHub` → repo. (No logo image required; text hero.)
- **Features grid (4):** drawn from the crate's real selling points —
  - "Typed, bound values" — every value is a placeholder + typed bind, never
    interpolated.
  - "Three dialects, one API" — Postgres / MySQL / SQLite via a `Dialect`
    marker; mixing is a compile error.
  - "Fallible compilation" — `try_*` twins return `BuildError`; the panicking
    API stays for static queries.
  - "sqlx integration" — hand off `(sql, binds)` straight to sqlx, or use the
    typed `fetch_*` helpers.

## Theme & config (`.vitepress/config.mts`)

- `base: '/chain-builder/'` (GitHub Pages project site).
- `title: 'chain-builder'`, `description` from the crate.
- `themeConfig.search = { provider: 'local' }`.
- `themeConfig.nav`: Guide (`/introduction`), Reference (`/binds`), Cookbook
  (`/cookbook/http-filters-pagination`), crates.io
  (`https://crates.io/crates/chain-builder`), GitHub (repo). Plus
  `socialLinks: [{ icon: 'github', link: <repo> }]`.
- `themeConfig.sidebar`: mirror `SUMMARY.md`'s five groups, in order:
  - **Getting Started**: Introduction, Getting Started
  - **Query Building**: SELECT, WHERE, JOIN, GROUP BY · HAVING · ORDER · LIMIT,
    CTE & UNION, INSERT · UPDATE · DELETE, Upsert & RETURNING, Row Locking,
    Dynamic Building
  - **Reference**: Binds & Values, Error Handling, Executing with sqlx,
    Dialect Differences
  - **Cookbook**: HTTP Filters & Pagination, Mapping Errors to HTTP Status,
    Multi-tenant with `.db()`, Bulk Insert & Upsert, Case-insensitive Search
  - **Under the Hood**: Security Model, Internals
- Dark/light toggle: default VitePress (no config).
- No `editLink`, no carbon.

### Brand palette (`.vitepress/theme/custom.css`)

User-provided green scale:

```
50 #E8FCF6  100 #C0F7E5  200 #97F2D5  300 #6FECC4  400 #46E7B4
500 #1BCF96  600 #18B986  700 #139069  800 #0D684B  900 #083F2E  950 #031710
```

Mapped to VitePress brand tokens, chosen for WCAG-AA text contrast:

```css
:root {
  --vp-c-brand-1: #139069;   /* links / emphasis on light bg */
  --vp-c-brand-2: #18B986;   /* hover */
  --vp-c-brand-3: #0D684B;   /* solid button bg — white text passes AA */
  --vp-c-brand-soft: rgba(27, 207, 150, 0.14);
}
.dark {
  --vp-c-brand-1: #46E7B4;   /* links on dark bg */
  --vp-c-brand-2: #6FECC4;   /* hover */
  --vp-c-brand-3: #139069;   /* solid button bg on dark */
  --vp-c-brand-soft: rgba(27, 207, 150, 0.16);
}
```

The implementer verifies actual contrast on the built page and nudges shades
within the provided scale if any element fails AA (the mapping above is the
starting point, not a hard contract).

## Tooling

- Package manager **npm**; commit `package-lock.json`; CI uses `npm ci`.
- VitePress `^1.6` (latest 1.x), Node 20.
- `package.json` scripts: `docs:dev`, `docs:build`, `docs:preview` pointing at
  the `docs/site` root (the workflow runs from `docs/site`).

## CI (`.github/workflows/docs.yml` rewrite)

Same job shape (build → deploy), retargeted:

- Triggers: `paths` `docs/book/**` → `docs/site/**` (keep the workflow-self
  path).
- Build job: `actions/checkout@v5`, `actions/setup-node@v4` (Node 20, `cache:
  npm`, `cache-dependency-path: docs/site/package-lock.json`), `npm ci` +
  `npm run docs:build` (both run in `docs/site`), then
  `actions/upload-pages-artifact@v3` with `path: docs/site/.vitepress/dist`.
- Deploy job: unchanged (`actions/deploy-pages@v4`, `main`-only, `pages`
  concurrency group).

## Verification (acceptance gate)

1. `npm ci && npm run docs:build` in `docs/site` exits 0. VitePress
   **dead-link detection** failing the build is the port-completeness check —
   any broken ported link blocks the build.
2. No `,ignore` / `,no_run` info strings remain in `docs/site/**/*.md`
   (`grep -rn ',ignore\|,no_run' docs/site` returns nothing).
3. All 22 pages appear in the sidebar; the page set matches the old
   `SUMMARY.md` exactly.
4. `npm run docs:dev` opened once and eyeballed: hero renders, green brand
   visible on buttons/links, sidebar groups correct, local search returns
   results, dark/light toggle works.
5. `cargo build` / `cargo test` untouched and still green (sanity — the change
   is docs-only and adds no Rust).

## Acceptance criteria

- [ ] `docs/site/` is a working VitePress project; `docs/book/` removed.
- [ ] 22 pages ported verbatim (prose unchanged; only fence info strings
      transformed); no dead links (build-enforced).
- [ ] Home hero + 4-feature grid; nav + 5-group sidebar matching `SUMMARY.md`.
- [ ] Local search, dark/light toggle, green brand palette applied (AA-checked).
- [ ] `docs.yml` builds VitePress and deploys `docs/site/.vitepress/dist` to
      Pages; triggers on `docs/site/**`.
- [ ] `npm run docs:build` green; `git status` clean after a build (dist/cache
      gitignored).

## Known limitations / notes

- Snippets remain non-executed (same as mdBook); the SQL-comment ↔ test
  byte-match invariant is preserved by not touching snippet bodies, not by
  tooling.
- First Pages deploy after merge serves the VitePress build; the URL is
  unchanged. The `pages` concurrency group prevents overlap with any in-flight
  mdBook deploy.
- Adds a Node toolchain to the repo (docs-only); the Rust crate build is
  unaffected and shares no lockfile.

## Open questions resolved during design

- **Replace vs coexist?** Replace — one source of truth; mdBook removed.
- **Port vs restructure?** Port verbatim in the existing IA.
- **Search?** Local (minisearch), not Algolia.
- **Brand?** User-supplied green scale, AA-mapped to VitePress brand tokens.
