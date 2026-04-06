use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pegboard {
	/// Time to delay an actor from rescheduling after a rescheduling failure.
	///
	/// Unit is in milliseconds.
	pub base_retry_timeout: Option<usize>,
	/// How long to wait for an ack response from the outbound request layer before setting actor as lost.
	///
	/// Unit is in milliseconds.
	pub actor_allocation_threshold: Option<i64>,
	/// How long to wait after creating and not receiving a starting state before setting actor as lost.
	///
	/// Unit is in milliseconds.
	pub actor_start_threshold: Option<i64>,
	/// How long to wait after stopping and not receiving a stop state before setting actor as lost.
	///
	/// Unit is in milliseconds.
	pub actor_stop_threshold: Option<i64>,
	/// How long to wait after starting to attempt to reallocate before before setting actor to sleep.
	///
	/// Unit is in milliseconds.
	pub actor_retry_duration_threshold: Option<i64>,
	/// How long an actor goes without retries before it's retry count is reset to 0, effectively resetting its
	/// backoff to 0.
	///
	/// Unit is in milliseconds.
	pub retry_reset_duration: Option<i64>,
	/// Maximum exponent for the reschedule backoff calculation.
	///
	/// This controls the maximum backoff duration when rescheduling actors.
	pub reschedule_backoff_max_exponent: Option<usize>,
	/// How long after last ping before considering a runner ineligible for allocation.
	///
	/// Unit is in milliseconds.
	pub runner_eligible_threshold: Option<i64>,
	/// How long to wait after last ping before forcibly removing a runner from the database
	/// and deleting its workflow, evicting all actors.
	///
	/// Note that the runner may still be running and can reconnect.
	///
	/// Unit is in milliseconds.
	pub runner_lost_threshold: Option<i64>,
	/// How long after last ping before considering a hibernating request disconnected.
	///
	/// Unit is in milliseconds.
	pub hibernating_request_eligible_threshold: Option<i64>,
	/// Time to delay a serverless runner from attempting a new outbound connection after a connection failure.
	///
	/// Unit is in milliseconds.
	pub serverless_base_retry_timeout: Option<usize>,
	/// How long a serverless runner goes without connection failures before it's retry count is reset to 0,
	/// effectively resetting its backoff to 0.
	///
	/// Unit is in milliseconds.
	pub serverless_retry_reset_duration: Option<i64>,
	/// Maximum exponent for the serverless backoff calculation.
	///
	/// This controls the maximum backoff duration when serverlessly connecting to runners.
	pub serverless_backoff_max_exponent: Option<usize>,

	/// Global pool desired max.
	pub pool_desired_max_override: Option<u32>,

	/// Default metadata poll interval for serverless runners when not specified in runner config.
	///
	/// Unit is in milliseconds.
	pub default_metadata_poll_interval: Option<u64>,

	/// Minimum metadata poll interval for serverless runners.
	///
	/// The actual poll interval will be the maximum of this value and the runner config's
	/// `metadata_poll_interval` setting. This prevents excessive polling even if the
	/// runner config specifies a very short interval.
	///
	/// Unit is in milliseconds.
	pub min_metadata_poll_interval: Option<u64>,

	/// Number of consecutive successes required to clear an active runner pool error.
	///
	/// This prevents a single success from clearing an error during flapping conditions.
	/// Higher values provide more stability but slower recovery from transient errors.
	pub runner_pool_consecutive_successes_to_clear_error: Option<u32>,

	/// Amount of runners to query from the allocation queue and choose at random when allocating an actor.
	pub actor_allocation_candidate_sample_size: Option<usize>,

	/// Max response payload size in bytes from actors.
	pub runner_max_response_payload_body_size: Option<usize>,
	/// Ping interval for runner updates in milliseconds.
	pub runner_update_ping_interval_ms: Option<u64>,
	/// Max time since last pong before the runner connection is terminated. Unit is in milliseconds.
	pub runner_ping_timeout_ms: Option<i64>,
	/// GC interval for actor event demuxer in milliseconds.
	pub runner_event_demuxer_gc_interval_ms: Option<u64>,
	/// Max time since last seen before actor is considered stale, in milliseconds.
	pub runner_event_demuxer_max_last_seen_ms: Option<u64>,

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
	pub gateway_http_max_request_body_size: Option<usize>,

	// === Envoy Settings ===
	/// How long to wait before considering an envoy lost and evicting all of its actors.
	///
	/// Unit is in milliseconds.
	pub envoy_lost_threshold: Option<i64>,
	/// Max time since last pong before the envoy connection is terminated. Unit is in milliseconds.
	pub envoy_ping_timeout: Option<i64>,
	/// GC interval for actor event demuxer in milliseconds.
	pub envoy_event_demuxer_gc_interval: Option<u64>,
	/// Max time since last seen before actor is considered stale, in milliseconds.
	pub envoy_event_demuxer_max_last_seen_threshold: Option<u64>,
	/// Max response payload size in bytes from actors.
	pub envoy_max_response_payload_size: Option<usize>,
	/// Ping interval for envoy updates in milliseconds.
	pub envoy_update_ping_interval: Option<u64>,
	/// How long after last ping before considering a envoy ineligible for allocation.
	///
	/// Unit is in milliseconds.
	pub envoy_eligible_threshold: Option<i64>,

	// === Serverless Settings ===
	/// Drain grace period for serverless runners.
	///
	/// This time is subtracted from the configured request duration. Once `duration - grace` is reached, the
	/// runner is sent stop commands for all of its actors. After the grace period is over (i.e. the full
	/// duration is reached) the runner websocket is forcibly closed.
	///
	/// Unit is in milliseconds.
	pub serverless_drain_grace_period: Option<u64>,

	// === KV Preload Settings ===
	/// Maximum total size of all preloaded KV data sent with the actor start command.
	/// Setting to 0 disables all preloading.
	///
	/// Unit is in bytes. Default: 1,048,576 (1 MiB).
	pub preload_max_total_bytes: Option<u64>,
}

impl Pegboard {
	pub fn base_retry_timeout(&self) -> usize {
		self.base_retry_timeout.unwrap_or(2000)
	}

	pub fn actor_allocation_threshold(&self) -> i64 {
		self.actor_allocation_threshold.unwrap_or(2_000)
	}

	pub fn actor_start_threshold(&self) -> i64 {
		self.actor_start_threshold.unwrap_or(30_000)
	}

	/// When changing this default, update
	/// website/src/content/docs/actors/versions.mdx (SIGTERM Handling section).
	pub fn actor_stop_threshold(&self) -> i64 {
		self.actor_stop_threshold.unwrap_or(30_000)
	}

	pub fn actor_retry_duration_threshold(&self) -> i64 {
		self.actor_retry_duration_threshold.unwrap_or(300_000)
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

	pub fn runner_max_response_payload_body_size(&self) -> usize {
		self.runner_max_response_payload_body_size
			.unwrap_or(20 * 1024 * 1024) // 20 MiB
	}

	pub fn runner_update_ping_interval_ms(&self) -> u64 {
		self.runner_update_ping_interval_ms.unwrap_or(3_000)
	}

	pub fn runner_ping_timeout_ms(&self) -> i64 {
		self.runner_ping_timeout_ms.unwrap_or(15_000)
	}

	pub fn runner_event_demuxer_gc_interval_ms(&self) -> u64 {
		self.runner_event_demuxer_gc_interval_ms.unwrap_or(30_000)
	}

	pub fn runner_event_demuxer_max_last_seen_ms(&self) -> u64 {
		self.runner_event_demuxer_max_last_seen_ms.unwrap_or(30_000)
	}

	pub fn envoy_lost_threshold(&self) -> i64 {
		self.envoy_lost_threshold.unwrap_or(15_000)
	}

	pub fn envoy_ping_timeout(&self) -> i64 {
		self.envoy_ping_timeout.unwrap_or(15_000)
	}

	pub fn envoy_event_demuxer_gc_interval(&self) -> u64 {
		self.envoy_event_demuxer_gc_interval.unwrap_or(30_000)
	}

	pub fn envoy_event_demuxer_max_last_seen_threshold(&self) -> u64 {
		self.envoy_event_demuxer_max_last_seen_threshold
			.unwrap_or(30_000)
	}

	pub fn envoy_max_response_payload_size(&self) -> usize {
		self.envoy_max_response_payload_size
			.unwrap_or(20 * 1024 * 1024) // 20 MiB
	}

	pub fn envoy_update_ping_interval(&self) -> u64 {
		self.envoy_update_ping_interval.unwrap_or(3_000)
	}

	pub fn envoy_eligible_threshold(&self) -> i64 {
		self.envoy_eligible_threshold.unwrap_or(10_000)
	}

	pub fn serverless_drain_grace_period(&self) -> u64 {
		self.serverless_drain_grace_period.unwrap_or(10_000)
	}

	pub fn preload_max_total_bytes(&self) -> u64 {
		self.preload_max_total_bytes.unwrap_or(1_048_576)
	}
}
