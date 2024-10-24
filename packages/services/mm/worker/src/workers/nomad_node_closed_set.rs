use chirp_worker::prelude::*;
use proto::backend::pkg::*;

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../../redis-scripts/nomad_node_closed_unset.lua"));
}

#[derive(Debug, sqlx::FromRow)]
struct LobbyRow {
	lobby_id: Uuid,
	namespace_id: Uuid,
	lobby_group_id: Uuid,
	max_players_normal: i64,
	max_players_party: i64,
}

#[worker(name = "mm-nomad-node-closed-set")]
async fn worker(
	ctx: &OperationContext<mm::msg::nomad_node_closed_set::Message>,
) -> GlobalResult<()> {
	let datacenter_id = unwrap_ref!(ctx.datacenter_id).as_uuid();

	// Select all lobbies in the node
	let lobby_rows = sql_fetch_all!(
		[ctx, LobbyRow]
		"
		UPDATE db_mm_state.lobbies AS l
		SET is_closed = $2
		FROM db_job_state.run_meta_nomad AS n
		WHERE
			l.run_id = n.run_id AND
			l.stop_ts IS NULL AND
			n.node_id = $1
		RETURNING
			lobby_id, namespace_id, lobby_group_id, max_players_normal, max_players_party
		",
		&ctx.nomad_node_id,
		ctx.is_closed,
	)
	.await?;

	// Update matchmaking index
	if ctx.is_closed {
		let mut pipe = redis::pipe();
		pipe.atomic();

		for lobby in lobby_rows {
			pipe.zrem(
				util_mm::key::lobby_available_spots(
					lobby.namespace_id,
					datacenter_id,
					lobby.lobby_group_id,
					util_mm::JoinKind::Normal,
				),
				lobby.lobby_id.to_string(),
			)
			.zrem(
				util_mm::key::lobby_available_spots(
					lobby.namespace_id,
					datacenter_id,
					lobby.lobby_group_id,
					util_mm::JoinKind::Party,
				),
				lobby.lobby_id.to_string(),
			);
		}

		pipe.query_async(&mut ctx.redis_mm().await?).await?;
	}

	// NOTE: Don't do anything on undrain

	Ok(())
}
