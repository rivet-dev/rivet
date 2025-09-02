use std::{
	collections::{HashMap, hash_map::DefaultHasher},
	future::Future,
	hash::{Hash, Hasher},
	net::SocketAddr,
	sync::{Arc, Mutex, Once},
	time::Duration,
};

// Copy the original file with required fixes
use anyhow::*;
use http_body_util::Full;
use hyper::{Request, Response, StatusCode, body::Bytes, service::service_fn};
use hyper_util::rt::TokioIo;
use rivet_guard_core::proxy_service::{
	CacheKeyFn, MaxInFlightConfig, MiddlewareConfig, MiddlewareFn, MiddlewareResponse,
	RateLimitConfig, RetryConfig, RouteConfig, RouteTarget, RoutingFn, RoutingOutput,
	RoutingTimeout, TimeoutConfig,
};
use rivet_util::Id;
use tokio::{net::TcpListener, sync::oneshot, task::JoinHandle};
use tracing::Level;
use uuid::Uuid;

// Initialize tracing once
static INIT_TRACING: Once = Once::new();

// Setup tracing for tests
pub fn init_tracing() {
	INIT_TRACING.call_once(|| {
		let subscriber = tracing_subscriber::fmt()
			.with_max_level(Level::DEBUG)
			.with_test_writer()
			.compact()
			.finish();

		tracing::subscriber::set_global_default(subscriber)
			.expect("Failed to set tracing subscriber");
	});
}

// Test server for routing requests to
pub struct TestServer {
	pub addr: SocketAddr,
	pub request_log: Arc<Mutex<Vec<TestRequest>>>,
	shutdown_tx: Option<oneshot::Sender<()>>,
	handle: Option<JoinHandle<()>>,
}

#[derive(Debug, Clone)]
pub struct TestRequest {
	pub method: String,
	pub uri: String,
	pub headers: HashMap<String, String>,
	pub body: Vec<u8>,
}

impl TestServer {
	pub async fn new() -> Self {
		Self::with_handler(|_req, _request_log| {
			// Default handler simply logs the request and returns 200 OK
			let response = Response::builder()
				.status(StatusCode::OK)
				.body(Full::new(Bytes::from("OK")))
				.unwrap();

			Box::pin(
				async move { std::result::Result::<_, std::convert::Infallible>::Ok(response) },
			)
		})
		.await
	}

	pub async fn with_handler<F, Fut>(handler: F) -> Self
	where
		F: Fn(Request<hyper::body::Incoming>, Arc<Mutex<Vec<TestRequest>>>) -> Fut
			+ Send
			+ 'static
			+ Clone,
		Fut: Future<Output = std::result::Result<Response<Full<Bytes>>, std::convert::Infallible>>
			+ Send,
	{
		// Bind to a random available port
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let addr = listener.local_addr().unwrap();

		let request_log = Arc::new(Mutex::new(Vec::new()));
		let request_log_clone = request_log.clone();

		let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

		// Start the server
		let handle = tokio::spawn(async move {
			let mut shutdown_rx = shutdown_rx;

			loop {
				// Use select to check for shutdown signal
				let accept_fut = listener.accept();
				let accept_or_shutdown = tokio::select! {
					result = accept_fut => Some(result),
					_ = &mut shutdown_rx => None,
				};

				// Break the loop if shutdown was requested
				let (stream, _) = match accept_or_shutdown {
					Some(Result::Ok(value)) => value,
					Some(Err(_)) => break,
					None => break,
				};

				let io = TokioIo::new(stream);
				let request_log = request_log_clone.clone();
				let handler = handler.clone();

				tokio::spawn(async move {
					// Create a service function for this connection
					let service = service_fn(move |req: Request<hyper::body::Incoming>| {
						// Clone these for the async move block
						let request_log = request_log.clone();
						let handler = handler.clone();

						async move {
							// Capture request details
							let method = req.method().to_string();
							let uri = req.uri().to_string();

							// Extract headers
							let mut headers = HashMap::new();
							for (name, value) in req.headers() {
								if let Result::Ok(v) = value.to_str() {
									headers.insert(name.to_string(), v.to_string());
								}
							}

							// Store request for later inspection
							let test_req = TestRequest {
								method,
								uri,
								headers,
								body: Vec::new(), // Body will be consumed by handler
							};

							request_log.lock().unwrap().push(test_req);

							// Call the custom handler
							handler(req, request_log.clone()).await
						}
					});

					// IMPORTANT: Use with_upgrades() for WebSocket handling
					if let Err(err) = hyper::server::conn::http1::Builder::new()
						.serve_connection(io, service)
						.with_upgrades() // Added this critical line
						.await
					{
						eprintln!("Error serving connection: {:?}", err);
					}
				});
			}
		});

		Self {
			addr,
			request_log,
			shutdown_tx: Some(shutdown_tx),
			handle: Some(handle),
		}
	}

	// Add delay handler that waits for a specified time before responding
	pub async fn with_delay(delay_ms: u64) -> Self {
		Self::with_handler(move |_req, _request_log| {
			let delay = delay_ms;
			Box::pin(async move {
				// Simulate processing delay
				tokio::time::sleep(Duration::from_millis(delay)).await;

				let response = Response::builder()
					.status(StatusCode::OK)
					.body(Full::new(Bytes::from("Delayed OK")))
					.unwrap();

				Result::<_, std::convert::Infallible>::Ok(response)
			})
		})
		.await
	}

	// Create a handler that returns a specific status code
	pub async fn with_status(status_code: StatusCode) -> Self {
		Self::with_handler(move |_req, _request_log| {
			let status = status_code;
			Box::pin(async move {
				let response = Response::builder()
					.status(status)
					.body(Full::new(Bytes::from(format!(
						"Status: {}",
						status.as_u16()
					))))
					.unwrap();

				Result::<_, std::convert::Infallible>::Ok(response)
			})
		})
		.await
	}

	// Create a TestServer with a specific server address
	pub async fn with_addr(addr: SocketAddr) -> Self {
		// Create a server bound to the specific address
		let listener = TcpListener::bind(addr).await.unwrap();
		let request_log = Arc::new(Mutex::new(Vec::new()));
		let request_log_clone = request_log.clone();

		let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

		// Start the server with the same handler as new()
		let handle = tokio::spawn(async move {
			let mut shutdown_rx = shutdown_rx;

			loop {
				// Use select to check for shutdown signal
				let accept_fut = listener.accept();
				let accept_or_shutdown = tokio::select! {
					result = accept_fut => Some(result),
					_ = &mut shutdown_rx => None,
				};

				// Break the loop if shutdown was requested
				let (stream, _) = match accept_or_shutdown {
					Some(Result::Ok(value)) => value,
					Some(Err(_)) => break,
					None => break,
				};

				let io = TokioIo::new(stream);
				let request_log = request_log_clone.clone();

				tokio::spawn(async move {
					// Create a service function for this connection
					let service = service_fn(move |req: Request<hyper::body::Incoming>| {
						// Clone these for the async move block
						let request_log = request_log.clone();

						async move {
							// Capture request details
							let method = req.method().to_string();
							let uri = req.uri().to_string();

							// Extract headers
							let mut headers = HashMap::new();
							for (name, value) in req.headers() {
								if let Result::Ok(v) = value.to_str() {
									headers.insert(name.to_string(), v.to_string());
								}
							}

							// Store request for later inspection
							// Read the request body
							let body = req.into_body();
							let body_bytes = match http_body_util::BodyExt::collect(body).await {
								Result::Ok(collected) => collected.to_bytes(),
								Err(_) => Bytes::from("Failed to read body"),
							};

							let body_vec = body_bytes.to_vec();
							let body_str = String::from_utf8_lossy(&body_vec).to_string();

							let test_req = TestRequest {
								method,
								uri,
								headers,
								body: body_vec,
							};

							request_log.lock().unwrap().push(test_req);

							// Return a 200 OK response echoing back the body
							let response = Response::builder()
								.status(StatusCode::OK)
								.body(Full::new(Bytes::from(format!("Received: {}", body_str))))
								.unwrap();

							Result::<_, std::convert::Infallible>::Ok(response)
						}
					});

					// IMPORTANT: Use with_upgrades() for WebSocket handling
					if let Err(err) = hyper::server::conn::http1::Builder::new()
						.serve_connection(io, service)
						.with_upgrades() // Added this critical line
						.await
					{
						eprintln!("Error serving connection: {:?}", err);
					}
				});
			}
		});

		Self {
			addr,
			request_log,
			shutdown_tx: Some(shutdown_tx),
			handle: Some(handle),
		}
	}

	// Create a TestServer with a specific server address and custom handler
	pub async fn with_handler_and_addr<F, Fut>(addr: SocketAddr, handler: F) -> Self
	where
		F: Fn(Request<hyper::body::Incoming>, Arc<Mutex<Vec<TestRequest>>>) -> Fut
			+ Send
			+ 'static
			+ Clone,
		Fut: Future<Output = Result<Response<Full<Bytes>>, std::convert::Infallible>> + Send,
	{
		// Create a server bound to the specific address
		let listener = TcpListener::bind(addr).await.unwrap();
		let request_log = Arc::new(Mutex::new(Vec::new()));
		let request_log_clone = request_log.clone();

		let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

		// Start the server with the custom handler
		let handle = tokio::spawn(async move {
			let mut shutdown_rx = shutdown_rx;

			loop {
				// Use select to check for shutdown signal
				let accept_fut = listener.accept();
				let accept_or_shutdown = tokio::select! {
					result = accept_fut => Some(result),
					_ = &mut shutdown_rx => None,
				};

				// Break the loop if shutdown was requested
				let (stream, _) = match accept_or_shutdown {
					Some(Result::Ok(value)) => value,
					Some(Err(_)) => break,
					None => break,
				};

				let io = TokioIo::new(stream);
				let request_log = request_log_clone.clone();
				let handler = handler.clone();

				tokio::spawn(async move {
					// Create a service function for this connection
					let service = service_fn(move |req: Request<hyper::body::Incoming>| {
						// Clone these for the async move block
						let request_log = request_log.clone();
						let handler = handler.clone();

						async move {
							// Capture request details
							let method = req.method().to_string();
							let uri = req.uri().to_string();

							// Extract headers
							let mut headers = HashMap::new();
							for (name, value) in req.headers() {
								if let Result::Ok(v) = value.to_str() {
									headers.insert(name.to_string(), v.to_string());
								}
							}

							// Store request for later inspection
							let test_req = TestRequest {
								method,
								uri,
								headers,
								body: Vec::new(), // Body will be consumed by handler
							};

							request_log.lock().unwrap().push(test_req);

							// Call the custom handler
							handler(req, request_log.clone()).await
						}
					});

					// IMPORTANT: Use with_upgrades() for WebSocket handling
					if let Err(err) = hyper::server::conn::http1::Builder::new()
						.serve_connection(io, service)
						.with_upgrades() // Added this critical line
						.await
					{
						eprintln!("Error serving connection: {:?}", err);
					}
				});
			}
		});

		Self {
			addr,
			request_log,
			shutdown_tx: Some(shutdown_tx),
			handle: Some(handle),
		}
	}

	// Get the count of requests received
	pub fn request_count(&self) -> usize {
		self.request_log.lock().unwrap().len()
	}

	// Get the last request received
	pub fn last_request(&self) -> Option<TestRequest> {
		let log = self.request_log.lock().unwrap();
		log.last().cloned()
	}
}

impl Drop for TestServer {
	fn drop(&mut self) {
		// Send shutdown signal
		if let Some(tx) = self.shutdown_tx.take() {
			let _ = tx.send(());
		}

		// Abort the server task if it's still running
		if let Some(handle) = self.handle.take() {
			handle.abort();
		}
	}
}

// Mock routing function for testing
pub fn create_test_routing_fn(test_server: &TestServer) -> RoutingFn {
	let addr = test_server.addr;

	Arc::new(
		move |_hostname: &str,
		      path: &str,
		      _port_type: rivet_guard_core::proxy_service::PortType,
		      _headers: &hyper::HeaderMap| {
			Box::pin(async move {
				let actor_id = Some(Id::new_v1(0));

				// Create a single target for simplicity in tests
				let target = RouteTarget {
					actor_id,
					host: addr.ip().to_string(),
					port: addr.port(),
					path: path.to_string(),
				};

				Ok(RoutingOutput::Route(RouteConfig {
					targets: vec![target],
					timeout: RoutingTimeout {
						routing_timeout: 5, // 5 seconds for routing timeout
					},
				}))
			})
		},
	)
}

// Create a default configuration for testing
pub fn create_test_config(
	mutate: impl Fn(&mut rivet_config::config::guard::Guard),
) -> rivet_config::Config {
	let mut root = rivet_config::config::Root::default();
	let mut guard = rivet_config::config::guard::Guard {
		host: None,    // Use default host
		port: Some(0), // Use 0 to let the OS choose a port
		https: None,   // No HTTPS by default in tests
	};
	mutate(&mut guard);
	root.guard = Some(guard);

	rivet_config::Config::from_root(root)
}

pub fn create_test_cache_key_fn() -> CacheKeyFn {
	Arc::new(
		move |hostname: &str,
		      path: &str,
		      _port_type: rivet_guard_core::proxy_service::PortType,
		      _headers: &hyper::HeaderMap| {
			// Extract just the hostname, stripping the port if present
			let hostname_only = hostname.split(':').next().unwrap_or(hostname);

			let mut hasher = DefaultHasher::new();
			hostname_only.hash(&mut hasher);
			path.hash(&mut hasher);
			Ok(hasher.finish())
		},
	)
}

// Create a test middleware function with customization
pub fn create_test_middleware_fn(
	mutate: impl Fn(&mut MiddlewareConfig) + Send + Sync + 'static,
) -> MiddlewareFn {
	// Clone the mutate function to avoid lifetime issues
	let mutate = Arc::new(mutate);

	Arc::new(move |_actor_id: &Id, _headers: &hyper::HeaderMap| {
		let mutate_clone = mutate.clone();
		Box::pin(async move {
			// Create the default middleware config
			let mut middleware_config = MiddlewareConfig {
				rate_limit: RateLimitConfig {
					requests: 100, // 100 requests
					period: 60,    // per 60 seconds
				},
				max_in_flight: MaxInFlightConfig {
					amount: 10, // 10 concurrent requests
				},
				retry: RetryConfig {
					max_attempts: 3,      // 3 retry attempts
					initial_interval: 50, // 50ms initial interval
				},
				timeout: TimeoutConfig {
					request_timeout: 2, // 2 seconds for requests
				},
			};

			// Apply the mutation
			mutate_clone(&mut middleware_config);

			// Return the modified middleware config
			Ok(MiddlewareResponse::Ok(middleware_config))
		})
	})
}

// Helper to start rivet-guard with a custom config and middleware
pub async fn start_guard(
	config: rivet_config::Config,
	routing_fn: RoutingFn,
) -> (SocketAddr, oneshot::Sender<()>) {
	// Default cache key fn
	let cache_key_fn = create_test_cache_key_fn();

	// Use the default middleware function
	let middleware_fn = create_test_middleware_fn(|_config| {
		// Default middleware uses the values in create_test_middleware_fn
	});

	start_guard_with_middleware(config, routing_fn, cache_key_fn, middleware_fn).await
}

// Helper to start rivet-guard with a custom config and custom middleware function
pub async fn start_guard_with_middleware(
	config: rivet_config::Config,
	routing_fn: RoutingFn,
	cache_key_fn: CacheKeyFn,
	middleware_fn: MiddlewareFn,
) -> (SocketAddr, oneshot::Sender<()>) {
	let (addr_tx, addr_rx) = oneshot::channel();
	let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

	let config_clone = config.clone();
	let routing_fn_clone = routing_fn.clone();
	let cache_key_fn_clone = cache_key_fn.clone();
	let middleware_fn_clone = middleware_fn.clone();

	tokio::spawn(async move {
		let guard_config = config_clone.guard();

		// Create a listener to get the assigned port
		let port = guard_config.port.unwrap_or(0);
		let addr: SocketAddr = ([127, 0, 0, 1], port).into();
		let listener = TcpListener::bind(addr).await.unwrap();
		let server_addr = listener.local_addr().unwrap();

		// Send the server address back
		let _ = addr_tx.send(server_addr);

		// Create the guard state and factory
		let factory = Arc::new(rivet_guard_core::proxy_service::ProxyServiceFactory::new(
			config_clone,
			routing_fn_clone,
			cache_key_fn_clone,
			middleware_fn_clone,
			rivet_guard_core::proxy_service::PortType::Http, // Default port type for tests
			None,                                            // No ClickHouse inserter for tests
		));

		// Run the server until shutdown signal
		let server = async {
			loop {
				// Use select to check for shutdown signal
				let accept_fut = listener.accept();
				let accept_or_shutdown = tokio::select! {
					result = accept_fut => Some(result),
					_ = &mut shutdown_rx => None,
				};

				// Break the loop if shutdown was requested
				let (stream, remote_addr) = match accept_or_shutdown {
					Some(Result::Ok(value)) => value,
					Some(Err(_)) => break,
					None => break,
				};

				let io = TokioIo::new(stream);
				let factory_clone = factory.clone();

				// Create a proxy service for this connection
				let proxy_service = factory_clone.create_service(remote_addr);

				tokio::spawn(async move {
					let service = service_fn(move |req| {
						let service_clone = proxy_service.clone();
						async move { service_clone.process(req).await }
					});

					// IMPORTANT: Use with_upgrades() for WebSocket handling
					if let Err(err) = hyper::server::conn::http1::Builder::new()
						.serve_connection(io, service)
						.with_upgrades() // Added this critical line
						.await
					{
						eprintln!("Error serving connection: {:?}", err);
					}
				});
			}

			Result::<(), Error>::Ok(())
		};

		let _ = server.await;
	});

	// Wait for the server address
	let addr = addr_rx.await.unwrap();

	(addr, shutdown_tx)
}

// Helper to make HTTP requests to the guard server
pub async fn make_request(
	uri: &str,
	host: &str,
	method: hyper::Method,
) -> Result<Response<hyper::body::Incoming>, Box<dyn std::error::Error + Send + Sync>> {
	let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
		.build_http();

	let request = Request::builder()
		.method(method)
		.uri(uri)
		.header(hyper::header::HOST, host)
		.body(http_body_util::Empty::<bytes::Bytes>::new())?;

	Result::<Response<hyper::body::Incoming>, Box<dyn std::error::Error + Send + Sync>>::Ok(
		client.request(request).await?,
	)
}

// Helper to make HTTP requests with a body
pub async fn make_request_with_body(
	uri: &str,
	host: &str,
	method: hyper::Method,
	body: String,
) -> Result<Response<hyper::body::Incoming>, Box<dyn std::error::Error + Send + Sync>> {
	let client = hyper_util::client::legacy::Client::builder(hyper_util::rt::TokioExecutor::new())
		.build_http();

	let request = Request::builder()
		.method(method)
		.uri(uri)
		.header(hyper::header::HOST, host)
		.header(hyper::header::CONTENT_TYPE, "text/plain")
		.header(hyper::header::CONTENT_LENGTH, body.len().to_string())
		.body(http_body_util::Full::new(bytes::Bytes::from(body)))?;

	Result::<Response<hyper::body::Incoming>, Box<dyn std::error::Error + Send + Sync>>::Ok(
		client.request(request).await?,
	)
}
