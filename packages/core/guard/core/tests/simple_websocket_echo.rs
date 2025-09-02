use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode, body::Incoming};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

// Import specific Message type for tokio_tungstenite
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as TokioMessage};
// Import Message type for hyper_tungstenite
use hyper_tungstenite::tungstenite::Message as HyperMessage;

#[tokio::test]
async fn test_websocket_echo() {
	// Start WebSocket server
	println!("Starting WebSocket server...");
	let server_addr = start_websocket_server().await;
	println!("WebSocket server started at: {}", server_addr);

	// Connect to the WebSocket server
	println!("Connecting to WebSocket server...");
	let ws_url = format!("ws://{}/ws", server_addr);
	let (mut ws_stream, response) = connect_async(&ws_url)
		.await
		.expect("Failed to connect to WebSocket server");

	println!(
		"WebSocket connection established with status: {}",
		response.status()
	);
	println!("Response headers:");
	for (name, value) in response.headers() {
		println!(
			"  {} = {}",
			name,
			value.to_str().unwrap_or("(binary value)")
		);
	}

	// Send a test message
	let test_message = "Hello WebSocket";
	println!("Sending message: {}", test_message);
	ws_stream
		.send(TokioMessage::Text(test_message.into()))
		.await
		.expect("Failed to send message");

	// Receive the echoed message
	println!("Waiting for response...");
	let response = ws_stream.next().await;
	match response {
		Some(Ok(msg)) => match msg {
			TokioMessage::Text(text) => {
				println!("Received echo response: {}", text);
				assert_eq!(text, test_message);
			}
			other => panic!("Expected text message, got: {:?}", other),
		},
		Some(Err(e)) => panic!("Error receiving message: {}", e),
		None => panic!("WebSocket stream ended unexpectedly"),
	}

	// Test binary echo
	let binary_data = vec![1, 2, 3, 4, 5];
	println!("Sending binary message...");
	ws_stream
		.send(TokioMessage::Binary(binary_data.clone().into()))
		.await
		.expect("Failed to send binary message");

	// Receive binary response
	match ws_stream.next().await {
		Some(Ok(msg)) => match msg {
			TokioMessage::Binary(data) => {
				println!("Received binary response: {} bytes", data.len());
				assert_eq!(data.to_vec(), binary_data);
			}
			other => panic!("Expected binary message, got: {:?}", other),
		},
		Some(Err(e)) => panic!("Error receiving binary message: {}", e),
		None => panic!("WebSocket stream ended unexpectedly"),
	}

	// Test ping/pong
	println!("Sending ping...");
	let ping_data = b"ping_test".to_vec();
	ws_stream
		.send(TokioMessage::Ping(ping_data.clone().into()))
		.await
		.expect("Failed to send ping");

	// Receive pong
	match ws_stream.next().await {
		Some(Ok(msg)) => match msg {
			TokioMessage::Pong(data) => {
				println!("Received pong response: {} bytes", data.len());
				assert_eq!(data.to_vec(), ping_data);
			}
			other => panic!("Expected pong message, got: {:?}", other),
		},
		Some(Err(e)) => panic!("Error receiving pong: {}", e),
		None => panic!("WebSocket stream ended unexpectedly"),
	}

	// Close the connection
	println!("Closing WebSocket connection...");
	ws_stream
		.close(None)
		.await
		.expect("Failed to close WebSocket connection");
	println!("Test completed successfully");
}

async fn start_websocket_server() -> SocketAddr {
	// Bind to a random port
	let listener = TcpListener::bind("127.0.0.1:0")
		.await
		.expect("Failed to bind");
	let addr = listener.local_addr().expect("Failed to get local address");

	// Keep a handle to prevent the server from shutting down
	let (_shutdown_tx, _shutdown_rx) = oneshot::channel::<()>();

	// Spawn the server task
	tokio::spawn(async move {
		println!("Server: Started and waiting for connections");

		loop {
			// Accept connections
			let accept_result = listener.accept().await;

			// Handle the connection
			let (stream, remote_addr) = match accept_result {
				Ok(conn) => {
					println!("Server: Accepted connection from {}", conn.1);
					conn
				}
				Err(e) => {
					eprintln!("Server: Error accepting connection: {}", e);
					continue;
				}
			};

			// Convert stream to TokioIo
			let socket = TokioIo::new(stream);

			// Spawn a task to handle the connection
			tokio::spawn(async move {
				let service = service_fn(|req: Request<Incoming>| {
					async {
						// Check if this is a WebSocket upgrade request
						if !hyper_tungstenite::is_upgrade_request(&req) {
							println!("Server: Received non-WebSocket request");
							return Ok::<_, std::convert::Infallible>(
								Response::builder()
									.status(StatusCode::BAD_REQUEST)
									.body(Full::new(Bytes::from("Not a WebSocket request")))
									.unwrap(),
							);
						}

						println!("Server: Processing WebSocket upgrade request");

						// Log request headers for debugging
						println!("Server: WebSocket request headers:");
						for (name, value) in req.headers() {
							if let Ok(val) = value.to_str() {
								println!("Server:   {} = {}", name, val);
							}
						}

						// Perform the WebSocket upgrade
						let (response, websocket) = match hyper_tungstenite::upgrade(req, None) {
							Ok(upgrade) => {
								println!("Server: Upgrade successful");
								upgrade
							}
							Err(e) => {
								println!("Server: Upgrade failed: {}", e);
								return Ok(Response::builder()
									.status(StatusCode::INTERNAL_SERVER_ERROR)
									.body(Full::new(Bytes::from(format!(
										"WebSocket upgrade error: {}",
										e
									))))
									.unwrap());
							}
						};

						// Log response status and headers
						println!(
							"Server: Returning response with status: {}",
							response.status()
						);
						for (name, value) in response.headers() {
							if let Ok(val) = value.to_str() {
								println!("Server:   {} = {}", name, val);
							}
						}

						// IMPORTANT: Handle the WebSocket in a separate task, AFTER returning the response
						tokio::spawn(async move {
							println!("Server: Waiting for WebSocket to be ready...");

							match websocket.await {
								Ok(ws_stream) => {
									println!(
										"Server: WebSocket connection established successfully"
									);

									// Split the WebSocket stream
									let (mut write, mut read) = ws_stream.split();

									// Echo loop
									println!("Server: Starting echo loop");
									while let Some(message_result) = read.next().await {
										match message_result {
											Ok(msg) => {
												match &msg {
													HyperMessage::Text(text) => {
														println!(
															"Server: Received text message: {}",
															text
														);
													}
													HyperMessage::Binary(data) => {
														println!(
															"Server: Received binary message of {} bytes",
															data.len()
														);
													}
													HyperMessage::Ping(data) => {
														println!(
															"Server: Received ping of {} bytes",
															data.len()
														);
													}
													HyperMessage::Pong(_) => {
														println!("Server: Received pong");
													}
													HyperMessage::Close(_) => {
														println!("Server: Received close message");
													}
													_ => {
														println!(
															"Server: Received unknown message type"
														);
													}
												}

												println!("Server: Echoing message back");

												// Handle the message based on its type
												let response = match msg {
													HyperMessage::Ping(data) => {
														println!(
															"Server: Converting ping to pong response"
														);
														HyperMessage::Pong(data)
													}
													other => other, // Echo back all other message types unchanged
												};

												if let Err(e) = write.send(response).await {
													println!(
														"Server: Error sending message: {}",
														e
													);
													break;
												}

												if let Err(e) = write.flush().await {
													println!("Server: Error flushing: {}", e);
													break;
												}
											}
											Err(e) => {
												println!("Server: Error reading message: {}", e);
												break;
											}
										}
									}
									println!("Server: WebSocket loop ended");
								}
								Err(e) => {
									println!("Server: Error in WebSocket handshake: {}", e);
								}
							}
						});

						// The key insight: For WebSocket upgrades, we must return the original response
						// without altering it or mapping its body to ensure the WebSocket handshake completes correctly
						let websocket_response = response;
						Ok(websocket_response)
					}
				});

				// IMPORTANT: Add .with_upgrades() to properly handle WebSocket connections
				if let Err(err) = hyper::server::conn::http1::Builder::new()
					.serve_connection(socket, service)
					.with_upgrades()
					.await
				{
					eprintln!("Error serving connection: {:?}", err);
				}
			});
		}
	});

	// Sleep a brief moment to ensure the server is ready
	tokio::time::sleep(Duration::from_millis(100)).await;

	addr
}
