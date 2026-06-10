import { readFileSync, writeFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { defineConfig } from 'vitepress'

const repo = 'https://github.com/AssetsArt/chain-builder'

// Pages in reading order (mirrors the sidebar) — backs the generated
// `llms-full.txt`. index.md (the hero home) is intentionally excluded.
const PAGES = [
  'introduction',
  'getting-started',
  'query/select',
  'query/where',
  'query/join',
  'query/group-having-order-limit',
  'query/cte-union',
  'query/insert-update-delete',
  'query/upsert-returning',
  'query/locking',
  'query/dynamic',
  'binds',
  'error-handling',
  'sqlx',
  'dialects',
  'cookbook/http-filters-pagination',
  'cookbook/http-error-mapping',
  'cookbook/multi-tenant',
  'cookbook/bulk-insert-upsert',
  'cookbook/search',
  'security',
  'internals',
]

export default defineConfig({
  title: 'Chain builder',
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
  // Generate llms-full.txt (every page concatenated, in reading order) into the
  // built site root. The hand-authored index lives at public/llms.txt. Both are
  // served at /chain-builder/llms{,-full}.txt.
  buildEnd(siteConfig) {
    const stripFrontmatter = (s: string) =>
      s.replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n/, '')
    const parts = [
      '# Chain builder — full documentation',
      '',
      '> A typed, dialect-aware SQL query builder for Rust (PostgreSQL, MySQL,',
      '> SQLite). Every page of the documentation site, concatenated in reading',
      '> order. Source: https://github.com/AssetsArt/chain-builder',
    ]
    for (const slug of PAGES) {
      const md = readFileSync(resolve(siteConfig.srcDir, `${slug}.md`), 'utf8')
      parts.push('', '', `<!-- source: ${slug}.md -->`, '', stripFrontmatter(md).trim())
    }
    writeFileSync(resolve(siteConfig.outDir, 'llms-full.txt'), `${parts.join('\n')}\n`)
  },
})
