//! Safe ClickHouse user-defined query builder
//!
//! This crate provides a safe way to build ClickHouse queries from user-defined expressions
//! while protecting against SQL injection attacks. All user inputs are properly validated
//! and bound using parameterized queries.
//!
//! # Example
//!
//! ```rust
//! use clickhouse_user_query::*;
//!
//! // Define the schema of allowed properties
//! let schema = Schema::new(vec![
//!     Property::new("user_id".to_string(), false, PropertyType::String).unwrap(),
//!     Property::new("metadata".to_string(), true, PropertyType::String).unwrap(),
//!     Property::new("active".to_string(), false, PropertyType::Bool).unwrap(),
//!     Property::new("tags".to_string(), false, PropertyType::ArrayString).unwrap(),
//! ]).unwrap();
//!
//! // Build a complex query expression
//! let query_expr = QueryExpr::And {
//!     exprs: vec![
//!         QueryExpr::StringEqual {
//!             property: "user_id".to_string(),
//!             subproperty: None,
//!             value: "12345".to_string(),
//!         },
//!         QueryExpr::BoolEqual {
//!             property: "active".to_string(),
//!             subproperty: None,
//!             value: true,
//!         },
//!         QueryExpr::ArrayContains {
//!             property: "tags".to_string(),
//!             subproperty: None,
//!             values: vec!["premium".to_string(), "verified".to_string()],
//!         },
//!     ],
//! };
//!
//! // Create the safe query builder
//! let builder = UserDefinedQueryBuilder::new(&schema, &query_expr).unwrap();
//!
//! // Use with ClickHouse client (commented out since clickhouse client not available in tests)
//! // let query = clickhouse::Client::default()
//! //     .query("SELECT * FROM users WHERE ?")
//! //     .bind(builder.where_expr());
//! // let final_query = builder.bind_to(query);
//! ```

// Re-export all public types for convenience
pub use builder::UserDefinedQueryBuilder;
pub use error::{Result, UserQueryError};
pub use query::QueryExpr;
pub use schema::{Property, PropertyType, Schema};

pub mod builder;
pub mod error;
pub mod query;
pub mod schema;
