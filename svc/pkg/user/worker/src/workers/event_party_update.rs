use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker(name = "user-event-party-update")]
async fn worker(ctx: OperationContext<party::msg::update::Message>) -> GlobalResult<()> {
	let party_id = internal_unwrap!(ctx.party_id).as_uuid();

	let member_list = op!([ctx] party_member_list {
		party_ids: vec![party_id.into()],
	})
	.await?;
	let party = internal_unwrap_owned!(member_list.parties.first());

	for user_id in &party.user_ids {
		msg!([ctx] user::msg::event(user_id) {
			user_id: Some(*user_id),
			event: Some(backend::user::event::Event {
				kind: Some(backend::user::event::event::Kind::PartyUpdate(backend::user::event::PartyUpdate {})),
			}),
		})
		.await?;
	}

	Ok(())
}
