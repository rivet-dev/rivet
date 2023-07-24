use rivet_api::models;
use rivet_operation::prelude::*;
use types::rivet::backend::{self, pkg::*};

use crate::convert;

pub fn message(
	current_user_id: Uuid,
	message: &backend::chat::Message,
	users: &[backend::user::User],
	parties: &[backend::party::Party],
	party_invites: &[backend::party::Invite],
	games: &[convert::GameWithNamespaceIds],
) -> GlobalResult<models::ChatMessage> {
	// Read body message
	let backend_body_kind = internal_unwrap!(message.body);
	let backend_body_kind = internal_unwrap!(backend_body_kind.kind);

	// Build message body
	let msg_body = {
		use backend::chat::message_body as backend_body;

		match backend_body_kind {
			backend_body::Kind::Custom(backend_body::Custom {
				sender_user_id: _,
				plugin_id: _,
				body: _,
			}) => {
				internal_panic!("Unimplemented");
			}
			backend_body::Kind::Text(backend_body::Text {
				sender_user_id,
				body,
			}) => {
				let sender = internal_unwrap_owned!(users
					.iter()
					.find(|user| &user.user_id == sender_user_id));

				models::ChatMessageBody {
					text: Some(Box::new(models::ChatMessageBodyText {
						sender: Box::new(convert::identity::handle_without_presence(
							current_user_id,
							sender,
						)?),
						body: body.to_owned(),
					})),
					..Default::default()
				}
			}
			backend_body::Kind::ChatCreate(backend_body::ChatCreate {}) => {
				models::ChatMessageBody {
					chat_create: Some(serde_json::json!({})),
					..Default::default()
				}
			}
			backend_body::Kind::Deleted(backend_body::Deleted { sender_user_id }) => {
				let sender = internal_unwrap_owned!(users
					.iter()
					.find(|user| &user.user_id == sender_user_id));

				models::ChatMessageBody {
					deleted: Some(Box::new(models::ChatMessageBodyDeleted {
						sender: Box::new(convert::identity::handle_without_presence(
							current_user_id,
							sender,
						)?),
					})),
					..Default::default()
				}
			}
			backend_body::Kind::UserFollow(backend_body::UserFollow {}) => {
				models::ChatMessageBody {
					identity_follow: Some(serde_json::json!({})),
					..Default::default()
				}
			}
			backend_body::Kind::TeamJoin(backend_body::TeamJoin { user_id }) => {
				let user =
					internal_unwrap_owned!(users.iter().find(|user| &user.user_id == user_id));

				models::ChatMessageBody {
					group_join: Some(Box::new(models::ChatMessageBodyGroupJoin {
						identity: Box::new(convert::identity::handle_without_presence(
							current_user_id,
							user,
						)?),
					})),
					..Default::default()
				}
			}
			backend_body::Kind::TeamLeave(backend_body::TeamLeave { user_id }) => {
				let user =
					internal_unwrap_owned!(users.iter().find(|user| &user.user_id == user_id));

				models::ChatMessageBody {
					group_leave: Some(Box::new(models::ChatMessageBodyGroupLeave {
						identity: Box::new(convert::identity::handle_without_presence(
							current_user_id,
							user,
						)?),
					})),
					..Default::default()
				}
			}
			backend_body::Kind::TeamMemberKick(backend_body::TeamMemberKick { user_id }) => {
				let user =
					internal_unwrap_owned!(users.iter().find(|user| &user.user_id == user_id));

				models::ChatMessageBody {
					group_member_kick: Some(Box::new(models::ChatMessageBodyGroupMemberKick {
						identity: Box::new(convert::identity::handle_without_presence(
							current_user_id,
							user,
						)?),
					})),
					..Default::default()
				}
			}
			backend_body::Kind::PartyInvite(backend_body::PartyInvite {
				sender_user_id,
				party_id,
				invite_id,
				invite_token,
			}) => {
				let sender = internal_unwrap_owned!(users
					.iter()
					.find(|user| &user.user_id == sender_user_id));

				let party = parties.iter().find(|party| &party.party_id == party_id);
				let invite = party_invites
					.iter()
					.find(|invite| &invite.invite_id == invite_id);

				models::ChatMessageBody {
					party_invite: Some(Box::new(models::ChatMessageBodyPartyInvite {
						sender: Box::new(convert::identity::handle_without_presence(
							current_user_id,
							sender,
						)?),
						party: party
							.map(|party| convert::party::handle(party, games))
							.transpose()?
							.map(Box::new),
						invite_token: (party.is_some() && invite.is_some())
							.then(|| invite_token.to_owned()),
					})),
					..Default::default()
				}
			}
			backend_body::Kind::PartyJoinRequest(backend_body::PartyJoinRequest {
				sender_user_id,
			}) => {
				let sender = internal_unwrap_owned!(users
					.iter()
					.find(|user| &user.user_id == sender_user_id));

				models::ChatMessageBody {
					party_join_request: Some(Box::new(models::ChatMessageBodyPartyJoinRequest {
						sender: Box::new(convert::identity::handle_without_presence(
							current_user_id,
							sender,
						)?),
					})),
					..Default::default()
				}
			}
			backend_body::Kind::PartyJoin(backend_body::PartyJoin { user_id }) => {
				let user =
					internal_unwrap_owned!(users.iter().find(|user| &user.user_id == user_id));

				models::ChatMessageBody {
					party_join: Some(Box::new(models::ChatMessageBodyPartyJoin {
						identity: Box::new(convert::identity::handle_without_presence(
							current_user_id,
							user,
						)?),
					})),
					..Default::default()
				}
			}
			backend_body::Kind::PartyLeave(backend_body::PartyLeave { user_id }) => {
				let user =
					internal_unwrap_owned!(users.iter().find(|user| &user.user_id == user_id));

				models::ChatMessageBody {
					party_leave: Some(Box::new(models::ChatMessageBodyPartyLeave {
						identity: Box::new(convert::identity::handle_without_presence(
							current_user_id,
							user,
						)?),
					})),
					..Default::default()
				}
			}
			backend_body::Kind::PartyActivityChange(backend_body::PartyActivityChange {
				state,
			}) => models::ChatMessageBody {
				// TODO:
				// party_activity_change: Some(Box::new(models::ChatMessageBodyPartyActivityChange {
				// 	activity: Box::new(convert::party::activity(
				// 		state.clone().map(ApiInto::api_into).as_ref(),
				// 		games,
				// 	)?),
				// })),
				..Default::default()
			},
		}
	};

	Ok(models::ChatMessage {
		chat_message_id: internal_unwrap!(message.chat_message_id).as_uuid(),
		thread_id: internal_unwrap!(message.thread_id).as_uuid(),
		send_ts: util::timestamp::to_string(message.send_ts)?,
		body: Box::new(msg_body),
	})
}

// Returns `None` when the thread no longer exists
pub fn thread(
	current_user_id: Uuid,
	tail_message: &backend::chat::Message,
	threads: &[backend::chat::Thread],
	users: &[backend::user::User],
	parties: &[backend::party::Party],
	party_invites: &[backend::party::Invite],
	teams: &[backend::team::Team],
	dev_teams: &[backend::team::DevTeam],
	games: &[convert::GameWithNamespaceIds],
	last_read_threads: &[chat::last_read_ts_get::response::Thread],
	unread_count_threads: &[chat_thread::unread_count::response::ThreadTail],
) -> GlobalResult<Option<models::ChatThread>> {
	let thread = internal_unwrap_owned!(threads
		.iter()
		.find(|t| t.thread_id == tail_message.thread_id));

	let topic = topic_context(
		current_user_id,
		users,
		parties,
		teams,
		dev_teams,
		games,
		internal_unwrap!(thread.topic),
	)?;

	topic
		.map(|topic| {
			let thread_id = internal_unwrap!(thread.thread_id).as_uuid();

			Ok(models::ChatThread {
				thread_id,
				create_ts: util::timestamp::to_string(thread.create_ts)?,
				topic: Box::new(topic),
				tail_message: Some(Box::new(message(
					current_user_id,
					tail_message,
					users,
					parties,
					party_invites,
					games,
				)?)),
				last_read_ts: util::timestamp::to_string(
					last_read_threads
						.iter()
						.find(|t| t.thread_id == thread.thread_id)
						.map(|t| t.last_read_ts)
						.unwrap_or_default(),
				)?,
				unread_count: unread_count_threads
					.iter()
					.find(|t| t.thread_id == thread.thread_id)
					.map(|t| t.unread_count)
					.unwrap_or_default() as i64,
				external: Box::new(models::ChatThreadExternalLinks {
					chat: util::route::thread(thread_id),
				}),
			})
		})
		.transpose()
}

// Returns `None` when the thread no longer exists
fn topic_context(
	current_user_id: Uuid,
	users: &[backend::user::User],
	parties: &[backend::party::Party],
	teams: &[backend::team::Team],
	dev_teams: &[backend::team::DevTeam],
	games: &[convert::GameWithNamespaceIds],
	topic: &backend::chat::Topic,
) -> GlobalResult<Option<models::ChatTopic>> {
	let topic_kind = internal_unwrap!(topic.kind);

	let topic_context = match topic_kind {
		backend::chat::topic::Kind::Team(team) => {
			let _team_id = internal_unwrap!(team.team_id).as_uuid();

			let team = teams.iter().find(|ug| ug.team_id == team.team_id);

			team.map(|team| {
				let is_developer = dev_teams
					.iter()
					.any(|dev_team| team.team_id == dev_team.team_id);

				GlobalResult::Ok(models::ChatTopic {
					group: Some(Box::new(models::ChatTopicGroup {
						group: Box::new(convert::group::handle(team, is_developer)?),
					})),
					..Default::default()
				})
			})
			.transpose()?
		}
		backend::chat::topic::Kind::Party(party) => {
			let party = parties.iter().find(|p| p.party_id == party.party_id);

			party
				.map(|party| {
					GlobalResult::Ok(models::ChatTopic {
						party: Some(Box::new(models::ChatTopicParty {
							party: Box::new(convert::party::handle(party, games)?),
						})),
						..Default::default()
					})
				})
				.transpose()?
		}
		backend::chat::topic::Kind::Direct(direct) => {
			let user_a = users.iter().find(|u| u.user_id == direct.user_a_id);
			let user_b = users.iter().find(|u| u.user_id == direct.user_b_id);

			match user_a.zip(user_b) {
				Some((user_a, user_b)) => Some(models::ChatTopic {
					direct: Some(Box::new(models::ChatTopicDirect {
						identity_a: Box::new(convert::identity::handle_without_presence(
							current_user_id,
							user_a,
						)?),
						identity_b: Box::new(convert::identity::handle_without_presence(
							current_user_id,
							user_b,
						)?),
					})),
					..Default::default()
				}),
				None => {
					tracing::warn!("direct chat participants no longer exist");
					None
				}
			}
		}
	};

	Ok(topic_context)
}
