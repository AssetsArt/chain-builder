---
layout: home
hero:
  name: Chain builder
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
