use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker(name = "user-event-chat-last-read-ts-update")]
async fn worker(
	ctx: &OperationContext<chat::msg::last_read_ts_update::Message>,
) -> GlobalResult<()> {
	let user_id = internal_unwrap!(ctx.user_id).as_uuid();

	msg!([ctx] user::msg::event(user_id) {
		user_id: Some(user_id.into()),
		event: Some(backend::user::event::Event {
			kind: Some(backend::user::event::event::Kind::ChatRead(backend::user::event::ChatRead {
				thread_id: ctx.thread_id,
				read_ts: ctx.read_ts,
			})),
		}),
	})
	.await?;

	Ok(())
}
