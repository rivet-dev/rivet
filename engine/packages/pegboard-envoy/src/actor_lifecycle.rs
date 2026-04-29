use anyhow::Result;
use rivet_envoy_protocol as protocol;

use crate::conn::Conn;

pub async fn stop_actor(conn: &Conn, checkpoint: &protocol::ActorCheckpoint) -> Result<()> {
	conn.actor_dbs.remove_async(&checkpoint.actor_id).await;
	Ok(())
}

pub async fn shutdown_conn_actors(conn: &Conn) {
	conn.actor_dbs.clear_sync();
}
