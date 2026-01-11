use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pegboard {
	/// Time to delay an actor from rescheduling after a rescheduling failure.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub base_retry_timeout: Option<usize>,
	/// How long to wait after creating and not receiving a starting state before setting actor as lost.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub actor_start_threshold: Option<i64>,
	/// How long to wait after stopping and not receiving a stop state before setting actor as lost.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub actor_stop_threshold: Option<i64>,
	/// How long an actor goes without retries before it's retry count is reset to 0, effectively resetting its
	/// backoff to 0.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub retry_reset_duration: Option<i64>,
	/// Maximum exponent for the reschedule backoff calculation.
	///
	/// This controls the maximum backoff duration when rescheduling actors.
	///
	/// **Experimental**
	pub reschedule_backoff_max_exponent: Option<usize>,
	/// How long after last ping before considering a runner ineligible for allocation.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub runner_eligible_threshold: Option<i64>,
	/// How long to wait after last ping before forcibly removing a runner from the database
	/// and deleting its workflow, evicting all actors.
	///
	/// Note that the runner may still be running and can reconnect.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub runner_lost_threshold: Option<i64>,
	/// How long after last ping before considering a hibernating request disconnected.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub hibernating_request_eligible_threshold: Option<i64>,
	/// Time to delay a serverless runner from attempting a new outbound connection after a connection failure.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub serverless_base_retry_timeout: Option<usize>,
	/// How long a serverless runner goes without connection failures before it's retry count is reset to 0,
	/// effectively resetting its backoff to 0.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub serverless_retry_reset_duration: Option<i64>,
	/// Maximum exponent for the serverless backoff calculation.
	///
	/// This controls the maximum backoff duration when serverlessly connecting to runners.
	///
	/// **Experimental**
	pub serverless_backoff_max_exponent: Option<usize>,

	/// Global pool desired max.
	pub pool_desired_max_override: Option<u32>,

	/// Default metadata poll interval for serverless runners when not specified in runner config.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub default_metadata_poll_interval: Option<u64>,

	/// Minimum metadata poll interval for serverless runners.
	///
	/// The actual poll interval will be the maximum of this value and the runner config's
	/// `metadata_poll_interval` setting. This prevents excessive polling even if the
	/// runner config specifies a very short interval.
	///
	/// Unit is in milliseconds.
	///
	/// **Experimental**
	pub min_metadata_poll_interval: Option<u64>,

	/// Number of consecutive successes required to clear an active runner pool error.
	///
	/// This prevents a single success from clearing an error during flapping conditions.
	/// Higher values provide more stability but slower recovery from transient errors.
	///
	/// **Experimental**
	pub runner_pool_consecutive_successes_to_clear_error: Option<u32>,
	/// Amount of runners to query from the allocation queue and choose at random when allocating an actor.
	///
	/// **Experimental**
	pub actor_allocation_candidate_sample_size: Option<usize>,

	// === Gateway Settings ===
	/// WebSocket open/handshake timeout in milliseconds.
	pub gateway_websocket_open_timeout_ms: Option<u64>,
	/// Timeout for response to start in milliseconds.
	pub gateway_response_start_timeout_ms: Option<u64>,
	/// Ping interval for gateway updates in milliseconds.
	pub gateway_update_ping_interval_ms: Option<u64>,
	/// GC interval for in-flight requests in milliseconds.
	pub gateway_gc_interval_ms: Option<u64>,
	/// Tunnel ping timeout in milliseconds.
	pub gateway_tunnel_ping_timeout_ms: Option<i64>,
	/// Hibernating WebSocket message ack timeout in milliseconds.
	pub gateway_hws_message_ack_timeout_ms: Option<u64>,
	/// Max pending message buffer size for hibernating WebSockets in bytes.
	pub gateway_hws_max_pending_size: Option<u64>,
	/// Max HTTP request body size in bytes for requests to actors.
	///
	/// Note: guard-core also enforces a larger limit (default 256 MiB) as a first line of defense.
	/// See `Guard::http_max_request_body_size`.
	pub gateway_http_max_request_body_size: Option<usize>,
	/// Rate limit: number of requests allowed per period.
	pub gateway_rate_limit_requests: Option<u64>,
	/// Rate limit: period in seconds.
	pub gateway_rate_limit_period_secs: Option<u64>,
	/// Maximum concurrent in-flight requests per actor per IP.
	pub gateway_max_in_flight: Option<usize>,
	/// HTTP request timeout in seconds for actor traffic.
	///
	/// This is the outer timeout for the entire request lifecycle.
	/// Should be slightly longer than `gateway_response_start_timeout_ms` to provide a grace period.
	pub gateway_actor_request_timeout_secs: Option<u64>,
	/// HTTP request timeout in seconds for API traffic (api-public).
	pub gateway_api_request_timeout_secs: Option<u64>,
	/// Maximum retry attempts for failed requests.
	pub gateway_retry_max_attempts: Option<u32>,
	/// Initial retry interval in milliseconds (doubles with each attempt).
	pub gateway_retry_initial_interval_ms: Option<u64>,
	/// WebSocket proxy task timeout in seconds.
	pub gateway_ws_proxy_timeout_secs: Option<u64>,
	/// WebSocket connection attempt timeout in seconds.
	pub gateway_ws_connect_timeout_secs: Option<u64>,
	/// WebSocket send message timeout in seconds.
	pub gateway_ws_send_timeout_secs: Option<u64>,
	/// WebSocket flush timeout in seconds.
	pub gateway_ws_flush_timeout_secs: Option<u64>,

	// === API Settings ===
	/// Rate limit for API traffic: number of requests allowed per period.
	pub api_rate_limit_requests: Option<u64>,
	/// Rate limit for API traffic: period in seconds.
	pub api_rate_limit_period_secs: Option<u64>,
	/// Maximum concurrent in-flight requests for API traffic.
	pub api_max_in_flight: Option<usize>,
	/// Maximum retry attempts for API traffic.
	pub api_retry_max_attempts: Option<u32>,
	/// Initial retry interval for API traffic in milliseconds.
	pub api_retry_initial_interval_ms: Option<u64>,
	/// Max HTTP request body size in bytes for API traffic.
	pub api_max_http_request_body_size: Option<usize>,

	// === Runner Settings ===
	/// Max HTTP response body size in bytes from actors.
	pub runner_http_max_response_body_size: Option<usize>,
	/// Ping interval for runner updates in milliseconds.
	pub runner_update_ping_interval_ms: Option<u64>,
	/// GC interval for actor event demuxer in milliseconds.
	pub runner_event_demuxer_gc_interval_ms: Option<u64>,
	/// Max time since last seen before actor is considered stale, in milliseconds.
	pub runner_event_demuxer_max_last_seen_ms: Option<u64>,
}

impl Pegboard {
	pub fn base_retry_timeout(&self) -> usize {
		self.base_retry_timeout.unwrap_or(2000)
	}

	pub fn actor_start_threshold(&self) -> i64 {
		self.actor_start_threshold.unwrap_or(30_000)
	}

	pub fn actor_stop_threshold(&self) -> i64 {
		self.actor_stop_threshold.unwrap_or(30_000)
	}

	pub fn retry_reset_duration(&self) -> i64 {
		self.retry_reset_duration.unwrap_or(10 * 60 * 1000)
	}

	pub fn reschedule_backoff_max_exponent(&self) -> usize {
		self.reschedule_backoff_max_exponent.unwrap_or(8)
	}

	pub fn runner_eligible_threshold(&self) -> i64 {
		self.runner_eligible_threshold.unwrap_or(10_000)
	}

	pub fn runner_lost_threshold(&self) -> i64 {
		self.runner_lost_threshold.unwrap_or(15_000)
	}

	pub fn hibernating_request_eligible_threshold(&self) -> i64 {
		self.hibernating_request_eligible_threshold
			.unwrap_or(90_000)
	}

	pub fn serverless_base_retry_timeout(&self) -> usize {
		self.serverless_base_retry_timeout.unwrap_or(2000)
	}

	pub fn serverless_retry_reset_duration(&self) -> i64 {
		self.serverless_retry_reset_duration
			.unwrap_or(10 * 60 * 1000)
	}

	pub fn serverless_backoff_max_exponent(&self) -> usize {
		self.serverless_backoff_max_exponent.unwrap_or(8)
	}

	pub fn runner_pool_error_consecutive_successes_to_clear(&self) -> u32 {
		self.runner_pool_consecutive_successes_to_clear_error
			.unwrap_or(3)
	}

	pub fn default_metadata_poll_interval(&self) -> u64 {
		self.default_metadata_poll_interval.unwrap_or(10_000)
	}

	pub fn min_metadata_poll_interval(&self) -> u64 {
		self.min_metadata_poll_interval.unwrap_or(5_000)
	}

	pub fn actor_allocation_candidate_sample_size(&self) -> usize {
		self.actor_allocation_candidate_sample_size.unwrap_or(100)
	}

	// === Gateway Settings ===

	pub fn gateway_websocket_open_timeout_ms(&self) -> u64 {
		self.gateway_websocket_open_timeout_ms.unwrap_or(15_000)
	}

	pub fn gateway_response_start_timeout_ms(&self) -> u64 {
		self.gateway_response_start_timeout_ms
			.unwrap_or(5 * 60 * 1000) // 5 minutes
	}

	pub fn gateway_update_ping_interval_ms(&self) -> u64 {
		self.gateway_update_ping_interval_ms.unwrap_or(3_000)
	}

	pub fn gateway_gc_interval_ms(&self) -> u64 {
		self.gateway_gc_interval_ms.unwrap_or(15_000)
	}

	pub fn gateway_tunnel_ping_timeout_ms(&self) -> i64 {
		self.gateway_tunnel_ping_timeout_ms.unwrap_or(30_000)
	}

	pub fn gateway_hws_message_ack_timeout_ms(&self) -> u64 {
		self.gateway_hws_message_ack_timeout_ms.unwrap_or(30_000)
	}

	pub fn gateway_hws_max_pending_size(&self) -> u64 {
		self.gateway_hws_max_pending_size
			.unwrap_or(128 * 1024 * 1024) // 128 MiB
	}

	pub fn gateway_http_max_request_body_size(&self) -> usize {
		self.gateway_http_max_request_body_size
			.unwrap_or(128 * 1024 * 1024) // 128 MiB
	}

	pub fn gateway_rate_limit_requests(&self) -> u64 {
		self.gateway_rate_limit_requests.unwrap_or(1200)
	}

	pub fn gateway_rate_limit_period_secs(&self) -> u64 {
		self.gateway_rate_limit_period_secs.unwrap_or(60)
	}

	pub fn gateway_max_in_flight(&self) -> usize {
		self.gateway_max_in_flight.unwrap_or(32)
	}

	pub fn gateway_actor_request_timeout_secs(&self) -> u64 {
		self.gateway_actor_request_timeout_secs.unwrap_or(6 * 60) // 6 minutes
	}

	pub fn gateway_api_request_timeout_secs(&self) -> u64 {
		self.gateway_api_request_timeout_secs.unwrap_or(60) // 1 minute
	}

	pub fn gateway_retry_max_attempts(&self) -> u32 {
		self.gateway_retry_max_attempts.unwrap_or(7)
	}

	pub fn gateway_retry_initial_interval_ms(&self) -> u64 {
		self.gateway_retry_initial_interval_ms.unwrap_or(150)
	}

	pub fn gateway_ws_proxy_timeout_secs(&self) -> u64 {
		self.gateway_ws_proxy_timeout_secs.unwrap_or(30)
	}

	pub fn gateway_ws_connect_timeout_secs(&self) -> u64 {
		self.gateway_ws_connect_timeout_secs.unwrap_or(5)
	}

	pub fn gateway_ws_send_timeout_secs(&self) -> u64 {
		self.gateway_ws_send_timeout_secs.unwrap_or(5)
	}

	pub fn gateway_ws_flush_timeout_secs(&self) -> u64 {
		self.gateway_ws_flush_timeout_secs.unwrap_or(2)
	}

	// === API Settings ===

	pub fn api_rate_limit_requests(&self) -> u64 {
		self.api_rate_limit_requests.unwrap_or(1200)
	}

	pub fn api_rate_limit_period_secs(&self) -> u64 {
		self.api_rate_limit_period_secs.unwrap_or(60)
	}

	pub fn api_max_in_flight(&self) -> usize {
		self.api_max_in_flight.unwrap_or(32)
	}

	pub fn api_retry_max_attempts(&self) -> u32 {
		self.api_retry_max_attempts.unwrap_or(3)
	}

	pub fn api_retry_initial_interval_ms(&self) -> u64 {
		self.api_retry_initial_interval_ms.unwrap_or(100)
	}

	pub fn api_max_http_request_body_size(&self) -> usize {
		self.api_max_http_request_body_size
			.unwrap_or(256 * 1024 * 1024) // 256 MiB
	}

	// === Runner Settings ===

	pub fn runner_http_max_response_body_size(&self) -> usize {
		self.runner_http_max_response_body_size
			.unwrap_or(128 * 1024 * 1024) // 128 MiB
	}

	pub fn runner_update_ping_interval_ms(&self) -> u64 {
		self.runner_update_ping_interval_ms.unwrap_or(3_000)
	}

	pub fn runner_event_demuxer_gc_interval_ms(&self) -> u64 {
		self.runner_event_demuxer_gc_interval_ms.unwrap_or(30_000)
	}

	pub fn runner_event_demuxer_max_last_seen_ms(&self) -> u64 {
		self.runner_event_demuxer_max_last_seen_ms.unwrap_or(30_000)
	}
}
