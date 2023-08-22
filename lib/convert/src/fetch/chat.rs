use rivet_api::models;
use rivet_operation::prelude::*;
use types::rivet::{
	backend::{self, pkg::*},
	common,
};

use crate::{convert, fetch};

struct ChatMessagePrefetch {
	pub user_ids: Vec<common::Uuid>,
	pub namespace_ids: Vec<common::Uuid>,
}

pub struct ChatThreadPrefetch {
	pub user_ids: Vec<common::Uuid>,
	pub namespace_ids: Vec<common::Uuid>,
	pub team_ids: Vec<common::Uuid>,
}

pub async fn messages(
	ctx: &OperationContext<()>,
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

pub async fn threads(
	ctx: &OperationContext<()>,
	current_user_id: Uuid,
	threads: &[backend::chat::Thread],
	tail_messages: &[backend::chat::Message],
) -> GlobalResult<(Vec<models::ChatThread>, user::get::Response)> {
	let thread_ids = threads
		.iter()
		.map(|t| Ok(internal_unwrap_owned!(t.thread_id)))
		.collect::<GlobalResult<Vec<_>>>()?;

	// Fetch thread metadata
	let ChatThreadPrefetch {
		user_ids,
		team_ids,
		namespace_ids,
	} = prefetch_threads(threads, tail_messages)?;

	let (teams, users_res, dev_teams, (chat_last_read_ts_res, chat_thread_unread_count_res)) = tokio::try_join!(
		async {
			if !team_ids.is_empty() {
				let teams_res = op!([ctx] team_get {
					team_ids: team_ids.clone(),
				})
				.await?;

				Ok(teams_res.teams.clone())
			} else {
				Ok(Vec::new())
			}
		},
		op!([ctx] user_get {
			user_ids: user_ids.clone(),
		}),
		async {
			if !team_ids.is_empty() {
				let team_dev_get = op!([ctx] team_dev_get {
					team_ids: team_ids.clone(),
				})
				.await?;

				Ok(team_dev_get.teams.clone())
			} else {
				Ok(Vec::new())
			}
		},
		// TODO: Optimize this to be cached in Redis
		async {
			let chat_last_read_ts_res = op!([ctx] chat_last_read_ts_get {
				user_id: Some(current_user_id.into()),
				thread_ids: thread_ids.clone(),
			})
			.await?;

			// TODO: Optimize this to be cached in Redis (i.e. INCR unread count if exists on message send)
			let chat_thread_unread_count_res = op!([ctx] chat_thread_unread_count {
				user_id: Some(current_user_id.into()),
				thread_ids: thread_ids.clone(),
				read_ts_threads: chat_last_read_ts_res
					.threads
					.clone()
					.into_iter()
					.map(|v| chat_thread::unread_count::request::ReadTsThread {
						thread_id: v.thread_id,
						last_read_ts: v.last_read_ts,
					})
					.collect::<Vec<_>>(),
			})
			.await?;

			Ok((chat_last_read_ts_res, chat_thread_unread_count_res))
		}
	)?;

	Ok((
		tail_messages
			.iter()
			.flat_map(|message| {
				let thread = convert::chat::thread(
					current_user_id,
					message,
					threads,
					&users_res.users,
					&teams,
					&dev_teams,
					&[],
					&chat_last_read_ts_res.threads,
					&chat_thread_unread_count_res.threads,
				);

				match thread {
					Ok(Some(thread)) => Some(Ok(thread)),
					Ok(None) => {
						tracing::warn!(?message, "thread could not be created");
						None
					}
					Err(err) => Some(Err(err)),
				}
			})
			.collect::<GlobalResult<Vec<_>>>()?,
		users_res,
	))
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

fn prefetch_threads(
	threads: &[backend::chat::Thread],
	tail_messages: &[backend::chat::Message],
) -> GlobalResult<ChatThreadPrefetch> {
	let mut user_ids = Vec::new();
	let mut team_ids = Vec::new();

	// Prefetch all required data for building thread
	for thread in threads {
		let topic = internal_unwrap!(thread.topic);

		match internal_unwrap!(topic.kind) {
			backend::chat::topic::Kind::Team(team) => {
				team_ids.push(internal_unwrap_owned!(team.team_id));
			}
			backend::chat::topic::Kind::Direct(direct) => {
				user_ids.push(internal_unwrap_owned!(direct.user_a_id));
				user_ids.push(internal_unwrap_owned!(direct.user_b_id));
			}
		}
	}

	// Prefetch chat message info
	let ChatMessagePrefetch {
		user_ids: msg_user_ids,
		namespace_ids,
	} = prefetch_messages(tail_messages)?;
	user_ids.extend(msg_user_ids);

	Ok(ChatThreadPrefetch {
		user_ids,
		namespace_ids,
		team_ids,
	})
}
