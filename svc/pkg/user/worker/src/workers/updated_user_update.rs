use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker(name = "user-updated-user-update")]
async fn worker(ctx: &OperationContext<user::msg::update::Message>) -> GlobalResult<()> {
	let user_id = internal_unwrap!(ctx.user_id);

	msg!([ctx] user::msg::updated(user_id) {
		user_id: ctx.user_id,
		update: Some(backend::user::update::Update {
			kind: Some(backend::user::update::update::Kind::Update(backend::user::update::UserUpdate {
			})),
		}),
	})
	.await?;

	Ok(())
}
