use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "mm-lobby-find-lobby-ready")]
async fn worker(
	ctx: &OperationContext<mm::msg::lobby_ready_complete::Message>,
) -> GlobalResult<()> {
	let lobby_id = unwrap_ref!(ctx.lobby_id).as_uuid();

	// TODO: Is there a race condition here for new queries?

	let query_list = op!([ctx] mm_lobby_find_lobby_query_list {
		lobby_id: Some(lobby_id.into())
	})
	.await?;
	op!([ctx] mm_lobby_find_try_complete {
		query_ids: query_list.query_ids.clone(),
	})
	.await?;

	Ok(())
}
