use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use rivet_convert::ApiInto;

#[worker(name = "user-event-user-presence-update")]
async fn worker(ctx: OperationContext<user_presence::msg::update::Message>) -> GlobalResult<()> {
	let user_id = internal_unwrap!(ctx.user_id);

	msg!([ctx] user::msg::event(user_id) {
		user_id: ctx.user_id,
		event: Some(backend::user::event::Event {
			kind: Some(backend::user::event::event::Kind::PresenceUpdate(backend::user::event::PresenceUpdate {
				kind: ctx.kind.clone().map(ApiInto::api_into),
			})),
		}),
	})
	.await?;

	Ok(())
}
