use anyhow::Result;
use rivet_envoy_protocol as protocol;

use crate::conn::Conn;

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
