use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "mm-lobby-state-get")]
async fn handle(
	ctx: OperationContext<mm::lobby_state_get::Request>,
) -> GlobalResult<mm::lobby_state_get::Response> {
	let lobby_ids = ctx
		.lobby_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	// Fetch lobby states
	let mut pipe = redis::pipe();
	for lobby_id in &lobby_ids {
		pipe.hget(
			util_mm::key::lobby_config(*lobby_id),
			util_mm::key::lobby_config::STATE_JSON,
		);
	}
	let states = pipe
		.query_async::<_, Vec<Option<String>>>(&mut ctx.redis_mm().await?)
		.await?;

	// Map to lobbies
	let lobbies = lobby_ids
		.iter()
		.zip(states.into_iter())
		.map(|(lobby_id, state)| mm::lobby_state_get::response::Lobby {
			lobby_id: Some((*lobby_id).into()),
			state_json: state,
		})
		.collect::<Vec<_>>();

	Ok(mm::lobby_state_get::Response { lobbies })
}
