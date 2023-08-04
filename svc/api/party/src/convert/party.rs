use std::convert::TryInto;

use proto::backend;
use rivet_operation::prelude::*;
use rivet_party_server::models;

use crate::convert;

pub fn handle(
	party: &backend::party::Party,
	games: &[convert::GameWithNamespaceIds],
) -> GlobalResult<models::PartyHandle> {
	let party_id = internal_unwrap!(party.party_id).as_uuid();

	Ok(models::PartyHandle {
		party_id: party_id.to_string(),
		create_ts: util::timestamp::to_chrono(party.create_ts)?,
		activity: convert::party::activity(party.state.as_ref(), games)?,
		external: models::PartyExternalLinks {
			chat: util::route::party_chat(party_id),
		},
	})
}

pub fn summary(
	current_user_id: Uuid,
	party: &backend::party::Party,
	games: &[convert::GameWithNamespaceIds],
	members: &[backend::party::PartyMember],
	users: &[backend::user::User],
	threads: &[backend::chat::Thread],
) -> GlobalResult<models::PartySummary> {
	let publicity = internal_unwrap!(party.publicity);
	let thread = internal_unwrap_owned!(threads.iter().find(|thread| match &thread.topic {
		Some(backend::chat::Topic {
			kind: Some(backend::chat::topic::Kind::Party(p)),
		}) => p.party_id == party.party_id,
		_ => false,
	}));

	let party_id = internal_unwrap!(party.party_id).as_uuid();
	let thread_id = internal_unwrap!(thread.thread_id).as_uuid();

	Ok(models::PartySummary {
		party_id: party_id.to_string(),
		create_ts: util::timestamp::to_chrono(party.create_ts)?,
		party_size: party.party_size.try_into()?,
		activity: convert::party::activity(party.state.as_ref(), games)?,
		publicity: models::PartyPublicity {
			public: convert::party::publicity_level(publicity.public),
			mutual_followers: convert::party::publicity_level(publicity.friends),
			groups: convert::party::publicity_level(publicity.teams),
		},
		external: models::PartyExternalLinks {
			chat: util::route::party_chat(party_id),
		},
		members: convert::party::members(current_user_id, party, members, users)?,
		thread_id: thread_id.to_string(),
	})
}

pub fn profile(
	current_user_id: Uuid,
	party: &backend::party::Party,
	games: &[convert::GameWithNamespaceIds],
	members: &[backend::party::PartyMember],
	users: &[backend::user::User],
	thread: &backend::chat::Thread,
	invites: &[backend::party::Invite],
) -> GlobalResult<models::PartyProfile> {
	let party_id = internal_unwrap!(party.party_id).as_uuid();
	let thread_id = internal_unwrap!(thread.thread_id).as_uuid();

	Ok(models::PartyProfile {
		party_id: party_id.to_string(),
		create_ts: util::timestamp::to_chrono(party.create_ts)?,
		party_size: party.party_size.try_into()?,
		activity: convert::party::activity(party.state.as_ref(), games)?,
		publicity: publicity(internal_unwrap!(party.publicity)),
		external: models::PartyExternalLinks {
			chat: util::route::party_chat(party_id),
		},
		members: convert::party::members(current_user_id, party, members, users)?,
		thread_id: thread_id.to_string(),
		invites: if internal_unwrap!(party.leader_user_id).as_uuid() == current_user_id {
			convert::party::invites(invites)?
		} else {
			Vec::new()
		},
	})
}

fn activity(
	state: Option<&backend::party::party::State>,
	games: &[convert::GameWithNamespaceIds],
) -> GlobalResult<models::PartyActivity> {
	// Fetch activity
	let activity = match &state {
		None => models::PartyActivity::Idle(models::PartyActivityIdle {}),
		Some(backend::party::party::State::MatchmakerFindingLobby(
			backend::party::party::StateMatchmakerFindingLobby { namespace_id, .. },
		)) => {
			let namespace_id = internal_unwrap!(namespace_id).as_uuid();
			let game = internal_unwrap_owned!(games
				.iter()
				.find(|game| game.namespace_ids.iter().any(|n| n == &namespace_id)));

			let game = convert::game_handle(&game.game)?;

			models::PartyActivity::MatchmakerFindingLobby(
				models::PartyActivityMatchmakerFindingLobby { game },
			)
		}
		Some(backend::party::party::State::MatchmakerLobby(
			backend::party::party::StateMatchmakerLobby {
				namespace_id,
				lobby_id,
			},
		)) => {
			let namespace_id = internal_unwrap!(namespace_id).as_uuid();
			let game = internal_unwrap_owned!(games
				.iter()
				.find(|game| game.namespace_ids.iter().any(|n| n == &namespace_id)));
			let lobby_id = internal_unwrap!(lobby_id).as_uuid();

			let game = convert::game_handle(&game.game)?;

			models::PartyActivity::MatchmakerLobby(models::PartyActivityMatchmakerLobby {
				game,
				lobby: models::PartyMatchmakerLobby {
					lobby_id: lobby_id.to_string(),
				},
			})
		}
	};

	Ok(activity)
}

pub fn publicity_level(level: i32) -> models::PartyPublicityLevel {
	match backend::party::party::PublicityLevel::from_i32(level) {
		None | Some(backend::party::party::PublicityLevel::None) => {
			models::PartyPublicityLevel::None
		}
		Some(backend::party::party::PublicityLevel::View) => models::PartyPublicityLevel::View,
		Some(backend::party::party::PublicityLevel::Join) => models::PartyPublicityLevel::Join,
	}
}

pub fn publicity_level_to_proto(publicity: models::PartyPublicityLevel) -> i32 {
	match publicity {
		models::PartyPublicityLevel::Unknown(_) | models::PartyPublicityLevel::None => {
			backend::party::party::PublicityLevel::None as i32
		}
		models::PartyPublicityLevel::View => backend::party::party::PublicityLevel::View as i32,
		models::PartyPublicityLevel::Join => backend::party::party::PublicityLevel::Join as i32,
	}
}

pub fn publicity(publicity: &backend::party::party::Publicity) -> models::PartyPublicity {
	models::PartyPublicity {
		public: publicity_level(publicity.public),
		mutual_followers: publicity_level(publicity.friends),
		groups: publicity_level(publicity.teams),
	}
}

pub fn members(
	current_user_id: Uuid,
	party: &backend::party::Party,
	members: &[backend::party::PartyMember],
	users: &[backend::user::User],
) -> GlobalResult<Vec<models::PartyMemberSummary>> {
	Ok(members
		.iter()
		.map(|member| {
			if member.party_id == party.party_id {
				let user =
					internal_unwrap_owned!(users.iter().find(|x| x.user_id == member.user_id));

				Ok(Some(models::PartyMemberSummary {
					identity: convert::identity::handle_without_presence(current_user_id, user)?,
					is_leader: party.leader_user_id == member.user_id,
					join_ts: util::timestamp::to_chrono(member.create_ts)?,
					state: match &member.state {
						None => {
							models::PartyMemberState::Inactive(models::PartyMemberStateInactive {})
						}
						Some(backend::party::party_member::State::MatchmakerReady(
							backend::party::party_member::StateMatchmakerReady {},
						)) => models::PartyMemberState::MatchmakerReady(
							models::PartyMemberStateMatchmakerReady {},
						),
						Some(
							backend::party::party_member::State::MatchmakerFindingLobby(_)
							| backend::party::party_member::State::MatchmakerFindingLobbyDirect(_),
						) => models::PartyMemberState::MatchmakerFindingLobby(
							models::PartyMemberStateMatchmakerFindingLobby {},
						),
						Some(backend::party::party_member::State::MatchmakerLobby(
							backend::party::party_member::StateMatchmakerLobby {
								player_id, ..
							},
						)) => {
							let player_id = internal_unwrap!(player_id).as_uuid();
							models::PartyMemberState::MatchmakerLobby(
								models::PartyMemberStateMatchmakerLobby {
									player_id: player_id.to_string(),
								},
							)
						}
					},
				}))
			} else {
				Ok(None)
			}
		})
		.collect::<GlobalResult<Vec<_>>>()?
		.into_iter()
		.flatten()
		.collect::<Vec<_>>())
}

pub fn invites(invites: &[backend::party::Invite]) -> GlobalResult<Vec<models::PartyInvite>> {
	invites
		.iter()
		.map(|invite| -> GlobalResult<_> {
			Ok(models::PartyInvite {
				invite_id: internal_unwrap!(invite.invite_id).as_uuid().to_string(),
				create_ts: util::timestamp::to_chrono(invite.create_ts)?,
				token: invite.token.to_owned(),
				alias: invite
					.alias
					.as_ref()
					.map(|alias| -> GlobalResult<models::PartyInviteAlias> {
						Ok(models::PartyInviteAlias {
							namespace_id: internal_unwrap!(alias.namespace_id)
								.as_uuid()
								.to_string(),
							alias: alias.alias.clone(),
						})
					})
					.transpose()?,
				external: models::PartyInviteExternalLinks {
					invite: util::route::party_invite(invite.token.as_str()),
				},
			})
		})
		.collect::<GlobalResult<Vec<_>>>()
}
