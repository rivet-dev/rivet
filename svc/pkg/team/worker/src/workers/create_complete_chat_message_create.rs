use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker(name = "team-create-complete-chat-message-create")]
async fn worker(ctx: &OperationContext<team::msg::create_complete::Message>) -> GlobalResult<()> {
	// Create new chat with team
	op!([ctx] chat_message_create_with_topic {
		chat_message_id: Some(Uuid::new_v4().into()),
		topic: Some(backend::chat::Topic {
			kind: Some(backend::chat::topic::Kind::Team(
				backend::chat::topic::Team {
					team_id: ctx.team_id,
				},
			)),
		}),
		send_ts: util::timestamp::now(),
		body: Some(backend::chat::MessageBody {
			kind: Some(backend::chat::message_body::Kind::ChatCreate(
				backend::chat::message_body::ChatCreate {},
			)),
		}),
	})
	.await?;

	Ok(())
}
