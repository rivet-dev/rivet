use proto::backend;
use rivet_group_server::models;
use rivet_operation::prelude::*;

use crate::convert;

pub fn handle(
	party: &backend::party::Party,
	games: &[convert::GameWithNamespaceIds],
) -> GlobalResult<models::PartyHandle> {
	let party_id = internal_unwrap!(party.party_id).as_uuid();

	Ok(models::PartyHandle {
		party_id: party_id.to_string(),
		create_ts: util::timestamp::to_chrono(party.create_ts)?,
		activity: convert::party::activity(party, games)?,
		external: models::PartyExternalLinks {
			chat: util::route::party_chat(&party_id),
		},
	})
}

fn activity(
	party: &backend::party::Party,
	games: &[convert::GameWithNamespaceIds],
) -> GlobalResult<models::PartyActivity> {
	// Fetch activity
	let activity = match &party.state {
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
