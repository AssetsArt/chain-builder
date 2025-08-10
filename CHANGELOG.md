# Changelog

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

