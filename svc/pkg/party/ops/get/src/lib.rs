use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "party-get")]
async fn handle(ctx: OperationContext<party::get::Request>) -> GlobalResult<party::get::Response> {
	// TODO:
	return Ok(party::get::Response {
		parties: Vec::new(),
	});

	let mut redis = ctx.redis_party().await?;
	let party_ids = ctx
		.party_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	// Query all parties
	let mut pipe = redis::pipe();
	for &party_id in &party_ids {
		pipe.cmd("JSON.GET")
			.arg(util_party::key::party_config(party_id));
	}

	// Fetch & convert parties
	let parties = pipe
		.query_async::<_, Vec<Option<String>>>(&mut redis)
		.await?
		.into_iter()
		.zip(party_ids.iter())
		.filter_map(|(json, party_id)| json.map(|json| (json, party_id)))
		.filter_map(|(json, &party_id)| {
			match serde_json::from_str::<util_party::key::party_config::Config>(&json) {
				Ok(x) => Some((party_id, x)),
				Err(err) => {
					tracing::error!(?party_id, ?err, ?json, "failed to parse party config");
					None
				}
			}
		})
		.map(|(party_id, data)| {
			use util_party::key::party_config::State;
			backend::party::Party {
				party_id: Some(party_id.into()),
				create_ts: data.create_ts,
				leader_user_id: data.leader_user_id.map(Into::into),
				party_size: data.party_size,
				state_change_ts: data.state_change_ts,
				state: match data.state {
					State::Idle {} => None,
					State::MatchmakerFindingLobby {
						namespace_id,
						query_id,
					} => Some(backend::party::party::State::MatchmakerFindingLobby(
						backend::party::party::StateMatchmakerFindingLobby {
							namespace_id: Some(namespace_id.into()),
							query_id: Some(query_id.into()),
						},
					)),
					State::MatchmakerLobby {
						namespace_id,
						lobby_id,
					} => Some(backend::party::party::State::MatchmakerLobby(
						backend::party::party::StateMatchmakerLobby {
							namespace_id: Some(namespace_id.into()),
							lobby_id: Some(lobby_id.into()),
						},
					)),
				},
				publicity: Some(backend::party::party::Publicity {
					public: convert_publicity(data.publicity.public) as i32,
					friends: convert_publicity(data.publicity.friends) as i32,
					teams: convert_publicity(data.publicity.teams) as i32,
				}),
			}
		})
		.collect::<Vec<_>>();

	Ok(party::get::Response { parties })
}

fn convert_publicity(
	level: util_party::key::party_config::PublicityLevel,
) -> backend::party::party::PublicityLevel {
	match level {
		util_party::key::party_config::PublicityLevel::None => {
			backend::party::party::PublicityLevel::None
		}
		util_party::key::party_config::PublicityLevel::View => {
			backend::party::party::PublicityLevel::View
		}
		util_party::key::party_config::PublicityLevel::Join => {
			backend::party::party::PublicityLevel::Join
		}
	}
}
