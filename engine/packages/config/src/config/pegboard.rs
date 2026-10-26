use anyhow::{Result, bail};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pegboard {
	/// Maximum SQLite actor sessions retained by one node connection (default 256).
	pub sqlite_sessions: Option<usize>,
	/// Idle time after which a remote SQL session is closed and reopened on next use.
	///
	/// Unit is in milliseconds (default 30000).
	pub sqlite_remote_idle_ms: Option<u64>,
	/// Threads shared by every remote SQL session in this process (default 4).
	pub sqlite_remote_worker_threads: Option<usize>,
	/// Largest response, in logical bytes, that one remote SQL request may return. A larger
	/// result fails the request instead of being sent.
	///
	/// Unit is in bytes (default 25165824, 24 MiB; at most 64 MiB).
	pub sqlite_remote_max_response_bytes: Option<usize>,
	/// Sends SQLite pages the client is likely to read next along with the pages it asked for
	/// (default true). When off, the engine sends only requested pages but keeps maintaining its
	/// prefetch hints, so turning it back on needs no rebuild. Runtime configurable.
	pub sqlite_server_prefetch: Option<bool>,
	/// Messages queued for one node socket writer (default 128, 1..=65536).
	#[serde(rename = "node_socket_queue_capacity")]
	pub envoy_socket_queue_capacity: Option<usize>,
	/// Messages queued toward one gateway tunnel stream on a node connection (default 128,
	/// 1..=65536).
	#[serde(rename = "node_tunnel_queue_capacity")]
	pub envoy_tunnel_queue_capacity: Option<usize>,
	/// Commands and tunnel messages queued toward one actor on a node connection (default 128,
	/// 1..=65536).
	#[serde(rename = "node_actor_lane_capacity")]
	pub envoy_actor_lane_capacity: Option<usize>,
	/// Events, KV and SQLite requests queued from one actor on a node connection (default 128,
	/// 1..=65536).
	#[serde(rename = "node_actor_inbound_capacity")]
	pub envoy_actor_inbound_capacity: Option<usize>,
	/// Retained actor handles per node connection (default 16384, 1..=65536).
	#[serde(rename = "node_actor_handles_capacity")]
	pub envoy_actor_handles_capacity: Option<usize>,
	/// Longest time a node connection holds a gateway request for an actor generation whose
	/// start command it has not forwarded yet, in milliseconds (default 1000). A gateway sends a
	/// request right behind the start, so this keeps the request from reaching the node first.
	/// When it runs out the request is forwarded anyway, and the node refuses it as not found.
	#[serde(rename = "node_start_wait_ms")]
	pub envoy_start_wait_ms: Option<u64>,
	/// Lost checks one process holds for closed node connections that may not reconnect
	/// (default 4096, 1..=65536). When full, the check due soonest is dropped and its actors are
	/// found lost on their next request.
	#[serde(rename = "node_lost_checks_capacity")]
	pub envoy_lost_checks_capacity: Option<usize>,
	/// Gateway workers one actor keeps waiting for its start (default 64, 1..=1024). A request
	/// that finds the list full waits on its fallback poll alone.
	pub actor_start_waiters_capacity: Option<u32>,
	/// Timeout of one start notice to a waiting gateway worker. Unit is in milliseconds (default
	/// 2000, 100..=30000).
	pub actor_start_notice_timeout_ms: Option<u64>,
	/// Actors one gateway process waits on at once for a start notice (default 65536,
	/// 1..=1048576). When full, a new request waits on its fallback poll alone.
	pub gateway_start_waits_capacity: Option<usize>,
	/// Retries of refused requests one gateway process allows, as a percentage of the requests
	/// it receives (default 10, 0..=100). A refused request past this budget fails with
	/// `guard.service_unavailable` and its retry hint, so the client owns the next attempt.
	pub gateway_retry_budget_percent: Option<u32>,
	/// Retries one gateway process can bank for a burst of refusals; the budget starts full
	/// (default 100, 0..=100000).
	pub gateway_retry_budget_max_tokens: Option<u32>,
	/// node connects one process admits per second once its burst is spent (default 200,
	/// 1..=100000). A connect past the budget is closed at once with
	/// `node.connect_overloaded` and a retry hint, before any work, so a fleet reconnecting
	/// after an outage cannot overwhelm guard.
	#[serde(rename = "node_connect_admissions_per_second")]
	pub envoy_connect_admissions_per_second: Option<u32>,
	/// node connects one process can admit in a burst; the budget starts full (default 400,
	/// 1..=1000000).
	#[serde(rename = "node_connect_admission_burst")]
	pub envoy_connect_admission_burst: Option<u32>,
	/// Retry hint sent with a shed node connect. The node waits between one and two hints, so
	/// shed connects return spread out, and caps the hint at 5 s so a shed node still retries
	/// well inside the lost threshold. The same hint goes with a connect refused because storage
	/// could not register the node yet. Unit is in milliseconds (default 1000, 100..=5000).
	#[serde(rename = "node_connect_retry_after_ms")]
	pub envoy_connect_retry_after_ms: Option<u64>,
	/// Time after a gateway request starts waiting for an actor's start notice before its first
	/// fallback read. Unit is in milliseconds (default 2000, 100..=30000).
	pub gateway_start_poll_after_ms: Option<u64>,
	/// Time between later fallback reads while a gateway request waits for an actor's start.
	/// Unit is in milliseconds (default 2000, 100..=30000).
	pub gateway_start_poll_interval_ms: Option<u64>,
	/// Time an actor WebSocket upgrade may wait for its actor, as while storage recovers after an
	/// engine restart. Past it the gateway accepts the upgrade and closes it with
	/// `guard.service_unavailable` and a retry hint, so a client without a connect timeout
	/// retries instead of waiting forever. Unit is in milliseconds (default 15000, 1000..=45000).
	pub gateway_websocket_open_timeout_ms: Option<u64>,
	/// Port for the new partition-backed gateway and node Hyper listener (default 6422).
	pub gateway_port: Option<u16>,

	// === Envoy Settings ===
	/// How long a disconnected node keeps its actors before stopping them, advertised to the
	/// node on connect. The engine waits as long after a connection closes before it judges the
	/// node's actors lost, so both sides agree. It must exceed the node client's longest
	/// reconnect backoff (30 s), or an engine restart stops every actor.
	///
	/// Unit is in milliseconds (default 40000).
	#[serde(rename = "node_lost_threshold")]
	pub envoy_lost_threshold: Option<i64>,
	/// Max time since last pong before the node connection is terminated. Unit is in milliseconds.
	#[serde(rename = "node_ping_timeout")]
	pub envoy_ping_timeout: Option<i64>,
	/// Ping interval for node updates in milliseconds.
	#[serde(rename = "node_update_ping_interval")]
	pub envoy_update_ping_interval: Option<u64>,
	/// Time a lifecycle event may spend retrying storage command deadlines before its node
	/// socket is closed for reconnection. Defaults to 120000 ms.
	#[serde(rename = "node_event_delivery_timeout_ms")]
	pub envoy_event_delivery_timeout_ms: Option<u64>,
	/// Maximum age of the process-local node directory that actor allocation reads instead of
	/// listing the node index on every allocation. Must be greater than 0.
	///
	/// Unit is in milliseconds. Defaults to 1000.
	#[serde(rename = "node_directory_cache_ms")]
	pub envoy_directory_cache_ms: Option<u64>,
	/// Maximum age of a node directory listing that allocation still uses while one background
	/// listing replaces it. Allocation waits for a listing only past this age. Must be at least
	/// `node_directory_cache_ms`.
	///
	/// Unit is in milliseconds. Defaults to 5000.
	#[serde(rename = "node_directory_max_stale_ms")]
	pub envoy_directory_max_stale_ms: Option<u64>,
	/// Hashed index bucket volumes for each namespace and actor name (default 8, 1..=256).
	///
	/// This is part of the index address format. Changing it on a running cluster strands
	/// existing rows in buckets that listing and destroy no longer target, so treat it as fixed
	/// per cluster.
	pub actor_index_buckets_per_name: Option<u64>,
	/// Rows an actor listing's lazy orphan deletes may queue at once (default 1024).
	pub actor_index_reaper_max_pending: Option<usize>,
	/// Lazy orphan deletes an actor listing runs at once (default 16).
	pub actor_index_reaper_max_concurrency: Option<usize>,
	/// Stale node row deletes one reader may queue at once (default 256).
	#[serde(rename = "node_index_reaper_max_pending")]
	pub envoy_index_reaper_max_pending: Option<usize>,
	/// Stale node row deletes one reader runs at once (default 8).
	#[serde(rename = "node_index_reaper_max_concurrency")]
	pub envoy_index_reaper_max_concurrency: Option<usize>,
	/// Lifetime of a cached node record read on a routing fallback, in milliseconds
	/// (default 2000).
	#[serde(rename = "node_record_cache_ms")]
	pub envoy_record_cache_ms: Option<u64>,
	/// node records cached for routing fallbacks in one process (default 10000).
	#[serde(rename = "node_record_cache_capacity")]
	pub envoy_record_cache_capacity: Option<u64>,
	/// Actor key reservations and key epochs cached in one process, each (default 100000).
	/// Reservations never change, so a cached one is kept until evicted.
	pub actor_key_cache_capacity: Option<u64>,
	/// Maximum actor index bucket pages one listing reads at once (default 32).
	pub actor_index_list_concurrency: Option<usize>,
	/// Per-class handler concurrency for the storage commands pegboard registers on storage workers.
	#[serde(default)]
	pub command_concurrency: CommandConcurrency,
}

impl Pegboard {
	pub fn validate(&self) -> Result<()> {
		if self.envoy_event_delivery_timeout_ms == Some(0) {
			bail!("pegboard.node_event_delivery_timeout_ms must be greater than 0");
		}

		if self.envoy_directory_cache_ms == Some(0) {
			bail!("pegboard.node_directory_cache_ms must be greater than 0");
		}
		if self.envoy_directory_max_stale_ms() < self.envoy_directory_cache_ms() {
			bail!(
				"pegboard.node_directory_max_stale_ms must be at least pegboard.node_directory_cache_ms"
			);
		}

		if !(1..=256).contains(&self.actor_index_buckets_per_name()) {
			bail!("pegboard.actor_index_buckets_per_name must be in 1..=256");
		}
		if self.envoy_record_cache_ms == Some(0) || self.envoy_record_cache_capacity == Some(0) {
			bail!("pegboard.node_record_cache limits must be greater than 0");
		}
		if self.actor_key_cache_capacity == Some(0) {
			bail!("pegboard.actor_key_cache_capacity must be greater than 0");
		}
		if self.actor_index_reaper_max_pending == Some(0)
			|| self.actor_index_reaper_max_concurrency == Some(0)
			|| self.envoy_index_reaper_max_pending == Some(0)
			|| self.envoy_index_reaper_max_concurrency == Some(0)
		{
			bail!("pegboard.actor_index_reaper limits must be greater than 0");
		}

		if self.actor_index_list_concurrency == Some(0) {
			bail!("pegboard.actor_index_list_concurrency must be greater than 0");
		}
		if !(1..=256).contains(&self.sqlite_remote_worker_threads()) {
			bail!("pegboard.sqlite_remote_worker_threads must be in 1..=256");
		}

		for (key, capacity) in [
			(
				"node_actor_handles_capacity",
				self.envoy_actor_handles_capacity(),
			),
			(
				"node_socket_queue_capacity",
				self.envoy_socket_queue_capacity(),
			),
			(
				"node_tunnel_queue_capacity",
				self.envoy_tunnel_queue_capacity(),
			),
			("node_actor_lane_capacity", self.envoy_actor_lane_capacity()),
			(
				"node_actor_inbound_capacity",
				self.envoy_actor_inbound_capacity(),
			),
			(
				"node_lost_checks_capacity",
				self.envoy_lost_checks_capacity(),
			),
		] {
			if !(1..=65_536).contains(&capacity) {
				bail!("pegboard.{key} must be in 1..=65536");
			}
		}

		if !(1..=1024).contains(&self.actor_start_waiters_capacity()) {
			bail!("pegboard.actor_start_waiters_capacity must be in 1..=1024");
		}
		if !(1..=1_048_576).contains(&self.gateway_start_waits_capacity()) {
			bail!("pegboard.gateway_start_waits_capacity must be in 1..=1048576");
		}
		if self.gateway_retry_budget_percent() > 100 {
			bail!("pegboard.gateway_retry_budget_percent must be in 0..=100");
		}
		if self.gateway_retry_budget_max_tokens() > 100_000 {
			bail!("pegboard.gateway_retry_budget_max_tokens must be in 0..=100000");
		}
		if !(1..=100_000).contains(&self.envoy_connect_admissions_per_second()) {
			bail!("pegboard.node_connect_admissions_per_second must be in 1..=100000");
		}
		if !(1..=1_000_000).contains(&self.envoy_connect_admission_burst()) {
			bail!("pegboard.node_connect_admission_burst must be in 1..=1000000");
		}
		if !(100..=5_000).contains(&self.envoy_connect_retry_after_ms()) {
			bail!("pegboard.node_connect_retry_after_ms must be in 100..=5000");
		}
		if !(1_000..=45_000).contains(&self.gateway_websocket_open_timeout_ms()) {
			bail!("pegboard.gateway_websocket_open_timeout_ms must be in 1000..=45000");
		}
		for (key, value) in [
			(
				"actor_start_notice_timeout_ms",
				self.actor_start_notice_timeout_ms(),
			),
			(
				"gateway_start_poll_after_ms",
				self.gateway_start_poll_after_ms(),
			),
			(
				"gateway_start_poll_interval_ms",
				self.gateway_start_poll_interval_ms(),
			),
		] {
			if !(100..=30_000).contains(&value) {
				bail!("pegboard.{key} must be in 100..=30000");
			}
		}

		self.command_concurrency.validate()?;

		Ok(())
	}

	pub fn actor_index_buckets_per_name(&self) -> u64 {
		self.actor_index_buckets_per_name.unwrap_or(8)
	}

	pub fn envoy_record_cache_ms(&self) -> u64 {
		self.envoy_record_cache_ms.unwrap_or(2_000)
	}

	pub fn envoy_record_cache_capacity(&self) -> u64 {
		self.envoy_record_cache_capacity.unwrap_or(10_000)
	}

	pub fn actor_key_cache_capacity(&self) -> u64 {
		self.actor_key_cache_capacity.unwrap_or(100_000)
	}

	pub fn envoy_index_reaper_max_pending(&self) -> usize {
		self.envoy_index_reaper_max_pending.unwrap_or(256)
	}

	pub fn envoy_index_reaper_max_concurrency(&self) -> usize {
		self.envoy_index_reaper_max_concurrency.unwrap_or(8)
	}

	pub fn actor_index_reaper_max_pending(&self) -> usize {
		self.actor_index_reaper_max_pending.unwrap_or(1024)
	}

	pub fn actor_index_reaper_max_concurrency(&self) -> usize {
		self.actor_index_reaper_max_concurrency.unwrap_or(16)
	}

	pub fn actor_index_list_concurrency(&self) -> usize {
		self.actor_index_list_concurrency.unwrap_or(32)
	}

	pub fn envoy_socket_queue_capacity(&self) -> usize {
		self.envoy_socket_queue_capacity.unwrap_or(128)
	}

	pub fn envoy_tunnel_queue_capacity(&self) -> usize {
		self.envoy_tunnel_queue_capacity.unwrap_or(128)
	}

	pub fn envoy_actor_lane_capacity(&self) -> usize {
		self.envoy_actor_lane_capacity.unwrap_or(128)
	}

	pub fn envoy_actor_inbound_capacity(&self) -> usize {
		self.envoy_actor_inbound_capacity.unwrap_or(128)
	}

	pub fn envoy_actor_handles_capacity(&self) -> usize {
		self.envoy_actor_handles_capacity.unwrap_or(16_384)
	}

	pub fn envoy_start_wait_ms(&self) -> u64 {
		self.envoy_start_wait_ms.unwrap_or(1_000)
	}

	pub fn actor_start_waiters_capacity(&self) -> u32 {
		self.actor_start_waiters_capacity.unwrap_or(64)
	}

	pub fn actor_start_notice_timeout_ms(&self) -> u64 {
		self.actor_start_notice_timeout_ms.unwrap_or(2_000)
	}

	pub fn gateway_websocket_open_timeout_ms(&self) -> u64 {
		self.gateway_websocket_open_timeout_ms.unwrap_or(15_000)
	}

	pub fn gateway_start_poll_after_ms(&self) -> u64 {
		self.gateway_start_poll_after_ms.unwrap_or(2_000)
	}

	pub fn gateway_start_poll_interval_ms(&self) -> u64 {
		self.gateway_start_poll_interval_ms.unwrap_or(2_000)
	}

	pub fn gateway_start_waits_capacity(&self) -> usize {
		self.gateway_start_waits_capacity.unwrap_or(65_536)
	}

	pub fn gateway_retry_budget_percent(&self) -> u32 {
		self.gateway_retry_budget_percent.unwrap_or(10)
	}

	pub fn gateway_retry_budget_max_tokens(&self) -> u32 {
		self.gateway_retry_budget_max_tokens.unwrap_or(100)
	}

	pub fn envoy_connect_admissions_per_second(&self) -> u32 {
		self.envoy_connect_admissions_per_second.unwrap_or(200)
	}

	pub fn envoy_connect_admission_burst(&self) -> u32 {
		self.envoy_connect_admission_burst.unwrap_or(400)
	}

	pub fn envoy_connect_retry_after_ms(&self) -> u64 {
		self.envoy_connect_retry_after_ms.unwrap_or(1_000)
	}

	pub fn envoy_lost_checks_capacity(&self) -> usize {
		self.envoy_lost_checks_capacity.unwrap_or(4096)
	}

	pub fn sqlite_sessions(&self) -> usize {
		self.sqlite_sessions.unwrap_or(256)
	}

	pub fn sqlite_server_prefetch(&self) -> bool {
		self.sqlite_server_prefetch.unwrap_or(true)
	}

	pub fn sqlite_remote_idle_ms(&self) -> u64 {
		self.sqlite_remote_idle_ms.unwrap_or(30_000)
	}

	pub fn sqlite_remote_worker_threads(&self) -> usize {
		self.sqlite_remote_worker_threads.unwrap_or(4)
	}

	pub fn sqlite_remote_max_response_bytes(&self) -> usize {
		self.sqlite_remote_max_response_bytes
			.unwrap_or(24 * 1024 * 1024)
			.clamp(1, 64 * 1024 * 1024)
	}

	pub fn envoy_event_delivery_timeout_ms(&self) -> u64 {
		self.envoy_event_delivery_timeout_ms.unwrap_or(120_000)
	}

	pub fn envoy_directory_cache_ms(&self) -> u64 {
		self.envoy_directory_cache_ms.unwrap_or(1_000)
	}

	pub fn envoy_directory_max_stale_ms(&self) -> u64 {
		self.envoy_directory_max_stale_ms.unwrap_or(5_000)
	}

	pub fn envoy_lost_threshold(&self) -> i64 {
		self.envoy_lost_threshold.unwrap_or(40_000)
	}

	pub fn envoy_ping_timeout(&self) -> i64 {
		self.envoy_ping_timeout.unwrap_or(15_000)
	}

	pub fn envoy_update_ping_interval(&self) -> u64 {
		self.envoy_update_ping_interval.unwrap_or(3_000)
	}
}

/// Maximum simultaneous handlers per pegboard command class on one storage worker. Each value is
/// further capped by `storage.kv_commands.max_inflight_commands`.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CommandConcurrency {
	/// Actor state machine events, including node lifecycle reports. Defaults to 256.
	pub actor_event: Option<usize>,
	/// Actor record reads by the gateway, the API and storage fencing. Defaults to 256.
	pub actor_read: Option<usize>,
	/// Actor creation. Defaults to 256.
	pub actor: Option<usize>,
	/// Index inserts, deletes and listings. Defaults to 256.
	pub index: Option<usize>,
}

impl CommandConcurrency {
	pub fn validate(&self) -> Result<()> {
		if [self.actor_event, self.actor_read, self.actor, self.index].contains(&Some(0)) {
			bail!("pegboard.command_concurrency values must be greater than 0");
		}
		Ok(())
	}

	pub fn actor_event(&self) -> usize {
		self.actor_event.unwrap_or(256)
	}

	pub fn actor_read(&self) -> usize {
		self.actor_read.unwrap_or(256)
	}

	pub fn actor(&self) -> usize {
		self.actor.unwrap_or(256)
	}

	pub fn index(&self) -> usize {
		self.index.unwrap_or(256)
	}
}
