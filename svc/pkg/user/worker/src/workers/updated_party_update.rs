use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker(name = "user-updated-party-update")]
async fn worker(ctx: &OperationContext<party::msg::update::Message>) -> GlobalResult<()> {
	let party_id = unwrap_ref!(ctx.party_id);

	let member_list = op!([ctx] party_member_list {
		party_ids: vec![*party_id],
	})
	.await?;
	let party = unwrap!(member_list.parties.first());

	for user_id in &party.user_ids {
		msg!([ctx] user::msg::updated(user_id) {
			user_id: Some(*user_id),
			update: Some(backend::user::update::Update {
				kind: Some(backend::user::update::update::Kind::PartyUpdate(backend::user::update::PartyUpdate {})),
			}),
		})
		.await?;
	}

	Ok(())
}
