use anyhow::{Result, bail};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{net::IpAddr, path::PathBuf};

pub const DEFAULT_WEBSOCKET_MAX_MESSAGE_SIZE: usize = 32 * 1024 * 1024;
pub const DEFAULT_WEBSOCKET_MAX_FRAME_SIZE: usize = 32 * 1024 * 1024;

const DEFAULT_PROXY_RETRY_MAX_ATTEMPTS: u32 = 7;
const DEFAULT_PROXY_RETRY_INITIAL_INTERVAL_MS: u64 = 150;
const DEFAULT_UPSTREAM_REQUEST_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_WEBSOCKET_SETUP_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_WEBSOCKET_CONNECT_ATTEMPT_TIMEOUT_MS: u64 = 5_000;
const DEFAULT_WEBSOCKET_SEND_TIMEOUT_MS: u64 = 5_000;
const DEFAULT_WEBSOCKET_FLUSH_TIMEOUT_MS: u64 = 2_000;
const DEFAULT_HTTP_CLIENT_POOL_IDLE_TIMEOUT_MS: u64 = 30_000;
const DEFAULT_ADMISSION_CLIENT_STATE_CACHE_CAPACITY: u64 = 10_000;
const DEFAULT_ADMISSION_CLIENT_STATE_CACHE_IDLE_TIMEOUT_MS: u64 = 60 * 60 * 1_000;

#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Guard {
	/// Host for HTTP traffic
	pub host: Option<IpAddr>,
	/// Port for HTTP traffic
	pub port: Option<u16>,
	/// Enables TCP_NODELAY on accepted Guard sockets.
	pub tcp_nodelay: Option<bool>,
	/// Enables the internal websocket health route for debug and latency testing. This is intended
	/// for websocket ping/pong verification and should remain disabled in normal deployments.
	pub enable_websocket_health_route: Option<bool>,
	/// TTL for cached route lookups in milliseconds.
	pub route_cache_ttl_ms: Option<u64>,
	/// Backstop timeout for route resolution in milliseconds. Primary timeout signals live
	/// inside each guard routing phase.
	pub route_timeout_ms: Option<u64>,
	/// Timeout for dispatching to each guard routing module in milliseconds.
	pub route_dispatch_timeout_ms: Option<u64>,
	/// Timeout for resolving api-public routes in milliseconds.
	pub route_api_public_timeout_ms: Option<u64>,
	/// Timeout for resolving compute routes in milliseconds.
	pub route_compute_timeout_ms: Option<u64>,
	/// Timeout for guard-owned route authorization checks in milliseconds.
	pub route_auth_check_timeout_ms: Option<u64>,
	/// Timeout for subscribing to pegboard actor routing events in milliseconds.
	pub route_pegboard_subscribe_timeout_ms: Option<u64>,
	/// Timeout for fetching pegboard actor routing state in milliseconds.
	pub route_pegboard_fetch_actor_timeout_ms: Option<u64>,
	/// Timeout for pegboard actor route authorization checks in milliseconds.
	pub route_pegboard_auth_check_timeout_ms: Option<u64>,
	/// Timeout for sending pegboard actor wake signals in milliseconds.
	pub route_pegboard_wake_signal_timeout_ms: Option<u64>,
	/// Timeout for resolving pegboard actor query routes in milliseconds.
	pub route_pegboard_resolve_query_timeout_ms: Option<u64>,
	/// Timeout for waiting for an actor to become ready in milliseconds.
	pub actor_ready_timeout_ms: Option<u64>,
	/// Timeout sent with actor force-wake requests in milliseconds.
	pub actor_force_wake_pending_timeout_ms: Option<i64>,

	/// Fixed-window request limit applied per client IP. Omit to disable request rate limiting.
	pub rate_limit: Option<GuardRateLimit>,
	/// Maximum concurrent requests applied per client IP. Omit to disable the in-flight limit.
	#[schemars(range(min = 1))]
	pub max_in_flight: Option<usize>,
	/// Maximum number of proxy attempts, including the initial attempt.
	#[schemars(range(min = 1))]
	pub proxy_retry_max_attempts: Option<u32>,
	/// Initial exponential retry backoff in milliseconds.
	#[schemars(range(min = 1))]
	pub proxy_retry_initial_interval_ms: Option<u64>,
	/// Timeout for receiving upstream HTTP response headers in milliseconds.
	#[schemars(range(min = 1))]
	pub upstream_request_timeout_ms: Option<u64>,
	/// Timeout for completing the client WebSocket upgrade in milliseconds.
	#[schemars(range(min = 1))]
	pub websocket_setup_timeout_ms: Option<u64>,
	/// Timeout for each upstream WebSocket connection attempt in milliseconds.
	#[schemars(range(min = 1))]
	pub websocket_connect_attempt_timeout_ms: Option<u64>,
	/// Timeout for forwarding a WebSocket message in milliseconds.
	#[schemars(range(min = 1))]
	pub websocket_send_timeout_ms: Option<u64>,
	/// Timeout for flushing forwarded WebSocket messages in milliseconds.
	#[schemars(range(min = 1))]
	pub websocket_flush_timeout_ms: Option<u64>,
	/// Idle timeout for pooled upstream HTTP connections in milliseconds.
	#[schemars(range(min = 1))]
	pub http_client_pool_idle_timeout_ms: Option<u64>,
	/// Maximum number of IP-keyed admission state entries when an admission limit is enabled.
	#[schemars(range(min = 1))]
	pub admission_client_state_cache_capacity: Option<u64>,
	/// Idle timeout for IP-keyed admission state in milliseconds.
	#[schemars(range(min = 1))]
	pub admission_client_state_cache_idle_timeout_ms: Option<u64>,

	/// Enable & configure HTTPS
	pub https: Option<Https>,

	/// Max HTTP request body size in bytes (first line of defense).
	pub http_max_request_body_size: Option<usize>,
	/// Max WebSocket message size in bytes.
	pub websocket_max_message_size: Option<usize>,
	/// Max WebSocket frame size in bytes.
	pub websocket_max_frame_size: Option<usize>,

	/// Enables W3C trace context propagation (extract from incoming requests, inject into
	/// upstream requests/websockets).
	pub trace_propagation: Option<bool>,
}

impl Guard {
	pub fn validate(&self) -> Result<()> {
		if let Some(rate_limit) = &self.rate_limit {
			if rate_limit.requests == 0 {
				bail!("guard.rate_limit.requests must be greater than 0");
			}
			if rate_limit.period_ms == 0 {
				bail!("guard.rate_limit.period_ms must be greater than 0");
			}
		}

		if self.max_in_flight == Some(0) {
			bail!("guard.max_in_flight must be greater than 0");
		}

		for (name, value) in [
			(
				"proxy_retry_max_attempts",
				self.proxy_retry_max_attempts.map(u64::from),
			),
			(
				"proxy_retry_initial_interval_ms",
				self.proxy_retry_initial_interval_ms,
			),
			(
				"upstream_request_timeout_ms",
				self.upstream_request_timeout_ms,
			),
			(
				"websocket_setup_timeout_ms",
				self.websocket_setup_timeout_ms,
			),
			(
				"websocket_connect_attempt_timeout_ms",
				self.websocket_connect_attempt_timeout_ms,
			),
			("websocket_send_timeout_ms", self.websocket_send_timeout_ms),
			(
				"websocket_flush_timeout_ms",
				self.websocket_flush_timeout_ms,
			),
			(
				"http_client_pool_idle_timeout_ms",
				self.http_client_pool_idle_timeout_ms,
			),
			(
				"admission_client_state_cache_capacity",
				self.admission_client_state_cache_capacity,
			),
			(
				"admission_client_state_cache_idle_timeout_ms",
				self.admission_client_state_cache_idle_timeout_ms,
			),
		] {
			if value == Some(0) {
				bail!("guard.{name} must be greater than 0");
			}
		}

		Ok(())
	}

	pub fn host(&self) -> IpAddr {
		self.host.unwrap_or(crate::defaults::hosts::GUARD)
	}

	pub fn port(&self) -> u16 {
		self.port.unwrap_or(crate::defaults::ports::GUARD)
	}

	pub fn tcp_nodelay(&self) -> bool {
		self.tcp_nodelay.unwrap_or(false)
	}

	pub fn enable_websocket_health_route(&self) -> bool {
		self.enable_websocket_health_route.unwrap_or(false)
	}

	pub fn route_cache_ttl(&self) -> std::time::Duration {
		std::time::Duration::from_millis(self.route_cache_ttl_ms.unwrap_or(60 * 10 * 1000))
	}

	pub fn route_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(self.route_timeout_ms.unwrap_or(60_000))
	}

	pub fn route_dispatch_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(self.route_dispatch_timeout_ms.unwrap_or(55_000))
	}

	pub fn route_api_public_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(self.route_api_public_timeout_ms.unwrap_or(5_000))
	}

	pub fn route_compute_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(self.route_compute_timeout_ms.unwrap_or(10_000))
	}

	pub fn route_auth_check_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(self.route_auth_check_timeout_ms.unwrap_or(5_000))
	}

	pub fn route_pegboard_subscribe_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(self.route_pegboard_subscribe_timeout_ms.unwrap_or(2_000))
	}

	pub fn route_pegboard_fetch_actor_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(
			self.route_pegboard_fetch_actor_timeout_ms.unwrap_or(5_000),
		)
	}

	pub fn route_pegboard_auth_check_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(self.route_pegboard_auth_check_timeout_ms.unwrap_or(5_000))
	}

	pub fn route_pegboard_wake_signal_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(
			self.route_pegboard_wake_signal_timeout_ms.unwrap_or(5_000),
		)
	}

	pub fn route_pegboard_resolve_query_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(
			self.route_pegboard_resolve_query_timeout_ms
				.unwrap_or(15_000),
		)
	}

	pub fn actor_ready_timeout(&self) -> std::time::Duration {
		// Keep this high because serverless cold starts can take 10 to 20 seconds.
		// If this grows again, verify route_timeout_ms and route_dispatch_timeout_ms leave enough outer budget.
		std::time::Duration::from_millis(self.actor_ready_timeout_ms.unwrap_or(30_000))
	}

	pub fn actor_force_wake_pending_timeout(&self) -> i64 {
		self.actor_force_wake_pending_timeout_ms
			.unwrap_or(60 * 1000)
	}

	pub fn rate_limit(&self) -> Option<&GuardRateLimit> {
		self.rate_limit.as_ref()
	}

	pub fn max_in_flight(&self) -> Option<usize> {
		self.max_in_flight
	}

	pub fn proxy_retry_max_attempts(&self) -> u32 {
		self.proxy_retry_max_attempts
			.unwrap_or(DEFAULT_PROXY_RETRY_MAX_ATTEMPTS)
	}

	pub fn proxy_retry_initial_interval_ms(&self) -> u64 {
		self.proxy_retry_initial_interval_ms
			.unwrap_or(DEFAULT_PROXY_RETRY_INITIAL_INTERVAL_MS)
	}

	pub fn upstream_request_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(
			self.upstream_request_timeout_ms
				.unwrap_or(DEFAULT_UPSTREAM_REQUEST_TIMEOUT_MS),
		)
	}

	pub fn websocket_setup_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(
			self.websocket_setup_timeout_ms
				.unwrap_or(DEFAULT_WEBSOCKET_SETUP_TIMEOUT_MS),
		)
	}

	pub fn websocket_connect_attempt_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(
			self.websocket_connect_attempt_timeout_ms
				.unwrap_or(DEFAULT_WEBSOCKET_CONNECT_ATTEMPT_TIMEOUT_MS),
		)
	}

	pub fn websocket_send_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(
			self.websocket_send_timeout_ms
				.unwrap_or(DEFAULT_WEBSOCKET_SEND_TIMEOUT_MS),
		)
	}

	pub fn websocket_flush_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(
			self.websocket_flush_timeout_ms
				.unwrap_or(DEFAULT_WEBSOCKET_FLUSH_TIMEOUT_MS),
		)
	}

	pub fn http_client_pool_idle_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(
			self.http_client_pool_idle_timeout_ms
				.unwrap_or(DEFAULT_HTTP_CLIENT_POOL_IDLE_TIMEOUT_MS),
		)
	}

	pub fn admission_client_state_cache_capacity(&self) -> u64 {
		self.admission_client_state_cache_capacity
			.unwrap_or(DEFAULT_ADMISSION_CLIENT_STATE_CACHE_CAPACITY)
	}

	pub fn admission_client_state_cache_idle_timeout(&self) -> std::time::Duration {
		std::time::Duration::from_millis(
			self.admission_client_state_cache_idle_timeout_ms
				.unwrap_or(DEFAULT_ADMISSION_CLIENT_STATE_CACHE_IDLE_TIMEOUT_MS),
		)
	}

	pub fn http_max_request_body_size(&self) -> usize {
		self.http_max_request_body_size.unwrap_or(20 * 1024 * 1024) // 20 MiB
	}

	pub fn websocket_max_message_size(&self) -> usize {
		self.websocket_max_message_size
			.unwrap_or(DEFAULT_WEBSOCKET_MAX_MESSAGE_SIZE)
	}

	pub fn websocket_max_frame_size(&self) -> usize {
		self.websocket_max_frame_size
			.unwrap_or(DEFAULT_WEBSOCKET_MAX_FRAME_SIZE)
	}

	pub fn trace_propagation(&self) -> bool {
		self.trace_propagation.unwrap_or(false)
	}
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GuardRateLimit {
	/// Number of requests allowed during each fixed window.
	#[schemars(range(min = 1))]
	pub requests: u64,
	/// Fixed-window duration in milliseconds.
	#[schemars(range(min = 1))]
	pub period_ms: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
#[derive(Default)]
pub struct Https {
	pub port: u16, // Port for HTTPS traffic
	pub tls: Tls,  // TLS configuration
}

#[derive(Debug, Serialize, Deserialize, Clone, JsonSchema)]
#[serde(deny_unknown_fields)]
#[derive(Default)]
pub struct Tls {
	pub actor_cert_path: PathBuf,
	pub actor_key_path: PathBuf,
	pub api_cert_path: PathBuf,
	pub api_key_path: PathBuf,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn admission_limits_are_disabled_by_default() {
		let guard = Guard::default();

		assert!(guard.rate_limit().is_none());
		assert!(guard.max_in_flight().is_none());
	}

	#[test]
	fn proxy_operational_defaults_preserve_existing_behavior() {
		let guard = Guard::default();

		assert_eq!(guard.proxy_retry_max_attempts(), 7);
		assert_eq!(guard.proxy_retry_initial_interval_ms(), 150);
		assert_eq!(
			guard.upstream_request_timeout(),
			std::time::Duration::from_secs(30)
		);
		assert_eq!(
			guard.websocket_setup_timeout(),
			std::time::Duration::from_secs(30)
		);
		assert_eq!(
			guard.websocket_connect_attempt_timeout(),
			std::time::Duration::from_secs(5)
		);
		assert_eq!(
			guard.websocket_send_timeout(),
			std::time::Duration::from_secs(5)
		);
		assert_eq!(
			guard.websocket_flush_timeout(),
			std::time::Duration::from_secs(2)
		);
		assert_eq!(
			guard.http_client_pool_idle_timeout(),
			std::time::Duration::from_secs(30)
		);
		assert_eq!(guard.admission_client_state_cache_capacity(), 10_000);
		assert_eq!(
			guard.admission_client_state_cache_idle_timeout(),
			std::time::Duration::from_secs(60 * 60)
		);
	}

	#[test]
	fn admission_limits_reject_zero_values() {
		let guard = Guard {
			rate_limit: Some(GuardRateLimit {
				requests: 0,
				period_ms: 60_000,
			}),
			..Default::default()
		};
		assert!(guard.validate().is_err());

		let guard = Guard {
			max_in_flight: Some(0),
			..Default::default()
		};
		assert!(guard.validate().is_err());
	}
}
