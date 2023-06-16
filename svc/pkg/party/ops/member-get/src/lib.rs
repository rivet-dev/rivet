use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[operation(name = "party-member-get")]
async fn handle(
	ctx: OperationContext<party::member_get::Request>,
) -> GlobalResult<party::member_get::Response> {
	// TODO:
	return Ok(party::member_get::Response {
		party_members: Vec::new(),
	});

	let mut redis = ctx.redis_party().await?;
	let user_ids = ctx
		.user_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	// Query all party members
	let mut pipe = redis::pipe();
	for &user_id in &user_ids {
		pipe.cmd("JSON.GET")
			.arg(util_party::key::party_member_config(user_id));
	}

	// Fetch & convert party members
	let party_members = pipe
		.query_async::<_, Vec<Option<String>>>(&mut redis)
		.await?
		.into_iter()
		.zip(user_ids.iter())
		.filter_map(|(json, user_id)| json.map(|json| (json, user_id)))
		.filter_map(|(json, &user_id)| {
			match serde_json::from_str::<util_party::key::party_member_config::Config>(&json) {
				Ok(x) => Some((user_id, x)),
				Err(err) => {
					tracing::error!(?user_id, ?err, ?json, "failed to parse party member config");
					None
				}
			}
		})
		.map(|(user_id, data)| {
			use util_party::key::party_member_config::State;
			backend::party::PartyMember {
				party_id: Some(data.party_id.into()),
				user_id: Some(user_id.into()),
				create_ts: data.create_ts,
				state_change_ts: data.state_change_ts,
				state: match data.state {
					State::Inactive {} => None,
					State::MatchmakerReady {} => {
						Some(backend::party::party_member::State::MatchmakerReady(
							backend::party::party_member::StateMatchmakerReady {},
						))
					}
					State::MatchmakerFindingLobby {
						player_id,
						player_token,
					} => Some(backend::party::party_member::State::MatchmakerFindingLobby(
						backend::party::party_member::StateMatchmakerFindingLobby {
							player_id: Some(player_id.into()),
							player_token,
						},
					)),
					State::MatchmakerFindingLobbyDirect {
						direct_query_id,
						player_id,
						player_token,
					} => Some(
						backend::party::party_member::State::MatchmakerFindingLobbyDirect(
							backend::party::party_member::StateMatchmakerFindingLobbyDirect {
								direct_query_id: Some(direct_query_id.into()),
								player_id: Some(player_id.into()),
								player_token,
							},
						),
					),
					State::MatchmakerLobby {
						player_id,
						player_token,
					} => Some(backend::party::party_member::State::MatchmakerLobby(
						backend::party::party_member::StateMatchmakerLobby {
							player_id: Some(player_id.into()),
							player_token,
						},
					)),
				},
				client_info: data
					.client_info
					.map(|client_info| backend::net::ClientInfo {
						user_agent: client_info.user_agent.clone(),
						remote_address: client_info.remote_address,
					}),
			}
		})
		.collect::<Vec<_>>();

	Ok(party::member_get::Response { party_members })
}
