use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(Clone, sqlx::FromRow)]
struct LobbyRow {
	lobby_id: Uuid,
	total_player_count: i64,
	registered_player_count: i64,
}

impl From<LobbyRow> for mm::lobby_player_count::response::Lobby {
	fn from(value: LobbyRow) -> Self {
		mm::lobby_player_count::response::Lobby {
			lobby_id: Some(value.lobby_id.into()),
			total_player_count: value.total_player_count as u32,
			registered_player_count: value.registered_player_count as u32,
		}
	}
}

#[operation(name = "mm-lobby-player-count")]
async fn handle(
	ctx: OperationContext<mm::lobby_player_count::Request>,
) -> GlobalResult<mm::lobby_player_count::Response> {
	let lobby_ids = ctx
		.lobby_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	// Fetch total and registered player counts
	let mut pipe = redis::pipe();
	for lobby_id in &lobby_ids {
		pipe.zcard(util_mm::key::lobby_player_ids(*lobby_id));
		pipe.zcard(util_mm::key::lobby_registered_player_ids(*lobby_id));
	}
	let player_counts = pipe
		.query_async::<_, Vec<u32>>(&mut ctx.redis_mm().await?)
		.await?;

	// Map to lobbies
	let lobbies = lobby_ids
		.iter()
		.zip(player_counts.iter().step_by(2))
		.zip(player_counts.iter().skip(1).step_by(2))
		.map(|((ns_id, total_player_count), registered_player_count)| {
			mm::lobby_player_count::response::Lobby {
				lobby_id: Some((*ns_id).into()),
				total_player_count: *total_player_count,
				registered_player_count: *registered_player_count,
			}
		})
		.collect::<Vec<_>>();

	Ok(mm::lobby_player_count::Response { lobbies })
}
