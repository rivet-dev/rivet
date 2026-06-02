use std::time::Instant;

use anyhow::Result;
use rivet_envoy_protocol as protocol;

use crate::{
	conn::{ActorStopMeta, Conn},
	metrics,
};

pub async fn stop_actor(conn: &Conn, checkpoint: &protocol::ActorCheckpoint) -> Result<()> {
	// Depot owns SQLite correctness in FDB. The connection only holds a perf cache, so
	// lifecycle stop evicts stale local state without touching storage.
	conn.actor_dbs.remove_async(&checkpoint.actor_id).await;
	conn.remote_sqlite_executors
		.retain_sync(|(actor_id, _), _| actor_id != &checkpoint.actor_id);
	Ok(())
}

pub async fn shutdown_conn_actors(conn: &Conn) {
	// See `stop_actor`. This drops only per-connection cache entries.
	conn.actor_dbs.clear_sync();
	conn.remote_sqlite_executors.clear_sync();
}

/// Convert the BARE `StopActorReason` enum into a bounded metric label.
pub fn stop_reason_label(reason: &protocol::StopActorReason) -> &'static str {
	match reason {
		protocol::StopActorReason::SleepIntent => "sleep_intent",
		protocol::StopActorReason::StopIntent => "stop_intent",
		protocol::StopActorReason::Destroy => "destroy",
		protocol::StopActorReason::GoingAway => "going_away",
		protocol::StopActorReason::Lost => "lost",
	}
}

/// Record that the engine dispatched a start command for the actor. Tracks the start
/// timestamp so we can emit `actor_lifetime_seconds` at stop time.
pub async fn record_actor_start(conn: &Conn, actor_id: &str, create_ts_ms: i64) {
	let _ = conn
		.actor_started_at
		.insert_async(actor_id.to_string(), create_ts_ms)
		.await;
}

/// Record that the engine dispatched a stop command for the actor. Captures the stop
/// reason + dispatch instant for `actor_stop_total` and `actor_stop_to_close_seconds`,
/// and increments `pegboard_actor_lost_total` for the `engine_command` origin when the
/// reason is `Lost`.
pub async fn record_actor_stop_dispatch(
	conn: &Conn,
	actor_id: &str,
	reason: &protocol::StopActorReason,
) {
	let reason_label = stop_reason_label(reason);
	let _ = conn
		.actor_stop_meta
		.insert_async(
			actor_id.to_string(),
			ActorStopMeta {
				reason: reason_label,
				dispatched_at: Instant::now(),
			},
		)
		.await;

	if matches!(reason, protocol::StopActorReason::Lost) {
		metrics::ACTOR_LOST_TOTAL
			.with_label_values(&[
				conn.namespace_id.to_string().as_str(),
				&conn.pool_name,
				"engine_command",
			])
			.inc();
	}
}
