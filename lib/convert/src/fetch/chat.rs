use rivet_api::models;
use rivet_operation::prelude::*;
use types::rivet::{
	backend::{self, pkg::*},
	common,
};

use crate::{convert, fetch};

struct ChatMessagePrefetch {
	pub user_ids: Vec<common::Uuid>,
	pub party_ids: Vec<common::Uuid>,
	pub party_invite_ids: Vec<common::Uuid>,
	pub namespace_ids: Vec<common::Uuid>,
}

pub struct ChatThreadPrefetch {
	pub user_ids: Vec<common::Uuid>,
	pub party_ids: Vec<common::Uuid>,
	pub party_invite_ids: Vec<common::Uuid>,
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
		party_ids,
		party_invite_ids,
		namespace_ids,
	} = prefetch_messages(messages)?;
	let (users_res, party_invites_res, (parties, games)) = tokio::try_join!(
		op!([ctx] user_get {
			user_ids: user_ids,
		}),
		op!([ctx] party_invite_get {
			invite_ids: party_invite_ids,
		}),
		fetch::party::parties_and_games(ctx, party_ids, namespace_ids),
	)?;

	messages
		.iter()
		.map(|message| {
			convert::chat::message(
				current_user_id,
				message,
				&users_res.users,
				&parties.parties,
				&party_invites_res.invites,
				&games,
			)
		})
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
		party_ids,
		party_invite_ids,
		team_ids,
		namespace_ids,
	} = prefetch_threads(threads, tail_messages)?;

	let (
		(parties, games),
		party_invites,
		teams,
		users_res,
		dev_teams,
		(chat_last_read_ts_res, chat_thread_unread_count_res),
	) = tokio::try_join!(
		fetch::party::parties_and_games(ctx, party_ids, namespace_ids),
		async {
			if !party_invite_ids.is_empty() {
				let invites_res = op!([ctx] party_invite_get {
					invite_ids: party_invite_ids,
				})
				.await?;

				Ok(invites_res.invites.clone())
			} else {
				Ok(Vec::new())
			}
		},
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
					&parties.parties,
					&party_invites,
					&teams,
					&dev_teams,
					&games,
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
	let mut party_ids = Vec::<common::Uuid>::new();
	let mut party_invite_ids = Vec::<common::Uuid>::new();
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
			backend_body::Kind::PartyInvite(backend_body::PartyInvite {
				sender_user_id,
				party_id,
				invite_id,
				..
			}) => {
				user_ids.push(*internal_unwrap!(sender_user_id));
				party_ids.push(*internal_unwrap!(party_id));

				if let Some(invite_id) = invite_id {
					party_invite_ids.push(*invite_id);
				}
			}
			backend_body::Kind::PartyJoinRequest(backend_body::PartyJoinRequest {
				sender_user_id,
			}) => {
				user_ids.push(*internal_unwrap!(sender_user_id));
			}
			backend_body::Kind::PartyJoin(backend_body::PartyJoin { user_id }) => {
				user_ids.push(*internal_unwrap!(user_id));
			}
			backend_body::Kind::PartyLeave(backend_body::PartyLeave { user_id }) => {
				user_ids.push(*internal_unwrap!(user_id));
			}
			backend_body::Kind::PartyActivityChange(backend_body::PartyActivityChange {
				state,
			}) => match state {
				Some(backend_body::party_activity_change::State::MatchmakerFindingLobby(
					backend::party::party::StateMatchmakerFindingLobby { namespace_id, .. },
				)) => namespace_ids.push(*internal_unwrap!(namespace_id)),
				Some(backend_body::party_activity_change::State::MatchmakerLobby(
					backend::party::party::StateMatchmakerLobby { namespace_id, .. },
				)) => namespace_ids.push(*internal_unwrap!(namespace_id)),
				_ => {}
			},
			backend_body::Kind::ChatCreate(_) | backend_body::Kind::UserFollow(_) => {}
		}
	}

	Ok(ChatMessagePrefetch {
		user_ids,
		party_ids,
		party_invite_ids,
		namespace_ids,
	})
}

fn prefetch_threads(
	threads: &[backend::chat::Thread],
	tail_messages: &[backend::chat::Message],
) -> GlobalResult<ChatThreadPrefetch> {
	let mut user_ids = Vec::new();
	let mut party_ids = Vec::new();
	let mut team_ids = Vec::new();

	// Prefetch all required data for building thread
	for thread in threads {
		let topic = internal_unwrap!(thread.topic);

		match internal_unwrap!(topic.kind) {
			backend::chat::topic::Kind::Team(team) => {
				team_ids.push(internal_unwrap_owned!(team.team_id));
			}
			backend::chat::topic::Kind::Party(party) => {
				party_ids.push(internal_unwrap_owned!(party.party_id));
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
		party_ids: msg_party_ids,
		party_invite_ids,
		namespace_ids,
	} = prefetch_messages(tail_messages)?;
	user_ids.extend(msg_user_ids);
	party_ids.extend(msg_party_ids);

	Ok(ChatThreadPrefetch {
		user_ids,
		party_ids,
		party_invite_ids,
		namespace_ids,
		team_ids,
	})
}
