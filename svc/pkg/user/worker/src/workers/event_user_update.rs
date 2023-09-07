use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker(name = "user-event-user-update")]
async fn worker(ctx: &OperationContext<user::msg::update::Message>) -> GlobalResult<()> {
	let user_id = internal_unwrap!(ctx.user_id);

	msg!([ctx] user::msg::event(user_id) {
		user_id: ctx.user_id,
		event: Some(backend::user::event::Event {
			kind: Some(backend::user::event::event::Kind::UserUpdate(backend::user::event::UserUpdate {
			})),
		}),
	})
	.await?;

	Ok(())
}
