use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use serde_json::json;

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../../redis-scripts/create.lua"));
}

#[worker(name = "chat-message-create")]
async fn worker(ctx: &OperationContext<chat_message::msg::create::Message>) -> GlobalResult<()> {
	let crdb = ctx.crdb("db-chat").await?;

	let chat_message_id = internal_unwrap!(ctx.chat_message_id).as_uuid();
	let thread_id = internal_unwrap!(ctx.thread_id).as_uuid();
	let body = internal_unwrap!(ctx.body);
	let sender_user_id = get_sender_user_id(body)?;

	// Encode body
	let mut body_buf = Vec::with_capacity(body.encoded_len());
	body.encode(&mut body_buf)?;

	sqlx::query(indoc!(
		"
		INSERT INTO messages (
			message_id,
			thread_id,
			send_ts,
			body,
			sender_user_id
		)
		VALUES ($1, $2, $3, $4, $5)
		"
	))
	.bind(chat_message_id)
	.bind(thread_id)
	.bind(ctx.send_ts)
	.bind(&body_buf)
	.bind(sender_user_id)
	.execute(&crdb)
	.await?;

	// Build chat message
	let message = backend::chat::Message {
		chat_message_id: Some(chat_message_id.into()),
		thread_id: Some(thread_id.into()),
		send_ts: ctx.send_ts,
		body: Some(body.clone()),
	};

	// Encode message
	let mut message_buf = Vec::with_capacity(message.encoded_len());
	message.encode(&mut message_buf)?;

	msg!([ctx] chat_thread::msg::update(thread_id) {
		kind: Some(chat_thread::msg::update::message::Kind::ChatMessage(message.clone())),
	})
	.await?;

	// TODO: Move this to a separate worker?

	// Fetch thread participants
	let thread_participants_res = op!([ctx] chat_thread_participant_list {
		thread_ids: vec![thread_id.into()],
	})
	.await?;
	let thread_participants = internal_unwrap_owned!(thread_participants_res.threads.first());

	// Initiate redis script
	let mut script = REDIS_SCRIPT.prepare_invoke();
	script
		.key(util_chat::key::thread_tail_message(thread_id))
		.arg(util_chat::key::THREAD_TAIL_MESSAGE_EXPIRE_DURATION)
		.arg(util_chat::key::USER_THREAD_HISTORY_EXPIRE_DURATION)
		.arg(thread_id.to_string())
		.arg(chat_message_id.to_string())
		.arg(ctx.send_ts)
		.arg(message_buf.clone());

	// Thread update event
	for participant in &thread_participants.participants {
		let user_id = internal_unwrap!(participant.user_id).as_uuid();

		// Add this message to each thread participant's thread history in redis
		script.key(util_chat::key::user_thread_history_loaded(user_id));
		script.key(util_chat::key::user_thread_history(user_id));

		match internal_unwrap!(body.kind) {
			// Don't send "chat created" notification
			backend::chat::message_body::Kind::ChatCreate(_) => {}
			_ => {
				msg!([ctx] push_notification::msg::create(user_id) {
					user_id: Some(user_id.into()),
					thread_id: Some(thread_id.into()),
					message: Some(message.clone()),
					service: backend::notification::NotificationService::Firebase as i32,
					tag: Some(chat_message_id.to_string()),
				})
				.await?;
			}
		}
	}

	let participant_user_ids = thread_participants
		.participants
		.iter()
		.filter_map(|p| p.user_id)
		.collect::<Vec<_>>();
	msg!([ctx] chat_message::msg::create_complete(thread_id, chat_message_id) {
		thread_id: Some(thread_id.into()),
		chat_message_id: Some(chat_message_id.into()),
		chat_message: Some(message.clone()),
		participant_user_ids: participant_user_ids,
	})
	.await?;

	// Run Redis after inserting messages
	script
		.invoke_async::<_, ()>(&mut ctx.redis_cache().await?)
		.await?;

	let analytics_body = match internal_unwrap!(body.kind) {
		// TODO: Implement custom messages
		backend::chat::message_body::Kind::Custom(_) => json!({ "custom": {} }),
		backend::chat::message_body::Kind::Text(body) => {
			json!({ "text": { "body_len": body.body.len() } })
		}
		backend::chat::message_body::Kind::ChatCreate(_) => json!({ "chat_create": {} }),
		backend::chat::message_body::Kind::Deleted(_) => json!({ "deleted": {} }),
		backend::chat::message_body::Kind::UserFollow(_) => json!({ "user_follow": {} }),
		backend::chat::message_body::Kind::TeamJoin(_) => json!({ "team_join": {} }),
		backend::chat::message_body::Kind::TeamLeave(_) => json!({ "team_leave": {} }),
		backend::chat::message_body::Kind::TeamMemberKick(_) => json!({ "team_member_kick": {} }),
	};
	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "chat.message.create".into(),
				properties_json: Some(serde_json::to_string(&json!({
					"user_id": sender_user_id,
					"thread_id": thread_id,
					"body": analytics_body
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(())
}

fn get_sender_user_id(body: &backend::chat::MessageBody) -> GlobalResult<Option<Uuid>> {
	let kind = internal_unwrap!(body.kind);

	use backend::chat::message_body as backend_body;
	let sender_user_id = match kind {
		backend_body::Kind::Custom(body) => Some(internal_unwrap!(body.sender_user_id).as_uuid()),
		backend_body::Kind::Text(body) => Some(internal_unwrap!(body.sender_user_id).as_uuid()),
		_ => None,
	};

	Ok(sender_user_id)
}
