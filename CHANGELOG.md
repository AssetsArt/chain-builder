# Changelog

## [0.1.26] - 2024-01-15

### Added
- **Improved Project Structure**: Reorganized the codebase for better maintainability and clarity
- **Enhanced Documentation**: Added comprehensive documentation with examples and API reference
- **Better Error Handling**: Improved error handling in statement compiler to avoid panics
- **Type Safety**: Enhanced type safety throughout the codebase
- **SQLite Support**: Full SQLite database support with dedicated compiler
  - SQLite-specific SQL generation
  - SQLite sqlx integration (`to_sqlx_query_sqlite()`)
  - SQLite LIMIT/OFFSET syntax support
  - Complete test coverage for SQLite functionality
- **Advanced WHERE Clauses**:
  - `where_ilike()` - Case-insensitive LIKE
  - `where_column()` - Column-to-column comparison
  - `where_exists()` / `where_not_exists()` - EXISTS subqueries
  - `where_json_contains()` - JSON operations (MySQL)
- **HAVING Clauses**:
  - `having()` - Basic HAVING conditions
  - `having_between()` - HAVING BETWEEN
  - `having_in()` / `having_not_in()` - HAVING IN/NOT IN
  - `having_raw()` - Raw HAVING SQL
- **Aggregate Functions**:
  - `select_count()` - COUNT aggregate
  - `select_sum()` - SUM aggregate
  - `select_avg()` - AVG aggregate
  - `select_max()` - MAX aggregate
  - `select_min()` - MIN aggregate
  - `select_alias()` - Column aliases
  - `select_raw()` - Raw SELECT expressions
  - `select_distinct()` - DISTINCT SELECT
- **Advanced JOINs**:
  - `full_outer_join()` - FULL OUTER JOIN
  - `cross_join()` - CROSS JOIN
  - `join_using()` - JOIN USING

### Changed
- **Module Organization**: 
  - Moved core types to `src/types.rs`
  - Moved main builder to `src/builder.rs`
  - Reorganized query functionality into `src/query/` module
  - Separated join functionality into `src/query/join/` module
- **Trait Design**: 
  - Improved trait definitions for better API consistency
  - Fixed method chaining issues
  - Resolved method conflicts between traits
- **SQL Generation**: 
  - Fixed IN operator to handle arrays properly
  - Fixed BETWEEN operator to handle value pairs correctly
  - Improved statement compiler logic

### Removed
- **Deprecated Files**: 
  - Removed old `src/operator.rs`
  - Removed old `src/join/` module
  - Removed old `src/query_builder/` module

### Fixed
- **Method Chaining**: Fixed issues with method chaining in query builders
- **Type Mismatches**: Fixed type conversion issues in test files
- **Import Conflicts**: Resolved import conflicts between modules
- **Documentation**: Fixed doctest compilation issues

### Technical Improvements
- **Code Organization**: Better separation of concerns
- **Maintainability**: Cleaner, more maintainable code structure
- **Extensibility**: More modular design for future enhancements
- **Testing**: All existing tests now pass successfully

## Migration Guide

### For Users
The public API remains largely the same, but you may need to update imports:

```rust
// Old
use chain_builder::{ChainBuilder, Client, Select, WhereClauses, QueryCommon, JoinMethods};

// New (same, but better organized internally)
use chain_builder::{ChainBuilder, Client, Select, WhereClauses, QueryCommon, JoinMethods};
```

### For Contributors
- Core types are now in `src/types.rs`
- Main builder logic is in `src/builder.rs`
- Query functionality is organized in `src/query/` module
- Database-specific code remains in `src/mysql/` module

## Future Plans
- PostgreSQL support
- Additional SQL features
- Performance optimizations
- More comprehensive documentation
