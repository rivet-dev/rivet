use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "mm-lobby-state-set")]
async fn worker(ctx: &OperationContext<mm::msg::lobby_state_set::Message>) -> GlobalResult<()> {
	let crdb = ctx.crdb("db-mm-state").await?;

	let lobby_id = internal_unwrap!(ctx.lobby_id).as_uuid();
	let mut pipe = redis::pipe();
	pipe.atomic();

	if let Some(state_json) = &ctx.state_json {
		pipe.hset(
			util_mm::key::lobby_config(lobby_id),
			util_mm::key::lobby_config::STATE_JSON,
			&state_json,
		);
	} else {
		pipe.hdel(
			util_mm::key::lobby_config(lobby_id),
			util_mm::key::lobby_config::STATE_JSON,
		);
	}

	pipe.query_async(&mut ctx.redis_mm().await?).await?;

	msg!([ctx] mm::msg::lobby_state_set_complete(lobby_id) {
		lobby_id: Some(lobby_id.into()),
	})
	.await?;

	Ok(())
}
