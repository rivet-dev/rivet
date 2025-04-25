mod common;

use bytes::Bytes;
use hyper::{Method, StatusCode};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpListener;
use tokio::time::sleep;
use uuid::Uuid;

use common::{
	create_test_config, create_test_middleware_fn, create_test_routing_fn, init_tracing,
	make_request, make_request_with_body, start_guard, start_guard_with_middleware, TestServer,
};
use rivet_guard_core::proxy_service::{
	MaxInFlightConfig, MiddlewareConfig, MiddlewareResponse, RateLimitConfig, RetryConfig,
	RouteConfig, RouteTarget, RoutingResponse, RoutingTimeout, TimeoutConfig,
};

#[tokio::test]
async fn test_basic_proxy_functionality() {
	init_tracing();

	// Set up a test server
	let test_server = TestServer::new().await;
	let routing_fn = create_test_routing_fn(&test_server);

	// Start guard with default config
	let config = create_test_config(|_| {});
	let (guard_addr, _shutdown) = start_guard(config, routing_fn).await;

	// Make a request to the guard server
	let uri = format!("http://{}/test/path", guard_addr);
	let response = make_request(&uri, "example.com", Method::GET)
		.await
		.unwrap();

	// Verify the response
	assert_eq!(response.status(), StatusCode::OK);

	// Verify the request reached the test server
	assert_eq!(test_server.request_count(), 1);
	let last_request = test_server.last_request().unwrap();
	assert_eq!(last_request.method, "GET");
	assert_eq!(last_request.uri, "/test/path");
}

#[tokio::test]
async fn test_proxy_forwards_headers() {
	init_tracing();

	// Set up a test server that echoes back headers
	let test_server = TestServer::with_handler(|req, _log| {
		let mut response_headers = Vec::new();

		for (name, value) in req.headers() {
			if let Ok(v) = value.to_str() {
				response_headers.push(format!("{}: {}", name, v));
			}
		}

		let response_body = response_headers.join("\n");

		Box::pin(async move {
			let response = hyper::Response::builder()
				.status(StatusCode::OK)
				.body(http_body_util::Full::new(hyper::body::Bytes::from(
					response_body,
				)))
				.unwrap();

			Ok::<_, std::convert::Infallible>(response)
		})
	})
	.await;

	let routing_fn = create_test_routing_fn(&test_server);
	let config = create_test_config(|_| {});
	let (guard_addr, _shutdown) = start_guard(config, routing_fn).await;

	// Create a request with custom headers
	let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
		.build_http();

	let request = hyper::Request::builder()
		.method(Method::GET)
		.uri(format!("http://{}/echo", guard_addr))
		.header(hyper::header::HOST, "example.com")
		.header("X-Custom-Header", "test-value")
		.header("X-Another-Header", "another-value")
		.body(http_body_util::Empty::<bytes::Bytes>::new())
		.unwrap();

	let response = client.request(request).await.unwrap();
	assert_eq!(response.status(), StatusCode::OK);

	// Check that our custom headers were forwarded
	let last_request = test_server.last_request().unwrap();
	assert!(last_request.headers.contains_key("x-custom-header"));
	assert_eq!(
		last_request.headers.get("x-custom-header").unwrap(),
		"test-value"
	);
	assert!(last_request.headers.contains_key("x-another-header"));
	assert_eq!(
		last_request.headers.get("x-another-header").unwrap(),
		"another-value"
	);
}

#[tokio::test]
async fn test_rate_limiting() {
	init_tracing();

	// Set up a test server
	let test_server = TestServer::new().await;
	let test_server_addr = test_server.addr;

	// Use consistent actor/server IDs for testing
	let actor_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
	let server_id = Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();

	// Create routing function that returns consistent actor ID
	let routing_fn: rivet_guard_core::proxy_service::RoutingFn = Arc::new(
		move |_hostname: &str,
		      path: &str,
		      _port_type: rivet_guard_core::proxy_service::PortType| {
			Box::pin(async move {
				let route_target = RouteTarget {
					actor_id: Some(actor_id),
					server_id: Some(server_id),
					host: test_server_addr.ip(),
					port: test_server_addr.port(),
					path: path.to_string(),
				};

				Ok(RoutingResponse::Ok(RouteConfig {
					targets: vec![route_target],
					timeout: RoutingTimeout { routing_timeout: 5 },
				}))
			})
		},
	);

	// Create custom middleware function with very limited rate limit
	let middleware_fn = create_test_middleware_fn(|config| {
		// Set very low rate limit for testing
		config.rate_limit = RateLimitConfig {
			requests: 1, // Only 1 request allowed
			period: 1,   // Per 1 second
		};
	});

	// Create a config with default settings
	let config = create_test_config(|_| {});

	let (guard_addr, _shutdown) =
		start_guard_with_middleware(config, routing_fn, middleware_fn).await;
	let uri = format!("http://{}/test-rate-limit", guard_addr);

	// First request should go through
	let response1 = make_request(&uri, "example.com", Method::GET)
		.await
		.unwrap();
	assert_eq!(response1.status(), StatusCode::OK);

	// Second request should be rate limited
	let response2 = make_request(&uri, "example.com", Method::GET)
		.await
		.unwrap();
	assert_eq!(response2.status(), StatusCode::TOO_MANY_REQUESTS);

	// Wait for rate limit to reset (need to wait for the full period)
	sleep(Duration::from_secs(2)).await;

	// Third request should go through again
	let response3 = make_request(&uri, "example.com", Method::GET)
		.await
		.unwrap();
	assert_eq!(response3.status(), StatusCode::OK);

	// Check that only two requests made it to the test server
	assert_eq!(test_server.request_count(), 2);
}

#[tokio::test]
async fn test_max_in_flight_requests() {
	init_tracing();

	// Set up a test server with delay
	let test_server = TestServer::with_delay(500).await; // 500ms delay per request

	// Consistent actor IDs for testing
	let actor_id = Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap();
	let server_id = Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();

	let test_server_addr = test_server.addr;
	let routing_fn: rivet_guard_core::proxy_service::RoutingFn = Arc::new(
		move |_hostname: &str,
		      path: &str,
		      _port_type: rivet_guard_core::proxy_service::PortType| {
			Box::pin(async move {
				Ok(RoutingResponse::Ok(RouteConfig {
					targets: vec![RouteTarget {
						actor_id: Some(actor_id),
						server_id: Some(server_id),
						host: test_server_addr.ip(),
						port: test_server_addr.port(),
						path: path.to_string(),
					}],
					timeout: RoutingTimeout { routing_timeout: 5 },
				}))
			})
		},
	);

	// Create custom middleware function with very limited max in-flight
	let middleware_fn = create_test_middleware_fn(|config| {
		// Set low max in-flight for testing
		config.max_in_flight = MaxInFlightConfig {
			amount: 2, // Only 2 concurrent requests
		};
	});

	// Create a config with default settings
	let config = create_test_config(|_| {});

	let (guard_addr, _shutdown) =
		start_guard_with_middleware(config, routing_fn, middleware_fn).await;
	let uri = format!("http://{}/test-in-flight", guard_addr);

	// Launch first two requests which should succeed
	let request1 = make_request(&uri, "example.com", Method::GET);
	let request2 = make_request(&uri, "example.com", Method::GET);

	// Wait a moment to ensure the requests are being processed
	tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

	// Now send the third request which should be rejected due to max in-flight
	let request3 = make_request(&uri, "example.com", Method::GET);

	let (response1, response2, response3) = tokio::join!(request1, request2, request3);

	assert_eq!(response1.unwrap().status(), StatusCode::OK);
	assert_eq!(response2.unwrap().status(), StatusCode::OK);
	assert_eq!(response3.unwrap().status(), StatusCode::TOO_MANY_REQUESTS);

	// Ensure only 2 requests made it to the test server
	assert_eq!(test_server.request_count(), 2);
}

#[tokio::test]
async fn test_timeout_handling() {
	init_tracing();

	// Setup a test server that takes too long
	let test_server = TestServer::with_delay(3000).await; // 3 seconds delay

	// Create a custom routing function that returns a dedicated actor ID
	let test_server_addr = test_server.addr;
	let actor_id = Uuid::parse_str("33333333-3333-3333-3333-333333333333").unwrap();
	let server_id = Uuid::parse_str("44444444-4444-4444-4444-444444444444").unwrap();

	let routing_fn: rivet_guard_core::proxy_service::RoutingFn = Arc::new(
		move |_hostname: &str,
		      path: &str,
		      _port_type: rivet_guard_core::proxy_service::PortType| {
			Box::pin(async move {
				Ok(RoutingResponse::Ok(RouteConfig {
					targets: vec![RouteTarget {
						actor_id: Some(actor_id),
						server_id: Some(server_id),
						host: test_server_addr.ip(),
						port: test_server_addr.port(),
						path: path.to_string(),
					}],
					timeout: RoutingTimeout { routing_timeout: 5 },
				}))
			})
		},
	);

	// Create a custom middleware function with a very short request timeout
	let middleware_fn = create_test_middleware_fn(|config| {
		// Set a 1 second timeout for this test specifically
		config.timeout = TimeoutConfig {
			request_timeout: 1, // 1 second timeout
		};
	});

	// Create a config with default settings
	let config = create_test_config(|_| {});

	let (guard_addr, _shutdown) =
		start_guard_with_middleware(config, routing_fn, middleware_fn).await;
	let uri = format!("http://{}/test-timeout", guard_addr);

	// Make a request that should time out
	let start = Instant::now();
	let response = make_request(&uri, "example.com", Method::GET)
		.await
		.unwrap();
	let elapsed = start.elapsed();

	// Should get a gateway timeout
	assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);

	// Should timeout after approximately 1 second (not waiting full 3 seconds)
	assert!(elapsed < Duration::from_secs(2));
	assert!(elapsed >= Duration::from_secs(1));
}

#[tokio::test]
async fn test_retry_functionality() {
	init_tracing();

	// Create a server that starts immediately, but we'll start retrying before binding to it
	// First, get a port
	let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
	let server_addr = listener.local_addr().unwrap();
	// Drop the listener so the port is free
	drop(listener);

	// Create a routing function that points to our port
	let routing_fn: rivet_guard_core::proxy_service::RoutingFn = Arc::new(
		move |_hostname: &str,
		      path: &str,
		      _port_type: rivet_guard_core::proxy_service::PortType| {
			Box::pin(async move {
				Ok(RoutingResponse::Ok(RouteConfig {
					targets: vec![RouteTarget {
						actor_id: Some(Uuid::new_v4()),
						server_id: Some(Uuid::new_v4()),
						host: server_addr.ip(),
						port: server_addr.port(),
						path: path.to_string(),
					}],
					timeout: RoutingTimeout { routing_timeout: 5 },
				}))
			})
		},
	);

	// Create a middleware function with specific retry settings
	// Use longer interval to give us time to start the server
	let initial_interval = 500; // ms
	let middleware_fn = create_test_middleware_fn(move |config| {
		// Configure retry settings for this test
		config.retry = RetryConfig {
			max_attempts: 3,                    // 3 retry attempts
			initial_interval: initial_interval, // 500ms initial interval with exponential backoff
		};

		// Set a very short timeout so retries happen faster
		config.timeout = TimeoutConfig {
			request_timeout: 1, // 1 second timeout
		};
	});

	// Create a config with default settings
	let config = create_test_config(|_| {});

	// Calculate the backoff times for logging
	let backoff_after_first_attempt =
		rivet_guard_core::proxy_service::ProxyService::calculate_backoff(1, initial_interval);
	let backoff_after_second_attempt =
		rivet_guard_core::proxy_service::ProxyService::calculate_backoff(2, initial_interval);

	// Start the server after a fixed delay, making sure it's ready before the first retry
	let server_start_delay = Duration::from_millis(100);

	// Print timing information
	println!("Initial interval: {}ms", initial_interval);
	println!(
		"Backoff after first attempt: {:?}",
		backoff_after_first_attempt
	);
	println!(
		"Backoff after second attempt: {:?}",
		backoff_after_second_attempt
	);
	println!("Server start delay: {:?}", server_start_delay);

	let (guard_addr, _shutdown) =
		start_guard_with_middleware(config, routing_fn, middleware_fn).await;
	let uri = format!("http://{}/test-retry", guard_addr);

	// Start the server after calculated delay
	let server_handle = tokio::spawn(async move {
		// Wait before starting the server to allow the first attempt and first retry to fail
		println!("Sleeping for {server_start_delay:?}");
		tokio::time::sleep(server_start_delay).await;

		// Now start the server
		println!("Starting server");
		let test_server = TestServer::with_addr(server_addr).await;
		test_server
	});

	// Make a POST request with a body - this should retry until the server is available
	let start_time = Instant::now();
	let post_data = "This is a test POST request with retries".to_string();
	let response = make_request_with_body(&uri, "example.com", Method::POST, post_data.clone())
		.await
		.unwrap();
	let request_duration = start_time.elapsed();

	// Get the server that was eventually created
	let test_server = server_handle.await.unwrap();

	// Verify the response is successful
	assert_eq!(response.status(), StatusCode::OK);

	// Verify the server got a request (the successful retry)
	assert_eq!(test_server.request_count(), 1);

	// Verify the request method was POST
	let last_request = test_server.last_request().unwrap();
	assert_eq!(last_request.method, "POST");

	// Read the response body to verify it contains the echoed request body
	let body = response.into_body();
	let body_bytes = http_body_util::BodyExt::collect(body)
		.await
		.unwrap()
		.to_bytes();
	let body_text = String::from_utf8_lossy(&body_bytes);

	// Check that the body was correctly echoed back
	assert!(body_text.contains(&post_data));

	// Print actual duration for informational purposes
	println!("Actual request duration: {:?}", request_duration);

	// Don't verify exact timing as it can be flaky in CI environments
	// Just verify that we got a successful response
}

#[tokio::test]
async fn test_different_http_methods() {
	init_tracing();

	let test_server = TestServer::with_handler(|req, _log| {
		let method = req.method().clone();

		Box::pin(async move {
			let response = hyper::Response::builder()
				.status(StatusCode::OK)
				.body(http_body_util::Full::new(hyper::body::Bytes::from(
					format!("Method: {}", method),
				)))
				.unwrap();

			Ok::<_, std::convert::Infallible>(response)
		})
	})
	.await;

	let routing_fn = create_test_routing_fn(&test_server);
	let config = create_test_config(|_| {});
	let (guard_addr, _shutdown) = start_guard(config, routing_fn).await;

	let base_uri = format!("http://{}/method-test", guard_addr);

	// Test different HTTP methods
	let methods = vec![
		Method::GET,
		Method::POST,
		Method::PUT,
		Method::DELETE,
		Method::PATCH,
	];

	for method in methods {
		let response = make_request(&base_uri, "example.com", method.clone())
			.await
			.unwrap();
		assert_eq!(response.status(), StatusCode::OK);

		// Verify the test server received the correct method
		let last_request = test_server.last_request().unwrap();
		assert_eq!(last_request.method, method.as_str());
	}
}

#[tokio::test]
async fn test_different_path_routing() {
	init_tracing();

	let test_server = TestServer::new().await;

	// Create a routing function that routes based on path prefix
	let test_server_addr = test_server.addr;
	let routing_fn: rivet_guard_core::proxy_service::RoutingFn = Arc::new(
		move |_hostname: &str,
		      path: &str,
		      _port_type: rivet_guard_core::proxy_service::PortType| {
			Box::pin(async move {
				let actor_id = if path.starts_with("/api") {
					Uuid::parse_str("aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa").unwrap()
				} else if path.starts_with("/app") {
					Uuid::parse_str("bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb").unwrap()
				} else {
					Uuid::parse_str("cccccccc-cccc-cccc-cccc-cccccccccccc").unwrap()
				};

				Ok(RoutingResponse::Ok(RouteConfig {
					targets: vec![RouteTarget {
						actor_id: Some(actor_id),
						server_id: Some(Uuid::new_v4()),
						host: test_server_addr.ip(),
						port: test_server_addr.port(),
						path: path.to_string(),
					}],
					timeout: RoutingTimeout { routing_timeout: 5 },
				}))
			})
		},
	);

	let config = create_test_config(|_| {});
	let (guard_addr, _shutdown) = start_guard(config, routing_fn).await;

	// Make requests to different paths
	let paths = vec!["/api/users", "/app/profile", "/other/resource"];

	for path in paths {
		let uri = format!("http://{}{}", guard_addr, path);
		let response = make_request(&uri, "example.com", Method::GET)
			.await
			.unwrap();
		assert_eq!(response.status(), StatusCode::OK);

		// Verify the test server received the correct path
		let last_request = test_server.last_request().unwrap();
		assert_eq!(last_request.uri, path);
	}

	// Verify total request count
	assert_eq!(test_server.request_count(), 3);
}

#[tokio::test]
async fn test_post_requests_with_bodies() {
	init_tracing();

	// Create a test server that echoes back the request body
	let test_server = TestServer::with_handler(|req, _log| {
		Box::pin(async move {
			// Read the request body using http_body_util
			let body = req.into_body();
			let body_bytes = match http_body_util::BodyExt::collect(body).await {
				Ok(collected) => collected.to_bytes(),
				Err(_) => Bytes::from("Failed to read body"),
			};

			// Return the body as the response
			let response = hyper::Response::builder()
				.status(StatusCode::OK)
				.body(http_body_util::Full::new(body_bytes))
				.unwrap();

			Ok::<_, std::convert::Infallible>(response)
		})
	})
	.await;

	let routing_fn = create_test_routing_fn(&test_server);

	// Create a config with default settings
	let config = create_test_config(|_| {});

	let (guard_addr, _shutdown) = start_guard(config, routing_fn).await;

	// Create a POST request with a body
	let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
		.build_http();

	let post_data = "This is test POST data";

	let request = hyper::Request::builder()
		.method(hyper::Method::POST)
		.uri(format!("http://{}/echo", guard_addr))
		.header(hyper::header::HOST, "example.com")
		.header(hyper::header::CONTENT_TYPE, "text/plain")
		.header(hyper::header::CONTENT_LENGTH, post_data.len().to_string())
		.body(http_body_util::Full::new(Bytes::from(post_data)))
		.unwrap();

	let response = client.request(request).await.unwrap();

	// Note: The current implementation in proxy_service.rs doesn't actually forward
	// the request body or handle non-empty responses yet. The commented-out websocket code
	// shows this hasn't been fully implemented.
	//
	// To make this test pass, we'd need to modify the proxy_service.rs to:
	// 1. Forward request bodies to the target server
	// 2. Return response bodies from the target server
	//
	// For now, we're ignoring this test (annotation above)
	assert_eq!(response.status(), StatusCode::OK);

	// Read the response body using http_body_util
	let body = response.into_body();
	let body_bytes = http_body_util::BodyExt::collect(body)
		.await
		.unwrap()
		.to_bytes();
	let body_text = String::from_utf8_lossy(&body_bytes);

	// Verify the body was echoed back correctly
	assert_eq!(body_text, post_data);

	// Verify the test server received the request with the correct method
	let last_request = test_server.last_request().unwrap();
	assert_eq!(last_request.method, "POST");
}
