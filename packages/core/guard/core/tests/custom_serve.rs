mod common;

use std::sync::{Arc, Mutex};

use anyhow::*;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::body::Incoming as BodyIncoming;
use hyper::{Method, Request, Response, StatusCode};
use hyper_tungstenite::HyperWebsocket;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use common::{create_test_cache_key_fn, create_test_config, init_tracing, start_guard};
use rivet_guard_core::custom_serve::CustomServeTrait;
use rivet_guard_core::proxy_service::{ResponseBody, RoutingFn, RoutingOutput};
use rivet_guard_core::request_context::RequestContext;

// Track what was called for testing
#[derive(Clone, Debug, Default)]
struct CallTracker {
	http_calls: Arc<Mutex<Vec<String>>>,
	websocket_calls: Arc<Mutex<Vec<String>>>,
}

// Test implementation of CustomServeTrait
struct TestCustomServe {
	tracker: CallTracker,
}

#[async_trait]
impl CustomServeTrait for TestCustomServe {
	async fn handle_request(
		&self,
		req: Request<BodyIncoming>,
		_request_context: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
		// Track this HTTP call
		let path = req.uri().path().to_string();
		self.tracker.http_calls.lock().unwrap().push(path.clone());

		// Read request body if present
		let (parts, body) = req.into_parts();
		let body_bytes = match http_body_util::BodyExt::collect(body).await {
			Result::Ok(collected) => collected.to_bytes(),
			Err(_) => Bytes::new(),
		};

		// Create a test response
		let response_body = format!(
			"Custom HTTP handler - Path: {}, Method: {}, Body: {}",
			parts.uri.path(),
			parts.method,
			String::from_utf8_lossy(&body_bytes)
		);

		let response = Response::builder()
			.status(StatusCode::OK)
			.header("X-Custom-Handler", "true")
			.body(ResponseBody::Full(Full::new(Bytes::from(response_body))))?;

		Ok(response)
	}

	async fn handle_websocket(
		&self,
		client_ws: HyperWebsocket,
		_headers: &hyper::HeaderMap,
		_path: &str,
		_request_context: &mut RequestContext,
	) -> std::result::Result<(), (HyperWebsocket, anyhow::Error)> {
		// Track this WebSocket call
		// Note: We no longer have the path from the request, so we'll track a generic call
		self.tracker
			.websocket_calls
			.lock()
			.unwrap()
			.push("websocket".to_string());

		// Get the actual WebSocket stream
		let ws = client_ws.await?;
		let (mut sink, mut stream) = ws.split();

		// Echo messages back with "Custom: " prefix
		while let Some(msg) = stream.next().await {
			match msg {
				Result::Ok(hyper_tungstenite::tungstenite::Message::Text(text)) => {
					let response = format!("Custom: {}", text);
					if let Err(e) = sink
						.send(hyper_tungstenite::tungstenite::Message::Text(
							response.into(),
						))
						.await
					{
						eprintln!("Failed to send WebSocket message: {}", e);
						break;
					}
				}
				Result::Ok(hyper_tungstenite::tungstenite::Message::Close(_)) => {
					break;
				}
				Result::Ok(_) => {}
				Err(e) => {
					eprintln!("WebSocket error: {}", e);
					break;
				}
			}
		}

		Ok(())
	}
}

// Create routing function that returns CustomServe
fn create_custom_serve_routing_fn(tracker: CallTracker) -> RoutingFn {
	Arc::new(
		move |_hostname: &str,
		      _path: &str,
		      _port_type: rivet_guard_core::proxy_service::PortType,
		      _headers: &hyper::HeaderMap| {
			let tracker = tracker.clone();
			Box::pin(async move {
				let custom_serve = TestCustomServe { tracker };
				Ok(RoutingOutput::CustomServe(Arc::new(custom_serve)))
			})
		},
	)
}

#[tokio::test]
async fn test_custom_serve_http_request() {
	init_tracing();

	// Create tracker to verify calls
	let tracker = CallTracker::default();

	// Create routing function that returns CustomServe
	let routing_fn = create_custom_serve_routing_fn(tracker.clone());

	// Start guard with custom routing
	let config = create_test_config(|_| {});
	let (guard_addr, _shutdown) = start_guard(config, routing_fn).await;

	// Make an HTTP request
	let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
		.build_http();

	let request = Request::builder()
		.method(Method::POST)
		.uri(format!("http://{}/test/custom/path", guard_addr))
		.header(hyper::header::HOST, "example.com")
		.header(hyper::header::CONTENT_TYPE, "text/plain")
		.body(Full::new(Bytes::from("Test Body")))
		.unwrap();

	let response = client.request(request).await.unwrap();

	// Verify response
	assert_eq!(response.status(), StatusCode::OK);
	assert_eq!(response.headers().get("X-Custom-Handler").unwrap(), "true");

	// Read response body
	let body = http_body_util::BodyExt::collect(response.into_body())
		.await
		.unwrap()
		.to_bytes();
	let body_str = String::from_utf8_lossy(&body);

	assert!(body_str.contains("Custom HTTP handler"));
	assert!(body_str.contains("Path: /test/custom/path"));
	assert!(body_str.contains("Method: POST"));
	assert!(body_str.contains("Body: Test Body"));

	// Verify the custom handler was called
	let http_calls = tracker.http_calls.lock().unwrap();
	assert_eq!(http_calls.len(), 1);
	assert_eq!(http_calls[0], "/test/custom/path");

	// Verify WebSocket handler was not called
	let ws_calls = tracker.websocket_calls.lock().unwrap();
	assert_eq!(ws_calls.len(), 0);
}

#[tokio::test]
async fn test_custom_serve_websocket() {
	init_tracing();

	// Create tracker to verify calls
	let tracker = CallTracker::default();

	// Create routing function that returns CustomServe
	let routing_fn = create_custom_serve_routing_fn(tracker.clone());

	// Start guard with custom routing
	let config = create_test_config(|_| {});
	let (guard_addr, _shutdown) = start_guard(config, routing_fn).await;

	// Connect to WebSocket through guard
	let ws_url = format!("ws://{}/ws/custom", guard_addr);
	let (mut ws_stream, response) = connect_async(&ws_url)
		.await
		.expect("Failed to connect to WebSocket");

	// Verify upgrade was successful
	assert_eq!(response.status(), StatusCode::SWITCHING_PROTOCOLS);

	// Send a test message
	let test_message = "Hello Custom WebSocket";
	ws_stream
		.send(Message::Text(test_message.to_string().into()))
		.await
		.expect("Failed to send WebSocket message");

	// Receive the echoed message with custom prefix
	let response = ws_stream.next().await;
	match response {
		Some(Result::Ok(Message::Text(text))) => {
			assert_eq!(text, format!("Custom: {}", test_message));
		}
		other => panic!("Expected text message, got: {:?}", other),
	}

	// Close the connection
	ws_stream
		.close(None)
		.await
		.expect("Failed to close WebSocket");

	// Give some time for async operations to complete
	tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

	// Verify the WebSocket handler was called
	let ws_calls = tracker.websocket_calls.lock().unwrap();
	assert_eq!(ws_calls.len(), 1);
	assert_eq!(ws_calls[0], "websocket");

	// Verify HTTP handler was not called
	let http_calls = tracker.http_calls.lock().unwrap();
	assert_eq!(http_calls.len(), 0);
}

#[tokio::test]
async fn test_custom_serve_multiple_requests() {
	init_tracing();

	// Create tracker to verify calls
	let tracker = CallTracker::default();

	// Create routing function that returns CustomServe
	let routing_fn = create_custom_serve_routing_fn(tracker.clone());

	// Start guard with custom routing
	let config = create_test_config(|_| {});
	let (guard_addr, _shutdown) = start_guard(config, routing_fn).await;

	let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
		.build_http();

	// Make multiple HTTP requests
	for i in 0..3 {
		let request = Request::builder()
			.method(Method::GET)
			.uri(format!("http://{}/test/path/{}", guard_addr, i))
			.header(hyper::header::HOST, "example.com")
			.body(http_body_util::Empty::<Bytes>::new())
			.unwrap();

		let response = client.request(request).await.unwrap();
		assert_eq!(response.status(), StatusCode::OK);
	}

	// Make multiple WebSocket connections
	for i in 0..2 {
		let ws_url = format!("ws://{}/ws/test/{}", guard_addr, i);
		let (mut ws_stream, _) = connect_async(&ws_url)
			.await
			.expect("Failed to connect to WebSocket");

		// Send and receive a message
		ws_stream
			.send(Message::Text(format!("Message {}", i).into()))
			.await
			.expect("Failed to send");

		let response = ws_stream.next().await;
		match response {
			Some(Result::Ok(Message::Text(text))) => {
				assert_eq!(text, format!("Custom: Message {}", i));
			}
			other => panic!("Unexpected response: {:?}", other),
		}

		ws_stream.close(None).await.expect("Failed to close");
	}

	// Give some time for async operations to complete
	tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

	// Verify all calls were tracked
	let http_calls = tracker.http_calls.lock().unwrap();
	assert_eq!(http_calls.len(), 3);
	assert!(http_calls.contains(&"/test/path/0".to_string()));
	assert!(http_calls.contains(&"/test/path/1".to_string()));
	assert!(http_calls.contains(&"/test/path/2".to_string()));

	let ws_calls = tracker.websocket_calls.lock().unwrap();
	assert_eq!(ws_calls.len(), 2);
	// Since we can't track paths anymore, just verify the count
	assert_eq!(ws_calls.iter().filter(|&s| s == "websocket").count(), 2);
}
