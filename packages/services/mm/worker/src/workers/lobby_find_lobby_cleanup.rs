use chirp_worker::prelude::*;
use proto::backend::{matchmaker::lobby_find::ErrorCode, pkg::*};
use redis::AsyncCommands;

#[worker(name = "mm-lobby-find-lobby-cleanup")]
async fn worker(ctx: &OperationContext<mm::msg::lobby_cleanup::Message>) -> GlobalResult<()> {
	let lobby_id = unwrap_ref!(ctx.lobby_id).as_uuid();
	let mut redis_mm = ctx.redis_mm().await?;

	// TODO: Is there a race condition here for new queries?

	// There can be a race condition between this and
	// mm-lobby-find-job-run-fail, but the error code (i.e.
	// LobbyStoppedPrematurely) will match regardless.
	let query_list = op!([ctx] mm_lobby_find_lobby_query_list {
		lobby_id: Some(lobby_id.into()),
	})
	.await?;
	op!([ctx] mm_lobby_find_fail {
		query_ids: query_list.query_ids.clone(),
		error_code: ErrorCode::LobbyStoppedPrematurely as i32,
		..Default::default()
	})
	.await?;

	// Remove queries
	redis_mm
		.unlink(util_mm::key::lobby_find_queries(lobby_id))
		.await?;

	Ok(())
}
