mod common;

use hyper::{Method, StatusCode};
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use common::{TestServer, create_test_config, init_tracing, start_guard};
use rivet_guard_core::RouteTarget;

// TODO:
//#[tokio::test]
//async fn test_metrics_incremented_on_request() {
//	init_tracing();
//
//	// Setup test server
//	let test_server = TestServer::new().await;
//
//	// Create routing function with consistent actor/server IDs
//	let actor_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
//	let server_id = Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();
//	let test_server_addr = test_server.addr;
//
//	let routing_fn = Arc::new(move |_hostname: &str, path: &str| {
//		Ok(RouteTarget {
//			actor_id,
//			server_id,
//			ip: test_server_addr.ip(),
//			port: test_server_addr.port(),
//			path: path.to_string(),
//		})
//	});
//
//	// Get the metrics before making requests
//	let before_total = rivet_guard_core::metrics::ACTOR_REQUEST_TOTAL
//		.with_label_values(&[
//			"11111111-1111-1111-1111-111111111111",
//			"22222222-2222-2222-2222-222222222222",
//			"GET",
//			"/test-metrics",
//		])
//		.get();
//
//	// Start guard and make a request
//	let config = create_test_config();
//	let (guard_addr, _shutdown) = start_guard(config, routing_fn).await;
//
//	let uri = format!("http://{}/test-metrics", guard_addr);
//	let response = common::make_request(&uri, "example.com", Method::GET)
//		.await
//		.unwrap();
//	assert_eq!(response.status(), StatusCode::OK);
//
//	// Check that metrics were incremented
//	let after_total = rivet_guard_core::metrics::ACTOR_REQUEST_TOTAL
//		.with_label_values(&[
//			"11111111-1111-1111-1111-111111111111",
//			"22222222-2222-2222-2222-222222222222",
//			"GET",
//			"/test-metrics",
//		])
//		.get();
//
//	assert!(after_total > before_total);
//}
//
//#[tokio::test]
//async fn test_request_duration_metrics() {
//	init_tracing();
//
//	// Setup test server with delay
//	let test_server = TestServer::with_delay(200).await; // 200ms delay
//
//	// Create routing function with consistent actor/server IDs
//	let actor_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
//	let server_id = Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();
//	let test_server_addr = test_server.addr;
//
//	let routing_fn = Arc::new(move |_hostname: &str, path: &str| {
//		Ok(RouteTarget {
//			actor_id,
//			server_id,
//			ip: test_server_addr.ip(),
//			port: test_server_addr.port(),
//			path: path.to_string(),
//		})
//	});
//
//	// Get duration metrics
//	let before_samples = rivet_guard_core::metrics::ACTOR_REQUEST_DURATION
//		.with_label_values(&[
//			"11111111-1111-1111-1111-111111111111",
//			"22222222-2222-2222-2222-222222222222",
//			"200",
//		])
//		.get_sample_count();
//
//	// Start guard and make a request
//	let config = create_test_config();
//	let (guard_addr, _shutdown) = start_guard(config, routing_fn).await;
//
//	let uri = format!("http://{}/test-duration", guard_addr);
//	let response = common::make_request(&uri, "example.com", Method::GET)
//		.await
//		.unwrap();
//	assert_eq!(response.status(), StatusCode::OK);
//
//	// Check that duration metrics were updated
//	let after_samples = rivet_guard_core::metrics::ACTOR_REQUEST_DURATION
//		.with_label_values(&[
//			"11111111-1111-1111-1111-111111111111",
//			"22222222-2222-2222-2222-222222222222",
//			"200",
//		])
//		.get_sample_count();
//
//	assert!(after_samples > before_samples);
//}
//
//#[tokio::test]
//async fn test_error_metrics() {
//	init_tracing();
//
//	// Setup test server that returns errors
//	let test_server = TestServer::with_status(StatusCode::INTERNAL_SERVER_ERROR).await;
//
//	// Create routing function with consistent actor/server IDs
//	let actor_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
//	let server_id = Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();
//	let test_server_addr = test_server.addr;
//
//	let routing_fn = Arc::new(move |_hostname: &str, path: &str| {
//		Ok(RouteTarget {
//			actor_id,
//			server_id,
//			ip: test_server_addr.ip(),
//			port: test_server_addr.port(),
//			path: path.to_string(),
//		})
//	});
//
//	// Get error metrics
//	let before_errors = rivet_guard_core::metrics::ACTOR_REQUEST_ERRORS
//		.with_label_values(&[
//			"11111111-1111-1111-1111-111111111111",
//			"22222222-2222-2222-2222-222222222222",
//			"500",
//		])
//		.get();
//
//	// Create a config with no retries to make testing simpler
//	let mut config = create_test_config();
//	Arc::get_mut(&mut config).unwrap().retry_config = RetryConfig {
//		max_attempts: 1, // Don't retry
//		initial_interval: 50,
//	};
//
//	// Start guard and make a request
//	let (guard_addr, _shutdown) = start_guard(config, routing_fn).await;
//
//	let uri = format!("http://{}/test-error", guard_addr);
//	let response = common::make_request(&uri, "example.com", Method::GET)
//		.await
//		.unwrap();
//	assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
//
//	// Check that error metrics were updated
//	let after_errors = rivet_guard_core::metrics::ACTOR_REQUEST_ERRORS
//		.with_label_values(&[
//			"11111111-1111-1111-1111-111111111111",
//			"22222222-2222-2222-2222-222222222222",
//			"500",
//		])
//		.get();
//
//	assert!(after_errors > before_errors);
//}
//
//#[tokio::test]
//async fn test_pending_request_metrics() {
//	init_tracing();
//
//	// Setup test server with delay
//	let test_server = TestServer::with_delay(300).await; // 300ms delay
//
//	// Create routing function with consistent actor/server IDs
//	let actor_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
//	let server_id = Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();
//	let test_server_addr = test_server.addr;
//
//	let routing_fn = Arc::new(move |_hostname: &str, path: &str| {
//		Ok(RouteTarget {
//			actor_id,
//			server_id,
//			ip: test_server_addr.ip(),
//			port: test_server_addr.port(),
//			path: path.to_string(),
//		})
//	});
//
//	// Start guard
//	let config = create_test_config();
//	let (guard_addr, _shutdown) = start_guard(config, routing_fn).await;
//
//	let uri = format!("http://{}/test-pending", guard_addr);
//
//	// Check initial pending count
//	let initial_pending = rivet_guard_core::metrics::ACTOR_REQUEST_PENDING
//		.with_label_values(&[
//			"11111111-1111-1111-1111-111111111111",
//			"22222222-2222-2222-2222-222222222222",
//			"GET",
//			"/test-pending",
//		])
//		.get();
//
//	assert_eq!(initial_pending, 0);
//
//	// Start a request but don't await it yet
//	let request_future = common::make_request(&uri, "example.com", Method::GET);
//
//	// Give time for the request to be processed but not completed
//	tokio::time::sleep(Duration::from_millis(50)).await;
//
//	// Check that pending count increased
//	let during_pending = rivet_guard_core::metrics::ACTOR_REQUEST_PENDING
//		.with_label_values(&[
//			"11111111-1111-1111-1111-111111111111",
//			"22222222-2222-2222-2222-222222222222",
//			"GET",
//			"/test-pending",
//		])
//		.get();
//
//	assert!(during_pending > 0);
//
//	// Complete the request
//	let response = request_future.await.unwrap();
//	assert_eq!(response.status(), StatusCode::OK);
//
//	// Give time for metrics to update
//	tokio::time::sleep(Duration::from_millis(50)).await;
//
//	// Check that pending count decreased back to 0
//	let final_pending = rivet_guard_core::metrics::ACTOR_REQUEST_PENDING
//		.with_label_values(&[
//			"11111111-1111-1111-1111-111111111111",
//			"22222222-2222-2222-2222-222222222222",
//			"GET",
//			"/test-pending",
//		])
//		.get();
//
//	assert_eq!(final_pending, 0);
//}
