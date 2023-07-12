use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use redis_util::RedisResult;

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../../redis-scripts/member_state_resolve.lua"));
	static ref REDIS_SCRIPT_SET_PLAYER_TOKEN: redis::Script = redis::Script::new(include_str!("../../redis-scripts/member_state_resolve_set_player_token.lua"));
}

#[derive(Debug, serde::Deserialize)]
enum RedisResponse {
	#[serde(rename = "mm_finding_lobby")]
	MatchmakerFindingLobby {
		namespace_id: Uuid,
		lobby_id: Uuid,
		client_info: Option<String>,
	},
}

#[worker(name = "party-member-state-resolve")]
async fn worker(
	ctx: &OperationContext<party::msg::member_state_resolve::Message>,
) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let mut redis = ctx.redis_party().await?;

	let party_id = internal_unwrap!(ctx.party_id).as_uuid();
	let user_id = internal_unwrap!(ctx.user_id).as_uuid();

	// Create query ID and player ID for the direct seek. These will only be
	// used if player is set to find a lobby.
	let direct_query_id = Uuid::new_v4();
	let direct_player_id = Uuid::new_v4();

	let redis_res = REDIS_SCRIPT
		.arg(util::timestamp::now())
		.arg(party_id.to_string())
		.arg(user_id.to_string())
		.arg(serde_json::to_string(
			&util_party::key::party_member_config::State::MatchmakerFindingLobbyDirect {
				direct_query_id,
				player_id: direct_player_id,
				player_token: None,
			},
		)?)
		.key(util_party::key::party_config(party_id))
		.key(util_party::key::party_member_config(user_id))
		.invoke_async::<_, RedisResult<Option<String>>>(&mut redis)
		.await?;

	let response = match redis_res.as_ref().map_err(String::as_str) {
		Ok(Some(action)) => {
			// Parse response
			serde_json::from_str::<RedisResponse>(action)?
		}
		Ok(None) => {
			tracing::info!("no update to party member");
			return Ok(());
		}
		Err("PARTY_DOES_NOT_EXIST") => {
			tracing::info!("party does not exist, likely a race condition");
			return Ok(());
		}
		Err("PARTY_MEMBER_DOES_NOT_EXIST") => {
			tracing::info!("party member does not exist, likely a race condition");
			return Ok(());
		}
		Err("PARTY_MEMBER_NOT_IN_PARTY") => {
			tracing::info!("party member not in party, likely a race condition");
			return Ok(());
		}
		Err(_) => internal_panic!("unknown redis error"),
	};
	tracing::info!(?response, "redis response");

	match response {
		RedisResponse::MatchmakerFindingLobby {
			namespace_id,
			lobby_id,
			client_info,
		} => {
			let query_id = direct_query_id;
			let player_id = direct_player_id;

			// Parse client info
			let client_info = client_info
				.map(|client_info| {
					serde_json::from_str::<util_party::key::party_member_config::ClientInfo>(
						&client_info,
					)
				})
				.transpose()?
				.map(|client_info| backend::net::ClientInfo {
					user_agent: client_info.user_agent.clone(),
					remote_address: client_info.remote_address,
				});

			// Create player token
			let token_res = op!([ctx] token_create {
				issuer: "party-member-state-resolve".into(),
				token_config: Some(token::create::request::TokenConfig {
					// Has to be greater than the player register time since this
					// token is used in the player disconnect too.
					ttl: util::duration::days(90),
				}),
				refresh_token_config: None,
				client: client_info.clone(),
				kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
					entitlements: vec![
						proto::claims::Entitlement {
							kind: Some(
								proto::claims::entitlement::Kind::MatchmakerPlayer(proto::claims::entitlement::MatchmakerPlayer {
									player_id: Some(player_id.into()),
								})
							)
						}
					],
				})),
				label: Some("player".into()),
				..Default::default()
			})
			.await?;
			let token = internal_unwrap!(token_res.token);
			let token_session_id = internal_unwrap!(token_res.session_id).as_uuid();

			// Set player token. Do this before msg-mm-lobby-find so there's not a race condition
			// with changing the party member's state.
			REDIS_SCRIPT_SET_PLAYER_TOKEN
				.arg(query_id.to_string())
				.arg(&token.token)
				.key(util_party::key::party_member_config(user_id))
				.invoke_async(&mut redis)
				.await?;

			// Start find
			msg!([ctx] mm::msg::lobby_find(namespace_id, query_id) {
				namespace_id: Some(namespace_id.into()),
				query_id: Some(query_id.into()),
				join_kind: backend::matchmaker::query::JoinKind::Party as i32,
				players: vec![mm::msg::lobby_find::Player {
					player_id: Some(player_id.into()),
					token_session_id: Some(token_session_id.into()),
					client_info,
				}],
				query: Some(mm::msg::lobby_find::message::Query::Direct(backend::matchmaker::query::Direct {
					lobby_id: Some(lobby_id.into()),
				})),
			})
			.await?;

			msg!([ctx] party::msg::update(party_id) {
				party_id: Some(party_id.into()),
			})
			.await?;
		}
	}

	Ok(())
}
