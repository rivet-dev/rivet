use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use http_body_util::Full;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode, body::Incoming};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

#[tokio::test]
async fn test_simple_websocket() {
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
		.send(Message::Text(test_message.to_string().into()))
		.await
		.expect("Failed to send message");

	// Receive the echoed message
	println!("Waiting for response...");
	let response = ws_stream.next().await;
	match response {
		Some(Ok(msg)) => match msg {
			Message::Text(text) => {
				println!("Received echo response: {}", text);
				assert_eq!(text, test_message);
			}
			other => panic!("Expected text message, got: {:?}", other),
		},
		Some(Err(e)) => panic!("Error receiving message: {}", e),
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
                                                    hyper_tungstenite::tungstenite::Message::Text(text) => {
                                                        println!("Server: Received text message: {}", text);
                                                    },
                                                    hyper_tungstenite::tungstenite::Message::Binary(data) => {
                                                        println!("Server: Received binary message of {} bytes", data.len());
                                                    },
                                                    hyper_tungstenite::tungstenite::Message::Ping(_) => {
                                                        println!("Server: Received ping");
                                                    },
                                                    hyper_tungstenite::tungstenite::Message::Pong(_) => {
                                                        println!("Server: Received pong");
                                                    },
                                                    hyper_tungstenite::tungstenite::Message::Close(_) => {
                                                        println!("Server: Received close message");
                                                    },
                                                    _ => {
                                                        println!("Server: Received unknown message type");
                                                    }
                                                }

												println!("Server: Echoing message back");
												match write.send(msg).await {
													Ok(_) => println!(
														"Server: Message sent successfully"
													),
													Err(e) => {
														println!(
															"Server: Error sending message: {}",
															e
														);
														break;
													}
												}

												match write.flush().await {
													Ok(_) => println!(
														"Server: Write flushed successfully"
													),
													Err(e) => {
														println!("Server: Error flushing: {}", e);
														break;
													}
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
