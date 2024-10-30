use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "cloud-device-link-complete")]
async fn worker(
	ctx: &OperationContext<cloud::msg::device_link_complete::Message>,
) -> GlobalResult<()> {
	let device_link_id = unwrap_ref!(ctx.device_link_id).as_uuid();

	let res = op!([ctx] cloud_game_token_create {
		game_id: ctx.game_id,
	})
	.await?;

	msg!([ctx] cloud::msg::device_link_complete_complete(device_link_id) {
		device_link_id: ctx.device_link_id,
		game_id: ctx.game_id,
		cloud_token: res.token.clone(),
	})
	.await?;

	Ok(())
}
