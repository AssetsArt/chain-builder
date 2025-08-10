//! Main ChainBuilder implementation for building SQL queries

use serde_json::Value;
use crate::types::{Client, Method, Select, Common};
use crate::query::QueryBuilder;

/// Main query builder for constructing SQL queries
/// 
/// This is the primary interface for building SQL queries with a fluent API.
/// It supports all major SQL operations including SELECT, INSERT, UPDATE, DELETE,
/// as well as complex features like JOINs, CTEs, and subqueries.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChainBuilder {
    /// Database client type
    client: Client,
    /// Database name
    pub(crate) db: Option<String>,
    /// Table name
    pub(crate) table: Option<String>,
    /// Raw table expression with optional bind parameters
    pub(crate) table_raw: Option<(String, Option<Vec<Value>>)>,
    /// Table alias
    pub(crate) as_name: Option<String>,
    /// SELECT clauses
    pub(crate) select: Vec<Select>,
    /// Query builder for WHERE clauses and other parts
    pub(crate) query: QueryBuilder,
    /// SQL operation method
    pub(crate) method: Method,
    /// Data for INSERT/UPDATE operations
    pub(crate) insert_update: Value,
    /// Generated SQL string (cached)
    pub(crate) sql_str: String,
    /// Whether to use DISTINCT
    pub(crate) is_distinct: bool,
}

impl ChainBuilder {
    /// Create a new ChainBuilder with the specified client
    pub fn new(client: Client) -> ChainBuilder {
        ChainBuilder {
            client,
            table: None,
            table_raw: None,
            select: Vec::new(),
            as_name: None,
            db: None,
            query: QueryBuilder::default(),
            method: Method::Select,
            insert_update: Value::Null,
            sql_str: String::new(),
            is_distinct: false,
        }
    }

    /// Create a new ChainBuilder for MySQL
    #[cfg(feature = "mysql")]
    pub fn new_mysql() -> ChainBuilder {
        ChainBuilder::new(Client::Mysql)
    }
    
    #[cfg(feature = "sqlite")]
    pub fn new_sqlite() -> ChainBuilder {
        ChainBuilder::new(Client::Sqlite)
    }

    /// Set the database name
    pub fn db(&mut self, db: &str) -> &mut ChainBuilder {
        self.db = Some(db.to_string());
        self
    }

    /// Set the table name
    pub fn table(&mut self, table: &str) -> &mut ChainBuilder {
        self.table = Some(table.to_string());
        self
    }

    /// Set a raw table expression
    pub fn table_raw(
        &mut self,
        table: &str,
        val: Option<Vec<Value>>,
    ) -> &mut ChainBuilder {
        self.table_raw = Some((table.to_string(), val));
        self
    }

    /// Enable DISTINCT
    pub fn distinct(&mut self) -> &mut ChainBuilder {
        self.is_distinct = true;
        self
    }

    /// Add a SELECT clause
    pub fn select(&mut self, select: Select) -> &mut ChainBuilder {
        self.method = Method::Select;
        self.select.push(select);
        self
    }
    
    /// Add a raw SELECT expression
    pub fn select_raw(&mut self, sql: &str, binds: Option<Vec<Value>>) -> &mut ChainBuilder {
        self.method = Method::Select;
        self.select.push(Select::Raw(sql.to_string(), binds));
        self
    }
    
    /// Add DISTINCT SELECT
    pub fn select_distinct(&mut self, columns: Vec<String>) -> &mut ChainBuilder {
        self.method = Method::Select;
        self.is_distinct = true;
        self.select.push(Select::Columns(columns));
        self
    }
    
    /// Add COUNT aggregate function
    pub fn select_count(&mut self, column: &str) -> &mut ChainBuilder {
        self.method = Method::Select;
        self.select.push(Select::Raw(format!("COUNT({})", column), None));
        self
    }
    
    /// Add SUM aggregate function
    pub fn select_sum(&mut self, column: &str) -> &mut ChainBuilder {
        self.method = Method::Select;
        self.select.push(Select::Raw(format!("SUM({})", column), None));
        self
    }
    
    /// Add AVG aggregate function
    pub fn select_avg(&mut self, column: &str) -> &mut ChainBuilder {
        self.method = Method::Select;
        self.select.push(Select::Raw(format!("AVG({})", column), None));
        self
    }
    
    /// Add MAX aggregate function
    pub fn select_max(&mut self, column: &str) -> &mut ChainBuilder {
        self.method = Method::Select;
        self.select.push(Select::Raw(format!("MAX({})", column), None));
        self
    }
    
    /// Add MIN aggregate function
    pub fn select_min(&mut self, column: &str) -> &mut ChainBuilder {
        self.method = Method::Select;
        self.select.push(Select::Raw(format!("MIN({})", column), None));
        self
    }
    
    /// Add SELECT with alias
    pub fn select_alias(&mut self, column: &str, alias: &str) -> &mut ChainBuilder {
        self.method = Method::Select;
        self.select.push(Select::Raw(format!("{} AS {}", column, alias), None));
        self
    }

    /// Set INSERT data
    pub fn insert(&mut self, data: Value) -> &mut ChainBuilder {
        self.method = Method::Insert;
        self.insert_update = data;
        self
    }

    /// Set INSERT multiple rows data
    pub fn insert_many(&mut self, data: Vec<Value>) -> &mut ChainBuilder {
        self.method = Method::InsertMany;
        self.insert_update = Value::Array(data);
        self
    }

    /// Set UPDATE data
    pub fn update(&mut self, data: Value) -> &mut ChainBuilder {
        self.method = Method::Update;
        self.insert_update = data;
        self
    }
    
    /// Add INSERT IGNORE (MySQL)
    pub fn insert_ignore(&mut self, data: Value) -> &mut ChainBuilder {
        self.method = Method::Insert;
        self.insert_update = data;
        // Note: This will need special handling in the SQL compiler
        self
    }
    
    /// Add UPSERT (INSERT ... ON DUPLICATE KEY UPDATE)
    pub fn insert_or_update(&mut self, data: Value, _update_data: Value) -> &mut ChainBuilder {
        self.method = Method::Insert;
        self.insert_update = data;
        // Note: This will need special handling in the SQL compiler
        self
    }
    
    /// Add raw UPDATE statement
    pub fn update_raw(&mut self, _sql: &str, _binds: Option<Vec<Value>>) -> &mut ChainBuilder {
        self.method = Method::Update;
        // Note: This will need special handling in the SQL compiler
        self
    }
    
    /// Increment a column value
    pub fn increment(&mut self, column: &str, amount: i64) -> &mut ChainBuilder {
        self.method = Method::Update;
        let update_data = serde_json::json!({
            column: format!("{} + {}", column, amount)
        });
        self.insert_update = update_data;
        self
    }
    
    /// Decrement a column value
    pub fn decrement(&mut self, column: &str, amount: i64) -> &mut ChainBuilder {
        self.method = Method::Update;
        let update_data = serde_json::json!({
            column: format!("{} - {}", column, amount)
        });
        self.insert_update = update_data;
        self
    }

    /// Set DELETE operation
    pub fn delete(&mut self) -> &mut ChainBuilder {
        self.method = Method::Delete;
        self
    }

    /// Set table alias
    pub fn as_name(&mut self, name: &str) -> &mut ChainBuilder {
        self.as_name = Some(name.to_string());
        self
    }

    /// Add a WITH clause (CTE)
    pub fn with(&mut self, alias: &str, chain_builder: ChainBuilder) -> &mut ChainBuilder {
        self.query
            .query_common
            .push(Common::With(alias.to_string(), false, chain_builder));
        self
    }

    /// Add a recursive WITH clause (CTE)
    pub fn with_recursive(
        &mut self,
        alias: &str,
        chain_builder: ChainBuilder,
    ) -> &mut ChainBuilder {
        self.query
            .query_common
            .push(Common::With(alias.to_string(), true, chain_builder));
        self
    }

    /// Add a UNION clause
    pub fn union(&mut self, chain_builder: ChainBuilder) -> &mut ChainBuilder {
        self.query
            .query_common
            .push(Common::Union(false, chain_builder));
        self
    }

    /// Add a UNION ALL clause
    pub fn union_all(&mut self, chain_builder: ChainBuilder) -> &mut ChainBuilder {
        self.query
            .query_common
            .push(Common::Union(true, chain_builder));
        self
    }

    /// Configure query parts (WHERE, JOIN, etc.)
    pub fn query(&mut self, query: impl FnOnce(&mut QueryBuilder)) {
        query(&mut self.query);
    }

    /// Add raw SQL
    pub fn add_raw(&mut self, sql: &str, val: Option<Vec<Value>>) {
        self.query.raw.push((sql.to_string(), val));
    }

    /// Generate SQL string and bind parameters
    pub fn to_sql(&mut self) -> (String, Vec<Value>) {
        match self.client {
            #[cfg(feature = "mysql")]
            Client::Mysql => {
                let rs = crate::mysql::merge_to_sql(crate::mysql::to_sql(self));
                self.sql_str = rs.0.clone();
                (self.sql_str.clone(), rs.1)
            }
            #[cfg(feature = "sqlite")]
            Client::Sqlite => {
                let rs = crate::sqlite::merge_to_sql(crate::sqlite::to_sql(self));
                self.sql_str = rs.0.clone();
                (self.sql_str.clone(), rs.1)
            }
            #[cfg(feature = "postgres")]
            Client::Postgres => {
                panic!("PostgreSQL support not yet implemented");
            }
            _ => {
                panic!("Unsupported database client");
            }
        }
    }
}
