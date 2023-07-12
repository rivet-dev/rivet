use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker(name = "user-event-party-member-update")]
async fn worker(ctx: &OperationContext<party::msg::member_update::Message>) -> GlobalResult<()> {
	let user_id = internal_unwrap!(ctx.user_id).as_uuid();

	msg!([ctx] user::msg::event(user_id) {
		user_id: Some(user_id.into()),
		event: Some(backend::user::event::Event {
			kind: Some(backend::user::event::event::Kind::PartyUpdate(backend::user::event::PartyUpdate {})),
		}),
	})
	.await?;

	Ok(())
}
