use std::{collections::HashMap, str::FromStr};

use api_helper::{
	anchor::{WatchIndexQuery, WatchResponse},
	ctx::Ctx,
};
use proto::{
	backend::{self, pkg::*},
	common,
};
use rivet_chat_server::models;
use rivet_claims::ClaimsDecode;
use rivet_convert::ApiTryInto;
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{assert, auth::Auth, convert, fetch, utils};

// Determine the topic
enum ThreadOrTopic {
	Thread { thread_id: Uuid },
	Topic(backend::chat::Topic),
}

// MARK: POST /messages
pub async fn send_chat_message(
	ctx: Ctx<Auth>,
	body: models::SendChatMessageRequest,
) -> GlobalResult<models::SendChatMessageResponse> {
	let current_user_id = ctx.auth().dual_user(ctx.op_ctx()).await?;

	let thread_or_topic = match body.topic {
		models::SendChatTopic::ThreadId(thread_id) => ThreadOrTopic::Thread {
			thread_id: util::uuid::parse(&thread_id)?,
		},
		models::SendChatTopic::GroupId(group_id) => ThreadOrTopic::Topic(backend::chat::Topic {
			kind: Some(backend::chat::topic::Kind::Team(
				backend::chat::topic::Team {
					team_id: Some(util::uuid::parse(&group_id)?.into()),
				},
			)),
		}),
		models::SendChatTopic::PartyId(party_id) => ThreadOrTopic::Topic(backend::chat::Topic {
			kind: Some(backend::chat::topic::Kind::Party(
				backend::chat::topic::Party {
					party_id: Some(util::uuid::parse(&party_id)?.into()),
				},
			)),
		}),
		models::SendChatTopic::IdentityId(identity_id) => {
			ThreadOrTopic::Topic(backend::chat::Topic {
				kind: Some(backend::chat::topic::Kind::Direct(
					backend::chat::topic::Direct {
						user_a_id: Some(current_user_id.into()),
						user_b_id: Some(util::uuid::parse(&identity_id)?.into()),
					},
				)),
			})
		}
	};

	// Validate party before getting or creating a thread
	let (party_id, invite) =
		if let models::SendMessageBody::PartyInvite(ref message) = body.message_body {
			let party_id = unwrap_with_owned!(
				utils::get_current_party(ctx.op_ctx(), current_user_id).await?,
				PARTY_IDENTITY_NOT_IN_ANY_PARTY
			);

			assert::party_leader(ctx.op_ctx(), party_id, current_user_id).await?;

			let invite = rivet_claims::decode(&message.token)??.as_party_invite()?;

			(Some(party_id), Some(invite))
		} else {
			(None, None)
		};

	// Build body
	let msg_body = match body.message_body {
		models::SendMessageBody::Text(message) => {
			backend::chat::message_body::Kind::Text(backend::chat::message_body::Text {
				sender_user_id: Some(current_user_id.into()),
				body: message.body,
			})
		}
		models::SendMessageBody::PartyInvite(message) => {
			backend::chat::message_body::Kind::PartyInvite(
				backend::chat::message_body::PartyInvite {
					sender_user_id: Some(current_user_id.into()),
					party_id: Some(internal_unwrap_owned!(party_id).into()),
					invite_id: Some(internal_unwrap_owned!(invite).invite_id.into()),
					invite_token: message.token,
				},
			)
		}
	};

	// Validate message
	{
		// Get topic
		let topic = match thread_or_topic {
			ThreadOrTopic::Thread { thread_id } => {
				// Get topic from thread ID
				let res = op!([ctx] chat_thread_get {
					thread_ids: vec![thread_id.into()],
				})
				.await?;
				let thread = unwrap_with_owned!(res.threads.first(), CHAT_THREAD_NOT_FOUND);
				let topic = internal_unwrap!(thread.topic);

				topic.clone()
			}
			ThreadOrTopic::Topic(ref topic) => topic.clone(),
		};

		op!([ctx] chat_message_validate {
			message: Some(chat_message::validate::request::Message {
				topic: Some(topic),
				body: Some(backend::chat::MessageBody {
					kind: Some(msg_body.clone())
				}),
			})
		})
		.await?;
	}

	// Get or create the thread ID
	let thread_id = match thread_or_topic {
		ThreadOrTopic::Thread { thread_id } => thread_id,
		ThreadOrTopic::Topic(topic) => {
			// Find the thread ID
			let res = op!([ctx] chat_thread_get_or_create_for_topic {
				topic: Some(topic.clone()),
			})
			.await?;

			internal_unwrap!(res.thread_id).as_uuid()
		}
	};

	// Validate participant
	assert::chat_thread_participant(&ctx, thread_id, current_user_id).await?;

	// Send user's message
	let chat_message_id = Uuid::new_v4();
	let message_ts = util::timestamp::now();
	msg!([ctx] chat_message::msg::create(thread_id, chat_message_id) -> chat_thread::msg::update(thread_id) {
		chat_message_id: Some(chat_message_id.into()),
		thread_id: Some(thread_id.into()),
		send_ts: message_ts,
		body: Some(backend::chat::MessageBody {
			kind: Some(msg_body),
		}),
	})
	.await?;

	// This is published after the above rpc call because this should only be
	// called if the above is successful.
	msg!([ctx] chat::msg::last_read_ts_set(current_user_id, thread_id) {
		user_id: Some(current_user_id.into()),
		thread_id: Some(thread_id.into()),
		last_read_ts: message_ts,
	})
	.await?;

	Ok(models::SendChatMessageResponse {
		chat_message_id: chat_message_id.to_string(),
	})
}

// MARK: GET /threads/{}/history
#[derive(Debug, Serialize, Deserialize)]
pub struct GetThreadHistoryQuery {
	ts: Option<chrono::DateTime<chrono::Utc>>,
	count: u32,
	query_direction: Option<String>,
}

pub async fn thread_history(
	ctx: Ctx<Auth>,
	thread_id: Uuid,
	_watch_index: WatchIndexQuery,
	query: GetThreadHistoryQuery,
) -> GlobalResult<models::GetThreadHistoryResponse> {
	let current_user_id = ctx.auth().dual_user(ctx.op_ctx()).await?;

	assert_with!(
		query.count <= 512,
		API_BAD_QUERY_PARAMETER,
		parameter = "count",
		error = "parameter too high"
	);

	assert::chat_thread_participant(&ctx, thread_id, current_user_id).await?;

	let list_res = op!([ctx] chat_message_list {
		thread_id: Some(thread_id.into()),
		ts: query.ts.map(|ts| ts.timestamp_millis()).unwrap_or_else(util::timestamp::now),
		count: query.count,
		query_direction: match query
			.query_direction
			.unwrap_or_else(|| "before".to_owned())
			.as_str()
		{
			"before" => chat_message::list::request::QueryDirection::Before,
			"after" => chat_message::list::request::QueryDirection::After,
			"before_and_after" => chat_message::list::request::QueryDirection::BeforeAndAfter,
			_ => {
				panic_with!(
					API_BAD_QUERY_PARAMETER,
					parameter = "query_direction",
					error = r#"Must be one of "before", "after", "before_and_after""#,
				);
			}
		} as i32,
	})
	.await?;

	let chat_messages = fetch::chat::messages(&ctx, current_user_id, &list_res.messages).await?;

	Ok(models::GetThreadHistoryResponse { chat_messages })
}

// MARK: GET /threads/{}/topic
pub async fn thread_topic(
	ctx: Ctx<Auth>,
	thread_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::GetThreadTopicResponse> {
	let current_user_id = ctx.auth().dual_user(ctx.op_ctx()).await?;

	// Validate permissions
	assert::chat_thread_participant(&ctx, thread_id, current_user_id).await?;

	let threads_res = op!([ctx] chat_thread_get {
		thread_ids: vec![thread_id.into()],
	})
	.await?;

	let thread = threads_res.threads.first();
	let thread = internal_unwrap!(thread);
	let topic = internal_unwrap!(thread.topic).clone();

	Ok(models::GetThreadTopicResponse {
		topic: topic.try_into()?,
	})
}

// MARK: GET /threads/{}/live
pub async fn thread_live(
	ctx: Ctx<Auth>,
	thread_id: Uuid,
	watch_index: WatchIndexQuery,
) -> GlobalResult<models::WatchThreadResponse> {
	let current_user_id = ctx.auth().dual_user(ctx.op_ctx()).await?;

	// Validate permissions
	assert::chat_thread_participant(&ctx, thread_id, current_user_id).await?;

	// Wait for an update if needed
	let (new_messages, typing_status_change, update_ts) = if let Some(anchor) =
		watch_index.to_consumer()?
	{
		let mut new_messages = Vec::new();
		let mut typing_status_change = false;

		// Listen for thread updates
		let thread_tail = tail_all!([ctx, anchor, chirp_client::TailAllConfig::wait()] chat_thread::msg::update(thread_id)).await?;

		let latest_update_ts = thread_tail.messages.last().map(|msg| msg.msg_ts());
		for event in thread_tail.messages {
			match internal_unwrap!(event.kind) {
				chat_thread::msg::update::message::Kind::ChatMessage(chat_msg) => {
					new_messages.push(chat_msg.clone());
				}
				chat_thread::msg::update::message::Kind::TypingStatus(_) => {
					typing_status_change = true;
				}
			};
		}

		(new_messages, typing_status_change, latest_update_ts)
	} else {
		Default::default()
	};
	let update_ts = update_ts.unwrap_or_else(util::timestamp::now);

	let typing_statuses = if typing_status_change {
		let topic_key = util_chat::key::typing_statuses(thread_id);
		let res = redis::pipe()
			.atomic()
			.hgetall(topic_key.clone())
			.query_async::<_, Vec<HashMap<String, Vec<u8>>>>(&mut ctx.cache_handle().redis())
			.await?;

		if let Some(typing_statuses) = res.first() {
			let user_ids = typing_statuses
				.iter()
				.map(|(user_id, _)| {
					Uuid::from_str(user_id.as_str())
						.map(Into::<common::Uuid>::into)
						.map_err(Into::<GlobalError>::into)
				})
				.collect::<GlobalResult<Vec<_>>>()?;

			// Get users
			let users_res = op!([ctx] user_get {
				user_ids: user_ids,
			})
			.await?;

			Some(
				typing_statuses
					.iter()
					.map(|(user_id, proto)| {
						let user_id = Some(Into::<common::Uuid>::into(Uuid::from_str(
							user_id.as_str(),
						)?));
						let user = users_res.users.iter().find(|u| u.user_id == user_id);
						let identity = convert::identity::handle_without_presence(
							current_user_id,
							internal_unwrap_owned!(user),
						)?;

						// Convert typing status from proto to rust type
						let status = match backend::chat::TypingStatus::decode(proto.as_slice())?
							.try_into()?
						{
							models::ChatTypingStatus::Idle(_) => None,
							status => Some(models::ChatIdentityTypingStatus { identity, status }),
						};

						GlobalResult::Ok(status)
					})
					.collect::<GlobalResult<Vec<_>>>()?
					.into_iter()
					.flatten()
					.collect::<Vec<_>>(),
			)
		} else {
			None
		}
	} else {
		None
	};

	if !new_messages.is_empty() {
		let chat_messages = fetch::chat::messages(&ctx, current_user_id, &new_messages).await?;

		Ok(models::WatchThreadResponse {
			chat_messages,
			typing_statuses,
			watch: convert::watch_response(WatchResponse::new(update_ts + 1)),
		})
	}
	// Don't call rpc services when there are no events
	else {
		Ok(models::WatchThreadResponse {
			chat_messages: Vec::new(),
			typing_statuses,
			watch: convert::watch_response(WatchResponse::new(update_ts + 1)),
		})
	}
}

// MARK: POST /threads/{}/read
pub async fn set_thread_read(
	ctx: Ctx<Auth>,
	thread_id: Uuid,
	body: models::SetThreadReadRequest,
) -> GlobalResult<models::SetThreadReadResponse> {
	let current_user_id = ctx.auth().dual_user(ctx.op_ctx()).await?;

	assert::chat_thread_participant(&ctx, thread_id, current_user_id).await?;

	msg!([ctx] chat::msg::last_read_ts_set(current_user_id, thread_id) {
		user_id: Some(current_user_id.into()),
		thread_id: Some(thread_id.into()),
		last_read_ts: body.last_read_ts.timestamp_millis(),
	})
	.await?;

	Ok(models::SetThreadReadResponse {})
}

// MARK: PUT /threads/{}/typing-status
pub async fn set_typing_status(
	ctx: Ctx<Auth>,
	thread_id: Uuid,
	body: models::SetTypingStatusRequest,
) -> GlobalResult<models::SetTypingStatusResponse> {
	let current_user_id = ctx.auth().dual_user(ctx.op_ctx()).await?;

	assert::chat_thread_participant(&ctx, thread_id, current_user_id).await?;

	let typing_status = match body.status {
		models::ChatTypingStatus::Idle(_) => backend::chat::TypingStatus {
			kind: Some(backend::chat::typing_status::Kind::Idle(
				backend::chat::typing_status::Idle {},
			)),
		},
		models::ChatTypingStatus::Typing(_) => backend::chat::TypingStatus {
			kind: Some(backend::chat::typing_status::Kind::Typing(
				backend::chat::typing_status::Typing {},
			)),
		},
	};

	op!([ctx] user_thread_typing_status_set {
		user_id: Some(current_user_id.into()),
		thread_id: Some(thread_id.into()),
		status: Some(typing_status),
		no_broadcast: false,
	})
	.await?;

	Ok(models::SetTypingStatusResponse {})
}
