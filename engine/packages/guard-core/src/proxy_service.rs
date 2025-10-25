use anyhow::{Context, Result, bail};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, StatusCode, body::Incoming as BodyIncoming, header::HeaderName};
use hyper_tungstenite;
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use moka::future::Cache;
use rand;
use rivet_api_builder::{ErrorResponse, RawErrorResponse};
use rivet_api_builder::{RequestIds, X_RIVET_RAY_ID};
use rivet_error::{INTERNAL_ERROR, RivetError};
use rivet_metrics::KeyValue;
use rivet_util::Id;
use serde_json;
use std::{
	borrow::Cow,
	collections::HashMap as StdHashMap,
	net::SocketAddr,
	sync::Arc,
	time::{Duration, Instant},
};
use tokio::sync::Mutex;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::{
	client::IntoClientRequest,
	protocol::{CloseFrame, frame::coding::CloseCode},
};
use tracing::Instrument;
use url::Url;
use uuid::Uuid;

use crate::{
	WebSocketHandle, custom_serve::CustomServeTrait, errors, metrics,
	request_context::RequestContext,
};

pub const X_FORWARDED_FOR: HeaderName = HeaderName::from_static("x-forwarded-for");
pub const X_RIVET_ERROR: HeaderName = HeaderName::from_static("x-rivet-error");
const ROUTE_CACHE_TTL: Duration = Duration::from_secs(60 * 10); // 10 minutes
const PROXY_STATE_CACHE_TTL: Duration = Duration::from_secs(60 * 60); // 1 hour
const WEBSOCKET_CLOSE_LINGER: Duration = Duration::from_millis(100); // Keep TCP connection open briefly after WebSocket close

/// Response body type that can handle both streaming and buffered responses
#[derive(Debug)]
pub enum ResponseBody {
	/// Buffered response body
	Full(Full<Bytes>),
	/// Streaming response body
	Incoming(BodyIncoming),
}

impl http_body::Body for ResponseBody {
	type Data = Bytes;
	type Error = Box<dyn std::error::Error + Send + Sync>;

	fn poll_frame(
		self: std::pin::Pin<&mut Self>,
		cx: &mut std::task::Context<'_>,
	) -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
		match self.get_mut() {
			ResponseBody::Full(body) => {
				let pin = std::pin::Pin::new(body);
				match pin.poll_frame(cx) {
					std::task::Poll::Ready(Some(Ok(frame))) => {
						std::task::Poll::Ready(Some(Ok(frame)))
					}
					std::task::Poll::Ready(Some(Err(e))) => {
						std::task::Poll::Ready(Some(Err(Box::new(e))))
					}
					std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
					std::task::Poll::Pending => std::task::Poll::Pending,
				}
			}
			ResponseBody::Incoming(body) => {
				let pin = std::pin::Pin::new(body);
				match pin.poll_frame(cx) {
					std::task::Poll::Ready(Some(Ok(frame))) => {
						std::task::Poll::Ready(Some(Ok(frame)))
					}
					std::task::Poll::Ready(Some(Err(e))) => {
						std::task::Poll::Ready(Some(Err(Box::new(e))))
					}
					std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
					std::task::Poll::Pending => std::task::Poll::Pending,
				}
			}
		}
	}

	fn is_end_stream(&self) -> bool {
		match self {
			ResponseBody::Full(body) => body.is_end_stream(),
			ResponseBody::Incoming(body) => body.is_end_stream(),
		}
	}

	fn size_hint(&self) -> http_body::SizeHint {
		match self {
			ResponseBody::Full(body) => body.size_hint(),
			ResponseBody::Incoming(body) => body.size_hint(),
		}
	}
}

// Routing types
#[derive(Clone, Debug)]
pub struct RouteTarget {
	pub actor_id: Option<Id>,
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

#[derive(Clone)]
pub enum RoutingOutput {
	/// Return the data to route to.
	Route(RouteConfig),
	/// Return a custom response.
	Response(StructuredResponse),
	/// Return a custom serve handler.
	CustomServe(Arc<dyn CustomServeTrait>),
}

#[derive(Clone, Debug)]
pub struct StructuredResponse {
	pub status: StatusCode,
	pub message: Cow<'static, str>,
	pub docs: Option<Cow<'static, str>>,
}

impl StructuredResponse {
	pub fn build_response(&self) -> Result<Response<ResponseBody>> {
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
			.body(ResponseBody::Full(Full::new(bytes)))?;

		Ok(response)
	}
}

#[derive(Clone)]
enum ResolveRouteOutput {
	Target(RouteTarget),
	Response(StructuredResponse),
	CustomServe(Arc<dyn CustomServeTrait>),
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
			&'a hyper::HeaderMap,
		) -> futures::future::BoxFuture<'a, Result<RoutingOutput>>
		+ Send
		+ Sync,
>;

pub type CacheKeyFn = Arc<
	dyn for<'a> Fn(&'a str, &'a str, PortType, &'a hyper::HeaderMap) -> Result<u64> + Send + Sync,
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
	dyn for<'a> Fn(
			&'a Id,
			&'a hyper::HeaderMap,
		) -> futures::future::BoxFuture<'a, Result<MiddlewareResponse>>
		+ Send
		+ Sync,
>;

// Cache for routing results
struct RouteCache {
	cache: Cache<u64, RouteConfig>,
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
	async fn get(&self, key: &u64) -> Option<RouteConfig> {
		self.cache.get(key).await
	}

	#[tracing::instrument(skip_all)]
	async fn insert(&self, key: u64, result: RouteConfig) {
		self.cache.insert(key, result).await;

		metrics::ROUTE_CACHE_COUNT.record(self.cache.entry_count(), &[]);
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
	config: rivet_config::Config,
	routing_fn: RoutingFn,
	cache_key_fn: CacheKeyFn,
	middleware_fn: MiddlewareFn,
	route_cache: RouteCache,
	rate_limiters: Cache<(Id, std::net::IpAddr), Arc<Mutex<RateLimiter>>>,
	in_flight_counters: Cache<(Id, std::net::IpAddr), Arc<Mutex<InFlightCounter>>>,
	port_type: PortType,
	clickhouse_inserter: Option<clickhouse_inserter::ClickHouseInserterHandle>,
}

impl ProxyState {
	pub fn new(
		config: rivet_config::Config,
		routing_fn: RoutingFn,
		cache_key_fn: CacheKeyFn,
		middleware_fn: MiddlewareFn,
		port_type: PortType,
		clickhouse_inserter: Option<clickhouse_inserter::ClickHouseInserterHandle>,
	) -> Self {
		Self {
			config,
			routing_fn,
			cache_key_fn,
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
		headers: &hyper::HeaderMap,
		ignore_cache: bool,
	) -> Result<ResolveRouteOutput> {
		// Extract just the hostname, stripping the port if present
		let hostname_only = hostname.split(':').next().unwrap_or(hostname);

		tracing::debug!(
			hostname = %hostname_only,
			path = %path,
			port_type = ?port_type,
			"Resolving route for request"
		);

		let cache_key = (self.cache_key_fn)(hostname_only, &path, port_type.clone(), headers)?;

		// Check cache first
		if !ignore_cache {
			if let Some(result) = self.route_cache.get(&cache_key).await {
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

		let res = timeout(
			default_timeout,
			(self.routing_fn)(hostname_only, path, port_type, headers),
		)
		.await
		.map_err(|_| {
			errors::RequestTimeout {
				timeout_seconds: default_timeout.as_secs(),
			}
			.build()
		})?;

		match res? {
			RoutingOutput::Route(result) => {
				tracing::debug!(
					hostname = %hostname_only,
					path = %path,
					targets_count = result.targets.len(),
					timeout_secs = result.timeout.routing_timeout,
					"Received routing result"
				);

				// Cache the result
				self.route_cache.insert(cache_key, result.clone()).await;
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
						"Selected target for request"
					);
					Ok(ResolveRouteOutput::Target(target.clone()))
				} else {
					tracing::warn!(
						hostname = %hostname_only,
						path = %path,
						"No route targets available from result"
					);
					Err(errors::NoRouteTargets.build())
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
			RoutingOutput::CustomServe(handler) => {
				tracing::debug!(
					hostname = %hostname_only,
					path = %path,
					"Routing returned custom serve handler"
				);
				Ok(ResolveRouteOutput::CustomServe(handler))
			}
		}
	}

	#[tracing::instrument(skip_all)]
	async fn get_middleware_config(
		&self,
		actor_id: &Id,
		headers: &hyper::HeaderMap,
	) -> Result<MiddlewareConfig> {
		// Call the middleware function with a timeout
		let default_timeout = Duration::from_secs(5); // Default 5 seconds

		let middleware_result =
			timeout(default_timeout, (self.middleware_fn)(actor_id, headers)).await;

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
		actor_id: &Option<Id>,
		headers: &hyper::HeaderMap,
	) -> Result<bool> {
		let Some(actor_id) = *actor_id else {
			// No rate limiting when actor_id is None
			return Ok(true);
		};

		// Get actor-specific middleware config
		let middleware_config = self.get_middleware_config(&actor_id, headers).await?;

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
			metrics::RATE_LIMITER_COUNT.record(self.rate_limiters.entry_count(), &[]);
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
		actor_id: &Option<Id>,
		headers: &hyper::HeaderMap,
	) -> Result<bool> {
		let Some(actor_id) = *actor_id else {
			// No in-flight limiting when actor_id is None
			return Ok(true);
		};

		// Get actor-specific middleware config
		let middleware_config = self.get_middleware_config(&actor_id, headers).await?;

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
				metrics::IN_FLIGHT_COUNTER_COUNT.record(self.in_flight_counters.entry_count(), &[]);
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
	async fn release_in_flight(&self, ip_addr: std::net::IpAddr, actor_id: &Option<Id>) {
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
		start_time: Instant,
		request_context: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
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

		// Set request body size in analytics (will be updated with actual size later)

		let target_res = self
			.state
			.resolve_route(
				host,
				&path,
				self.state.port_type.clone(),
				req.headers(),
				false,
			)
			.await;

		let duration_secs = start_time.elapsed().as_secs_f64();
		metrics::RESOLVE_ROUTE_DURATION.record(duration_secs, &[]);

		// Resolve target
		let target = target_res?;
		if let ResolveRouteOutput::Response(response) = &target {
			// Return the custom response
			return response.build_response();
		}

		let actor_id = if let ResolveRouteOutput::Target(target) = &target {
			target.actor_id
		} else {
			None
		};

		// Extract IP address from remote_addr
		let client_ip = self.remote_addr.ip();

		// Apply rate limiting
		if !self
			.state
			.check_rate_limit(client_ip, &actor_id, req.headers())
			.await?
		{
			return Err(errors::RateLimit.build());
		}

		// Check in-flight limit
		if !self
			.state
			.acquire_in_flight(client_ip, &actor_id, req.headers())
			.await?
		{
			return Err(errors::RateLimit.build());
		}

		// Increment metrics
		metrics::PROXY_REQUEST_PENDING.add(1, &[]);
		metrics::PROXY_REQUEST_TOTAL.add(1, &[]);

		// Prepare to release in-flight counter when done
		let state_clone = self.state.clone();
		crate::defer! {
			tokio::spawn(async move {
				state_clone.release_in_flight(client_ip, &actor_id).await;
			}.instrument(tracing::info_span!("release_in_flight_task")));
		}

		// Update request context with target info
		if let Some(actor_id) = actor_id {
			request_context.service_actor_id = Some(actor_id);
		}

		let res = if hyper_tungstenite::is_upgrade_request(&req) {
			self.handle_websocket_upgrade(req, target, request_context)
				.await
		} else {
			self.handle_http_request(req, target, request_context).await
		};

		let status = match &res {
			Ok(resp) => resp.status().as_u16().to_string(),
			Err(_) => "error".to_string(),
		};

		// Record metrics
		let duration_secs = start_time.elapsed().as_secs_f64();
		metrics::PROXY_REQUEST_DURATION
			.record(duration_secs, &[KeyValue::new("status", status.clone())]);

		metrics::PROXY_REQUEST_PENDING.add(-1, &[]);

		res
	}

	#[tracing::instrument(skip_all)]
	async fn handle_http_request(
		&self,
		req: Request<BodyIncoming>,
		resolved_route: ResolveRouteOutput,
		request_context: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
		// Get middleware config for this actor if it exists
		let middleware_config = if let ResolveRouteOutput::Target(target) = &resolved_route
			&& let Some(actor_id) = &target.actor_id
		{
			self.state
				.get_middleware_config(actor_id, req.headers())
				.await?
		} else {
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

		// Set up retry with backoff from middleware config
		let max_attempts = middleware_config.retry.max_attempts;
		let initial_interval = middleware_config.retry.initial_interval;
		let timeout_duration = Duration::from_secs(middleware_config.timeout.request_timeout);

		match resolved_route {
			ResolveRouteOutput::Target(mut target) => {
				// Set service IP from target
				if let Ok(target_ip) =
					format!("{}:{}", target.host, target.port).parse::<std::net::SocketAddr>()
				{
					request_context.service_ip = Some(target_ip.ip());
				}

				// Read the request body before proceeding with retries
				let (req_parts, body) = req.into_parts();
				let req_body = match http_body_util::BodyExt::collect(body).await {
					Ok(collected) => collected.to_bytes(),
					Err(err) => {
						tracing::debug!(?err, "Failed to read request body");
						Bytes::new()
					}
				};

				// Set actual request body size in analytics
				request_context.client_request_body_bytes = Some(req_body.len() as u64);

				// Use a value-returning loop to handle both errors and successful responses
				let mut attempts = 0;
				while attempts < max_attempts {
					attempts += 1;

					// Use the common function to build request parts
					let builder = self
						.proxied_request_builder(&req_parts, &target)
						.map_err(|err| errors::HttpRequestBuildFailed(err.to_string()).build())?;

					// Create the final request with body
					let proxied_req = builder
						.body(Full::<Bytes>::new(req_body.clone()))
						.map_err(|err| errors::RequestBuildError(err.to_string()).build())?;

					// Send the request with timeout
					let res = timeout(timeout_duration, self.client.request(proxied_req))
						.await
						.map_err(|_| {
							errors::RequestTimeout {
								timeout_seconds: timeout_duration.as_secs(),
							}
							.build()
						})?;

					match res {
						Ok(resp) => {
							// Check if this is a retryable response
							if should_retry(resp.status(), resp.headers()) {
								// Request connect error, might retry
								tracing::debug!(
									"Request attempt {attempts} failed (service unavailable)"
								);

								// Use backoff and continue
								let backoff = Self::calculate_backoff(attempts, initial_interval);
								tokio::time::sleep(backoff).await;

								// Resolve target again, this time ignoring cache. This makes sure
								// we always re-fetch the route on error
								let ResolveRouteOutput::Target(new_target) = self
									.state
									.resolve_route(
										&host,
										&path,
										self.state.port_type.clone(),
										&req_parts.headers,
										true,
									)
									.await?
								else {
									bail!("resolved route does not match Target");
								};
								target = new_target;

								continue;
							}

							let (parts, body) = resp.into_parts();

							// Check if this is a streaming response by examining headers
							// let is_streaming = parts.headers.get("content-type")
							// 	.and_then(|ct| ct.to_str().ok())
							// 	.map(|ct| ct.contains("text/event-stream") || ct.contains("application/stream"))
							// 	.unwrap_or(false);
							let is_streaming = true;

							if is_streaming {
								// For streaming responses, pass through the body without buffering
								tracing::debug!("Detected streaming response, preserving stream");

								// We can't easily calculate response size for streaming, so set it to None
								request_context.guard_response_body_bytes = None;

								let streaming_body = ResponseBody::Incoming(body);
								return Ok(Response::from_parts(parts, streaming_body));
							} else {
								// For non-streaming responses, buffer as before
								let body_bytes = match BodyExt::collect(body).await {
									Ok(collected) => collected.to_bytes(),
									Err(_) => Bytes::new(),
								};

								// Set actual response body size in analytics
								request_context.guard_response_body_bytes =
									Some(body_bytes.len() as u64);

								let full_body = ResponseBody::Full(Full::new(body_bytes));
								return Ok(Response::from_parts(parts, full_body));
							}
						}
						Err(err) => {
							if !err.is_connect() || attempts >= max_attempts {
								tracing::error!(
									?err,
									?target,
									"Request error after {} attempts",
									attempts
								);

								return Err(errors::UpstreamError(format!(
									"Failed to connect to runner: {err}. Make sure your runners are healthy."
								))
								.build());
							} else {
								// Request connect error, might retry
								tracing::debug!(?err, "Request attempt {attempts} failed");

								// Use backoff and continue
								let backoff = Self::calculate_backoff(attempts, initial_interval);
								tokio::time::sleep(backoff).await;

								// Resolve target again, this time ignoring cache. This makes sure
								// we always re-fetch the route on error
								let ResolveRouteOutput::Target(new_target) = self
									.state
									.resolve_route(
										&host,
										&path,
										self.state.port_type.clone(),
										&req_parts.headers,
										true,
									)
									.await?
								else {
									bail!("resolved route does not match Target");
								};
								target = new_target;

								continue;
							}
						}
					}
				}

				// If we get here, all attempts failed
				return Err(errors::RetryAttemptsExceeded {
					attempts: max_attempts,
				}
				.build());
			}
			ResolveRouteOutput::Response(_) => {
				unreachable!()
			}
			ResolveRouteOutput::CustomServe(mut handler) => {
				let req_headers = req.headers().clone();

				// Collect request body
				let (req_parts, body) = req.into_parts();
				let collected_body = match http_body_util::BodyExt::collect(body).await {
					Ok(collected) => collected.to_bytes(),
					Err(err) => {
						tracing::debug!(?err, "Failed to read request body");
						Bytes::new()
					}
				};
				let req_collected = hyper::Request::from_parts(
					req_parts,
					Full::<Bytes>::new(collected_body.clone()),
				);

				// Attempt request
				let mut attempts = 0;
				while attempts < max_attempts {
					attempts += 1;

					let resp = handler
						.handle_request(req_collected.clone(), request_context)
						.await?;
					if should_retry(resp.status(), resp.headers()) {
						// Request connect error, might retry
						tracing::debug!("Request attempt {attempts} failed (service unavailable)");

						// Use backoff and continue
						let backoff = Self::calculate_backoff(attempts, initial_interval);
						tokio::time::sleep(backoff).await;

						// Refresh route (ignore cache) so subsequent requests can hit new target
						let ResolveRouteOutput::CustomServe(new_handler) = self
							.state
							.resolve_route(
								&host,
								&path,
								self.state.port_type.clone(),
								&req_headers,
								true,
							)
							.await?
						else {
							bail!("resolved route does not match CustomServe");
						};
						handler = new_handler;

						continue;
					}

					return Ok(resp);
				}

				// If we get here, all attempts failed
				return Err(errors::RetryAttemptsExceeded {
					attempts: max_attempts,
				}
				.build());
			}
		}
	}

	fn proxied_request_builder(
		&self,
		req_parts: &hyper::http::request::Parts,
		target: &RouteTarget,
	) -> Result<hyper::http::request::Builder> {
		let scheme = if target.port == 443 { "https" } else { "http" };

		// Bracket raw IPv6 hosts
		let host = if target.host.contains(':') && !target.host.starts_with('[') {
			format!("[{}]", target.host)
		} else {
			target.host.clone()
		};

		// Ensure path starts with a leading slash
		let path = if target.path.starts_with('/') {
			target.path.clone()
		} else {
			format!("/{}", target.path)
		};

		let url = Url::parse(&format!("{scheme}://{host}:{}{}", target.port, path))
			.context("invalid scheme/host/port when building URL")?;

		// Build the proxied request
		let mut builder = hyper::Request::builder()
			.method(req_parts.method.clone())
			.uri(url.to_string());

		// Add proxy headers
		{
			let headers = builder
				.headers_mut()
				.expect("request builder unexpectedly in error state");
			add_proxy_headers_with_addr(headers, &req_parts.headers, self.remote_addr)?;
		}

		Ok(builder)
	}

	#[tracing::instrument(skip_all)]
	async fn handle_websocket_upgrade(
		&self,
		req: Request<BodyIncoming>,
		target: ResolveRouteOutput,
		request_context: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
		// Get actor and server IDs for metrics and middleware
		let actor_id = match &target {
			ResolveRouteOutput::Target(target) => target.actor_id,
			_ => None,
		};

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

		// Capture headers before request is consumed
		let req_headers = req.headers().clone();
		let ray_id = req.extensions().get::<RequestIds>().map(|x| x.ray_id);

		// Get middleware config for this actor if it exists
		let middleware_config = match &actor_id {
			Some(actor_id) => {
				self.state
					.get_middleware_config(actor_id, &req_headers)
					.await?
			}
			None => {
				// Default middleware config for targets without actor_id
				tracing::debug!("Using default middleware config (no actor_id)");
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
		tracing::debug!("WebSocket upgrade request headers:");
		for (name, value) in req.headers() {
			if let Ok(val) = value.to_str() {
				tracing::debug!("  {}: {}", name, val);
			}
		}

		// Handle WebSocket upgrade properly with hyper_tungstenite
		tracing::debug!(%req_path, "Upgrading client connection to WebSocket");
		let (client_response, client_ws) = match hyper_tungstenite::upgrade(req, None) {
			Ok(x) => {
				tracing::debug!("Client WebSocket upgrade successful");
				x
			}
			Err(err) => {
				tracing::error!(?err, "Failed to upgrade client WebSocket");
				return Err(errors::ConnectionError {
					error_message: format!("Failed to upgrade client WebSocket: {}", err),
					remote_addr: self.remote_addr.to_string(),
				}
				.build());
			}
		};

		// Log response status and headers
		tracing::debug!(
			"Client upgrade response status: {}",
			client_response.status()
		);
		for (name, value) in client_response.headers() {
			if let Ok(val) = value.to_str() {
				tracing::debug!("Client upgrade response header - {}: {}", name, val);
			}
		}

		// Clone needed values for the spawned task
		let state = self.state.clone();
		let remote_addr = self.remote_addr;

		// Spawn a new task to handle the WebSocket bidirectional communication
		match target {
			ResolveRouteOutput::Target(mut target) => {
				tracing::debug!("Spawning task to handle WebSocket communication");
				tokio::spawn(
					async move {
						// Set up a timeout for the entire operation
						let timeout_duration = Duration::from_secs(30); // 30 seconds timeout
						tracing::debug!(
							"WebSocket proxy task started with {}s timeout",
							timeout_duration.as_secs()
						);

						// Use retry logic to connect to the upstream WebSocket server
						let mut attempts = 0;
						let mut upstream_ws = None;

						// First, wait for the client WebSocket to be ready (do this first to avoid race conditions)
						tracing::debug!("Waiting for client WebSocket to be ready...");
						let mut client_ws =
							match tokio::time::timeout(timeout_duration, client_ws).await {
								Ok(Ok(ws)) => {
									tracing::debug!("Client WebSocket is ready");
									ws
								}
								Ok(Err(err)) => {
									tracing::error!(?err, "Failed to get client WebSocket");
									return;
								}
								Err(_) => {
									tracing::error!(
										"Timeout waiting for client WebSocket to be ready after {}s",
										timeout_duration.as_secs()
									);
									return;
								}
							};

						// Now attempt to connect to the upstream server
						tracing::debug!("Attempting connect to upstream WebSocket");
						while attempts < max_attempts {
							attempts += 1;

							// Build the WebSocket URL using the url crate to properly handle IPv6 addresses
							let mut ws_url = match Url::parse("ws://example.com") {
								Ok(url) => url,
								Err(err) => {
									tracing::error!(?err, "Failed to create base WebSocket URL");
									return;
								}
							};

							// Wrap IPv6 addresses in brackets if not already wrapped
							let host = if target.host.contains(':') && !target.host.starts_with('[')
							{
								format!("[{}]", target.host)
							} else {
								target.host.clone()
							};

							if let Err(err) = ws_url.set_host(Some(&host)) {
								tracing::error!(?err, ?host, "Failed to set WebSocket host");
								return;
							}
							if let Err(err) = ws_url.set_port(Some(target.port)) {
								tracing::error!(?err, "Failed to set WebSocket port");
								return;
							}

							// Split path and query string
							if let Some(query_pos) = target.path.find('?') {
								let (path, query) = target.path.split_at(query_pos);
								ws_url.set_path(path);
								// Remove the leading '?' from query
								ws_url.set_query(Some(&query[1..]));
							} else {
								ws_url.set_path(&target.path);
								ws_url.set_query(None);
							}

							let target_url = ws_url.to_string();

							tracing::debug!(
								"WebSocket request attempt {}/{} to {}",
								attempts,
								max_attempts,
								target_url
							);

							// Build the websocket request with headers
							let mut ws_request = match target_url.into_client_request() {
								Ok(req) => req,
								Err(err) => {
									tracing::error!(?err, "Failed to create websocket request");
									return;
								}
							};

							// Add proxy headers to the websocket request
							if let Err(err) = add_proxy_headers_with_addr(
								ws_request.headers_mut(),
								&req_headers,
								remote_addr,
							) {
								tracing::error!(
									?err,
									"Failed to add proxy headers to websocket request"
								);
								return;
							}

							match tokio::time::timeout(
								Duration::from_secs(5), // 5 second timeout per connection attempt
								tokio_tungstenite::connect_async(ws_request),
							)
							.await
							{
								Ok(Ok((ws_stream, resp))) => {
									tracing::debug!(
										"Successfully connected to upstream WebSocket server"
									);
									tracing::debug!(
										"Upstream connection response status: {:?}",
										resp.status()
									);

									// Log headers for debugging
									for (name, value) in resp.headers() {
										if let Ok(val) = value.to_str() {
											tracing::debug!(
												"Upstream response header - {}: {}",
												name,
												val
											);
										}
									}

									upstream_ws = Some(ws_stream);
									break;
								}
								Ok(Err(err)) => {
									tracing::debug!(
										?err,
										"WebSocket request attempt {} failed",
										attempts
									);
								}
								Err(_) => {
									tracing::debug!(
										"WebSocket request attempt {} timed out after 5s",
										attempts
									);
								}
							}

							// Check if we've reached max attempts
							if attempts >= max_attempts {
								tracing::debug!(
									"All {} WebSocket connection attempts failed",
									max_attempts
								);

								// Send a close message to the client since we can't connect to upstream
								tracing::debug!(
									"Sending close message to client due to upstream connection failure"
								);
								let (mut client_sink, _) = client_ws.split();
								match client_sink
									.send(to_hyper_close(Some(err_to_close_frame(
										errors::RetryAttemptsExceeded { attempts }.build(),
										ray_id,
									))))
									.await
								{
									Ok(_) => {
										tracing::trace!("Successfully sent close message to client")
									}
									Err(err) => {
										tracing::error!(
											?err,
											"Failed to send close message to client"
										)
									}
								};

								match client_sink.flush().await {
									Ok(_) => {
										tracing::trace!(
											"Successfully flushed client sink after close"
										)
									}
									Err(err) => {
										tracing::error!(
											?err,
											"Failed to flush client sink after close"
										)
									}
								};

								return;
							}

							// Use backoff for the next attempt
							let backoff = Self::calculate_backoff(attempts, initial_interval);
							tracing::debug!(
								"Waiting for {:?} before next connection attempt",
								backoff
							);

							tokio::time::sleep(backoff).await;

							// Resolve target again, this time ignoring cache. This makes sure
							// we always re-fetch the route on error
							let new_target = state
								.resolve_route(
									&req_host,
									&req_path,
									state.port_type.clone(),
									&req_headers,
									true,
								)
								.await;

							match new_target {
								Ok(ResolveRouteOutput::Target(new_target)) => {
									target = new_target;
								}
								Ok(ResolveRouteOutput::Response(response)) => {
									tracing::debug!(
										status=?response.status,
										message=?response.message,
										docs=?response.docs,
										"got response instead of websocket target",
									);

									// Close the WebSocket connection with the response message
									let _ = client_ws
										.close(Some(str_to_close_frame(response.message.as_ref())))
										.await;
									return;
								}
								Ok(ResolveRouteOutput::CustomServe(_)) => {
									let _ = client_ws
										.close(Some(err_to_close_frame(
											errors::WebSocketTargetChanged.build(),
											ray_id,
										)))
										.await;
									return;
								}
								Err(err) => {
									tracing::error!(?err, "Routing error");
								}
							}
						}

						// If we couldn't connect to the upstream server, exit the task
						let upstream_ws = match upstream_ws {
							Some(ws) => {
								tracing::debug!(
									"Successfully established upstream WebSocket connection"
								);
								ws
							}
							Option::None => {
								tracing::error!(
									"Failed to establish upstream WebSocket connection (unexpected)"
								);
								return; // Should never happen due to checks above, but just in case
							}
						};

						// Now set up bidirectional communication between the client and upstream WebSockets
						tracing::debug!("Setting up bidirectional WebSocket proxying");
						let (client_sink, client_stream) = client_ws.split();
						let (upstream_sink, upstream_stream) = upstream_ws.split();

						// Create channels for coordinating shutdown between client and upstream
						let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

						// Manually forward messages from client to upstream server with shutdown coordination
						let client_to_upstream = async {
							tracing::debug!("Starting client-to-upstream forwarder");
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
													tracing::debug!("Client-to-upstream forwarder shutting down due to signal");
													break;
												}
											},
											Err(err) => {
												// Channel closed
												tracing::debug!(?err, "Client-to-upstream shutdown channel closed");
												break;
											}
										}
									}

									// Process next message from client
									msg_result = stream.next() => {
										match msg_result {
											Some(Ok(client_msg)) => {
												// Convert from hyper_tungstenite::Message to tokio_tungstenite::Message
												let upstream_msg = match client_msg {
													hyper_tungstenite::tungstenite::Message::Text(text) => {
														tokio_tungstenite::tungstenite::Message::Text(text)
													},
													hyper_tungstenite::tungstenite::Message::Binary(data) => {
														tokio_tungstenite::tungstenite::Message::Binary(data)
													},
													hyper_tungstenite::tungstenite::Message::Ping(data) => {
														tokio_tungstenite::tungstenite::Message::Ping(data)
													},
													hyper_tungstenite::tungstenite::Message::Pong(data) => {
														tokio_tungstenite::tungstenite::Message::Pong(data)
													},
													hyper_tungstenite::tungstenite::Message::Close(frame) => {
														// Signal shutdown to other direction
														let _ = shutdown_tx.send(true);

														to_hyper_close(frame)
													},
													hyper_tungstenite::tungstenite::Message::Frame(_) => {
														// Skip frames - they're an implementation detail
														continue;
													},
												};

												// Send the message with a timeout
												tracing::trace!("Sending message to upstream server");
												let send_result = tokio::time::timeout(
													Duration::from_secs(5),
													sink.send(upstream_msg)
												).await;

												match send_result {
													Ok(Ok(_)) => {
														tracing::trace!("Message sent to upstream successfully");
														// Flush the sink with a timeout
														tracing::trace!("Flushing upstream sink");
														let flush_result = tokio::time::timeout(
															Duration::from_secs(2),
															sink.flush()
														).await;

														if let Err(_) = flush_result {
															tracing::trace!("Timeout flushing upstream sink");
															let _ = shutdown_tx.send(true);
															break;
														} else if let Ok(Err(err)) = flush_result {
															tracing::trace!(?err, "Error flushing upstream sink");
															let _ = shutdown_tx.send(true);
															break;
														} else {
															tracing::trace!("Upstream sink flushed successfully");
														}
													},
													Ok(Err(err)) => {
														tracing::trace!(?err, "Error sending message to upstream");
														let _ = shutdown_tx.send(true);
														break;
													},
													Err(_) => {
														tracing::trace!("Timeout sending message to upstream after 5s");
														let _ = shutdown_tx.send(true);
														break;
													}
												}
											},
											Some(Err(err)) => {
												// Error receiving message from client
												tracing::trace!(?err, "Error receiving message from client");
												tracing::trace!(?err, "Error details");
												// Signal shutdown to other direction
												let _ = shutdown_tx.send(true);
												break;
											},
											None => {
												// End of stream
												tracing::trace!("Client WebSocket stream ended");
												// Signal shutdown to other direction
												let _ = shutdown_tx.send(true);
												break;
											}
										}
									}
								}
							}

							// Try to send a close frame - ignore errors as the connection might already be closed
							tracing::trace!("Attempting to send close message to upstream");
							match sink
								.send(tokio_tungstenite::tungstenite::Message::Close(None))
								.await
							{
								Ok(_) => {
									tracing::trace!("Close message sent to upstream successfully")
								}
								Err(err) => {
									tracing::trace!(
										?err,
										"Failed to send close message to upstream"
									)
								}
							};

							match sink.flush().await {
								Ok(_) => {
									tracing::trace!(
										"Upstream sink flushed successfully after close"
									)
								}
								Err(err) => {
									tracing::trace!(
										?err,
										"Failed to flush upstream sink after close"
									)
								}
							};

							tracing::debug!("Client-to-upstream task completed");
						};

						// Manually forward messages from upstream server to client with shutdown coordination
						let upstream_to_client = async {
							tracing::debug!("Starting upstream-to-client forwarder");
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
													tracing::debug!("Upstream-to-client forwarder shutting down due to signal");
													break;
												}
											},
											Err(err) => {
												// Channel closed
												tracing::debug!(?err, "Upstream-to-client shutdown channel closed");
												break;
											}
										}
									}

									// Process next message from upstream
									msg_result = stream.next() => {
										match msg_result {
											Some(Ok(upstream_msg)) => {
												// Convert from tokio_tungstenite::Message to hyper_tungstenite::Message
												let client_msg = match upstream_msg {
													tokio_tungstenite::tungstenite::Message::Text(text) => {
														hyper_tungstenite::tungstenite::Message::Text(text)
													},
													tokio_tungstenite::tungstenite::Message::Binary(data) => {
														hyper_tungstenite::tungstenite::Message::Binary(data)
													},
													tokio_tungstenite::tungstenite::Message::Ping(data) => {
														hyper_tungstenite::tungstenite::Message::Ping(data)
													},
													tokio_tungstenite::tungstenite::Message::Pong(data) => {
														hyper_tungstenite::tungstenite::Message::Pong(data)
													},
													tokio_tungstenite::tungstenite::Message::Close(frame) => {
														// Signal shutdown to other direction
														let _ = shutdown_tx.send(true);

														to_hyper_close(frame)
													},
													tokio_tungstenite::tungstenite::Message::Frame(_) => {
														// Skip frames - they're an implementation detail
														continue;
													},
												};

												// Send the message with a timeout
												tracing::trace!("Sending message to client");
												let send_result = tokio::time::timeout(
													Duration::from_secs(5),
													sink.send(client_msg)
												).await;

												match send_result {
													Ok(Ok(_)) => {
														tracing::trace!("Message sent to client successfully");
														// Flush the sink with a timeout
														tracing::trace!("Flushing client sink");
														let flush_result = tokio::time::timeout(
															Duration::from_secs(2),
															sink.flush()
														).await;

														if let Err(_) = flush_result {
															tracing::trace!("Timeout flushing client sink");
															let _ = shutdown_tx.send(true);
															break;
														} else if let Ok(Err(err)) = flush_result {
															tracing::trace!(?err, "Error flushing client sink");
															let _ = shutdown_tx.send(true);
															break;
														} else {
															tracing::trace!("Client sink flushed successfully");
														}
													},
													Ok(Err(err)) => {
														tracing::trace!(?err, "Error sending message to client");
														let _ = shutdown_tx.send(true);
														break;
													},
													Err(_) => {
														tracing::trace!("Timeout sending message to client after 5s");
														let _ = shutdown_tx.send(true);
														break;
													}
												}
											},
											Some(Err(err)) => {
												// Error receiving message from upstream
												tracing::trace!(?err, "Error receiving message from upstream");
												// Signal shutdown to other direction
												let _ = shutdown_tx.send(true);
												break;
											},
											None => {
												// End of stream
												tracing::trace!("Upstream WebSocket stream ended");
												// Signal shutdown to other direction
												let _ = shutdown_tx.send(true);
												break;
											}
										}
									}
								}
							}

							// Try to send a close frame - ignore errors as the connection might already be closed
							tracing::trace!("Attempting to send close message to client");
							match sink.send(to_hyper_close(None)).await {
								Ok(_) => {
									tracing::trace!("Close message sent to client successfully")
								}
								Err(err) => {
									tracing::trace!(?err, "Failed to send close message to client")
								}
							};

							match sink.flush().await {
								Ok(_) => {
									tracing::trace!("Client sink flushed successfully after close")
								}
								Err(err) => {
									tracing::trace!(?err, "Failed to flush client sink after close")
								}
							};

							tracing::trace!("Upstream-to-client task completed");
						};

						// Run both directions concurrently until either one completes or errors
						tracing::debug!("Starting bidirectional message forwarding");
						tokio::join!(client_to_upstream, upstream_to_client);
						tracing::debug!("Bidirectional message forwarding completed");
					}
					.instrument(tracing::info_span!("handle_ws_task_target")),
				);
			}
			ResolveRouteOutput::Response(_) => unreachable!(),
			ResolveRouteOutput::CustomServe(mut handlers) => {
				tracing::debug!(%req_path, "Spawning task to handle WebSocket communication");
				let mut request_context = request_context.clone();
				let req_headers = req_headers.clone();
				let req_path = req_path.clone();
				let req_host = req_host.clone();

				tokio::spawn(
					async move {
						let request_id = Uuid::new_v4();
						let mut attempts = 0u32;

						let ws_handle = WebSocketHandle::new(client_ws)
							.await
							.context("failed initiating websocket handle")?;

						loop {
							match handlers
								.handle_websocket(
									ws_handle.clone(),
									&req_headers,
									&req_path,
									&mut request_context,
									request_id,
								)
								.await
							{
								Ok(()) => {
									tracing::debug!("websocket handler complete, closing");

									// Send graceful close
									ws_handle
										.send(to_hyper_close(Some(CloseFrame {
											code: CloseCode::Normal,
											reason: "".into(),
										})))
										.await?;

									// Flush to ensure close frame is sent
									ws_handle.flush().await?;

									// Keep TCP connection open briefly to allow client to process close
									tokio::time::sleep(WEBSOCKET_CLOSE_LINGER).await;

									break;
								}
								Err(err) => {
									tracing::debug!(?err, "websocket handler error");

									attempts += 1;
									if attempts > max_attempts || !is_retryable_ws_error(&err) {
										tracing::debug!(?attempts, "WebSocket failed to reconnect");

										// Close WebSocket with error
										ws_handle
											.send(to_hyper_close(Some(err_to_close_frame(
												err, ray_id,
											))))
											.await?;

										// Flush to ensure close frame is sent
										ws_handle.flush().await?;

										// Keep TCP connection open briefly to allow client to process close
										tokio::time::sleep(WEBSOCKET_CLOSE_LINGER).await;

										break;
									} else {
										let backoff = ProxyService::calculate_backoff(
											attempts,
											initial_interval,
										);
										let backoff = Duration::from_millis(100);

										tracing::debug!(
											?backoff,
											"WebSocket attempt {attempts} failed (service unavailable)"
										);

										tokio::time::sleep(backoff).await;

										match state
											.resolve_route(
												&req_host,
												&req_path,
												state.port_type.clone(),
												&req_headers,
												true,
											)
											.await
										{
											Ok(ResolveRouteOutput::CustomServe(new_handlers)) => {
												handlers = new_handlers;
												continue;
											}
											Ok(ResolveRouteOutput::Response(response)) => {
												ws_handle
													.send(to_hyper_close(Some(str_to_close_frame(
														response.message.as_ref(),
													))))
													.await?;

												// Flush to ensure close frame is sent
												ws_handle.flush().await?;

												// Keep TCP connection open briefly to allow client to process close
												tokio::time::sleep(WEBSOCKET_CLOSE_LINGER).await;
											}
											Ok(ResolveRouteOutput::Target(_)) => {
												ws_handle
													.send(to_hyper_close(Some(err_to_close_frame(
														errors::WebSocketTargetChanged.build(),
														ray_id,
													))))
													.await?;

												// Flush to ensure close frame is sent
												ws_handle.flush().await?;

												// Keep TCP connection open briefly to allow client to process close
												tokio::time::sleep(WEBSOCKET_CLOSE_LINGER).await;

												break;
											}
											Err(err) => {
												ws_handle
													.send(to_hyper_close(Some(err_to_close_frame(
														err, ray_id,
													))))
													.await?;

												// Flush to ensure close frame is sent
												ws_handle.flush().await?;

												// Keep TCP connection open briefly to allow client to process close
												tokio::time::sleep(WEBSOCKET_CLOSE_LINGER).await;

												break;
											}
										}
									}
								}
							}
						}

						anyhow::Ok(())
					}
					.instrument(tracing::info_span!("handle_ws_task_custom_serve")),
				);
			}
		}

		// Return the response that will upgrade the client connection
		// For proper WebSocket handshaking, we need to preserve the original response
		// structure but convert it to our expected return type without modifying its content
		tracing::debug!("Returning WebSocket upgrade response to client");
		// Extract the parts from the response but preserve all headers and status
		let (mut parts, _) = client_response.into_parts();

		// Add Sec-WebSocket-Protocol header to the response
		// Many WebSocket clients (e.g. node-ws & Cloudflare) require a protocol in the response
		parts.headers.insert(
			"sec-websocket-protocol",
			hyper::header::HeaderValue::from_static("rivet"),
		);

		// Create a new response with an empty body - WebSocket upgrades don't need a body
		Ok(Response::from_parts(
			parts,
			ResponseBody::Full(Full::<Bytes>::new(Bytes::new())),
		))
	}
}

impl ProxyService {
	// Process an individual request
	#[tracing::instrument(name = "guard_request", skip_all, fields(ray_id, req_id))]
	pub async fn process(&self, mut req: Request<BodyIncoming>) -> Result<Response<ResponseBody>> {
		let start_time = Instant::now();

		let request_ids = RequestIds::new(self.state.config.dc_label());
		req.extensions_mut().insert(request_ids);

		tracing::Span::current()
			.record("req_id", request_ids.req_id.to_string())
			.record("ray_id", request_ids.ray_id.to_string());

		// Create request context for analytics tracking
		let mut request_context =
			RequestContext::new(self.state.clickhouse_inserter.clone(), request_ids);

		// Extract request information for logging and analytics before consuming the request
		let incoming_ray_id = req
			.headers()
			.get(X_RIVET_RAY_ID)
			.and_then(|h| h.to_str().ok())
			.and_then(|id| Id::parse(id).ok());
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
		tracing::debug!(
			?incoming_ray_id,
			ray_id=?request_ids.ray_id,
			req_id=?request_ids.req_id,
			method = %method,
			path = %path,
			host = %host,
			remote_addr = %self.remote_addr,
			uri = %uri_string,
			protocol = ?self.state.port_type,
			user_agent = ?user_agent,
			"Request received"
		);

		let is_websocket = hyper_tungstenite::is_upgrade_request(&req);

		// Used for ws error proxying later
		let mut mock_req_builder = Request::builder()
			.method(req.method().clone())
			.uri(req.uri().clone())
			.version(req.version().clone());
		if let Some(headers) = mock_req_builder.headers_mut() {
			*headers = req.headers().clone();
		}
		if let Some(extensions) = mock_req_builder.extensions_mut() {
			*extensions = req.extensions().clone();
		}
		let mock_req = mock_req_builder.body(())?;

		// Process the request
		let mut res = match self
			.handle_request(req, start_time, &mut request_context)
			.await
		{
			Ok(res) => res,
			Err(err) => {
				// Log the error
				tracing::error!(?err, "Request failed");

				metrics::PROXY_REQUEST_ERROR
					.add(1, &[KeyValue::new("error_type", err.to_string())]);

				// If we receive an error during a websocket request, we attempt to open the websocket anyway
				// so we can send the error via websocket instead of http. Most websocket clients don't handle
				// HTTP errors in a meaningful way resulting in unhelpful errors for the user
				if is_websocket {
					tracing::debug!("Upgrading client connection to WebSocket for error proxy");
					match hyper_tungstenite::upgrade(mock_req, None) {
						Ok((client_response, client_ws)) => {
							tracing::debug!("Client WebSocket upgrade for error proxy successful");

							tokio::spawn(
								async move {
									let ws_handle = match WebSocketHandle::new(client_ws).await {
										Ok(ws_handle) => ws_handle,
										Err(err) => {
											tracing::debug!(
												?err,
												"failed initiating websocket handle for error proxy"
											);
											return;
										}
									};
									let frame = err_to_close_frame(err, Some(request_ids.ray_id));

									// Manual conversion to handle different tungstenite versions
									let code_num: u16 = frame.code.into();
									let reason = frame.reason.clone();

									if let Err(err) = ws_handle
										.send(tokio_tungstenite::tungstenite::Message::Close(Some(
											tokio_tungstenite::tungstenite::protocol::CloseFrame {
												code: code_num.into(),
												reason,
											},
										)))
										.await
									{
										tracing::debug!(
											?err,
											"failed sending websocket error proxy"
										);
									}
								}
								.instrument(
									tracing::info_span!("ws_error_proxy_task", ?request_ids.ray_id),
								),
							);

							// Return the response that will upgrade the client connection
							// For proper WebSocket handshaking, we need to preserve the original response
							// structure but convert it to our expected return type without modifying its content
							tracing::debug!(
								"Returning WebSocket upgrade response for error proxy to client"
							);
							// Extract the parts from the response but preserve all headers and status
							let (mut parts, _) = client_response.into_parts();

							// Add Sec-WebSocket-Protocol header to the response
							// Many WebSocket clients (e.g. node-ws & Cloudflare) require a protocol in the response
							parts.headers.insert(
								"sec-websocket-protocol",
								hyper::header::HeaderValue::from_static("rivet"),
							);

							// Create a new response with an empty body - WebSocket upgrades don't need a body
							Response::from_parts(
								parts,
								ResponseBody::Full(Full::<Bytes>::new(Bytes::new())),
							)
						}
						Err(err) => {
							tracing::error!(
								?err,
								"Failed to upgrade client WebSocket for error proxy"
							);

							err_into_response(
								errors::ConnectionError {
									error_message: format!(
										"Failed to upgrade client WebSocket for error proxy: {}",
										err
									),
									remote_addr: self.remote_addr.to_string(),
								}
								.build(),
							)?
						}
					}
				} else {
					err_into_response(err)?
				}
			}
		};

		if is_websocket && res.status() != StatusCode::SWITCHING_PROTOCOLS {
			tracing::debug!("returned non-101 response to websocket");
		}

		// Add ray_id to response headers
		if let Ok(ray_id_value) = request_ids.ray_id.to_string().parse() {
			if let Some(existing_ray_id_value) = res
				.headers()
				.get(X_RIVET_RAY_ID)
				.and_then(|h| h.to_str().ok())
			{
				if ray_id_value != existing_ray_id_value {
					tracing::warn!(
						expected_ray_id=%request_ids.ray_id,
						received_ray_id=%existing_ray_id_value,
						"downstream service set ray id header to a different value",
					);
				}
			}

			res.headers_mut().insert(X_RIVET_RAY_ID, ray_id_value);
		}

		let status = res.status().as_u16();

		// Update request context with response details
		request_context.guard_response_status = Some(status);
		request_context.service_response_status = Some(status);

		if let Some(content_type) = res
			.headers()
			.get(hyper::header::CONTENT_TYPE)
			.and_then(|h| h.to_str().ok())
		{
			request_context.guard_response_content_type = Some(content_type.to_string());
		}

		if let Some(expires) = res
			.headers()
			.get(hyper::header::EXPIRES)
			.and_then(|h| h.to_str().ok())
		{
			request_context.service_response_http_expires = Some(expires.to_string());
		}

		if let Some(last_modified) = res
			.headers()
			.get(hyper::header::LAST_MODIFIED)
			.and_then(|h| h.to_str().ok())
		{
			request_context.service_response_http_last_modified = Some(last_modified.to_string());
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

		let content_length = res
			.headers()
			.get(hyper::header::CONTENT_LENGTH)
			.and_then(|h| h.to_str().ok())
			.and_then(|s| s.parse::<usize>().ok())
			.unwrap_or(0);

		// Log information about the completed request
		tracing::debug!(
			?incoming_ray_id,
			ray_id=?request_ids.ray_id,
			req_id=?request_ids.req_id,
			method = %method,
			path = %path,
			host = %host,
			remote_addr = %self.remote_addr,
			status = %status,
			content_length = %content_length,
			"Request completed"
		);

		Ok(res)
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
		cache_key_fn: CacheKeyFn,
		middleware_fn: MiddlewareFn,
		port_type: PortType,
		clickhouse_inserter: Option<clickhouse_inserter::ClickHouseInserterHandle>,
	) -> Self {
		let state = Arc::new(ProxyState::new(
			config,
			routing_fn,
			cache_key_fn,
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

fn add_proxy_headers_with_addr(
	headers: &mut hyper::HeaderMap,
	original_headers: &hyper::HeaderMap,
	remote_addr: SocketAddr,
) -> Result<()> {
	// Copy headers except Host
	for (key, value) in original_headers.iter() {
		if key != hyper::header::HOST {
			headers.insert(key.clone(), value.clone());
		}
	}

	// Add X-Forwarded-For header
	if let Some(existing) = original_headers.get(X_FORWARDED_FOR) {
		if let Ok(forwarded) = existing.to_str() {
			if !forwarded.contains(&remote_addr.ip().to_string()) {
				headers.insert(
					X_FORWARDED_FOR,
					hyper::header::HeaderValue::from_str(&format!(
						"{}, {}",
						forwarded,
						remote_addr.ip()
					))?,
				);
			}
		}
	} else {
		headers.insert(
			X_FORWARDED_FOR,
			hyper::header::HeaderValue::from_str(&remote_addr.ip().to_string())?,
		);
	}

	Ok(())
}

fn err_into_response(err: anyhow::Error) -> Result<Response<ResponseBody>> {
	let (status, error_response) =
		if let Some(rivet_err) = err.chain().find_map(|x| x.downcast_ref::<RivetError>()) {
			let status = match (rivet_err.group(), rivet_err.code()) {
				("api", "not_found") => StatusCode::NOT_FOUND,
				("api", "unauthorized") => StatusCode::UNAUTHORIZED,
				("api", "forbidden") => StatusCode::FORBIDDEN,
				("guard", "rate_limit") => StatusCode::TOO_MANY_REQUESTS,
				("guard", "upstream_error") => StatusCode::BAD_GATEWAY,
				("guard", "routing_error") => StatusCode::BAD_GATEWAY,
				("guard", "request_timeout") => StatusCode::GATEWAY_TIMEOUT,
				("guard", "retry_attempts_exceeded") => StatusCode::BAD_GATEWAY,
				("guard", "actor_not_found") => StatusCode::NOT_FOUND,
				("guard", "actor_destroyed") => StatusCode::NOT_FOUND,
				("guard", "service_unavailable") => StatusCode::SERVICE_UNAVAILABLE,
				("guard", "actor_ready_timeout") => StatusCode::SERVICE_UNAVAILABLE,
				("guard", "no_route") => StatusCode::NOT_FOUND,
				_ => StatusCode::BAD_REQUEST,
			};

			(status, ErrorResponse::from(rivet_err))
		} else if let Some(raw_err) = err
			.chain()
			.find_map(|x| x.downcast_ref::<RawErrorResponse>())
		{
			(raw_err.0, raw_err.1.clone())
		} else {
			(
				StatusCode::INTERNAL_SERVER_ERROR,
				ErrorResponse::from(&RivetError {
					schema: &rivet_error::INTERNAL_ERROR,
					meta: None,
					message: None,
				}),
			)
		};

	let body_json = serde_json::to_vec(&error_response)?;
	let bytes = Bytes::from(body_json);

	Response::builder()
		.status(status)
		.header(hyper::header::CONTENT_TYPE, "application/json")
		.body(ResponseBody::Full(Full::new(bytes)))
		.map_err(Into::into)
}

// Determine if a response should trigger a retry: 503 + x-rivet-error
fn should_retry(status: StatusCode, headers: &hyper::HeaderMap) -> bool {
	status == StatusCode::SERVICE_UNAVAILABLE && headers.contains_key(X_RIVET_ERROR)
}

// Determine if a websocket error is retryable (e.g., transient UPS/tunnel issues)
fn is_retryable_ws_error(err: &anyhow::Error) -> bool {
	if let Some(rivet_err) = err.chain().find_map(|x| x.downcast_ref::<RivetError>()) {
		rivet_err.group() == "guard" && rivet_err.code() == "websocket_service_unavailable"
	} else {
		false
	}
}

fn str_to_close_frame(err: &str) -> CloseFrame {
	// NOTE: reason cannot be more than 123 bytes as per the WS protocol spec
	let reason = rivet_util::safe_slice(err, 0, 123).into();

	CloseFrame {
		code: CloseCode::Error,
		reason,
	}
}

fn err_to_close_frame(err: anyhow::Error, ray_id: Option<Id>) -> CloseFrame {
	let rivet_err = err
		.chain()
		.find_map(|x| x.downcast_ref::<RivetError>())
		.cloned()
		.unwrap_or_else(|| RivetError::from(&INTERNAL_ERROR));

	let code = match (rivet_err.group(), rivet_err.code()) {
		("ws", "connection_closed") | ("ws", "eviction") => CloseCode::Normal,
		_ => CloseCode::Error,
	};

	let reason = if let Some(ray_id) = ray_id {
		format!("{}.{}#{}", rivet_err.group(), rivet_err.code(), ray_id)
	} else {
		format!("{}.{}", rivet_err.group(), rivet_err.code())
	};

	// NOTE: reason cannot be more than 123 bytes as per the WS protocol
	let reason = rivet_util::safe_slice(&reason, 0, 123).into();

	CloseFrame { code, reason }
}

fn to_hyper_close(frame: Option<CloseFrame>) -> hyper_tungstenite::tungstenite::Message {
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
}
