use proto::backend::{self, pkg::*};
use rivet_claims::ClaimsDecode;
use rivet_operation::prelude::*;

const MAX_TEXT_BODY_LEN: usize = 2048;

#[operation(name = "chat-message-body-validate")]
async fn handle(
	ctx: OperationContext<chat_message::body_validate::Request>,
) -> GlobalResult<chat_message::body_validate::Response> {
	let body = unwrap_ref!(ctx.body);

	match validate(&ctx, body).await {
		Ok(_) => {}
		Err(err) => {
			tracing::info!(?ctx.body, ?err, "body not valid");

			// Replace generic error with invalid body error
			if err.is(formatted_error::code::ERROR) {
				bail_with!(CHAT_INVALID_BODY);
			} else {
				return Err(err);
			}
		}
	}

	Ok(chat_message::body_validate::Response {})
}

async fn validate(
	ctx: &OperationContext<chat_message::body_validate::Request>,
	body: &backend::chat::MessageBody,
) -> GlobalResult<()> {
	use backend::chat::message_body::*;

	let body_kind = unwrap_ref!(body.kind);
	match &body_kind {
		Kind::Custom(Custom {
			sender_user_id,
			plugin_id,
			..
		}) => {
			if let Some(user_id) = sender_user_id {
				user_id;
			}
			ensure!(plugin_id.is_some());
		}
		Kind::Text(Text {
			sender_user_id,
			body,
		}) => {
			ensure!(sender_user_id.is_some());
			ensure_with!(!body.is_empty(), CHAT_INVALID_BODY, reason = "Empty body");
			ensure_with!(
				body.len() <= MAX_TEXT_BODY_LEN,
				CHAT_INVALID_BODY,
				reason = "Body too long"
			);
		}
		Kind::TeamJoin(TeamJoin { user_id }) => {
			ensure!(user_id.is_some());
		}
		Kind::TeamLeave(TeamLeave { user_id }) => {
			ensure!(user_id.is_some());
		}
		Kind::TeamMemberKick(TeamMemberKick { user_id }) => {
			ensure!(user_id.is_some());
		}
		Kind::ChatCreate(ChatCreate {}) | Kind::UserFollow(UserFollow {}) => {
			// Do nothing
		}
		Kind::Deleted(Deleted { sender_user_id }) => {
			ensure!(sender_user_id.is_some());
		}
	}

	Ok(())
}
