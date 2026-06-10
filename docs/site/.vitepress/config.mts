import { defineConfig } from 'vitepress'

const repo = 'https://github.com/AssetsArt/chain-builder'

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
})
