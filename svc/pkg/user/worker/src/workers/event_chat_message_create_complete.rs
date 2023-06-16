use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker(name = "user-event-chat-message-create-complete")]
async fn worker(
	ctx: OperationContext<chat_message::msg::create_complete::Message>,
) -> GlobalResult<()> {
	for user_id in &ctx.participant_user_ids {
		msg!([ctx] user::msg::event(user_id) {
			user_id: Some(*user_id),
			event: Some(backend::user::event::Event {
				kind: Some(backend::user::event::event::Kind::ChatMessage(backend::user::event::ChatMessage {
					chat_message: ctx.chat_message.clone(),
				})),
			}),
		})
		.await?;
	}

	Ok(())
}
