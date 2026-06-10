# VitePress Docs Site Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the mdBook docs site with a VitePress site (knexjs.org style, green brand), porting the 22 existing pages verbatim, deployed to the same GitHub Pages target.

**Architecture:** A new VitePress project rooted at `docs/site/` (config in `.vitepress/config.mts`, theme override in `.vitepress/theme/`). The 22 mdBook pages are copied with two mechanical transforms (code-fence info-string normalization + mdBook hidden-line removal). `docs.yml` is rewritten to build VitePress and deploy `docs/site/.vitepress/dist`. `docs/book/` is deleted. No Rust code changes.

**Tech Stack:** VitePress ^1.6 (Node 20+, npm), Shiki highlighting, VitePress local search. Spec: `docs/superpowers/specs/2026-06-10-vitepress-docs-design.md` — read it before starting.

**Conventions for every task:**
- The "test" for a docs task is `npm run docs:build` (VitePress fails the build on dead `.md` links) plus the grep gates below. Run builds from inside `docs/site`.
- Do NOT edit page prose or snippet bodies (the SQL-output comments are byte-matched to `tests/`). Only the two mechanical transforms in Task 2 touch page content.
- Commit after each task. Never `git push` (the orchestrator handles PR/merge).
- `node_modules/`, `.vitepress/dist/`, `.vitepress/cache/` are gitignored — never commit them.

---

### Task 1: Scaffold the VitePress project

**Files:**
- Create: `docs/site/package.json`
- Create: `docs/site/.gitignore`
- Create: `docs/site/.vitepress/config.mts`
- Create: `docs/site/.vitepress/theme/index.ts`
- Create: `docs/site/.vitepress/theme/custom.css` (empty stub; brand colors land in Task 4)
- Create: `docs/site/index.md` (temporary placeholder; real hero in Task 2)
- Create (generated): `docs/site/package-lock.json`

- [ ] **Step 1: Create `docs/site/package.json`**

```json
{
  "name": "chain-builder-docs",
  "version": "0.0.0",
  "private": true,
  "type": "module",
  "scripts": {
    "docs:dev": "vitepress dev",
    "docs:build": "vitepress build",
    "docs:preview": "vitepress preview"
  },
  "devDependencies": {
    "vitepress": "^1.6.4"
  }
}
```

- [ ] **Step 2: Create `docs/site/.gitignore`**

```gitignore
node_modules/
.vitepress/dist/
.vitepress/cache/
```

- [ ] **Step 3: Create `docs/site/.vitepress/config.mts`** (minimal; nav/sidebar/search added in Task 3)

```ts
import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'chain-builder',
  description:
    'A typed, dialect-aware SQL query builder for Rust (PostgreSQL/MySQL/SQLite)',
  base: '/chain-builder/',
})
```

- [ ] **Step 4: Create `docs/site/.vitepress/theme/index.ts`**

```ts
import DefaultTheme from 'vitepress/theme'
import './custom.css'

export default DefaultTheme
```

- [ ] **Step 5: Create `docs/site/.vitepress/theme/custom.css`** (empty stub)

```css
/* Brand palette lands in Task 4. */
```

- [ ] **Step 6: Create `docs/site/index.md`** (temporary placeholder)

```md
# chain-builder

Scaffold placeholder — replaced with the hero home in Task 2.
```

- [ ] **Step 7: Install dependencies (generates the lockfile) and build the stub**

```bash
cd docs/site && npm install && npm run docs:build
```
Expected: install succeeds, `vitepress build` exits 0, output written to `docs/site/.vitepress/dist`. `package-lock.json` now exists.

- [ ] **Step 8: Verify the working tree is clean of build artifacts**

Run: `cd /Users/detoro/code/chain-builder && git status --porcelain docs/site`
Expected: only `package.json`, `package-lock.json`, `.gitignore`, `.vitepress/config.mts`, `.vitepress/theme/index.ts`, `.vitepress/theme/custom.css`, `index.md` show as untracked. NOT `node_modules/`, `.vitepress/dist/`, or `.vitepress/cache/` (the `.gitignore` excludes them).

- [ ] **Step 9: Commit**

```bash
cd /Users/detoro/code/chain-builder
git add docs/site/package.json docs/site/package-lock.json docs/site/.gitignore docs/site/.vitepress docs/site/index.md
git commit -m "docs(site): scaffold VitePress project"
```

**BLOCKED fallback:** if `npm install` fails to resolve `vitepress@^1.6.4`, pin to the latest published `1.x` shown by `npm view vitepress version` and use that exact version; do not fall back to a `2.x` pre-release.

---

### Task 2: Port the 22 pages + write the real hero home

**Files:**
- Create: 22 files under `docs/site/` (mirrors of `docs/book/src/**/*.md` except `SUMMARY.md`)
- Modify: `docs/site/index.md` (replace placeholder with hero)

- [ ] **Step 1: Write the port script** `docs/site/port.mjs` (temporary; deleted in Step 5)

```js
// One-shot port: copy docs/book/src/*.md → docs/site/, applying two transforms.
import { readFileSync, writeFileSync, mkdirSync, readdirSync, statSync } from 'node:fs'
import { dirname, join, relative } from 'node:path'

const SRC = 'docs/book/src'
const DST = 'docs/site'

function walk(dir) {
  const out = []
  for (const name of readdirSync(dir)) {
    const p = join(dir, name)
    if (statSync(p).isDirectory()) out.push(...walk(p))
    else if (name.endsWith('.md') && name !== 'SUMMARY.md') out.push(p)
  }
  return out
}

// Split a line into (prefix, content) where prefix is leading whitespace plus
// any markdown blockquote markers (`>`), so fences keep working inside
// blockquote callouts (`> ```rust,ignore`) and indented list items.
function split(line) {
  const m = line.match(/^(\s*(?:>\s?)*)/)
  const prefix = m ? m[1] : ''
  return [prefix, line.slice(prefix.length)]
}

let fences = 0
let hidden = 0
for (const src of walk(SRC)) {
  const rel = relative(SRC, src)
  const dst = join(DST, rel)
  const lines = readFileSync(src, 'utf8').split('\n')
  const result = []
  let inRust = false
  for (const line of lines) {
    const [prefix, content] = split(line)
    if (!inRust && content.startsWith('```')) {
      const info = content.slice(3).trim()
      const lang = info.split(/[,\s]/)[0]
      if (lang === 'rust') {
        inRust = true
        fences++
        result.push(prefix + '```rust') // normalize rust,ignore / rust,no_run → rust
        continue
      }
      result.push(line)
      continue
    }
    if (inRust) {
      if (content.startsWith('```')) {
        inRust = false
        result.push(line)
        continue
      }
      // mdBook hidden-line: "# ..." or bare "#" (NOT "#[" / "#!" attributes).
      if (content.startsWith('# ') || content === '#') {
        hidden++
        continue
      }
      result.push(line)
      continue
    }
    result.push(line)
  }
  mkdirSync(dirname(dst), { recursive: true })
  writeFileSync(dst, result.join('\n'))
}
console.log(`ported pages; rust fences normalized: ${fences}; hidden lines stripped: ${hidden}`)
```

- [ ] **Step 2: Run the port script**

```bash
cd /Users/detoro/code/chain-builder && node docs/site/port.mjs
```
Expected output: `rust fences normalized: 156; hidden lines stripped: 2`. If either number differs, STOP and investigate before continuing — those are the spec's verified counts.

- [ ] **Step 3: Verify the transforms with grep gates**

```bash
cd /Users/detoro/code/chain-builder
grep -rn ',ignore\|,no_run' docs/site --include='*.md'        # expect: no output
grep -rn '^# async fn demo\|^# Ok(()) }' docs/site            # expect: no output
ls docs/site/*.md docs/site/query/*.md docs/site/cookbook/*.md | wc -l   # expect: 22 (incl. index.md = 23 total; see note)
```
Note: `docs/site/index.md` is the home (still the placeholder until Step 4), so the three globs list 23 files total (8 top-level incl. index.md + 9 query + 5 cookbook). The 22 *ported* pages are everything except `index.md`.

- [ ] **Step 4: Replace `docs/site/index.md` with the hero home**

```md
---
layout: home
hero:
  name: chain-builder
  text: Typed SQL query builder for Rust
  tagline: One dialect-aware builder for PostgreSQL, MySQL, and SQLite — values always bound, identifiers always escaped.
  actions:
    - theme: brand
      text: Get Started
      link: /introduction
    - theme: alt
      text: View on GitHub
      link: https://github.com/AssetsArt/chain-builder
features:
  - title: Typed, bound values
    details: Every value becomes a placeholder plus a typed bind — never interpolated into the SQL string.
  - title: Three dialects, one API
    details: Postgres, MySQL, and SQLite via a Dialect marker. Mixing dialects is a compile error, not a runtime surprise.
  - title: Fallible compilation
    details: try_* twins return a typed BuildError; the panicking API stays for static, hand-written queries.
  - title: sqlx integration
    details: Hand off (sql, binds) straight to sqlx, or use the typed fetch_* helpers.
---
```

- [ ] **Step 5: Delete the port script and build**

```bash
cd /Users/detoro/code/chain-builder && rm docs/site/port.mjs
cd docs/site && npm run docs:build
```
Expected: build exits 0. A dead-link error here means a ported relative link doesn't resolve — fix the LINK in the offending page to match the real target file (do not invent new targets); re-run.

- [ ] **Step 6: Commit**

```bash
cd /Users/detoro/code/chain-builder
git add docs/site/*.md docs/site/query docs/site/cookbook
git commit -m "docs(site): port 22 pages from mdBook + hero home"
```

**BLOCKED fallback:** if the build reports a dead link to an anchor (`#...`) rather than a file, that is a pre-existing cross-page anchor (the spec lists 5); VitePress does not normally fail on those — if it does under strict config, the fix is to confirm the target heading exists in the ported page (it does, the port is verbatim) and leave the link; do not delete it.

---

### Task 3: Full site config (nav, sidebar, search)

**Files:**
- Modify: `docs/site/.vitepress/config.mts` (replace with the complete config)

- [ ] **Step 1: Replace `docs/site/.vitepress/config.mts`** with the full config

```ts
import { defineConfig } from 'vitepress'

const repo = 'https://github.com/AssetsArt/chain-builder'

export default defineConfig({
  title: 'chain-builder',
  description:
    'A typed, dialect-aware SQL query builder for Rust (PostgreSQL/MySQL/SQLite)',
  base: '/chain-builder/',
  themeConfig: {
    search: { provider: 'local' },
    nav: [
      { text: 'Guide', link: '/introduction' },
      { text: 'Reference', link: '/binds' },
      { text: 'Cookbook', link: '/cookbook/http-filters-pagination' },
      { text: 'crates.io', link: 'https://crates.io/crates/chain-builder' },
    ],
    socialLinks: [{ icon: 'github', link: repo }],
    sidebar: [
      {
        text: 'Getting Started',
        items: [
          { text: 'Introduction', link: '/introduction' },
          { text: 'Getting Started', link: '/getting-started' },
        ],
      },
      {
        text: 'Query Building',
        items: [
          { text: 'SELECT', link: '/query/select' },
          { text: 'WHERE', link: '/query/where' },
          { text: 'JOIN', link: '/query/join' },
          { text: 'GROUP BY · HAVING · ORDER · LIMIT', link: '/query/group-having-order-limit' },
          { text: 'CTE & UNION', link: '/query/cte-union' },
          { text: 'INSERT · UPDATE · DELETE', link: '/query/insert-update-delete' },
          { text: 'Upsert & RETURNING', link: '/query/upsert-returning' },
          { text: 'Row Locking', link: '/query/locking' },
          { text: 'Dynamic Building', link: '/query/dynamic' },
        ],
      },
      {
        text: 'Reference',
        items: [
          { text: 'Binds & Values', link: '/binds' },
          { text: 'Error Handling', link: '/error-handling' },
          { text: 'Executing with sqlx', link: '/sqlx' },
          { text: 'Dialect Differences', link: '/dialects' },
        ],
      },
      {
        text: 'Cookbook',
        items: [
          { text: 'HTTP Filters & Pagination', link: '/cookbook/http-filters-pagination' },
          { text: 'Mapping Errors to HTTP Status', link: '/cookbook/http-error-mapping' },
          { text: 'Multi-tenant with .db()', link: '/cookbook/multi-tenant' },
          { text: 'Bulk Insert & Upsert', link: '/cookbook/bulk-insert-upsert' },
          { text: 'Case-insensitive Search', link: '/cookbook/search' },
        ],
      },
      {
        text: 'Under the Hood',
        items: [
          { text: 'Security Model', link: '/security' },
          { text: 'Internals', link: '/internals' },
        ],
      },
    ],
  },
})
```

- [ ] **Step 2: Build and verify the sidebar covers all 22 pages**

```bash
cd docs/site && npm run docs:build
```
Expected: exits 0. Cross-check: the sidebar has 2 + 9 + 4 + 5 + 2 = 22 leaf links, matching the ported page set (verifies no page is orphaned and no link is dead).

- [ ] **Step 3: Commit**

```bash
cd /Users/detoro/code/chain-builder
git add docs/site/.vitepress/config.mts
git commit -m "docs(site): nav, 5-group sidebar, local search"
```

---

### Task 4: Green brand theme

**Files:**
- Modify: `docs/site/.vitepress/theme/custom.css`

- [ ] **Step 1: Replace `docs/site/.vitepress/theme/custom.css`** with the brand palette

```css
/* chain-builder green brand — user-supplied scale, AA-mapped to VitePress tokens. */
:root {
  --vp-c-brand-1: #139069; /* links / emphasis on light bg (scale 700) */
  --vp-c-brand-2: #18b986; /* hover (600) */
  --vp-c-brand-3: #0d684b; /* solid button bg — white text passes AA (800) */
  --vp-c-brand-soft: rgba(27, 207, 150, 0.14);

  /* Hero name gradient (knex-style). */
  --vp-home-hero-name-color: transparent;
  --vp-home-hero-name-background: linear-gradient(120deg, #1bcf96, #139069);
}

.dark {
  --vp-c-brand-1: #46e7b4; /* links on dark bg (400) */
  --vp-c-brand-2: #6fecc4; /* hover (300) */
  --vp-c-brand-3: #139069; /* solid button bg on dark (700) */
  --vp-c-brand-soft: rgba(27, 207, 150, 0.16);
}
```

- [ ] **Step 2: Build**

```bash
cd docs/site && npm run docs:build
```
Expected: exits 0.

- [ ] **Step 3: Eyeball in the dev server** (manual, ~1 min)

```bash
cd docs/site && npm run docs:dev
```
Open the printed local URL. Verify: hero name shows the green gradient; "Get Started" button is green; sidebar links are green on hover; toggle dark mode (top-right) and confirm links/buttons use the lighter green and stay readable; the search box (top) opens and returns results for a query like "where". Stop the dev server (Ctrl-C) when done.

- [ ] **Step 4: Commit**

```bash
cd /Users/detoro/code/chain-builder
git add docs/site/.vitepress/theme/custom.css
git commit -m "docs(site): green brand palette (light + dark, AA)"
```

**BLOCKED fallback:** if white button text fails contrast against `--vp-c-brand-3` in light mode, darken it one step to `#083f2e` (scale 900); if dark-mode links are too dim, lighten `--vp-c-brand-1` to `#6fecc4` (300). Stay within the supplied scale.

---

### Task 5: Rewrite the deploy workflow

**Files:**
- Modify: `.github/workflows/docs.yml` (replace the whole file)

- [ ] **Step 1: Replace `.github/workflows/docs.yml`** with the VitePress build

```yaml
name: Docs

# Builds the VitePress documentation site and deploys it to GitHub Pages.
#
# Triggers:
#   - Pushing to `main` when the site or this workflow changes — build + deploy.
#   - Pull requests touching the same paths — build only (no deploy).
#   - Manually via the "Run workflow" button (workflow_dispatch).

on:
  push:
    branches: [main]
    paths:
      - "docs/site/**"
      - ".github/workflows/docs.yml"
  pull_request:
    paths:
      - "docs/site/**"
      - ".github/workflows/docs.yml"
  workflow_dispatch:

permissions:
  contents: read

concurrency:
  group: pages
  cancel-in-progress: false

jobs:
  build:
    name: Build site
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v5

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: npm
          cache-dependency-path: docs/site/package-lock.json

      - name: Install
        working-directory: docs/site
        run: npm ci

      - name: Build
        working-directory: docs/site
        run: npm run docs:build

      - name: Upload Pages artifact
        uses: actions/upload-pages-artifact@v3
        with:
          path: docs/site/.vitepress/dist

  deploy:
    name: Deploy to GitHub Pages
    needs: build
    if: github.event_name != 'pull_request' && github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    permissions:
      pages: write
      id-token: write
    environment:
      name: github-pages
      url: ${{ steps.deployment.outputs.page_url }}
    steps:
      - name: Deploy
        id: deployment
        uses: actions/deploy-pages@v4
```

- [ ] **Step 2: Validate the YAML parses**

```bash
cd /Users/detoro/code/chain-builder
python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/docs.yml')); print('yaml ok')"
```
Expected: `yaml ok`.

- [ ] **Step 3: Confirm the deploy job is byte-identical to the old one** (the only intended changes are name, triggers, and the build job's steps)

```bash
git diff .github/workflows/docs.yml | grep -E '^\+' | grep -iE 'deploy-pages|pages: write|id-token|github-pages|concurrency|contents: read' 
```
Expected: these lines appear as context-preserved (the deploy job, permissions, and concurrency are unchanged from the mdBook workflow).

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/docs.yml
git commit -m "ci(docs): build & deploy VitePress instead of mdBook"
```

---

### Task 6: Remove the old mdBook

**Files:**
- Delete: `docs/book/` (entire directory)

- [ ] **Step 1: Remove the directory**

```bash
cd /Users/detoro/code/chain-builder && git rm -r docs/book
```
Expected: git stages the deletion of `book.toml`, `src/SUMMARY.md`, and all 22 source pages.

- [ ] **Step 2: Confirm nothing else references `docs/book`**

```bash
grep -rn 'docs/book' .github/ README.md 2>/dev/null
```
Expected: no output. (If README links to the live site URL that is unchanged — only a literal `docs/book` path reference would be a problem.)

- [ ] **Step 3: Build still works (VitePress is independent of the removed dir)**

```bash
cd docs/site && npm run docs:build
```
Expected: exits 0.

- [ ] **Step 4: Commit**

```bash
cd /Users/detoro/code/chain-builder
git commit -m "docs: remove mdBook (replaced by VitePress site)"
```

---

### Task 7: Final verification

**Files:** none (verification only)

- [ ] **Step 1: Clean build from scratch**

```bash
cd /Users/detoro/code/chain-builder/docs/site
rm -rf .vitepress/dist .vitepress/cache
npm run docs:build
```
Expected: exits 0 with no dead-link warnings.

- [ ] **Step 2: Transform gates (must all be empty)**

```bash
cd /Users/detoro/code/chain-builder
grep -rn ',ignore\|,no_run' docs/site --include='*.md'
grep -rn '^# async fn demo\|^# Ok(()) }' docs/site
```
Expected: no output from either.

- [ ] **Step 3: Page-set parity**

```bash
cd /Users/detoro/code/chain-builder
find docs/site -name '*.md' -not -name 'index.md' | wc -l    # expect: 22
```

- [ ] **Step 4: Rust crate untouched (sanity)**

```bash
cd /Users/detoro/code/chain-builder
git diff --stat main -- src tests Cargo.toml Cargo.lock        # expect: no output (docs-only branch)
cargo build 2>&1 | tail -1                                     # expect: Finished
```

- [ ] **Step 5: Working tree clean**

```bash
cd /Users/detoro/code/chain-builder && git status --porcelain
```
Expected: no output (dist/cache gitignored, all changes committed).

---

## Self-review: spec coverage

| Spec section | Task |
|---|---|
| Directory layout (`docs/site/` tree) | 1, 2 |
| Remove `docs/book/` | 6 |
| Content port (info-string transform, 156 fences) | 2 |
| mdBook hidden-line removal (2 lines) | 2 |
| Internal-link survival (build-enforced) | 2, 7 |
| Home hero + 4 features | 2 |
| Theme/config: base, title, description | 1, 3 |
| nav (Guide/Reference/Cookbook/crates.io/GitHub) | 3 |
| sidebar (5 groups, 22 leaves) | 3 |
| Local search | 3 |
| Dark/light (default) | 3 (no config) |
| Green brand palette (light+dark, AA) | 4 |
| Tooling (npm, lockfile, VitePress ^1.6) | 1 |
| CI rewrite (paths, setup-node, dist path, deploy unchanged, contents:read) | 5 |
| Verification gates (build, grep, parity, cargo sanity) | 2, 4, 7 |

All spec sections map to a task. No placeholders. Property names (`--vp-c-brand-1/2/3`, sidebar link paths, `docs/site/.vitepress/dist`) are consistent across tasks.
