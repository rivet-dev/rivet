use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker(name = "user-event-team-member-remove")]
async fn worker(ctx: &OperationContext<team::msg::member_remove::Message>) -> GlobalResult<()> {
	let user_id = unwrap_ref!(ctx.user_id);

	msg!([ctx] user::msg::event(user_id) {
		user_id: ctx.user_id,
		event: Some(backend::user::event::Event {
			kind: Some(backend::user::event::event::Kind::TeamMemberRemove(backend::user::event::TeamMemberRemove {
				team_id: ctx.team_id,
			})),
		}),
	})
	.await?;

	Ok(())
}
