mod common;

use bytes::Bytes;
use futures_util::StreamExt;
use http_body_util::{BodyExt, Full};
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode, body::Incoming};
use hyper_util::rt::TokioIo;
use rivet_util::Id;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::mpsc;

use common::{create_test_config, init_tracing, start_guard};
use rivet_guard_core::proxy_service::{
	RouteConfig, RouteTarget, RoutingFn, RoutingOutput, RoutingTimeout,
};
use uuid::Uuid;

#[tokio::test]
async fn test_streaming_response_should_timeout() {
	// This test should demonstrate that streaming responses are broken
	// The test will timeout because the proxy buffers the entire response
	// before returning it, which never happens for a streaming endpoint

	init_tracing();

	println!("Starting streaming test server...");
	let (server_addr, message_sender) = start_streaming_server().await;
	println!("Streaming server started at: {}", server_addr);

	// Create a routing function that routes to our streaming server
	let routing_fn = create_streaming_routing_fn(server_addr);

	// Start guard proxy with the routing function
	let config = create_test_config(|_| {});
	let (guard_addr, _shutdown) = start_guard(config, routing_fn).await;
	println!("Guard proxy started at: {}", guard_addr);

	// Set up a test timeout - this should be shorter than what we expect
	// the proxy would take to buffer an infinite stream
	let test_timeout = Duration::from_secs(3);

	// Create an HTTP client to make requests to the guard proxy (not directly to our server)
	let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
		.build_http();

	// Construct the request URI pointing to the guard proxy
	let uri = format!("http://{}/stream", guard_addr);
	let request = Request::builder()
		.method("GET")
		.uri(&uri)
		.header("Host", "example.com") // Required for routing
		.header("Accept", "text/event-stream")
		.body(Full::<Bytes>::new(Bytes::new()))
		.expect("Failed to build request");

	println!("Making request through guard proxy: {}", uri);

	// Start the request
	let response_future = client.request(request);

	// This is the key test: if streaming works correctly, we should get a response
	// immediately when the server sends the first chunk. If streaming is broken
	// (response is buffered), this will timeout because the server never closes
	// the connection (it's an infinite stream).
	let response_result = tokio::time::timeout(test_timeout, response_future).await;

	match response_result {
		Ok(Ok(response)) => {
			println!("✅ Got response immediately: {}", response.status());

			// If we get here, streaming is working. Let's verify we can read data
			let (parts, body) = response.into_parts();

			// Try to read the first chunk with a timeout
			let mut body_stream = body.into_data_stream();
			let first_chunk_result =
				tokio::time::timeout(Duration::from_millis(500), body_stream.next()).await;

			match first_chunk_result {
				Ok(Some(Ok(chunk))) => {
					let chunk_str = String::from_utf8_lossy(&chunk);
					println!("✅ Received first chunk: {}", chunk_str);
					assert!(
						chunk_str.contains("data: "),
						"Chunk should contain streaming data"
					);
					println!(
						"✅ Streaming is working! Received {} bytes of data",
						chunk.len()
					);

					// If we got this far, streaming is working correctly!
					// The test was designed to timeout if streaming was broken
				}
				Ok(Some(Err(e))) => {
					panic!("❌ Error reading stream chunk: {}", e);
				}
				Ok(None) => {
					panic!("❌ Stream ended unexpectedly");
				}
				Err(_) => {
					panic!("❌ Timeout reading first chunk - streaming not working properly");
				}
			}
		}
		Ok(Err(e)) => {
			panic!("❌ HTTP request failed: {}", e);
		}
		Err(_) => {
			// This is what we expect to happen when streaming is broken
			println!(
				"❌ Test timed out after {}s - streaming is NOT working!",
				test_timeout.as_secs()
			);
			println!(
				"❌ This indicates the proxy is buffering the entire response before returning it"
			);
			panic!(
				"Streaming response test timed out - proxy is buffering responses instead of streaming"
			);
		}
	}

	// Close the message sender to shut down the server
	drop(message_sender);
}

async fn start_streaming_server() -> (SocketAddr, mpsc::Sender<String>) {
	// Create a channel for sending messages to the streaming endpoint
	let (message_tx, _message_rx) = mpsc::channel::<String>(100);

	// Bind to a random port
	let listener = TcpListener::bind("127.0.0.1:0")
		.await
		.expect("Failed to bind");
	let addr = listener.local_addr().expect("Failed to get local address");

	// Spawn the server task
	tokio::spawn(async move {
		println!("Streaming server: Started and waiting for connections");

		loop {
			// Accept connections
			let accept_result = listener.accept().await;

			// Handle the connection
			let (stream, _remote_addr) = match accept_result {
				Ok(conn) => {
					println!("Streaming server: Accepted connection from {}", conn.1);
					conn
				}
				Err(e) => {
					eprintln!("Streaming server: Error accepting connection: {}", e);
					continue;
				}
			};

			// Convert stream to TokioIo
			let socket = TokioIo::new(stream);

			// Spawn a task to handle the connection
			tokio::spawn(async move {
				let service = service_fn(move |req: Request<Incoming>| {
					async move {
						println!(
							"Streaming server: Received request: {} {}",
							req.method(),
							req.uri()
						);

						// Check if this is a streaming request
						if req.uri().path() != "/stream" {
							return Ok::<_, std::convert::Infallible>(
								Response::builder()
									.status(StatusCode::NOT_FOUND)
									.body(Full::new(Bytes::from("Not found")))
									.unwrap(),
							);
						}

						println!("Streaming server: Setting up streaming response");

						// Create a large response that will take time to fully buffer
						// This simulates streaming behavior - the proxy should return this immediately
						// but if it buffers, it will wait for the full response

						// Create a large response to simulate a slow streaming endpoint
						let mut large_data = String::new();
						large_data.push_str("data: stream-started\n\n");

						// Add a lot of data to make buffering take noticeable time
						for i in 0..1000 {
							large_data.push_str(&format!("data: chunk-{}\n\n", i));
						}

						// Add a delay to simulate network latency
						tokio::time::sleep(Duration::from_millis(500)).await;

						println!("Streaming server: Returning large streaming response");
						Ok(Response::builder()
							.status(StatusCode::OK)
							.header("Content-Type", "text/event-stream")
							.header("Cache-Control", "no-cache")
							.header("Connection", "keep-alive")
							.header("Transfer-Encoding", "chunked")
							.body(Full::new(Bytes::from(large_data)))
							.unwrap())
					}
				});

				if let Err(err) = hyper::server::conn::http1::Builder::new()
					.serve_connection(socket, service)
					.await
				{
					eprintln!("Streaming server: Error serving connection: {:?}", err);
				}
			});
		}
	});

	// Sleep a brief moment to ensure the server is ready
	tokio::time::sleep(Duration::from_millis(100)).await;

	(addr, message_tx)
}

// Helper function to create a routing function for our streaming test
fn create_streaming_routing_fn(server_addr: SocketAddr) -> RoutingFn {
	std::sync::Arc::new(
		move |_hostname: &str,
		      path: &str,
		      _port_type: rivet_guard_core::proxy_service::PortType,
		      _headers: &hyper::HeaderMap| {
			Box::pin(async move {
				println!("Guard: Routing request - path: {}", path);

				if path == "/stream" {
					let target = RouteTarget {
						actor_id: Some(Id::v1(Uuid::new_v4(), 0)),
						server_id: Some(Uuid::new_v4()),
						host: server_addr.ip().to_string(),
						port: server_addr.port(),
						path: path.to_string(),
					};

					Ok(RoutingOutput::Route(RouteConfig {
						targets: vec![target],
						timeout: RoutingTimeout {
							routing_timeout: 30, // 30 seconds for routing timeout
						},
					}))
				} else {
					use rivet_guard_core::proxy_service::StructuredResponse;
					Ok(RoutingOutput::Response(StructuredResponse {
						status: StatusCode::NOT_FOUND,
						message: std::borrow::Cow::Borrowed("Not found"),
						docs: None,
					}))
				}
			})
		},
	)
}
