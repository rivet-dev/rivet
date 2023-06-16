use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "party-member-kick")]
async fn worker(ctx: OperationContext<party::msg::member_kick::Message>) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let party_id = internal_unwrap!(ctx.party_id).as_uuid();
	let kick_user_id = internal_unwrap!(ctx.kick_user_id).as_uuid();

	msg!([ctx] party::msg::member_remove(party_id, kick_user_id) {
		party_id: Some(party_id.into()),
		user_id: Some(kick_user_id.into()),
		..Default::default()
	})
	.await?;

	Ok(())
}
