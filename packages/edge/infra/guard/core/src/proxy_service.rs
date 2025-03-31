#[allow(unused_imports)]
use bytes::Bytes;
use global_error::*;
use http_body_util::{Empty, Full};
use hyper::body::Incoming as BodyIncoming;
use hyper::{Request, Response, StatusCode};
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use std::{
	collections::HashMap,
	net::{IpAddr, SocketAddr},
	sync::Arc,
	time::{Duration, Instant},
};
use tokio::{sync::Mutex, time::timeout};
use uuid::Uuid;
use rand;

use crate::metrics;

// Routing types
#[derive(Clone, Debug)]
pub struct RouteTarget {
	pub actor_id: Option<Uuid>,
	pub server_id: Option<Uuid>,
	pub host: String,
	pub port: u16,
	pub path: String,
}

#[derive(Clone, Debug)]
pub struct RoutingTimeout {
	pub routing_timeout: u64, // in seconds
}

#[derive(Clone, Debug)]
pub struct RoutingResult {
	pub targets: Vec<RouteTarget>,
	pub timeout: RoutingTimeout,
}

#[derive(Clone, Debug)]
pub enum RoutingResponse {
    Ok(RoutingResult),
    NotFound,
}

/// Enum defining the type of port the request came in on
#[derive(Clone, Debug, PartialEq)]
pub enum PortType {
    Http,
    Https,
}

pub type RoutingFn = Arc<
	dyn for<'a> Fn(&'a str, &'a str, PortType) -> futures::future::BoxFuture<'a, GlobalResult<RoutingResponse>>
		+ Send
		+ Sync,
>;

#[derive(Clone, Debug)]
pub struct MiddlewareConfig {
	pub rate_limit: RateLimitConfig,
	pub max_in_flight: MaxInFlightConfig,
	pub retry: RetryConfig,
	pub timeout: TimeoutConfig,
}

#[derive(Clone, Debug)]
pub struct RateLimitConfig {
	pub requests: u64,
	pub period: u64,   // in seconds
}

#[derive(Clone, Debug)]
pub struct MaxInFlightConfig {
	pub amount: usize,
}

#[derive(Clone, Debug)]
pub struct RetryConfig {
	pub max_attempts: u32,
	pub initial_interval: u64, // in milliseconds
}

#[derive(Clone, Debug)]
pub struct TimeoutConfig {
	pub request_timeout: u64, // in seconds
}

#[derive(Clone, Debug)]
pub enum MiddlewareResponse {
    Ok(MiddlewareConfig),
    NotFound,
}

pub type MiddlewareFn = Arc<
	dyn for<'a> Fn(&'a Uuid) -> futures::future::BoxFuture<'a, GlobalResult<MiddlewareResponse>>
		+ Send
		+ Sync,
>;

// Cache for routing results
struct RouteCache {
	cache: HashMap<(String, String), RoutingResult>,
}

impl RouteCache {
	fn new() -> Self {
		Self {
			cache: HashMap::new(),
		}
	}

	fn get(&self, hostname: &str, path: &str) -> Option<&RoutingResult> {
		self.cache.get(&(hostname.to_owned(), path.to_owned()))
	}

	fn insert(&mut self, hostname: String, path: String, result: RoutingResult) {
		self.cache.insert((hostname, path), result);
	}
}

// Rate limiter
struct RateLimiter {
	requests_remaining: u64,
	reset_time: Instant,
	requests_limit: u64,
	period: Duration,
}

impl RateLimiter {
	fn new(requests: u64, period_seconds: u64) -> Self {
		Self {
			requests_remaining: requests,
			reset_time: Instant::now() + Duration::from_secs(period_seconds),
			requests_limit: requests,
			period: Duration::from_secs(period_seconds),
		}
	}

	fn try_acquire(&mut self) -> bool {
		let now = Instant::now();

		// Check if we need to reset the counter
		if now >= self.reset_time {
			self.requests_remaining = self.requests_limit;
			self.reset_time = now + self.period;
		}

		// Try to consume a request
		if self.requests_remaining > 0 {
			self.requests_remaining -= 1;
			true
		} else {
			false
		}
	}
}

// In-flight requests counter
struct InFlightCounter {
	count: usize,
	max: usize,
}

impl InFlightCounter {
	fn new(max: usize) -> Self {
		Self { count: 0, max }
	}

	fn try_acquire(&mut self) -> bool {
		if self.count < self.max {
			self.count += 1;
			true
		} else {
			false
		}
	}

	fn release(&mut self) {
		self.count = self.count.saturating_sub(1);
	}
}

// State shared across all request handlers
pub struct ProxyState {
	_config: rivet_config::Config, // Unused but kept for potential future use
	routing_fn: RoutingFn,
	middleware_fn: MiddlewareFn,
	route_cache: Mutex<RouteCache>,
	rate_limiters: Mutex<HashMap<Uuid, RateLimiter>>,
	in_flight_counters: Mutex<HashMap<Uuid, InFlightCounter>>,
	port_type: PortType,
}

impl ProxyState {
	pub fn new(config: rivet_config::Config, routing_fn: RoutingFn, middleware_fn: MiddlewareFn, port_type: PortType) -> Self {
		Self {
			_config: config,
			routing_fn,
			middleware_fn,
			route_cache: Mutex::new(RouteCache::new()),
			rate_limiters: Mutex::new(HashMap::new()),
			in_flight_counters: Mutex::new(HashMap::new()),
			port_type,
		}
	}

	async fn resolve_route(&self, hostname: &str, path: &str, port_type: PortType) -> GlobalResult<RouteTarget> {
		// Extract just the hostname, stripping the port if present
		let hostname_only = hostname.split(':').next().unwrap_or(hostname);

		// Check cache first
		let mut cache = self.route_cache.lock().await;
		if let Some(result) = cache.get(hostname_only, path) {
			// Choose a random target from the cached targets
			if let Some(target) = choose_random_target(&result.targets) {
				return Ok(target.clone());
			}
		}

		// Not in cache, call routing function with a default timeout
		let default_timeout = Duration::from_secs(5); // Default 5 seconds
		let routing_result = timeout(default_timeout, (self.routing_fn)(hostname_only, path, port_type)).await;

		// Handle timeout and routing errors
		let result = match routing_result {
			Ok(result) => match result? {
				RoutingResponse::Ok(result) => result,
				RoutingResponse::NotFound => bail!("Route not found for hostname: {}, path: {}", hostname_only, path),
			},
			Err(_) => {
				bail!(
					"Routing function timed out after {}s",
					default_timeout.as_secs()
				);
			}
		};

		// Make sure we have at least one target
		if result.targets.is_empty() {
			bail!("No route targets available for hostname: {}, path: {}", hostname_only, path);
		}

		// Insert into cache
		cache.insert(hostname_only.to_owned(), path.to_owned(), result.clone());

		// Choose a random target from the available targets
		match choose_random_target(&result.targets) {
			Some(target) => Ok(target.clone()),
			None => bail!("Failed to choose a route target for hostname: {}, path: {}", hostname_only, path),
		}
	}

	async fn get_middleware_config(&self, actor_id: &Uuid) -> GlobalResult<MiddlewareConfig> {
		// Call the middleware function with a timeout
		let default_timeout = Duration::from_secs(5); // Default 5 seconds
		
		let middleware_result = timeout(default_timeout, (self.middleware_fn)(actor_id)).await;
		
		match middleware_result {
			Ok(result) => match result? {
				MiddlewareResponse::Ok(config) => Ok(config),
				MiddlewareResponse::NotFound => {
					// Default values if middleware not found for this actor
					Ok(MiddlewareConfig {
						rate_limit: RateLimitConfig {
							requests: 100,  // 100 requests
							period: 60,     // per 60 seconds
						},
						max_in_flight: MaxInFlightConfig {
							amount: 20,     // 20 concurrent requests
						},
						retry: RetryConfig {
							max_attempts: 3,      // 3 retry attempts
							initial_interval: 100, // 100ms initial interval
						},
						timeout: TimeoutConfig {
							request_timeout: 30,   // 30 seconds for requests
						},
					})
				}
			},
			Err(_) => {
				// Default values if middleware times out
				Ok(MiddlewareConfig {
					rate_limit: RateLimitConfig {
						requests: 100,  // 100 requests
						period: 60,     // per 60 seconds
					},
					max_in_flight: MaxInFlightConfig {
						amount: 20,     // 20 concurrent requests
					},
					retry: RetryConfig {
						max_attempts: 3,      // 3 retry attempts
						initial_interval: 100, // 100ms initial interval
					},
					timeout: TimeoutConfig {
						request_timeout: 30,   // 30 seconds for requests
					},
				})
			}
		}
	}

	async fn check_rate_limit(&self, actor_id: &Option<Uuid>) -> GlobalResult<bool> {
		match actor_id {
			Some(id) => {
				let middleware_config = self.get_middleware_config(id).await?;

				let mut limiters = self.rate_limiters.lock().await;
				let limiter = limiters.entry(*id).or_insert_with(|| {
					RateLimiter::new(
						middleware_config.rate_limit.requests,
						middleware_config.rate_limit.period,
					)
				});

				Ok(limiter.try_acquire())
			},
			None => {
				// No actor ID means no rate limiting
				Ok(true)
			}
		}
	}

	async fn acquire_in_flight(&self, actor_id: &Option<Uuid>) -> GlobalResult<bool> {
		match actor_id {
			Some(id) => {
				let middleware_config = self.get_middleware_config(id).await?;

				let mut counters = self.in_flight_counters.lock().await;
				let counter = counters
					.entry(*id)
					.or_insert_with(|| InFlightCounter::new(middleware_config.max_in_flight.amount));

				Ok(counter.try_acquire())
			},
			None => {
				// No actor ID means no in-flight limiting
				Ok(true)
			}
		}
	}

	async fn release_in_flight(&self, actor_id: &Option<Uuid>) {
		if let Some(id) = actor_id {
			let mut counters = self.in_flight_counters.lock().await;
			if let Some(counter) = counters.get_mut(id) {
				counter.release();
			}
		}
	}
}

// Helper function to choose a random target from a list of targets
fn choose_random_target(targets: &[RouteTarget]) -> Option<&RouteTarget> {
    if targets.is_empty() {
        return None;
    }
    
    // Use a simple random index selection
    let random_index = rand::random::<usize>() % targets.len();
    targets.get(random_index)
}

// Proxy service
pub struct ProxyService {
	state: Arc<ProxyState>,
	remote_addr: SocketAddr,
	// Note: Using the hyper legacy client is the only option currently.
	// This is what reqwest uses under the hood. Eventually we'll migrate to h3 once it's ready.
	client: Client<hyper_util::client::legacy::connect::HttpConnector, Full<Bytes>>,
}

impl ProxyService {
	pub fn new(state: Arc<ProxyState>, remote_addr: SocketAddr) -> Self {
		// Create a client with the hyper-util legacy client
		let client = Client::builder(TokioExecutor::new())
			.pool_idle_timeout(Duration::from_secs(30))
			.build_http();

		Self {
			state,
			remote_addr,
			client,
		}
	}

	// Calculate backoff duration for a given retry attempt
	pub fn calculate_backoff(attempt: u32, initial_interval: u64) -> Duration {
		Duration::from_millis(initial_interval * 2u64.pow(attempt - 1))
	}

	async fn handle_request(
		&self,
		req: Request<BodyIncoming>,
	) -> GlobalResult<Response<Full<Bytes>>> {
		let host = req
			.headers()
			.get(hyper::header::HOST)
			.and_then(|h| h.to_str().ok())
			.unwrap_or("unknown");

		let path = req.uri().path();
		let method = req.method().clone();

		// Resolve target
		let target = match self.state.resolve_route(host, path, self.state.port_type.clone()).await {
			Ok(target) => target,
			Err(e) => {
				tracing::error!("Routing error: {}", e);
				return Ok(Response::builder()
					.status(StatusCode::BAD_GATEWAY)
					.body(Full::<Bytes>::new(Bytes::new()))?);
			}
		};

		let actor_id = target.actor_id;
		let server_id = target.server_id;

		// Convert UUIDs to strings for metrics, handling Optional fields
		let actor_id_str = actor_id.map_or_else(|| "none".to_string(), |id| id.to_string());
		let server_id_str = server_id.map_or_else(|| "none".to_string(), |id| id.to_string());

		// Apply rate limiting
		if !self.state.check_rate_limit(&actor_id).await? {
			metrics::ACTOR_REQUEST_ERRORS
				.with_label_values(&[&actor_id_str, &server_id_str, "429"])
				.inc();

			return Ok(Response::builder()
				.status(StatusCode::TOO_MANY_REQUESTS)
				.body(Full::<Bytes>::new(Bytes::new()))?);
		}

		// Check in-flight limit
		if !self.state.acquire_in_flight(&actor_id).await? {
			metrics::ACTOR_REQUEST_ERRORS
				.with_label_values(&[&actor_id_str, &server_id_str, "429"])
				.inc();

			return Ok(Response::builder()
				.status(StatusCode::TOO_MANY_REQUESTS)
				.body(Full::<Bytes>::new(Bytes::new()))?);
		}

		// Let's save path and method before consuming req
		let path_str = path;
		let method_str = method.as_str();

		// Increment metrics
		metrics::ACTOR_REQUEST_PENDING
			.with_label_values(&[
				&actor_id_str,
				&server_id_str,
				method_str,
				path_str,
			])
			.inc();

		metrics::ACTOR_REQUEST_TOTAL
			.with_label_values(&[
				&actor_id_str,
				&server_id_str,
				method_str,
				path_str,
			])
			.inc();

		// Create timer for duration metric
		let start_time = Instant::now();

		// Prepare to release in-flight counter when done
		let state_clone = self.state.clone();
		let actor_id_clone = actor_id;
		crate::defer! {
			tokio::spawn(async move {
				state_clone.release_in_flight(&actor_id_clone).await;
			});
		}

		//// Check for WebSocket upgrade
		//if req.headers().contains_key(hyper::header::UPGRADE) {
		//	return self.handle_websocket_upgrade(req, target).await;
		//}

		// Regular HTTP request
		self.handle_http_request(req, target, start_time).await
	}

	async fn handle_http_request(
		&self,
		req: Request<BodyIncoming>,
		host: RouteTarget,
		start_time: Instant,
	) -> GlobalResult<Response<Full<Bytes>>> {
		// Get middleware config for this actor if it exists
		let middleware_config = match &host.actor_id {
			Some(actor_id) => self.state.get_middleware_config(actor_id).await?,
			None => {
				// Default middleware config for targets without actor_id
				MiddlewareConfig {
					rate_limit: RateLimitConfig {
						requests: 100,  // 100 requests
						period: 60,     // per 60 seconds
					},
					max_in_flight: MaxInFlightConfig {
						amount: 20,     // 20 concurrent requests
					},
					retry: RetryConfig {
						max_attempts: 3,      // 3 retry attempts
						initial_interval: 100, // 100ms initial interval
					},
					timeout: TimeoutConfig {
						request_timeout: 30,   // 30 seconds for requests
					},
				}
			}
		};

		// Read the request body before proceeding with retries
		let (req_parts, body) = req.into_parts();
		let req_body = match http_body_util::BodyExt::collect(body).await {
			Ok(collected) => collected.to_bytes(),
			Err(e) => {
				tracing::warn!("Failed to read request body: {}", e);
				Bytes::new()
			}
		};
		// Build the proxied request
		let uri = format!("http://{}:{}{}", host.host, host.port, host.path);
		let mut builder = hyper::Request::builder()
			.method(req_parts.method.clone())
			.uri(&uri);

		// Copy headers
		let headers = builder.headers_mut().unwrap();
		for (key, value) in req_parts.headers.iter() {
			if key != hyper::header::HOST {
				headers.insert(key.clone(), value.clone());
			}
		}

		// Add X-Forwarded-For header
		if let Some(existing) = req_parts.headers.get(hyper::header::FORWARDED) {
			if let Ok(forwarded) = existing.to_str() {
				if !forwarded.contains(&self.remote_addr.ip().to_string()) {
					headers.insert(
						hyper::header::FORWARDED,
						hyper::header::HeaderValue::from_str(&format!(
							"{}, for={}",
							forwarded,
							self.remote_addr.ip()
						))?,
					);
				}
			}
		} else {
			headers.insert(
				hyper::header::FORWARDED,
				hyper::header::HeaderValue::from_str(&format!("for={}", self.remote_addr.ip()))?,
			);
		}

		// We'll build the request in the retry loop

		// Set up retry with backoff from middleware config
		let max_attempts = middleware_config.retry.max_attempts;
		let initial_interval = middleware_config.retry.initial_interval;
		let timeout_duration = Duration::from_secs(middleware_config.timeout.request_timeout);

		// Execute request with retry
		let actor_id = host.actor_id;
		let server_id = host.server_id;
		// Get string representations for metrics
		let actor_id_str = actor_id.map_or_else(|| "none".to_string(), |id| id.to_string());
		let server_id_str = server_id.map_or_else(|| "none".to_string(), |id| id.to_string());
		let target_ip = host.host;
		let target_port = host.port;

		// Use a value-returning loop to handle both errors and successful responses
		let (status_code, last_error) = 'retry: {
			let mut attempts = 0;

			while attempts < max_attempts {
				attempts += 1;

				// Create a new request for each attempt since we can't reuse requests
				let mut new_req = hyper::Request::builder()
					.method(req_parts.method.clone())
					.uri(&uri);

				// Copy headers
				let headers = new_req.headers_mut().unwrap();
				for (key, value) in req_parts.headers.iter() {
					if key != hyper::header::HOST {
						headers.insert(key.clone(), value.clone());
					}
				}

				let _proxied_req = new_req.body(Empty::<Bytes>::new())?;

				// We'll use the hyper-util client to make the request
				let uri_str = format!("http://{}:{}{}", target_ip, target_port, host.path);
				let uri = match uri_str.parse::<hyper::Uri>() {
					Ok(uri) => uri,
					Err(e) => {
						tracing::error!("Failed to parse URI: {}", e);

						let error = Some(global_error::ext::AssertionError::Panic {
							message: format!("URI parse error: {}", e),
							location: global_error::location!(),
						});

						metrics::ACTOR_REQUEST_ERRORS
							.with_label_values(&[
								&actor_id_str,
								&server_id_str,
								"uri_parse_error",
							])
							.inc();

						break 'retry (StatusCode::BAD_GATEWAY, error);
					}
				};

				// Update the request URI
				let mut builder = Request::builder().method(req_parts.method.clone()).uri(uri);

				// Copy headers
				let headers = builder.headers_mut().unwrap();
				for (key, value) in req_parts.headers.iter() {
					if key != hyper::header::HOST {
						headers.insert(key.clone(), value.clone());
					}
				}

				// Add X-Forwarded-For header
				if let Some(existing) = req_parts.headers.get(hyper::header::FORWARDED) {
					if let Ok(forwarded) = existing.to_str() {
						if !forwarded.contains(&self.remote_addr.ip().to_string()) {
							headers.insert(
								hyper::header::FORWARDED,
								hyper::header::HeaderValue::from_str(&format!(
									"{}, for={}",
									forwarded,
									self.remote_addr.ip()
								))?,
							);
						}
					}
				} else {
					headers.insert(
						hyper::header::FORWARDED,
						hyper::header::HeaderValue::from_str(&format!(
							"for={}",
							self.remote_addr.ip()
						))?,
					);
				}

				// Add request body if it's a method that typically has a body
				let proxied_req = match builder.body(Full::<Bytes>::new(req_body.clone())) {
					Ok(req) => req,
					Err(e) => {
						tracing::warn!("Failed to build request: {}", e);
						let error = Some(global_error::ext::AssertionError::Panic {
							message: format!("Request build error: {}", e),
							location: global_error::location!(),
						});
						break 'retry (StatusCode::INTERNAL_SERVER_ERROR, error);
					}
				};

				// We can only send the request once, so we need to handle the timeout case differently
				match timeout(timeout_duration, self.client.request(proxied_req)).await {
					Ok(Ok(resp)) => {
						// Record metrics
						let duration = start_time.elapsed();
						metrics::ACTOR_REQUEST_DURATION
							.with_label_values(&[
								&actor_id_str,
								&server_id_str,
								&resp.status().as_u16().to_string(),
							])
							.observe(duration.as_secs_f64());

						metrics::ACTOR_REQUEST_PENDING
							.with_label_values(&[
								&actor_id_str,
								&server_id_str,
								req_parts.method.as_str(),
								req_parts.uri.path(),
							])
							.dec();

						// Convert the hyper::body::Incoming to http_body_util::Full<Bytes>
						let (parts, body) = resp.into_parts();

						// Read the response body
						let body_bytes = match http_body_util::BodyExt::collect(body).await {
							Ok(collected) => collected.to_bytes(),
							Err(_) => Bytes::new(),
						};

						let full_body = Full::new(body_bytes);
						return Ok(Response::from_parts(parts, full_body));
					}
					Ok(Err(e)) => {
						// Request error, might retry
						tracing::warn!("Request attempt {} failed: {}", attempts, e);
						let error = Some(global_error::ext::AssertionError::Panic {
							message: format!("Request error: {}", e),
							location: global_error::location!(),
						});

						if attempts >= max_attempts {
							break 'retry (StatusCode::BAD_GATEWAY, error);
						} else {
							// Use backoff and continue
							let backoff = Self::calculate_backoff(attempts, initial_interval);
							tokio::time::sleep(backoff).await;
							continue;
						}
					}
					Err(_) => {
						// Timeout error
						tracing::warn!(
							"Request timed out after {} seconds",
							timeout_duration.as_secs()
						);
						let error = Some(global_error::ext::AssertionError::Panic {
							message: format!("Request timed out"),
							location: global_error::location!(),
						});

						metrics::ACTOR_REQUEST_ERRORS
							.with_label_values(&[
								&actor_id_str,
								&server_id_str,
								"timeout",
							])
							.inc();

						break 'retry (StatusCode::GATEWAY_TIMEOUT, error);
					}
				}
			}

			// If we get here, all attempts failed with non-timeout errors
			let error = Some(global_error::ext::AssertionError::Panic {
				message: format!("All retry attempts failed"),
				location: global_error::location!(),
			});
			(StatusCode::BAD_GATEWAY, error)
		};

		// Log the error
		tracing::error!("Request failed: {:?}", last_error);

		// Only increment the error metric if not already done (for timeout)
		if status_code == StatusCode::BAD_GATEWAY {
			metrics::ACTOR_REQUEST_ERRORS
				.with_label_values(&[&actor_id_str, &server_id_str, "502"])
				.inc();
		}

		metrics::ACTOR_REQUEST_PENDING
			.with_label_values(&[
				&actor_id_str,
				&server_id_str,
				req_parts.method.as_str(),
				req_parts.uri.path(),
			])
			.dec();

		Ok(Response::builder()
			.status(status_code)
			.body(Full::<Bytes>::new(Bytes::new()))?)
	}

	//async fn handle_websocket_upgrade(
	//	&self,
	//	req: Request<BodyIncoming>,
	//	target: RouteTarget,
	//) -> GlobalResult<Response<Full<Bytes>>> {
	//	// This is a simplified implementation
	//	// Real implementation would need to handle the WebSocket upgrade properly
	//
	//	// Build the proxied request with the upgrade headers
	//	let uri = format!("http://{}:{}{}", target.ip, target.port, target.path);
	//	let mut builder = hyper::Request::builder()
	//		.method(req.method().clone())
	//		.uri(&uri);
	//
	//	// Copy headers including upgrade headers
	//	let headers = builder.headers_mut().unwrap();
	//	for (key, value) in req.headers() {
	//		headers.insert(key.clone(), value.clone());
	//	}
	//
	//	// Add X-Forwarded-For header
	//	headers.insert(
	//		hyper::header::FORWARDED,
	//		hyper::header::HeaderValue::from_str(&format!("for={}", self.remote_addr.ip()))?,
	//	);
	//
	//	// Create the upgraded request with an empty body
	//	let upgraded_req = builder.body(Empty::<Bytes>::new())?;
	//
	//	// Parse the URI for hyper-util client
	//	let uri = match uri.parse::<hyper::Uri>() {
	//		Ok(uri) => uri,
	//		Err(e) => {
	//			error!("Failed to parse URI for WebSocket upgrade: {}", e);
	//			return Ok(Response::builder()
	//				.status(StatusCode::BAD_GATEWAY)
	//				.body(Full::<Bytes>::new(Bytes::new()))?);
	//		}
	//	};
	//
	//	// Update the request with the parsed URI
	//	let mut new_req = Request::builder().method(req.method().clone()).uri(uri);
	//
	//	// Copy all headers from the original request
	//	for (key, value) in upgraded_req.headers() {
	//		new_req = new_req.header(key, value);
	//	}
	//
	//	let upgraded_req = new_req.body(Empty::<Bytes>::new())?;
	//
	//	// Make the request using the hyper-util client
	//	match self.client.request(upgraded_req).await {
	//		Ok(resp) => {
	//			// If the response indicates a successful upgrade, pass it through
	//			let (parts, _) = resp.into_parts();
	//			Ok(Response::from_parts(parts, Full::new(Bytes::new())))
	//		}
	//		Err(e) => {
	//			error!("WebSocket upgrade failed: {}", e);
	//			Ok(Response::builder()
	//				.status(StatusCode::BAD_GATEWAY)
	//				.body(Full::<Bytes>::new(Bytes::new()))?)
	//		}
	//	}
	//}
}

impl ProxyService {
	// Process an individual request
	pub async fn process(&self, req: Request<BodyIncoming>) -> GlobalResult<Response<Full<Bytes>>> {
		self.handle_request(req).await
	}
}

impl Clone for ProxyService {
	fn clone(&self) -> Self {
		Self {
			state: self.state.clone(),
			remote_addr: self.remote_addr,
			client: self.client.clone(),
		}
	}
}

// Factory for creating proxy services
pub struct ProxyServiceFactory {
	state: Arc<ProxyState>,
}

impl ProxyServiceFactory {
	pub fn new(config: rivet_config::Config, routing_fn: RoutingFn, middleware_fn: MiddlewareFn, port_type: PortType) -> Self {
		let state = Arc::new(ProxyState::new(config, routing_fn, middleware_fn, port_type));
		Self { state }
	}

	// Create a new proxy service for the given remote address
	pub fn create_service(&self, remote_addr: SocketAddr) -> ProxyService {
		ProxyService::new(self.state.clone(), remote_addr)
	}
}

// Helper macro for defer-like functionality
#[macro_export]
macro_rules! defer {
    ($($body:tt)*) => {
        let _guard = {
            struct Guard<F: FnOnce()>(Option<F>);
            impl<F: FnOnce()> Drop for Guard<F> {
                fn drop(&mut self) {
                    if let Some(f) = self.0.take() {
                        f()
                    }
                }
            }
            Guard(Some(|| { $($body)* }))
        };
    };
}
