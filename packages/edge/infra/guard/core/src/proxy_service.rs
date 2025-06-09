// This file is modified by Claude Code

use std::{
	borrow::Cow,
	collections::HashMap as StdHashMap,
	net::SocketAddr,
	sync::Arc,
	time::{Duration, Instant},
};

use tokio::sync::Mutex;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use global_error::*;
use http_body_util::Full;
use hyper::body::{Body, Incoming as BodyIncoming};
use hyper::header::HeaderName;
use hyper::{Request, Response, StatusCode};
use hyper_tungstenite;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use moka::future::Cache;
use rand;
use serde_json;
use tokio::time::timeout;
use tracing::Instrument;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::metrics;
use crate::request_context::RequestContext;

const X_FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");
const ROUTE_CACHE_TTL: Duration = Duration::from_secs(60 * 10); // 10 minutes
const PROXY_STATE_CACHE_TTL: Duration = Duration::from_secs(60 * 60); // 1 hour

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
pub struct RouteConfig {
	pub targets: Vec<RouteTarget>,
	pub timeout: RoutingTimeout,
}

#[derive(Clone, Debug)]
pub enum RoutingOutput {
	/// Return the data to route to.
	Route(RouteConfig),
	/// Return a custom response.
	Response(StructuredResponse),
}

#[derive(Clone, Debug)]
pub struct StructuredResponse {
	pub status: StatusCode,
	pub message: Cow<'static, str>,
	pub docs: Option<Cow<'static, str>>,
}

impl StructuredResponse {
	pub fn build_response(&self) -> GlobalResult<Response<Full<Bytes>>> {
		let mut body = StdHashMap::new();
		body.insert("message", self.message.clone().into_owned());

		if let Some(docs) = &self.docs {
			body.insert("docs", docs.clone().into_owned());
		}

		let body_json = serde_json::to_string(&body)?;
		let bytes = Bytes::from(body_json);

		let response = Response::builder()
			.status(self.status)
			.header(hyper::header::CONTENT_TYPE, "application/json")
			.body(Full::new(bytes))?;

		Ok(response)
	}
}

#[derive(Clone, Debug)]
enum ResolveRouteOutput {
	Target(RouteTarget),
	Response(StructuredResponse),
}

/// Enum defining the type of port the request came in on
#[derive(Clone, Debug, PartialEq)]
pub enum PortType {
	Http,
	Https,
}

pub type RoutingFn = Arc<
	dyn for<'a> Fn(
			&'a str,
			&'a str,
			PortType,
		) -> futures::future::BoxFuture<'a, GlobalResult<RoutingOutput>>
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
	pub period: u64, // in seconds
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
	cache: Cache<(String, String), RouteConfig>,
}

impl RouteCache {
	fn new() -> Self {
		Self {
			cache: Cache::builder()
				.max_capacity(10_000)
				.time_to_live(ROUTE_CACHE_TTL)
				.build(),
		}
	}

	#[tracing::instrument(skip_all)]
	async fn get(&self, hostname: &str, path: &str) -> Option<RouteConfig> {
		self.cache
			.get(&(hostname.to_owned(), path.to_owned()))
			.await
	}

	#[tracing::instrument(skip_all)]
	async fn insert(&self, hostname: String, path: String, result: RouteConfig) {
		self.cache.insert((hostname, path), result).await;

		metrics::ROUTE_CACHE_SIZE.set(self.cache.entry_count() as i64);
	}

	#[tracing::instrument(skip_all)]
	async fn purge(&self, hostname: &str, path: &str) {
		self.cache
			.invalidate(&(hostname.to_owned(), path.to_owned()))
			.await;

		metrics::ROUTE_CACHE_SIZE.set(self.cache.entry_count() as i64);
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
	route_cache: RouteCache,
	rate_limiters: Cache<(Uuid, std::net::IpAddr), Arc<Mutex<RateLimiter>>>,
	in_flight_counters: Cache<(Uuid, std::net::IpAddr), Arc<Mutex<InFlightCounter>>>,
	port_type: PortType,
	clickhouse_inserter: Option<clickhouse_inserter::ClickHouseInserterHandle>,
}

impl ProxyState {
	pub fn new(
		config: rivet_config::Config,
		routing_fn: RoutingFn,
		middleware_fn: MiddlewareFn,
		port_type: PortType,
		clickhouse_inserter: Option<clickhouse_inserter::ClickHouseInserterHandle>,
	) -> Self {
		Self {
			_config: config,
			routing_fn,
			middleware_fn,
			route_cache: RouteCache::new(),
			rate_limiters: Cache::builder()
				.max_capacity(10_000)
				.time_to_live(PROXY_STATE_CACHE_TTL)
				.build(),
			in_flight_counters: Cache::builder()
				.max_capacity(10_000)
				.time_to_live(PROXY_STATE_CACHE_TTL)
				.build(),
			port_type,
			clickhouse_inserter,
		}
	}

	#[tracing::instrument(skip(self))]
	async fn resolve_route(
		&self,
		hostname: &str,
		path: &str,
		port_type: PortType,
		ignore_cache: bool,
	) -> GlobalResult<ResolveRouteOutput> {
		// Extract just the hostname, stripping the port if present
		let hostname_only = hostname.split(':').next().unwrap_or(hostname);

		tracing::debug!(
			hostname = %hostname_only,
			path = %path,
			port_type = ?port_type,
			"Resolving route for request"
		);

		// Check cache first
		if !ignore_cache {
			if let Some(result) = self.route_cache.get(hostname_only, path).await {
				// Choose a random target from the cached targets
				if let Some(target) = choose_random_target(&result.targets) {
					return Ok(ResolveRouteOutput::Target(target.clone()));
				}
			}
		}

		// Not in cache, call routing function with a default timeout
		// Default 15 seconds, routing functions should have their own internal timeouts that are shorter
		let default_timeout = Duration::from_secs(15);

		tracing::debug!(
			hostname = %hostname_only,
			path = %path,
			cache_hit = false,
			timeout_seconds = default_timeout.as_secs(),
			"Cache miss, calling routing function"
		);
		let routing_result = timeout(
			default_timeout,
			(self.routing_fn)(hostname_only, path, port_type),
		)
		.await;

		// Handle timeout and routing errors
		match routing_result {
			Ok(result) => match result? {
				RoutingOutput::Route(result) => {
					tracing::debug!(
						hostname = %hostname_only,
						path = %path,
						targets_count = result.targets.len(),
						timeout_secs = result.timeout.routing_timeout,
						"Received routing result"
					);

					// Cache the result
					self.route_cache
						.insert(hostname_only.to_owned(), path.to_owned(), result.clone())
						.await;
					tracing::debug!("Added route to cache");

					// Choose a random target
					if let Some(target) = choose_random_target(&result.targets) {
						tracing::debug!(
							hostname = %hostname_only,
							path = %path,
							target_host = %target.host,
							target_port = target.port,
							target_path = %target.path,
							actor_id = ?target.actor_id,
							server_id = ?target.server_id,
							"Selected target for request"
						);
						Ok(ResolveRouteOutput::Target(target.clone()))
					} else {
						tracing::warn!(
							hostname = %hostname_only,
							path = %path,
							"No route targets available from result"
						);
						Ok(ResolveRouteOutput::Response(StructuredResponse {
							status: StatusCode::BAD_GATEWAY,
							message: Cow::Borrowed("No route targets available"),
							docs: None,
						}))
					}
				}
				RoutingOutput::Response(response) => {
					tracing::debug!(
						hostname = %hostname_only,
						path = %path,
						status = ?response.status,
						"Routing returned custom response"
					);
					Ok(ResolveRouteOutput::Response(response))
				}
			},
			Err(_) => {
				tracing::warn!(
					hostname = %hostname_only,
					path = %path,
					timeout_seconds = default_timeout.as_secs(),
					"Routing function timed out"
				);
				Ok(ResolveRouteOutput::Response(StructuredResponse {
					status: StatusCode::GATEWAY_TIMEOUT,
					message: Cow::Owned(format!(
						"Routing function timed out after {}s",
						default_timeout.as_secs()
					)),
					docs: None,
				}))
			}
		}
	}

	#[tracing::instrument(skip_all)]
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
							requests: 100, // 100 requests
							period: 60,    // per 60 seconds
						},
						max_in_flight: MaxInFlightConfig {
							amount: 20, // 20 concurrent requests
						},
						retry: RetryConfig {
							max_attempts: 3,       // 3 retry attempts
							initial_interval: 100, // 100ms initial interval
						},
						timeout: TimeoutConfig {
							request_timeout: 30, // 30 seconds for requests
						},
					})
				}
			},
			Err(_) => {
				// Default values if middleware times out
				Ok(MiddlewareConfig {
					rate_limit: RateLimitConfig {
						requests: 100, // 100 requests
						period: 60,    // per 60 seconds
					},
					max_in_flight: MaxInFlightConfig {
						amount: 20, // 20 concurrent requests
					},
					retry: RetryConfig {
						max_attempts: 3,       // 3 retry attempts
						initial_interval: 100, // 100ms initial interval
					},
					timeout: TimeoutConfig {
						request_timeout: 30, // 30 seconds for requests
					},
				})
			}
		}
	}

	#[tracing::instrument(skip_all)]
	async fn check_rate_limit(
		&self,
		ip_addr: std::net::IpAddr,
		actor_id: &Option<Uuid>,
	) -> GlobalResult<bool> {
		let Some(actor_id) = *actor_id else {
			// No rate limiting when actor_id is None
			return Ok(true);
		};

		// Get actor-specific middleware config
		let middleware_config = self.get_middleware_config(&actor_id).await?;

		let cache_key = (actor_id, ip_addr);

		// Get existing limiter or create a new one
		let limiter_arc = if let Some(existing_limiter) = self.rate_limiters.get(&cache_key).await {
			existing_limiter
		} else {
			let new_limiter = Arc::new(Mutex::new(RateLimiter::new(
				middleware_config.rate_limit.requests,
				middleware_config.rate_limit.period,
			)));
			self.rate_limiters
				.insert(cache_key, new_limiter.clone())
				.await;
			metrics::RATE_LIMITER_COUNT.set(self.rate_limiters.entry_count() as i64);
			new_limiter
		};

		// Try to acquire from the limiter
		let result = {
			let mut limiter = limiter_arc.lock().await;
			limiter.try_acquire()
		};

		Ok(result)
	}

	#[tracing::instrument(skip_all)]
	async fn acquire_in_flight(
		&self,
		ip_addr: std::net::IpAddr,
		actor_id: &Option<Uuid>,
	) -> GlobalResult<bool> {
		let Some(actor_id) = *actor_id else {
			// No in-flight limiting when actor_id is None
			return Ok(true);
		};

		// Get actor-specific middleware config
		let middleware_config = self.get_middleware_config(&actor_id).await?;

		let cache_key = (actor_id, ip_addr);

		// Get existing counter or create a new one
		let counter_arc =
			if let Some(existing_counter) = self.in_flight_counters.get(&cache_key).await {
				existing_counter
			} else {
				let new_counter = Arc::new(Mutex::new(InFlightCounter::new(
					middleware_config.max_in_flight.amount,
				)));
				self.in_flight_counters
					.insert(cache_key, new_counter.clone())
					.await;
				metrics::IN_FLIGHT_COUNTER_COUNT.set(self.in_flight_counters.entry_count() as i64);
				new_counter
			};

		// Try to acquire from the counter
		let result = {
			let mut counter = counter_arc.lock().await;
			counter.try_acquire()
		};

		Ok(result)
	}

	#[tracing::instrument(skip_all)]
	async fn release_in_flight(&self, ip_addr: std::net::IpAddr, actor_id: &Option<Uuid>) {
		// Skip if actor_id is None (no in-flight tracking)
		let actor_id = match actor_id {
			Some(id) => *id,
			None => return, // No in-flight tracking when actor_id is None
		};

		let cache_key = (actor_id, ip_addr);
		if let Some(counter_arc) = self.in_flight_counters.get(&cache_key).await {
			let mut counter = counter_arc.lock().await;
			counter.release();
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

	#[tracing::instrument(skip_all)]
	async fn handle_request(
		&self,
		req: Request<BodyIncoming>,
		request_context: &mut RequestContext,
	) -> GlobalResult<Response<Full<Bytes>>> {
		let host = req
			.headers()
			.get(hyper::header::HOST)
			.and_then(|h| h.to_str().ok())
			.unwrap_or("unknown");

		let path = req
			.uri()
			.path_and_query()
			.map(|x| x.to_string())
			.unwrap_or_else(|| req.uri().path().to_string());

		let start_time = Instant::now();

		// Set request body size in analytics (will be updated with actual size later)

		let target_res = self
			.state
			.resolve_route(host, &path, self.state.port_type.clone(), false)
			.await;

		let duration_secs = start_time.elapsed().as_secs_f64();
		metrics::RESOLVE_ROUTE_DURATION.observe(duration_secs);

		// Resolve target
		let target = match target_res {
			Ok(ResolveRouteOutput::Target(target)) => target,
			Ok(ResolveRouteOutput::Response(response)) => {
				// Return the custom response
				return response.build_response();
			}
			Err(e) => {
				error!("Routing error: {}", e);
				return Ok(Response::builder()
					.status(StatusCode::BAD_GATEWAY)
					.body(Full::<Bytes>::new(Bytes::new()))?);
			}
		};

		let actor_id = target.actor_id;

		// Update request context with target info
		if let Some(actor_id) = actor_id {
			request_context.service_actor_id = Some(actor_id);
		}
		if let Some(server_id) = target.server_id {
			request_context.service_server_id = Some(server_id);
		}

		// Set service IP from target
		if let Ok(target_ip) =
			format!("{}:{}", target.host, target.port).parse::<std::net::SocketAddr>()
		{
			request_context.service_ip = Some(target_ip.ip());
		}

		// Extract IP address from remote_addr
		let client_ip = self.remote_addr.ip();

		// Apply rate limiting
		let res = if !self.state.check_rate_limit(client_ip, &actor_id).await? {
			Response::builder()
				.status(StatusCode::TOO_MANY_REQUESTS)
				.body(Full::<Bytes>::new(Bytes::new()))
				.map_err(Into::into)
		}
		// Check in-flight limit
		else if !self.state.acquire_in_flight(client_ip, &actor_id).await? {
			Response::builder()
				.status(StatusCode::TOO_MANY_REQUESTS)
				.body(Full::<Bytes>::new(Bytes::new()))
				.map_err(Into::into)
		} else {
			// Increment metrics
			metrics::PROXY_REQUEST_PENDING.inc();
			metrics::PROXY_REQUEST_TOTAL.inc();

			// Prepare to release in-flight counter when done
			let state_clone = self.state.clone();
			crate::defer! {
				tokio::spawn(async move {
					state_clone.release_in_flight(client_ip, &actor_id).await;
				}.instrument(tracing::info_span!("release_in_flight_task")));
			}

			// Branch for WebSocket vs HTTP handling
			// Both paths will handle their own metrics and error handling
			let res = if hyper_tungstenite::is_upgrade_request(&req) {
				// WebSocket upgrade
				self.handle_websocket_upgrade(req, target, request_context)
					.await
			} else {
				// Regular HTTP request
				self.handle_http_request(req, target, request_context).await
			};

			let status = match &res {
				Ok(resp) => resp.status().as_u16().to_string(),
				Err(_) => "error".to_string(),
			};

			// Record metrics
			let duration_secs = start_time.elapsed().as_secs_f64();
			metrics::PROXY_REQUEST_DURATION
				.with_label_values(&[&status])
				.observe(duration_secs);

			metrics::PROXY_REQUEST_PENDING.dec();

			res
		};

		if let Err(err) = &res {
			metrics::PROXY_REQUEST_ERROR
				.with_label_values(&[&err.to_string()])
				.inc();
		}

		// Update request context with response details
		match &res {
			Ok(resp) => {
				request_context.guard_response_status = Some(resp.status().as_u16());
				request_context.service_response_status = Some(resp.status().as_u16());

				if let Some(content_type) = resp
					.headers()
					.get(hyper::header::CONTENT_TYPE)
					.and_then(|h| h.to_str().ok())
				{
					request_context.guard_response_content_type = Some(content_type.to_string());
				}

				if let Some(expires) = resp
					.headers()
					.get(hyper::header::EXPIRES)
					.and_then(|h| h.to_str().ok())
				{
					request_context.service_response_http_expires = Some(expires.to_string());
				}

				if let Some(last_modified) = resp
					.headers()
					.get(hyper::header::LAST_MODIFIED)
					.and_then(|h| h.to_str().ok())
				{
					request_context.service_response_http_last_modified =
						Some(last_modified.to_string());
				}
			}
			Err(_) => {
				request_context.guard_response_status = Some(500);
				request_context.service_response_status = Some(500);
			}
		}

		// Set timing information
		request_context.service_response_duration_ms =
			Some(start_time.elapsed().as_millis() as u32);

		// Insert analytics event asynchronously
		let mut context_clone = request_context.clone();
		tokio::spawn(async move {
			if let Err(error) = context_clone.insert_event().await {
				tracing::warn!(?error, "failed to insert guard analytics event");
			}
		});

		res
	}

	#[tracing::instrument(skip_all)]
	async fn handle_http_request(
		&self,
		req: Request<BodyIncoming>,
		mut target: RouteTarget,
		request_context: &mut RequestContext,
	) -> GlobalResult<Response<Full<Bytes>>> {
		// Get middleware config for this actor if it exists
		let middleware_config = match &target.actor_id {
			Some(actor_id) => self.state.get_middleware_config(actor_id).await?,
			None => {
				// Default middleware config for targets without actor_id
				MiddlewareConfig {
					rate_limit: RateLimitConfig {
						requests: 100, // 100 requests
						period: 60,    // per 60 seconds
					},
					max_in_flight: MaxInFlightConfig {
						amount: 20, // 20 concurrent requests
					},
					retry: RetryConfig {
						max_attempts: 3,       // 3 retry attempts
						initial_interval: 100, // 100ms initial interval
					},
					timeout: TimeoutConfig {
						request_timeout: 30, // 30 seconds for requests
					},
				}
			}
		};

		let host = req
			.headers()
			.get(hyper::header::HOST)
			.and_then(|h| h.to_str().ok())
			.unwrap_or("unknown")
			.to_string();

		let path = req
			.uri()
			.path_and_query()
			.map(|x| x.to_string())
			.unwrap_or_else(|| req.uri().path().to_string());

		// Read the request body before proceeding with retries
		let (req_parts, body) = req.into_parts();
		let req_body = match http_body_util::BodyExt::collect(body).await {
			Ok(collected) => collected.to_bytes(),
			Err(e) => {
				warn!("Failed to read request body: {}", e);
				Bytes::new()
			}
		};

		// Set actual request body size in analytics
		request_context.client_request_body_bytes = Some(req_body.len() as u64);

		// Set up retry with backoff from middleware config
		let max_attempts = middleware_config.retry.max_attempts;
		let initial_interval = middleware_config.retry.initial_interval;
		let timeout_duration = Duration::from_secs(middleware_config.timeout.request_timeout);

		// Use a value-returning loop to handle both errors and successful responses
		let (status_code, last_error) = 'retry: {
			let mut attempts = 0;

			while attempts < max_attempts {
				attempts += 1;

				// Use the common function to build request parts
				let (uri_str, builder) = match self.build_proxied_request_parts(&req_parts, &target)
				{
					Ok(parts) => parts,
					Err(e) => {
						error!("Failed to build HTTP request: {}", e);

						let error = Some(global_error::ext::AssertionError::Panic {
							message: format!("Failed to build HTTP request: {}", e),
							location: global_error::location!(),
						});
						break 'retry (StatusCode::INTERNAL_SERVER_ERROR, error);
					}
				};

				// Parse the URI for hyper-util client
				let _uri = match uri_str.parse::<hyper::Uri>() {
					Ok(uri) => uri,
					Err(e) => {
						error!("Failed to parse URI: {}", e);

						let error = Some(global_error::ext::AssertionError::Panic {
							message: format!("URI parse error: {}", e),
							location: global_error::location!(),
						});
						break 'retry (StatusCode::BAD_GATEWAY, error);
					}
				};

				// Create the final request with body
				let proxied_req = match builder.body(Full::<Bytes>::new(req_body.clone())) {
					Ok(req) => req,
					Err(e) => {
						warn!("Failed to build request body: {}", e);

						let error = Some(global_error::ext::AssertionError::Panic {
							message: format!("Request build error: {}", e),
							location: global_error::location!(),
						});
						break 'retry (StatusCode::INTERNAL_SERVER_ERROR, error);
					}
				};

				// Send the request with timeout
				let request_send_start = Instant::now();
				match timeout(timeout_duration, self.client.request(proxied_req)).await {
					Ok(Ok(resp)) => {
						let response_receive_time = request_send_start.elapsed();

						// Convert the hyper::body::Incoming to http_body_util::Full<Bytes>
						let (parts, body) = resp.into_parts();

						// Read the response body
						let body_bytes = match http_body_util::BodyExt::collect(body).await {
							Ok(collected) => collected.to_bytes(),
							Err(_) => Bytes::new(),
						};

						// Set actual response body size in analytics
						request_context.guard_response_body_bytes = Some(body_bytes.len() as u64);

						let full_body = Full::new(body_bytes);
						return Ok(Response::from_parts(parts, full_body));
					}
					Ok(Err(e)) => {
						if !e.is_connect() || attempts >= max_attempts {
							let error = Some(global_error::ext::AssertionError::Panic {
								message: format!("Request error: {}", e),
								location: global_error::location!(),
							});
							break 'retry (StatusCode::BAD_GATEWAY, error);
						} else {
							// Request connect error, might retry
							warn!("Request attempt {} failed: {}", attempts, e);

							// Use backoff and continue
							let backoff = Self::calculate_backoff(attempts, initial_interval);

							let (_, new_target) = tokio::join!(
								tokio::time::sleep(backoff),
								// Resolve target again, this time ignoring cache. This makes sure
								// we always re-fetch the route on error
								self.state.resolve_route(
									&host,
									&path,
									self.state.port_type.clone(),
									true,
								),
							);

							target = match new_target {
								Ok(ResolveRouteOutput::Target(target)) => target,
								Ok(ResolveRouteOutput::Response(response)) => {
									return response.build_response()
								}
								Err(e) => {
									error!("Routing error: {}", e);
									return Ok(Response::builder()
										.status(StatusCode::BAD_GATEWAY)
										.body(Full::<Bytes>::new(Bytes::new()))?);
								}
							};

							continue;
						}
					}
					Err(_) => {
						// Timeout error
						warn!(
							"Request timed out after {} seconds",
							timeout_duration.as_secs()
						);

						let error = Some(global_error::ext::AssertionError::Panic {
							message: format!("Request timed out"),
							location: global_error::location!(),
						});
						break 'retry (StatusCode::GATEWAY_TIMEOUT, error);
					}
				}
			}

			// If we get here, all attempts failed
			let error = Some(global_error::ext::AssertionError::Panic {
				message: format!("All retry attempts failed"),
				location: global_error::location!(),
			});
			(StatusCode::BAD_GATEWAY, error)
		};

		// Log the error
		error!("Request failed: {:?}", last_error);

		Ok(Response::builder()
			.status(status_code)
			.body(Full::<Bytes>::new(Bytes::new()))?)
	}

	// Common function to build a request URI and headers
	fn build_proxied_request_parts(
		&self,
		req_parts: &hyper::http::request::Parts,
		target: &RouteTarget,
	) -> GlobalResult<(String, hyper::http::request::Builder)> {
		// Build the target URI
		let uri = format!("http://{}:{}{}", target.host, target.port, target.path);
		let mut builder = hyper::Request::builder()
			.method(req_parts.method.clone())
			.uri(&uri);

		// Copy headers except Host
		let headers = builder.headers_mut().unwrap();
		for (key, value) in req_parts.headers.iter() {
			if key != hyper::header::HOST {
				headers.insert(key.clone(), value.clone());
			}
		}

		// Add X-Forwarded-For header
		if let Some(existing) = req_parts.headers.get(X_FORWARDED_FOR) {
			if let Ok(forwarded) = existing.to_str() {
				if !forwarded.contains(&self.remote_addr.ip().to_string()) {
					headers.insert(
						X_FORWARDED_FOR,
						hyper::header::HeaderValue::from_str(&format!(
							"{}, {}",
							forwarded,
							self.remote_addr.ip()
						))?,
					);
				}
			}
		} else {
			headers.insert(
				X_FORWARDED_FOR,
				hyper::header::HeaderValue::from_str(&self.remote_addr.ip().to_string())?,
			);
		}

		Ok((uri, builder))
	}

	#[tracing::instrument(skip_all)]
	async fn handle_websocket_upgrade(
		&self,
		req: Request<BodyIncoming>,
		mut target: RouteTarget,
		_request_context: &mut RequestContext,
	) -> GlobalResult<Response<Full<Bytes>>> {
		// Get actor and server IDs for metrics and middleware
		let actor_id = target.actor_id;
		let server_id = target.server_id;
		let actor_id_str = actor_id.map_or_else(|| "none".to_string(), |id| id.to_string());
		let server_id_str = server_id.map_or_else(|| "none".to_string(), |id| id.to_string());

		// Parsed for retries later
		let req_host = req
			.headers()
			.get(hyper::header::HOST)
			.and_then(|h| h.to_str().ok())
			.unwrap_or("unknown")
			.to_string();
		let req_path = req
			.uri()
			.path_and_query()
			.map(|x| x.to_string())
			.unwrap_or_else(|| req.uri().path().to_string());

		// Log request details
		info!("WebSocket upgrade request for path: {}, target host: {}:{}, actor_id: {}, server_id: {}",
			target.path, target.host, target.port, actor_id_str, server_id_str);

		// Get middleware config for this actor if it exists
		let middleware_config = match &actor_id {
			Some(actor_id) => self.state.get_middleware_config(actor_id).await?,
			None => {
				// Default middleware config for targets without actor_id
				info!("Using default middleware config (no actor_id)");
				MiddlewareConfig {
					rate_limit: RateLimitConfig {
						requests: 100, // 100 requests
						period: 60,    // per 60 seconds
					},
					max_in_flight: MaxInFlightConfig {
						amount: 20, // 20 concurrent requests
					},
					retry: RetryConfig {
						max_attempts: 3,       // 3 retry attempts
						initial_interval: 100, // 100ms initial interval
					},
					timeout: TimeoutConfig {
						request_timeout: 30, // 30 seconds for requests
					},
				}
			}
		};

		// Set up retry with backoff from middleware config
		let max_attempts = middleware_config.retry.max_attempts;
		let initial_interval = middleware_config.retry.initial_interval;

		// Log the headers for debugging
		debug!("WebSocket upgrade request headers:");
		for (name, value) in req.headers() {
			if let Ok(val) = value.to_str() {
				debug!("  {}: {}", name, val);
			}
		}

		// Handle WebSocket upgrade properly with hyper_tungstenite
		// First, upgrade the client connection
		info!("Upgrading client connection to WebSocket");
		let (client_response, client_websocket) = match hyper_tungstenite::upgrade(req, None) {
			Ok(x) => {
				info!("Client WebSocket upgrade successful");
				x
			}
			Err(err) => {
				error!("Failed to upgrade client WebSocket: {}", err);
				bail!("Failed to upgrade client WebSocket: {err}")
			}
		};

		// Log response status and headers
		info!(
			"Client upgrade response status: {}",
			client_response.status()
		);
		for (name, value) in client_response.headers() {
			if let Ok(val) = value.to_str() {
				debug!("Client upgrade response header - {}: {}", name, val);
			}
		}

		// Clone needed values for the spawned task
		let state = self.state.clone();

		// Spawn a new task to handle the WebSocket bidirectional communication
		info!("Spawning task to handle WebSocket communication");
		tokio::spawn(
			async move {
				// Set up a timeout for the entire operation
				let timeout_duration = Duration::from_secs(30); // 30 seconds timeout
				info!(
					"WebSocket proxy task started with {}s timeout",
					timeout_duration.as_secs()
				);

				// Use retry logic to connect to the upstream WebSocket server
				let mut attempts = 0;
				let mut upstream_ws = None;

				// First, wait for the client WebSocket to be ready (do this first to avoid race conditions)
				info!("Waiting for client WebSocket to be ready...");
				let client_ws = match tokio::time::timeout(timeout_duration, client_websocket).await
				{
					Ok(Ok(ws)) => {
						info!("Client WebSocket is ready");
						ws
					}
					Ok(Err(e)) => {
						error!("Failed to get client WebSocket: {}", e);
						error!("Error details: {:?}", e);
						return;
					}
					Err(_) => {
						error!(
							"Timeout waiting for client WebSocket to be ready after {}s",
							timeout_duration.as_secs()
						);
						return;
					}
				};

				// Now attempt to connect to the upstream server
				info!("Attempting connect to upstream WebSocket");
				while attempts < max_attempts {
					attempts += 1;

					let target_url = format!("ws://{}:{}{}", target.host, target.port, target.path);
					info!(
						"WebSocket request attempt {}/{} to {}",
						attempts, max_attempts, target_url
					);

					match tokio::time::timeout(
						Duration::from_secs(5), // 5 second timeout per connection attempt
						tokio_tungstenite::connect_async(&target_url),
					)
					.await
					{
						Ok(Ok((ws_stream, resp))) => {
							info!("Successfully connected to upstream WebSocket server");
							debug!("Upstream connection response status: {:?}", resp.status());

							// Log headers for debugging
							for (name, value) in resp.headers() {
								if let Ok(val) = value.to_str() {
									debug!("Upstream response header - {}: {}", name, val);
								}
							}

							upstream_ws = Some(ws_stream);
							break;
						}
						Ok(Err(e)) => {
							warn!("WebSocket request attempt {} failed: {}", attempts, e);
							warn!("Error details: {:?}", e);
						}
						Err(_) => {
							warn!("WebSocket request attempt {} timed out after 5s", attempts);
						}
					}

					// Check if we've reached max attempts
					if attempts >= max_attempts {
						error!("All {} WebSocket connection attempts failed", max_attempts);

						// Send a close message to the client since we can't connect to upstream
						info!("Sending close message to client due to upstream connection failure");
						let (mut client_sink, _) = client_ws.split();
						match client_sink
							.send(hyper_tungstenite::tungstenite::Message::Close(Some(
								hyper_tungstenite::tungstenite::protocol::CloseFrame {
									code: 1011.into(), // 1011 = Server error
									reason: "Failed to connect to upstream server".into(),
								},
							)))
							.await
						{
							Ok(_) => info!("Successfully sent close message to client"),
							Err(e) => error!("Failed to send close message to client: {}", e),
						};

						match client_sink.flush().await {
							Ok(_) => info!("Successfully flushed client sink after close"),
							Err(e) => error!("Failed to flush client sink after close: {}", e),
						};

						return;
					}

					// Use backoff for the next attempt
					let backoff = Self::calculate_backoff(attempts, initial_interval);
					info!("Waiting for {:?} before next connection attempt", backoff);

					let (_, new_target) = tokio::join!(
						tokio::time::sleep(backoff),
						// Resolve target again, this time ignoring cache. This makes sure
						// we always re-fetch the route on error
						state.resolve_route(&req_host, &req_path, state.port_type.clone(), true,),
					);

					match new_target {
						Ok(ResolveRouteOutput::Target(new_target)) => {
							target = new_target;
						}
						Ok(ResolveRouteOutput::Response(_response)) => {
							error!("Expected target, got response")
						}
						Err(e) => error!("Routing error: {}", e),
					}
				}

				// If we couldn't connect to the upstream server, exit the task
				let upstream_ws = match upstream_ws {
					Some(ws) => {
						info!("Successfully established upstream WebSocket connection");
						ws
					}
					Option::None => {
						error!("Failed to establish upstream WebSocket connection (unexpected)");
						return; // Should never happen due to checks above, but just in case
					}
				};

				// Now set up bidirectional communication between the client and upstream WebSockets
				info!("Setting up bidirectional WebSocket proxying");
				let (client_sink, client_stream) = client_ws.split();
				let (upstream_sink, upstream_stream) = upstream_ws.split();

				// Create channels for coordinating shutdown between client and upstream
				let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

				// Manually forward messages from client to upstream server with shutdown coordination
				let client_to_upstream = async {
					info!("Starting client-to-upstream forwarder");
					let mut stream = client_stream;
					let mut sink = upstream_sink;
					let mut shutdown_rx = shutdown_rx.clone();

					loop {
						tokio::select! {
							// Check for shutdown signal
							shutdown_result = shutdown_rx.changed() => {
								match shutdown_result {
									Ok(_) => {
										if *shutdown_rx.borrow() {
											info!("Client-to-upstream forwarder shutting down due to signal");
											break;
										}
									},
									Err(e) => {
										// Channel closed
										info!("Client-to-upstream shutdown channel closed: {}", e);
										break;
									}
								}
							}

							// Process next message from client
							msg_result = stream.next() => {
								match msg_result {
									Some(Ok(client_msg)) => {
										// Debug output with message type
										match &client_msg {
											hyper_tungstenite::tungstenite::Message::Text(text) => {
												info!("Received text message from client: {} bytes", text.len());
												debug!("Client text message content: {}", text);
											},
											hyper_tungstenite::tungstenite::Message::Binary(data) => {
												info!("Received binary message from client: {} bytes", data.len());
											},
											hyper_tungstenite::tungstenite::Message::Ping(data) => {
												info!("Received ping from client: {} bytes", data.len());
											},
											hyper_tungstenite::tungstenite::Message::Pong(data) => {
												info!("Received pong from client: {} bytes", data.len());
											},
											hyper_tungstenite::tungstenite::Message::Close(frame) => {
												if let Some(f) = frame {
													info!("Received close from client with code: {}", u16::from(f.code));
												} else {
													info!("Received close from client without code");
												}
											},
											_ => {
												info!("Received unknown message type from client");
											}
										}

										// Convert from hyper_tungstenite::Message to tokio_tungstenite::Message
										let upstream_msg = match client_msg {
											hyper_tungstenite::tungstenite::Message::Text(text) => {
												info!("Converting text message to upstream format");
												tokio_tungstenite::tungstenite::Message::Text(text)
											},
											hyper_tungstenite::tungstenite::Message::Binary(data) => {
												info!("Converting binary message to upstream format");
												tokio_tungstenite::tungstenite::Message::Binary(data)
											},
											hyper_tungstenite::tungstenite::Message::Ping(data) => {
												info!("Converting ping message to upstream format");
												tokio_tungstenite::tungstenite::Message::Ping(data)
											},
											hyper_tungstenite::tungstenite::Message::Pong(data) => {
												info!("Converting pong message to upstream format");
												tokio_tungstenite::tungstenite::Message::Pong(data)
											},
											hyper_tungstenite::tungstenite::Message::Close(frame) => {
												info!("Converting close message to upstream format");
												// Signal shutdown to other direction
												let _ = shutdown_tx.send(true);

												if let Some(frame) = frame {
													// Manual conversion to handle different tungstenite versions
													let code_num: u16 = frame.code.into();
													let reason = frame.reason.clone();

													tokio_tungstenite::tungstenite::Message::Close(Some(
														tokio_tungstenite::tungstenite::protocol::CloseFrame {
															code: code_num.into(),
															reason,
														},
													))
												} else {
													tokio_tungstenite::tungstenite::Message::Close(None)
												}
											},
											hyper_tungstenite::tungstenite::Message::Frame(_) => {
												info!("Skipping frame message - implementation detail");
												// Skip frames - they're an implementation detail
												continue;
											},
										};

										// Send the message with a timeout
										info!("Sending message to upstream server");
										let send_result = tokio::time::timeout(
											Duration::from_secs(5),
											sink.send(upstream_msg)
										).await;

										match send_result {
											Ok(Ok(_)) => {
												info!("Message sent to upstream successfully");
												// Flush the sink with a timeout
												info!("Flushing upstream sink");
												let flush_result = tokio::time::timeout(
													Duration::from_secs(2),
													sink.flush()
												).await;

												if let Err(_) = flush_result {
													error!("Timeout flushing upstream sink");
													let _ = shutdown_tx.send(true);
													break;
												} else if let Ok(Err(e)) = flush_result {
													error!("Error flushing upstream sink: {}", e);
													let _ = shutdown_tx.send(true);
													break;
												} else {
													info!("Upstream sink flushed successfully");
												}
											},
											Ok(Err(e)) => {
												error!("Error sending message to upstream: {}", e);
												error!("Error details: {:?}", e);
												let _ = shutdown_tx.send(true);
												break;
											},
											Err(_) => {
												error!("Timeout sending message to upstream after 5s");
												let _ = shutdown_tx.send(true);
												break;
											}
										}
									},
									Some(Err(e)) => {
										// Error receiving message from client
										error!("Error receiving message from client: {}", e);
										error!("Error details: {:?}", e);
										// Signal shutdown to other direction
										let _ = shutdown_tx.send(true);
										break;
									},
									None => {
										// End of stream
										info!("Client WebSocket stream ended");
										// Signal shutdown to other direction
										let _ = shutdown_tx.send(true);
										break;
									}
								}
							}
						}
					}

					// Try to send a close frame - ignore errors as the connection might already be closed
					info!("Attempting to send close message to upstream");
					match sink
						.send(tokio_tungstenite::tungstenite::Message::Close(None))
						.await
					{
						Ok(_) => info!("Close message sent to upstream successfully"),
						Err(e) => warn!("Failed to send close message to upstream: {}", e),
					};

					match sink.flush().await {
						Ok(_) => info!("Upstream sink flushed successfully after close"),
						Err(e) => warn!("Failed to flush upstream sink after close: {}", e),
					};

					info!("Client-to-upstream task completed");
				};

				// Manually forward messages from upstream server to client with shutdown coordination
				let upstream_to_client = async {
					info!("Starting upstream-to-client forwarder");
					let mut stream = upstream_stream;
					let mut sink = client_sink;
					let mut shutdown_rx = shutdown_rx.clone();

					loop {
						tokio::select! {
							// Check for shutdown signal
							shutdown_result = shutdown_rx.changed() => {
								match shutdown_result {
									Ok(_) => {
										if *shutdown_rx.borrow() {
											info!("Upstream-to-client forwarder shutting down due to signal");
											break;
										}
									},
									Err(e) => {
										// Channel closed
										info!("Upstream-to-client shutdown channel closed: {}", e);
										break;
									}
								}
							}

							// Process next message from upstream
							msg_result = stream.next() => {
								match msg_result {
									Some(Ok(upstream_msg)) => {
										// Debug output with message type
										match &upstream_msg {
											tokio_tungstenite::tungstenite::Message::Text(text) => {
												info!("Received text message from upstream: {} bytes", text.len());
												debug!("Upstream text message content: {}", text);
											},
											tokio_tungstenite::tungstenite::Message::Binary(data) => {
												info!("Received binary message from upstream: {} bytes", data.len());
											},
											tokio_tungstenite::tungstenite::Message::Ping(data) => {
												info!("Received ping from upstream: {} bytes", data.len());
											},
											tokio_tungstenite::tungstenite::Message::Pong(data) => {
												info!("Received pong from upstream: {} bytes", data.len());
											},
											tokio_tungstenite::tungstenite::Message::Close(frame) => {
												if let Some(f) = frame {
													info!("Received close from upstream with code: {}", u16::from(f.code));
												} else {
													info!("Received close from upstream without code");
												}
											},
											_ => {
												info!("Received unknown message type from upstream");
											}
										}

										// Convert from tokio_tungstenite::Message to hyper_tungstenite::Message
										let client_msg = match upstream_msg {
											tokio_tungstenite::tungstenite::Message::Text(text) => {
												info!("Converting text message to client format");
												hyper_tungstenite::tungstenite::Message::Text(text)
											},
											tokio_tungstenite::tungstenite::Message::Binary(data) => {
												info!("Converting binary message to client format");
												hyper_tungstenite::tungstenite::Message::Binary(data)
											},
											tokio_tungstenite::tungstenite::Message::Ping(data) => {
												info!("Converting ping message to client format");
												hyper_tungstenite::tungstenite::Message::Ping(data)
											},
											tokio_tungstenite::tungstenite::Message::Pong(data) => {
												info!("Converting pong message to client format");
												hyper_tungstenite::tungstenite::Message::Pong(data)
											},
											tokio_tungstenite::tungstenite::Message::Close(frame) => {
												info!("Converting close message to client format");
												// Signal shutdown to other direction
												let _ = shutdown_tx.send(true);

												if let Some(frame) = frame {
													// Manual conversion to handle different tungstenite versions
													let code_num: u16 = frame.code.into();
													let reason = frame.reason.clone();

													hyper_tungstenite::tungstenite::Message::Close(Some(
														hyper_tungstenite::tungstenite::protocol::CloseFrame {
															code: code_num.into(),
															reason,
														},
													))
												} else {
													hyper_tungstenite::tungstenite::Message::Close(None)
												}
											},
											tokio_tungstenite::tungstenite::Message::Frame(_) => {
												info!("Skipping frame message - implementation detail");
												// Skip frames - they're an implementation detail
												continue;
											},
										};

										// Send the message with a timeout
										info!("Sending message to client");
										let send_result = tokio::time::timeout(
											Duration::from_secs(5),
											sink.send(client_msg)
										).await;

										match send_result {
											Ok(Ok(_)) => {
												info!("Message sent to client successfully");
												// Flush the sink with a timeout
												info!("Flushing client sink");
												let flush_result = tokio::time::timeout(
													Duration::from_secs(2),
													sink.flush()
												).await;

												if let Err(_) = flush_result {
													error!("Timeout flushing client sink");
													let _ = shutdown_tx.send(true);
													break;
												} else if let Ok(Err(e)) = flush_result {
													error!("Error flushing client sink: {}", e);
													let _ = shutdown_tx.send(true);
													break;
												} else {
													info!("Client sink flushed successfully");
												}
											},
											Ok(Err(e)) => {
												error!("Error sending message to client: {}", e);
												error!("Error details: {:?}", e);
												let _ = shutdown_tx.send(true);
												break;
											},
											Err(_) => {
												error!("Timeout sending message to client after 5s");
												let _ = shutdown_tx.send(true);
												break;
											}
										}
									},
									Some(Err(e)) => {
										// Error receiving message from upstream
										error!("Error receiving message from upstream: {}", e);
										error!("Error details: {:?}", e);
										// Signal shutdown to other direction
										let _ = shutdown_tx.send(true);
										break;
									},
									None => {
										// End of stream
										info!("Upstream WebSocket stream ended");
										// Signal shutdown to other direction
										let _ = shutdown_tx.send(true);
										break;
									}
								}
							}
						}
					}

					// Try to send a close frame - ignore errors as the connection might already be closed
					info!("Attempting to send close message to client");
					match sink
						.send(hyper_tungstenite::tungstenite::Message::Close(None))
						.await
					{
						Ok(_) => info!("Close message sent to client successfully"),
						Err(e) => warn!("Failed to send close message to client: {}", e),
					};

					match sink.flush().await {
						Ok(_) => info!("Client sink flushed successfully after close"),
						Err(e) => warn!("Failed to flush client sink after close: {}", e),
					};

					info!("Upstream-to-client task completed");
				};

				// Run both directions concurrently until either one completes or errors
				info!("Starting bidirectional message forwarding");
				tokio::join!(client_to_upstream, upstream_to_client);
				info!("Bidirectional message forwarding completed");
			}
			.instrument(tracing::info_span!("handle_ws_task")),
		);

		// Return the response that will upgrade the client connection
		// For proper WebSocket handshaking, we need to preserve the original response
		// structure but convert it to our expected return type without modifying its content
		info!("Returning WebSocket upgrade response to client");
		// Extract the parts from the response but preserve all headers and status
		let (parts, _) = client_response.into_parts();
		// Create a new response with an empty body - WebSocket upgrades don't need a body
		Ok(Response::from_parts(
			parts,
			Full::<Bytes>::new(Bytes::new()),
		))
	}
}

impl ProxyService {
	// Process an individual request
	#[tracing::instrument(skip_all)]
	pub async fn process(&self, req: Request<BodyIncoming>) -> GlobalResult<Response<Full<Bytes>>> {
		// Create request context for analytics tracking
		let mut request_context = RequestContext::new(self.state.clickhouse_inserter.clone());

		// Extract request information for logging and analytics before consuming the request
		let host = req
			.headers()
			.get(hyper::header::HOST)
			.and_then(|h| h.to_str().ok())
			.unwrap_or("unknown")
			.to_string();
		let uri_string = req.uri().to_string();
		let path = req
			.uri()
			.path_and_query()
			.map(|x| x.to_string())
			.unwrap_or_else(|| req.uri().path().to_string());
		let method = req.method().clone();

		let user_agent = req
			.headers()
			.get(hyper::header::USER_AGENT)
			.and_then(|h| h.to_str().ok())
			.map(|s| s.to_string());

		// Populate request context with available data
		request_context.client_ip = Some(self.remote_addr.ip());
		request_context.client_request_host = Some(host.clone());
		request_context.client_request_method = Some(method.to_string());
		request_context.client_request_path = Some(req.uri().path().to_string());
		request_context.client_request_protocol = Some(format!("{:?}", req.version()));
		request_context.client_request_scheme =
			Some(req.uri().scheme_str().unwrap_or("http").to_string());
		request_context.client_request_uri = Some(path.clone());
		request_context.client_src_port = Some(self.remote_addr.port());

		if let Some(referer) = req
			.headers()
			.get(hyper::header::REFERER)
			.and_then(|h| h.to_str().ok())
		{
			request_context.client_request_referer = Some(referer.to_string());
		}

		if let Some(ua) = &user_agent {
			request_context.client_request_user_agent = Some(ua.clone());
		}

		if let Some(requested_with) = req
			.headers()
			.get("x-requested-with")
			.and_then(|h| h.to_str().ok())
		{
			request_context.client_x_requested_with = Some(requested_with.to_string());
		}

		// TLS information would be set here if available (for HTTPS connections)
		// This requires TLS connection introspection and is marked for future enhancement

		// Debug log request information with structured fields (Apache-like access log)
		debug!(
			request_id = %request_context.request_id,
			method = %method,
			path = %path,
			host = %host,
			remote_addr = %self.remote_addr,
			uri = %uri_string,
			protocol = ?self.state.port_type,
			user_agent = ?user_agent,
			"Request received"
		);

		// Process the request
		let result = self.handle_request(req, &mut request_context).await;

		match &result {
			Ok(response) => {
				let status = response.status().as_u16();
				let content_length = response
					.headers()
					.get(hyper::header::CONTENT_LENGTH)
					.and_then(|h| h.to_str().ok())
					.and_then(|s| s.parse::<usize>().ok())
					.unwrap_or(0);

				// Log information about the completed request
				debug!(
					request_id = %request_context.request_id,
					method = %method,
					path = %path,
					host = %host,
					remote_addr = %self.remote_addr,
					status = %status,
					content_length = %content_length,
					"Request completed"
				);
			}
			Err(e) => {
				// Log error information
				debug!(
					request_id = %request_context.request_id,
					method = %method,
					path = %path,
					host = %host,
					remote_addr = %self.remote_addr,
					error = %e,
					"Request failed"
				);
			}
		}

		result
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
	pub fn new(
		config: rivet_config::Config,
		routing_fn: RoutingFn,
		middleware_fn: MiddlewareFn,
		port_type: PortType,
		clickhouse_inserter: Option<clickhouse_inserter::ClickHouseInserterHandle>,
	) -> Self {
		let state = Arc::new(ProxyState::new(
			config,
			routing_fn,
			middleware_fn,
			port_type,
			clickhouse_inserter,
		));
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
