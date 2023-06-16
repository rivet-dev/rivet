use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use rivet_convert::ApiInto;

#[worker(name = "user-updated-user-presence-update")]
async fn worker(ctx: OperationContext<user_presence::msg::update::Message>) -> GlobalResult<()> {
	let user_id = internal_unwrap!(ctx.user_id);

	msg!([ctx] user::msg::updated(user_id) {
		user_id: ctx.user_id,
		update: Some(backend::user::update::Update {
			kind: Some(backend::user::update::update::Kind::PresenceUpdate(backend::user::update::PresenceUpdate {
				kind: ctx.kind.clone().map(ApiInto::api_into),
			})),
		}),
	})
	.await?;

	Ok(())
}
