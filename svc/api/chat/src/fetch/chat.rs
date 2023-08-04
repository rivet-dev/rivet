use api_helper::ctx::Ctx;
use proto::{backend, common};
use rivet_chat_server::models;
use rivet_operation::prelude::*;

use crate::{auth::Auth, convert, fetch};

struct ChatMessagePrefetch {
	pub user_ids: Vec<common::Uuid>,
	pub namespace_ids: Vec<common::Uuid>,
}

pub async fn messages(
	ctx: &Ctx<Auth>,
	current_user_id: Uuid,
	messages: &[backend::chat::Message],
) -> GlobalResult<Vec<models::ChatMessage>> {
	let ChatMessagePrefetch {
		user_ids,
		namespace_ids,
	} = prefetch_messages(messages)?;
	let (users_res,) = tokio::try_join!(op!([ctx] user_get {
		user_ids: user_ids,
	}),)?;

	messages
		.iter()
		.map(|message| convert::chat::message(current_user_id, message, &users_res.users))
		.collect::<GlobalResult<Vec<_>>>()
}

fn prefetch_messages(messages: &[backend::chat::Message]) -> GlobalResult<ChatMessagePrefetch> {
	use backend::chat::message_body as backend_body;

	let mut user_ids = Vec::<common::Uuid>::new();
	let mut namespace_ids = Vec::<common::Uuid>::new();

	// Prefetch all user ids and party ids
	for message in messages {
		// Read body message
		let backend_body_kind = internal_unwrap!(message.body);
		let backend_body_kind = internal_unwrap!(backend_body_kind.kind);

		match backend_body_kind {
			backend_body::Kind::Custom(backend_body::Custom { sender_user_id, .. }) => {
				user_ids.push(*internal_unwrap!(sender_user_id));
			}
			backend_body::Kind::Text(backend_body::Text { sender_user_id, .. }) => {
				user_ids.push(*internal_unwrap!(sender_user_id));
			}
			backend_body::Kind::Deleted(backend_body::Deleted { sender_user_id }) => {
				user_ids.push(*internal_unwrap!(sender_user_id));
			}
			backend_body::Kind::TeamJoin(backend_body::TeamJoin { user_id }) => {
				user_ids.push(*internal_unwrap!(user_id));
			}
			backend_body::Kind::TeamLeave(backend_body::TeamLeave { user_id }) => {
				user_ids.push(*internal_unwrap!(user_id));
			}
			backend_body::Kind::TeamMemberKick(backend_body::TeamMemberKick { user_id }) => {
				user_ids.push(*internal_unwrap!(user_id));
			}
			backend_body::Kind::ChatCreate(_) | backend_body::Kind::UserFollow(_) => {}
		}
	}

	Ok(ChatMessagePrefetch {
		user_ids,
		namespace_ids,
	})
}
