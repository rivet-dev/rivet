use clickhouse::{Client, Row};
use clickhouse_user_query::*;
use serde::Deserialize;
use serde_json;
use testcontainers::{runners::AsyncRunner, ContainerAsync, GenericImage, core::ContainerPort};

#[derive(Row, Deserialize)]
struct UserRow {
    id: String,
}

struct TestSetup {
    client: Client,
    _container: ContainerAsync<GenericImage>,
}

impl TestSetup {
    async fn new() -> Self {
        let clickhouse_image = GenericImage::new("clickhouse/clickhouse-server", "23.8-alpine")
            .with_exposed_port(ContainerPort::Tcp(8123))
            .with_exposed_port(ContainerPort::Tcp(9000));
            
        let container = clickhouse_image.start().await.expect("Failed to start ClickHouse container");
        
        let port = container.get_host_port_ipv4(8123).await.expect("Failed to get port");
        let client = Client::default()
            .with_url(format!("http://localhost:{}", port));
        
        // Wait for ClickHouse to be ready and create test table
        let setup = Self {
            client,
            _container: container,
        };
        
        // Wait for ClickHouse to fully start up
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        
        setup.setup_test_data().await;
        setup
    }
    
    async fn setup_test_data(&self) {
        // Create test table with sample data
        self.client
            .query("CREATE TABLE IF NOT EXISTS test_users (
                id String,
                active Bool,
                metadata Map(String, String),
                tags Array(String),
                age UInt32,
                score Float64
            ) ENGINE = Memory")
            .execute()
            .await
            .expect("Failed to create test table");
        
        // Insert test data
        self.client
            .query("INSERT INTO test_users VALUES 
                ('user1', true, {'region': 'us-east', 'tier': 'premium'}, ['verified', 'premium'], 25, 95.5),
                ('user2', false, {'region': 'us-west', 'tier': 'basic'}, ['basic'], 30, 67.2),
                ('user3', true, {'region': 'eu', 'tier': 'premium'}, ['verified', 'premium', 'beta'], 22, 88.9)")
            .execute()
            .await
            .expect("Failed to insert test data");
    }
}

#[tokio::test]
async fn test_simple_query_execution() {
    let setup = TestSetup::new().await;
    
    // Create schema
    let schema = Schema::new(vec![
        Property::new("active".to_string(), false, PropertyType::Bool).unwrap(),
    ]).unwrap();
    
    // Create query
    let query_expr = QueryExpr::BoolEqual {
        property: "active".to_string(),
        subproperty: None,
        value: true,
    };
    
    // Build query
    let builder = UserDefinedQueryBuilder::new(&schema, &query_expr).unwrap();
    
    // Execute query
    let query = setup.client.query(&format!("SELECT id FROM test_users WHERE {}", builder.where_expr()));
    let query = builder.bind_to(query);
    
    let result: Vec<String> = query
        .fetch_all::<UserRow>()
        .await
        .expect("Query execution failed")
        .into_iter()
        .map(|user| user.id)
        .collect();
    
    // Should return user1 and user3 (active users)
    assert_eq!(result.len(), 2);
    assert!(result.contains(&"user1".to_string()));
    assert!(result.contains(&"user3".to_string()));
}

#[tokio::test]
async fn test_subproperty_query_execution() {
    let setup = TestSetup::new().await;
    
    // Create schema with map support
    let schema = Schema::new(vec![
        Property::new("metadata".to_string(), true, PropertyType::String).unwrap(),
    ]).unwrap();
    
    // Query for premium tier users
    let query_expr = QueryExpr::StringEqual {
        property: "metadata".to_string(),
        subproperty: Some("tier".to_string()),
        value: "premium".to_string(),
    };
    
    let builder = UserDefinedQueryBuilder::new(&schema, &query_expr).unwrap();
    
    let query = setup.client.query(&format!("SELECT id FROM test_users WHERE {}", builder.where_expr()));
    let query = builder.bind_to(query);
    
    let result: Vec<String> = query
        .fetch_all::<UserRow>()
        .await
        .expect("Query execution failed")
        .into_iter()
        .map(|user| user.id)
        .collect();
    
    // Should return user1 and user3 (premium tier)
    assert_eq!(result.len(), 2);
    assert!(result.contains(&"user1".to_string()));
    assert!(result.contains(&"user3".to_string()));
}

#[tokio::test]
async fn test_array_contains_query_execution() {
    let setup = TestSetup::new().await;
    
    // Create schema with array support
    let schema = Schema::new(vec![
        Property::new("tags".to_string(), false, PropertyType::ArrayString).unwrap(),
    ]).unwrap();
    
    // Query for users with specific tags
    let query_expr = QueryExpr::ArrayContains {
        property: "tags".to_string(),
        subproperty: None,
        values: vec!["verified".to_string(), "beta".to_string()],
    };
    
    let builder = UserDefinedQueryBuilder::new(&schema, &query_expr).unwrap();
    
    let query = setup.client.query(&format!("SELECT id FROM test_users WHERE {}", builder.where_expr()));
    let query = builder.bind_to(query);
    
    let result: Vec<String> = query
        .fetch_all::<UserRow>()
        .await
        .expect("Query execution failed")
        .into_iter()
        .map(|user| user.id)
        .collect();
    
    // Should return user1 and user3 (have verified) and user3 (has beta)
    assert_eq!(result.len(), 2);
    assert!(result.contains(&"user1".to_string()));
    assert!(result.contains(&"user3".to_string()));
}

#[tokio::test]
async fn test_complex_and_or_query_execution() {
    let setup = TestSetup::new().await;
    
    // Create comprehensive schema
    let schema = Schema::new(vec![
        Property::new("active".to_string(), false, PropertyType::Bool).unwrap(),
        Property::new("metadata".to_string(), true, PropertyType::String).unwrap(),
        Property::new("tags".to_string(), false, PropertyType::ArrayString).unwrap(),
    ]).unwrap();
    
    // Complex query: (active = true AND metadata['tier'] = 'premium') OR tags contains 'beta'
    let query_expr = QueryExpr::Or {
        exprs: vec![
            QueryExpr::And {
                exprs: vec![
                    QueryExpr::BoolEqual {
                        property: "active".to_string(),
                        subproperty: None,
                        value: true,
                    },
                    QueryExpr::StringEqual {
                        property: "metadata".to_string(),
                        subproperty: Some("tier".to_string()),
                        value: "premium".to_string(),
                    },
                ],
            },
            QueryExpr::ArrayContains {
                property: "tags".to_string(),
                subproperty: None,
                values: vec!["beta".to_string()],
            },
        ],
    };
    
    let builder = UserDefinedQueryBuilder::new(&schema, &query_expr).unwrap();
    
    let query = setup.client.query(&format!("SELECT id FROM test_users WHERE {}", builder.where_expr()));
    let query = builder.bind_to(query);
    
    let result: Vec<String> = query
        .fetch_all::<UserRow>()
        .await
        .expect("Query execution failed")
        .into_iter()
        .map(|user| user.id)
        .collect();
    
    // Should return:
    // - user1 (active=true AND tier=premium) 
    // - user3 (active=true AND tier=premium AND has beta tag)
    assert_eq!(result.len(), 2);
    assert!(result.contains(&"user1".to_string()));
    assert!(result.contains(&"user3".to_string()));
}

#[tokio::test]
async fn test_sql_injection_protection() {
    let setup = TestSetup::new().await;
    
    // Create schema
    let schema = Schema::new(vec![
        Property::new("metadata".to_string(), true, PropertyType::String).unwrap(),
    ]).unwrap();
    
    // Attempt SQL injection in subproperty
    let query_expr = QueryExpr::StringEqual {
        property: "metadata".to_string(),
        subproperty: Some("'; DROP TABLE test_users; --".to_string()),
        value: "malicious".to_string(),
    };
    
    let builder = UserDefinedQueryBuilder::new(&schema, &query_expr).unwrap();
    
    // Verify the query builds safely with proper escaping
    let where_clause = builder.where_expr();
    assert!(where_clause.contains("metadata['\\'; DROP TABLE test_users; --']"));
    assert!(where_clause.contains("= ?"));
    
    // Execute the query - it should run safely and return no results
    let query = setup.client.query(&format!("SELECT id FROM test_users WHERE {}", builder.where_expr()));
    let query = builder.bind_to(query);
    
    let result: Vec<String> = query
        .fetch_all::<UserRow>()
        .await
        .expect("Query execution should succeed safely")
        .into_iter()
        .map(|user| user.id)
        .collect();
    
    // Should return no results (not drop the table)
    assert_eq!(result.len(), 0);
    
    // Verify table still exists by running a simple query
    let table_check: Vec<String> = setup.client
        .query("SELECT id FROM test_users LIMIT 1")
        .fetch_all::<UserRow>()
        .await
        .expect("Table should still exist")
        .into_iter()
        .map(|user| user.id)
        .collect();
    
    assert!(!table_check.is_empty(), "Table should not have been dropped");
}

#[tokio::test]
async fn test_json_serialization_roundtrip() {
    let setup = TestSetup::new().await;
    
    // Create schema
    let schema = Schema::new(vec![
        Property::new("active".to_string(), false, PropertyType::Bool).unwrap(),
        Property::new("metadata".to_string(), true, PropertyType::String).unwrap(),
    ]).unwrap();
    
    // Create complex query
    let original_query = QueryExpr::And {
        exprs: vec![
            QueryExpr::BoolEqual {
                property: "active".to_string(),
                subproperty: None,
                value: true,
            },
            QueryExpr::StringEqual {
                property: "metadata".to_string(),
                subproperty: Some("tier".to_string()),
                value: "premium".to_string(),
            },
        ],
    };
    
    // Serialize to JSON
    let json = serde_json::to_string(&original_query).unwrap();
    
    // Deserialize from JSON
    let deserialized_query: QueryExpr = serde_json::from_str(&json).unwrap();
    
    // Build queries from both and verify they're identical
    let original_builder = UserDefinedQueryBuilder::new(&schema, &original_query).unwrap();
    let deserialized_builder = UserDefinedQueryBuilder::new(&schema, &deserialized_query).unwrap();
    
    assert_eq!(original_builder.where_expr(), deserialized_builder.where_expr());
    
    // Execute both queries and verify results are the same
    let query1 = setup.client.query(&format!("SELECT id FROM test_users WHERE {}", original_builder.where_expr()));
    let query1 = original_builder.bind_to(query1);
    
    let query2 = setup.client.query(&format!("SELECT id FROM test_users WHERE {}", deserialized_builder.where_expr()));
    let query2 = deserialized_builder.bind_to(query2);
    
    let result1: Vec<String> = query1
        .fetch_all::<UserRow>()
        .await
        .unwrap()
        .into_iter()
        .map(|user| user.id)
        .collect();
    
    let result2: Vec<String> = query2
        .fetch_all::<UserRow>()
        .await
        .unwrap()
        .into_iter()
        .map(|user| user.id)
        .collect();
    
    assert_eq!(result1, result2);
    assert_eq!(result1.len(), 2); // user1 and user3
}

#[tokio::test]
async fn test_numeric_query_execution() {
    let setup = TestSetup::new().await;
    
    // Create schema with number support
    let schema = Schema::new(vec![
        Property::new("score".to_string(), false, PropertyType::Number).unwrap(),
    ]).unwrap();
    
    // Query for users with score greater than 80
    let query_expr = QueryExpr::NumberGreater {
        property: "score".to_string(),
        subproperty: None,
        value: 80.0,
    };
    
    let builder = UserDefinedQueryBuilder::new(&schema, &query_expr).unwrap();
    
    let query = setup.client.query(&format!("SELECT id FROM test_users WHERE {}", builder.where_expr()));
    let query = builder.bind_to(query);
    
    let result: Vec<String> = query
        .fetch_all::<UserRow>()
        .await
        .expect("Query execution failed")
        .into_iter()
        .map(|user| user.id)
        .collect();
    
    // Should return user1 (95.5) and user3 (88.9), but not user2 (67.2)
    assert_eq!(result.len(), 2);
    assert!(result.contains(&"user1".to_string()));
    assert!(result.contains(&"user3".to_string()));
    assert!(!result.contains(&"user2".to_string()));
}

#[tokio::test]
async fn test_numeric_less_or_equal_query() {
    let setup = TestSetup::new().await;
    
    // Create schema with number support
    let schema = Schema::new(vec![
        Property::new("score".to_string(), false, PropertyType::Number).unwrap(),
    ]).unwrap();
    
    // Query for users with score <= 90
    let query_expr = QueryExpr::NumberLessOrEqual {
        property: "score".to_string(),
        subproperty: None,
        value: 90.0,
    };
    
    let builder = UserDefinedQueryBuilder::new(&schema, &query_expr).unwrap();
    
    let query = setup.client.query(&format!("SELECT id FROM test_users WHERE {}", builder.where_expr()));
    let query = builder.bind_to(query);
    
    let result: Vec<String> = query
        .fetch_all::<UserRow>()
        .await
        .expect("Query execution failed")
        .into_iter()
        .map(|user| user.id)
        .collect();
    
    // Should return user2 (67.2) and user3 (88.9), but not user1 (95.5)
    assert_eq!(result.len(), 2);
    assert!(result.contains(&"user2".to_string()));
    assert!(result.contains(&"user3".to_string()));
    assert!(!result.contains(&"user1".to_string()));
}