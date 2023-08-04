use std::convert::TryInto;

use rivet_api::models;
use rivet_operation::prelude::*;
use types::rivet::backend;

use crate::convert;

pub fn handle(
	party: &backend::party::Party,
	games: &[convert::GameWithNamespaceIds],
) -> GlobalResult<models::PartyHandle> {
	let party_id = internal_unwrap!(party.party_id).as_uuid();

	Ok(models::PartyHandle {
		party_id,
		create_ts: util::timestamp::to_string(party.create_ts)?,
		activity: Box::new(convert::party::activity(party.state.as_ref(), games)?),
		external: Box::new(models::PartyExternalLinks {
			chat: util::route::party_chat(party_id),
		}),
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
		party_id,
		create_ts: util::timestamp::to_string(party.create_ts)?,
		party_size: party.party_size.try_into()?,
		activity: Box::new(convert::party::activity(party.state.as_ref(), games)?),
		// TODO: Only party leader should be able to see this
		publicity: Box::new(models::PartyPublicity {
			public: convert::party::publicity_level(publicity.public),
			mutual_followers: convert::party::publicity_level(publicity.friends),
			groups: convert::party::publicity_level(publicity.teams),
		}),
		external: Box::new(models::PartyExternalLinks {
			chat: util::route::party_chat(party_id),
		}),
		members: convert::party::members(current_user_id, party, members, users)?,
		// TODO: Only members of party should be able to see this
		thread_id,
	})
}

pub fn activity(
	state: Option<&backend::party::party::State>,
	games: &[convert::GameWithNamespaceIds],
) -> GlobalResult<models::PartyActivity> {
	// Fetch activity
	let activity = match state {
		None => models::PartyActivity {
			idle: Some(serde_json::json!({})),
			..Default::default()
		},
		Some(backend::party::party::State::MatchmakerFindingLobby(
			backend::party::party::StateMatchmakerFindingLobby { namespace_id, .. },
		)) => {
			let namespace_id = internal_unwrap!(namespace_id).as_uuid();
			let game = internal_unwrap_owned!(games
				.iter()
				.find(|game| game.namespace_ids.iter().any(|n| n == &namespace_id)));

			let game = convert::game::handle(&game.game)?;

			models::PartyActivity {
				matchmaker_finding_lobby: Some(Box::new(
					models::PartyActivityMatchmakerFindingLobby {
						game: Box::new(game),
					},
				)),
				..Default::default()
			}
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

			let game = convert::game::handle(&game.game)?;

			models::PartyActivity {
				matchmaker_lobby: Some(Box::new(models::PartyActivityMatchmakerLobby {
					game: Box::new(game),
					lobby: Box::new(models::PartyMatchmakerLobby { lobby_id }),
				})),
				..Default::default()
			}
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
					identity: Box::new(convert::identity::handle_without_presence(
						current_user_id,
						user,
					)?),
					is_leader: party.leader_user_id == member.user_id,

					join_ts: util::timestamp::to_string(member.create_ts)?,
					state: Box::new(match &member.state {
						None => models::PartyMemberState {
							// inactive: Some(serde_json::json!({})),
							..Default::default()
						},
						Some(backend::party::party_member::State::MatchmakerReady(
							backend::party::party_member::StateMatchmakerReady {},
						)) => models::PartyMemberState {
							// matchmaker_ready: Some(serde_json::json!({})),
							..Default::default()
						},
						Some(
							backend::party::party_member::State::MatchmakerFindingLobby(_)
							| backend::party::party_member::State::MatchmakerFindingLobbyDirect(_),
						) => models::PartyMemberState {
							matchmaker_finding_lobby: Some(serde_json::json!({})),
							..Default::default()
						},
						Some(backend::party::party_member::State::MatchmakerLobby(
							backend::party::party_member::StateMatchmakerLobby {
								player_id, ..
							},
						)) => {
							let player_id = internal_unwrap!(player_id).as_uuid();
							models::PartyMemberState {
								matchmaker_lobby: Some(Box::new(
									models::PartyMemberStateMatchmakerLobby { player_id },
								)),
								..Default::default()
							}
						}
					}),
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
