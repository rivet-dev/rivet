use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct LobbyRow {
	lobby_id: Uuid,
	run_id: Option<Uuid>,
}

impl From<LobbyRow> for mm::lobby_for_run_id::response::Lobby {
	fn from(value: LobbyRow) -> Self {
		Self {
			lobby_id: Some(value.lobby_id.into()),
			run_id: value.run_id.map(Into::into),
		}
	}
}

#[operation(name = "mm-lobby-for-run-id")]
pub async fn handle(
	ctx: OperationContext<mm::lobby_for_run_id::Request>,
) -> GlobalResult<mm::lobby_for_run_id::Response> {
	let run_ids = ctx
		.run_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let lobbies = sql_fetch_all!(
		[ctx, LobbyRow]
		"
		SELECT lobby_id, run_id
		FROM db_mm_state.lobbies
		WHERE run_id = ANY($1)
		",
		run_ids,
	)
	.await?
	.into_iter()
	.map(Into::<mm::lobby_for_run_id::response::Lobby>::into)
	.collect();

	Ok(mm::lobby_for_run_id::Response { lobbies })
}
