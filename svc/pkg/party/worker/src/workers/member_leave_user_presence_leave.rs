use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "party-member-leave-user-presence-leave")]
async fn worker(ctx: &OperationContext<user_presence::msg::leave::Message>) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let user_id = internal_unwrap_owned!(ctx.user_id);

	// Fetch the party member if exists
	let party_member_res = op!([ctx] party_member_get {
		user_ids: vec![user_id],
	})
	.await?;
	if let Some(party_member) = party_member_res.party_members.first() {
		let party_id = internal_unwrap!(party_member.party_id).as_uuid();

		msg!([ctx] party::msg::member_remove(party_id, user_id) -> party::msg::member_remove_complete {
			party_id: party_member.party_id,
			user_id: ctx.user_id,
			..Default::default()
		})
		.await?;
	}

	Ok(())
}
