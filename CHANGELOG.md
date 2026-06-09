# Changelog

## [2.1.0] - 2026-06-09

### Added

- **Row locking** — `for_update()` / `for_share()` plus `skip_locked()` /
  `no_wait()` modifiers, rendered at the end of a `SELECT`. Honored by
  Postgres / MySQL; a silent no-op on SQLite (which locks the whole database,
  not rows). New `Dialect::supports_row_locking()` capability (default `true`,
  SQLite overrides to `false`). Public types: `Lock`, `LockStrength`, `LockWait`.
- **Aggregate SELECT helpers** — `select_count`/`select_sum`/`select_avg`/
  `select_min`/`select_max` (+ `_as` aliased variants) and `select_as(col, alias)`
  for plain column aliasing. Columns are escaped at compile time; `*` is passed
  through (`COUNT(*)`). Restores the 1.x aggregate ergonomics. Public types:
  `AggFn`, `SelectExpr`.
- **`Value::Decimal` (`decimal` feature)** — bind `rust_decimal::Decimal` for
  money / exact-numeric columns. Bound natively on Postgres (`NUMERIC`) and
  MySQL (`DECIMAL`); bound as TEXT on SQLite (no native decimal type).

### Notes

- All additions are backwards compatible; existing generated SQL is unchanged
  when the new builders are not used.

## [2.0.0] - 2026-06-09

### ⚠️ Breaking — complete rewrite

v2 is now the crate root; the 1.x API is **removed**. The builder is a ground-up
redesign — there is no in-place migration path; rewrite call sites against the new
API (see [README](README.md) / [docs/guide.md](docs/guide.md)).

### Added / Changed

- **`Dialect`-generic builder** `QueryBuilder<D>` over `Postgres` / `MySql` /
  `Sqlite` — mixing dialects is a compile error; dialect-correct placeholders
  (`$N` vs `?`) and identifier quoting in one place.
- **Typed binds** via `IntoBind` + internal `Value` enum (`serde_json::Value`
  removed from the core API; available behind the `json` feature).
- **All three sqlx drivers can be enabled simultaneously** — the 1.x limitation
  (duplicate `to_sqlx_query` defs across `sqlx_mysql`/`sqlx_sqlite`) is gone.
- Full surface: SELECT/INSERT/UPDATE/DELETE, WHERE (+ `or_where`/`and_where`
  groups, `ilike`, jsonb `@>`), JOINs, CTEs (`with`/`with_recursive`),
  `UNION`/`UNION ALL`, GROUP BY/HAVING/ORDER BY, LIMIT/OFFSET, `distinct`/
  `distinct_on`, `db()` qualification, upsert (`on_conflict_merge`/`_do_nothing`)
  + `RETURNING`, multi-row `insert_many`, `when`/`when_else`, `paginate`, raw
  escape hatches, and typed fetch (`fetch_all`/`one`/`optional`/`count`/`scalar`).
- Injection-safe by construction: identifiers always escaped, values always bound.

### Removed

- The entire 1.x API: `ChainBuilder`, `Client`, `Select`, `Statement`, the
  `WhereClauses`/`QueryCommon`/`HavingClauses`/`JoinMethods` traits, and the
  `mysql`/`sqlite`/`postgres`/`v2`/`dev-dependencies` cargo features.
- `serde` dependency (unused); `serde_json` is now optional (`json` feature).

### Features

`default = ["sqlx_mysql"]`; opt into `sqlx_sqlite`, `sqlx_postgres`, `json`.

## [1.0.2] - 2026-06-09

### Security

#### 🛡️ Identifier escaping (SQL injection hardening)
- **Automatic identifier escaping** for the structured API. Table, column, and
  alias names are now dialect-escaped — backticks for MySQL, double quotes for
  SQLite/PostgreSQL — with embedded quote characters doubled. Qualified names
  (`db.table.col`) are escaped segment-by-segment and `*` segments are preserved.
  This neutralizes SQL injection through attacker-controlled identifiers (e.g. a
  dynamic `ORDER BY` column from a request).
  - Applied to: `WHERE` columns, `SELECT` columns, `[db.]table` references and
    aliases, `INSERT`/`UPDATE` column keys, `GROUP BY` / `ORDER BY` columns,
    `JOIN` tables/aliases/`ON` columns, CTE aliases, `where_column`,
    `where_ilike`, `where_json_contains`, `join_using`, and the
    `select_count/sum/avg/max/min/alias` helpers.
  - **Not** escaped (expression/raw escape hatches, emitted verbatim — never pass
    untrusted input): all `*_raw` methods, `table_raw`, `add_raw`, `raw_join`,
    `on_raw`, and the `having*` helpers (which accept aggregate expressions like
    `COUNT(*)`).

#### 🧯 Denial-of-service hardening
- `insert_many` no longer panics (index-out-of-bounds) on an empty row set.
- `insert`, `insert_many`, and `update` no longer panic on a missing column
  value; they bind `NULL` defensively instead. Debug `println!` calls on these
  paths were removed.

### Changed

#### ⬆️ Dependencies
- **sqlx upgraded `0.8` → `0.9`**
  - Adapted to sqlx 0.9's `SqlSafeStr` requirement: builder-generated SQL is now
    wrapped with `sqlx::AssertSqlSafe` in the `to_sqlx_query*` / `count` helpers.
  - Updated `SqliteArguments` usages for the removed lifetime parameter in sqlx 0.9.
- Refreshed `Cargo.lock` to the latest compatible versions of all transitive deps.

### ⚠️ Breaking change
- Generated SQL now quotes identifiers. Pass **bare** names to the structured API
  (`"name"`, not `` "`name`" ``); pre-quoting would be double-escaped. If you
  relied on exact unquoted SQL output, update your expectations (or use the
  `*_raw` methods for verbatim fragments). See the README "Security" section.

## [1.0.0] - 2025-08-10

### 🎉 Major Release - Complete Rewrite and Enhancement

This is a major release with significant improvements, new features, and architectural changes. The library now supports both MySQL and SQLite with a modern, maintainable codebase.

### Added

#### 🗄️ Multi-Database Support
- **SQLite Support**: Full SQLite database support with dedicated compiler
  - SQLite-specific SQL generation with proper LIMIT/OFFSET syntax
  - SQLite sqlx integration (`to_sqlx_query_sqlite()`)
  - Complete test coverage for SQLite functionality
  - SQLite-specific type handling and bind parameter conversion

#### 🔧 Advanced WHERE Clauses
- **`where_ilike()`** - Case-insensitive LIKE (LOWER() for MySQL, ILIKE for future Postgres)
- **`where_column()`** - Column-to-column comparison (e.g., `users.age > profiles.min_age`)
- **`where_exists()` / `where_not_exists()`** - EXISTS subqueries with full query builder support
- **`where_json_contains()`** - JSON operations for MySQL (JSON_CONTAINS)
- **Enhanced subquery support** - Full query builder in subqueries

#### 📊 HAVING Clauses
- **`having()`** - Basic HAVING conditions with operators
- **`having_between()`** - HAVING BETWEEN with value ranges
- **`having_in()` / `having_not_in()`** - HAVING IN/NOT IN with value arrays
- **`having_raw()`** - Raw HAVING SQL with optional bind parameters

#### 🔢 Aggregate Functions
- **`select_count()`** - COUNT aggregate with column specification
- **`select_sum()`** - SUM aggregate for numeric columns
- **`select_avg()`** - AVG aggregate for numeric columns
- **`select_max()`** - MAX aggregate for any column type
- **`select_min()`** - MIN aggregate for any column type
- **`select_alias()`** - Column aliases (e.g., `user_id AS uid`)
- **`select_raw()`** - Raw SELECT expressions with optional bind parameters
- **`select_distinct()`** - DISTINCT SELECT with column specification

#### 🔗 Advanced JOINs
- **`full_outer_join()`** - FULL OUTER JOIN support
- **`cross_join()`** - CROSS JOIN with ON conditions
- **`join_using()`** - JOIN USING with column lists
- **Enhanced JOIN conditions** - Complex ON clauses with OR chains
- **Table aliases** - Support for table aliases in JOINs

#### 🏗️ Modern Architecture
- **Improved Project Structure**: Complete reorganization for better maintainability
  - Core types moved to `src/types.rs`
  - Main builder logic in `src/builder.rs`
  - Query functionality organized in `src/query/` module
  - Join functionality separated into `src/query/join/` module
  - Database-specific code in dedicated modules
- **Enhanced Documentation**: Comprehensive documentation with examples and API reference
- **Better Error Handling**: Improved error handling to avoid panics
- **Type Safety**: Enhanced type safety throughout the codebase

#### 🔌 sqlx Integration Enhancements
- **MySQL sqlx integration**: `to_sqlx_query()` and `to_sqlx_query_as<T>()`
- **SQLite sqlx integration**: `to_sqlx_query_sqlite()` with proper type handling
- **Count helper**: `count()` method for easy row counting
- **Proper type conversion**: Safe handling of all JSON types to database types

### Changed

#### 🏗️ Module Organization
- **Core Types**: Moved to `src/types.rs` for better organization
- **Builder Logic**: Centralized in `src/builder.rs` with improved API
- **Query System**: Reorganized into `src/query/` with clear separation of concerns
- **Join System**: Separated into `src/query/join/` with dedicated types
- **Database Compilers**: Clean separation between MySQL and SQLite compilers

#### 🔧 Trait Design
- **Improved trait definitions** for better API consistency
- **Fixed method chaining issues** for smoother development experience
- **Resolved method conflicts** between traits
- **Better trait organization** with clear responsibilities

#### 🗄️ SQL Generation
- **Fixed IN operator** to handle arrays properly
- **Fixed BETWEEN operator** to handle value pairs correctly
- **Improved statement compiler logic** for better SQL generation
- **Database-specific optimizations** for MySQL and SQLite

#### 📦 Package Structure
- **Feature flags**: Better feature organization (`mysql`, `sqlite`, `sqlx_mysql`, `sqlx_sqlite`)
- **Default features**: MySQL and sqlx_mysql enabled by default
- **Test organization**: Separate test files for MySQL and SQLite

### Removed

#### 🧹 Deprecated Files
- **Old `src/operator.rs`** - Replaced with improved operator system
- **Old `src/join/` module** - Reorganized into `src/query/join/`
- **Old `src/query_builder/` module** - Integrated into main query system

### Fixed

#### 🔧 Method Chaining
- **Fixed issues with method chaining** in query builders
- **Improved trait implementations** for better method resolution
- **Resolved method conflicts** between different traits

#### 🔧 Type System
- **Fixed type conversion issues** in test files
- **Resolved import conflicts** between modules
- **Improved type safety** throughout the codebase

#### 📚 Documentation
- **Fixed doctest compilation issues**
- **Updated examples** to reflect new API
- **Improved API documentation** with better examples

### Technical Improvements

#### 🏗️ Code Organization
- **Better separation of concerns** with modular design
- **Improved maintainability** with cleaner code structure
- **Enhanced extensibility** for future database support
- **Comprehensive testing** with all tests passing

#### 🔧 Performance
- **Optimized SQL generation** for better performance
- **Improved memory usage** with better data structures
- **Faster compilation** with better module organization

#### 🛡️ Safety
- **Enhanced error handling** to prevent panics
- **Better type safety** with improved type system
- **Safer SQL generation** with proper escaping

