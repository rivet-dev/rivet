//! Process-wide envoy-client metrics.
//!
//! These metrics live in the actor pod and are intended to make the
//! single-WebSocket transport between the actor and the engine debuggable
//! during incidents (e.g. pod-local handshake latency spikes where sent
//! requests get abandoned on disconnect).
//!
//! All labels here are bounded by code-defined enums. No `actor_id`,
//! `actor_key`, `runner_id`, `request_id`, or other unbounded values may be
//! added here.

use std::sync::LazyLock;

use rivet_metrics::prometheus::{
	Counter, Histogram, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, Opts,
	Registry,
};

const SQLITE_REQUEST_EXPIRED_LABELS: &[&str] = &["kind", "was_sent"];
const ENVOY_LOOP_ITER_LABELS: &[&str] = &["branch"];
const SQLITE_SEND_LABELS: &[&str] = &["kind"];
const WS_RECONNECT_LABELS: &[&str] = &["reason"];
const LOST_TIMER_ARMED_LABELS: &[&str] = &["reason"];
const LOST_TIMER_OUTCOME_LABELS: &[&str] = &["outcome"];
const LOST_THRESHOLD_SOURCE_LABELS: &[&str] = &["source"];
const ACTOR_EVICTED_LABELS: &[&str] = &["reason"];
const ACTOR_STOP_LABELS: &[&str] = &["reason"];
const ACTOR_LIFETIME_LABELS: &[&str] = &["reason"];

pub struct EnvoyClientMetrics {
	pub sqlite_request_expired_total: IntCounterVec,
	pub sqlite_requests_inflight: IntGauge,
	pub remote_sqlite_requests_inflight: IntGauge,
	pub kv_requests_inflight: IntGauge,
	pub envoy_loop_iteration_duration_seconds: HistogramVec,
	pub sqlite_request_total_duration_seconds: HistogramVec,
	pub sqlite_request_submit_duration_seconds: HistogramVec,
	pub sqlite_request_wait_duration_seconds: HistogramVec,
	pub ws_reconnect_total: IntCounterVec,
	pub ws_session_duration_seconds: Histogram,
	pub envoy_tx_depth: IntGauge,
	pub ws_tx_depth: IntGauge,
	pub lost_timer_armed_total: IntCounterVec,
	pub lost_timer_outcome_total: IntCounterVec,
	pub reconnect_within_grace_seconds: Histogram,
	pub lost_threshold_source_total: IntCounterVec,
	pub actor_evicted_total: IntCounterVec,
	pub outbound_queue_depth: IntGauge,
	pub ping_unhealthy_seconds_total: Counter,
	pub ping_unhealthy_recovered_total: IntCounter,
	pub actor_stop_total: IntCounterVec,
	pub actor_lifetime_seconds: HistogramVec,
}

impl EnvoyClientMetrics {
	fn new() -> Self {
		let sqlite_request_expired_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_envoy_client_sqlite_request_expired_total",
				"total VFS sqlite requests expired by cleanup (smoking-gun signal: was_sent=true means a request was abandoned without resolution)",
			),
			SQLITE_REQUEST_EXPIRED_LABELS,
		)
		.expect("create envoy_client_sqlite_request_expired_total counter");

		let sqlite_requests_inflight = IntGauge::new(
			"rivetkit_envoy_client_sqlite_requests_inflight",
			"current in-flight VFS sqlite requests tracked in envoy ctx",
		)
		.expect("create envoy_client_sqlite_requests_inflight gauge");

		let remote_sqlite_requests_inflight = IntGauge::new(
			"rivetkit_envoy_client_remote_sqlite_requests_inflight",
			"current in-flight remote sqlite (exec/execute) requests tracked in envoy ctx",
		)
		.expect("create envoy_client_remote_sqlite_requests_inflight gauge");

		let kv_requests_inflight = IntGauge::new(
			"rivetkit_envoy_client_kv_requests_inflight",
			"current in-flight KV requests tracked in envoy ctx",
		)
		.expect("create envoy_client_kv_requests_inflight gauge");

		let envoy_loop_iteration_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"rivetkit_envoy_client_envoy_loop_iteration_duration_seconds",
				"duration of one envoy_loop select branch in seconds; long tails indicate the single fan-in loop stalled",
			)
			.buckets(rivet_metrics::MICRO_BUCKETS.to_vec()),
			ENVOY_LOOP_ITER_LABELS,
		)
		.expect("create envoy_client_envoy_loop_iteration_duration_seconds histogram");

		let sqlite_request_total_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"rivetkit_envoy_client_sqlite_request_total_duration_seconds",
				"end-to-end duration from send_sqlite_request call to oneshot resolution in seconds",
			)
			.buckets(rivet_metrics::BUCKETS.to_vec()),
			SQLITE_SEND_LABELS,
		)
		.expect("create envoy_client_sqlite_request_total_duration_seconds histogram");

		let sqlite_request_submit_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"rivetkit_envoy_client_sqlite_request_submit_duration_seconds",
				"time from send_sqlite_request entry to envoy_tx.send return in seconds",
			)
			.buckets(rivet_metrics::MICRO_BUCKETS.to_vec()),
			SQLITE_SEND_LABELS,
		)
		.expect("create envoy_client_sqlite_request_submit_duration_seconds histogram");

		let sqlite_request_wait_duration_seconds = HistogramVec::new(
			HistogramOpts::new(
				"rivetkit_envoy_client_sqlite_request_wait_duration_seconds",
				"time from envoy_tx.send return to oneshot resolution in seconds",
			)
			.buckets(rivet_metrics::BUCKETS.to_vec()),
			SQLITE_SEND_LABELS,
		)
		.expect("create envoy_client_sqlite_request_wait_duration_seconds histogram");

		let ws_reconnect_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_envoy_client_ws_reconnect_total",
				"total websocket reconnect events by reason",
			),
			WS_RECONNECT_LABELS,
		)
		.expect("create envoy_client_ws_reconnect_total counter");

		let ws_session_duration_seconds = Histogram::with_opts(
			HistogramOpts::new(
				"rivetkit_envoy_client_ws_session_duration_seconds",
				"duration of a single websocket session from connect to disconnect in seconds",
			)
			.buckets(rivet_metrics::BUCKETS.to_vec()),
		)
		.expect("create envoy_client_ws_session_duration_seconds histogram");

		let envoy_tx_depth = IntGauge::new(
			"rivetkit_envoy_client_envoy_tx_depth",
			"current depth of the unbounded envoy_tx mpsc between WS read task and envoy_loop",
		)
		.expect("create envoy_client_envoy_tx_depth gauge");

		let ws_tx_depth = IntGauge::new(
			"rivetkit_envoy_client_ws_tx_depth",
			"current depth of the ws_tx mpsc between message producers and the websocket write task (frames enqueued but not yet written to the socket)",
		)
		.expect("create envoy_client_ws_tx_depth gauge");

		let lost_timer_armed_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_envoy_client_lost_timer_armed_total",
				"total lost-threshold timers armed in handle_conn_close, labeled by close reason",
			),
			LOST_TIMER_ARMED_LABELS,
		)
		.expect("create envoy_client_lost_timer_armed_total counter");

		let lost_timer_outcome_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_envoy_client_lost_timer_outcome_total",
				"total lost-threshold timer outcomes (fired vs cancelled by reconnect/init)",
			),
			LOST_TIMER_OUTCOME_LABELS,
		)
		.expect("create envoy_client_lost_timer_outcome_total counter");

		let reconnect_within_grace_seconds = Histogram::with_opts(
			HistogramOpts::new(
				"rivetkit_envoy_client_reconnect_within_grace_seconds",
				"seconds from WS close to successful reconnect/init when the reconnect beats the lost-threshold timer",
			)
			.buckets(rivet_metrics::BUCKETS.to_vec()),
		)
		.expect("create envoy_client_reconnect_within_grace_seconds histogram");

		let lost_threshold_source_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_envoy_client_lost_threshold_source_total",
				"source of the lost-threshold value used per timer arm (protocol metadata vs local fallback)",
			),
			LOST_THRESHOLD_SOURCE_LABELS,
		)
		.expect("create envoy_client_lost_threshold_source_total counter");

		let actor_evicted_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_envoy_client_actor_evicted_total",
				"total actors evicted by the envoy, labeled by eviction reason",
			),
			ACTOR_EVICTED_LABELS,
		)
		.expect("create envoy_client_actor_evicted_total counter");

		let outbound_queue_depth = IntGauge::new(
			"rivetkit_envoy_client_outbound_queue_depth",
			"current depth of the ToRivet outbound tunnel-message buffer awaiting a reconnect",
		)
		.expect("create envoy_client_outbound_queue_depth gauge");

		let ping_unhealthy_seconds_total = Counter::new(
			"rivetkit_envoy_client_ping_unhealthy_seconds_total",
			"cumulative seconds spent with is_ping_healthy()==false while the WS is still open",
		)
		.expect("create envoy_client_ping_unhealthy_seconds_total counter");

		let ping_unhealthy_recovered_total = IntCounter::new(
			"rivetkit_envoy_client_ping_unhealthy_recovered_total",
			"total transitions from ping-unhealthy back to ping-healthy without a WS close",
		)
		.expect("create envoy_client_ping_unhealthy_recovered_total counter");

		let actor_stop_total = IntCounterVec::new(
			Opts::new(
				"rivetkit_envoy_client_actor_stop_total",
				"total actor stops handled by the envoy, labeled by StopActorReason",
			),
			ACTOR_STOP_LABELS,
		)
		.expect("create envoy_client_actor_stop_total counter");

		let actor_lifetime_seconds = HistogramVec::new(
			HistogramOpts::new(
				"rivetkit_envoy_client_actor_lifetime_seconds",
				"actor lifetime from create_actor to stop in seconds, labeled by StopActorReason",
			)
			.buckets(rivet_metrics::LIFETIME_BUCKETS.to_vec()),
			ACTOR_LIFETIME_LABELS,
		)
		.expect("create envoy_client_actor_lifetime_seconds histogram");

		register(
			&rivet_metrics::REGISTRY,
			sqlite_request_expired_total.clone(),
		);
		register(&rivet_metrics::REGISTRY, sqlite_requests_inflight.clone());
		register(
			&rivet_metrics::REGISTRY,
			remote_sqlite_requests_inflight.clone(),
		);
		register(&rivet_metrics::REGISTRY, kv_requests_inflight.clone());
		register(
			&rivet_metrics::REGISTRY,
			envoy_loop_iteration_duration_seconds.clone(),
		);
		register(
			&rivet_metrics::REGISTRY,
			sqlite_request_total_duration_seconds.clone(),
		);
		register(
			&rivet_metrics::REGISTRY,
			sqlite_request_submit_duration_seconds.clone(),
		);
		register(
			&rivet_metrics::REGISTRY,
			sqlite_request_wait_duration_seconds.clone(),
		);
		register(&rivet_metrics::REGISTRY, ws_reconnect_total.clone());
		register(
			&rivet_metrics::REGISTRY,
			ws_session_duration_seconds.clone(),
		);
		register(&rivet_metrics::REGISTRY, envoy_tx_depth.clone());
		register(&rivet_metrics::REGISTRY, ws_tx_depth.clone());
		register(&rivet_metrics::REGISTRY, lost_timer_armed_total.clone());
		register(&rivet_metrics::REGISTRY, lost_timer_outcome_total.clone());
		register(
			&rivet_metrics::REGISTRY,
			reconnect_within_grace_seconds.clone(),
		);
		register(
			&rivet_metrics::REGISTRY,
			lost_threshold_source_total.clone(),
		);
		register(&rivet_metrics::REGISTRY, actor_evicted_total.clone());
		register(&rivet_metrics::REGISTRY, outbound_queue_depth.clone());
		register(
			&rivet_metrics::REGISTRY,
			ping_unhealthy_seconds_total.clone(),
		);
		register(
			&rivet_metrics::REGISTRY,
			ping_unhealthy_recovered_total.clone(),
		);
		register(&rivet_metrics::REGISTRY, actor_stop_total.clone());
		register(&rivet_metrics::REGISTRY, actor_lifetime_seconds.clone());

		Self {
			sqlite_request_expired_total,
			sqlite_requests_inflight,
			remote_sqlite_requests_inflight,
			kv_requests_inflight,
			envoy_loop_iteration_duration_seconds,
			sqlite_request_total_duration_seconds,
			sqlite_request_submit_duration_seconds,
			sqlite_request_wait_duration_seconds,
			ws_reconnect_total,
			ws_session_duration_seconds,
			envoy_tx_depth,
			ws_tx_depth,
			lost_timer_armed_total,
			lost_timer_outcome_total,
			reconnect_within_grace_seconds,
			lost_threshold_source_total,
			actor_evicted_total,
			outbound_queue_depth,
			ping_unhealthy_seconds_total,
			ping_unhealthy_recovered_total,
			actor_stop_total,
			actor_lifetime_seconds,
		}
	}
}

pub static METRICS: LazyLock<EnvoyClientMetrics> = LazyLock::new(EnvoyClientMetrics::new);

fn register<M>(registry: &Registry, metric: M)
where
	M: rivet_metrics::prometheus::core::Collector + Clone + Send + Sync + 'static,
{
	if let Err(error) = registry.register(Box::new(metric)) {
		tracing::warn!(
			?error,
			"envoy-client metric registration failed, using existing collector"
		);
	}
}
