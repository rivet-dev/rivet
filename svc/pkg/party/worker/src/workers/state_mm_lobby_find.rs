use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use redis_util::RedisResult;

lazy_static::lazy_static! {
	static ref REDIS_SCRIPT: redis::Script = redis::Script::new(include_str!("../../redis-scripts/state_mm_lobby_find.lua"));
}

mod redis_request {
	use super::Uuid;

	#[derive(Debug, serde::Serialize)]
	pub struct Request {
		pub ts: i64,
		pub state_json: String,

		/// Configurations for member matchmaker players.
		///
		/// Not all of these will be converted to players. The Redis script will
		/// return which members should actually be matchmaked.
		pub members: Vec<Member>,
	}

	#[derive(Debug, serde::Serialize)]
	pub struct Member {
		pub user_id: Uuid,
		pub state_json: String,
	}
}

#[worker(name = "party-state-mm-lobby-find")]
async fn worker(
	ctx: &OperationContext<party::msg::state_mm_lobby_find::Message>,
) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let mut redis = ctx.redis_party().await?;

	let party_id = internal_unwrap!(ctx.party_id).as_uuid();
	let namespace_id = internal_unwrap!(ctx.namespace_id).as_uuid();

	let find_query = match internal_unwrap!(ctx.query).clone() {
		party::msg::state_mm_lobby_find::message::Query::Direct(x) => {
			mm::msg::lobby_find::message::Query::Direct(x)
		}
		party::msg::state_mm_lobby_find::message::Query::LobbyGroup(x) => {
			mm::msg::lobby_find::message::Query::LobbyGroup(x)
		}
	};

	// Fetch party members
	let list_res = op!([ctx] party_member_list {
		party_ids: vec![party_id.into()],
	})
	.await?;

	let party_member_user_ids_proto = internal_unwrap_owned!(list_res.parties.first())
		.user_ids
		.clone();
	let party_member_user_ids = party_member_user_ids_proto
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	// Fetch party members for their client info
	let party_members_res = op!([ctx] party_member_get {
		user_ids: party_member_user_ids_proto,
	})
	.await?;

	// Create matchmaker players
	let mut players = Vec::new();
	let mut request_members = Vec::new();
	for &user_id in &party_member_user_ids {
		let user_id_proto = Some(user_id.into());
		let party_member = internal_unwrap_owned!(party_members_res
			.party_members
			.iter()
			.find(|party_member| party_member.user_id == user_id_proto));

		// Create player token
		let player_id = Uuid::new_v4();
		let token_res = op!([ctx] token_create {
			issuer: "party-state-mm-lobby-find".into(),
			token_config: Some(token::create::request::TokenConfig {
				// Has to be greater than the player register time since this
				// token is used in the player disconnect too.
				ttl: util::duration::days(90),
			}),
			refresh_token_config: None,
			client: party_member.client_info.clone(),
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

		// Create player
		players.push((
			user_id,
			mm::msg::lobby_find::Player {
				player_id: Some(player_id.into()),
				token_session_id: Some(token_session_id.into()),
				client_info: party_member.client_info.clone(),
			},
		));

		// Configure party member
		request_members.push(redis_request::Member {
			user_id,
			state_json: serde_json::to_string(
				&util_party::key::party_member_config::State::MatchmakerFindingLobby {
					player_id,
					player_token: token.token.clone(),
				},
			)?,
		});
	}

	// Set state
	let query_id = Uuid::new_v4();
	let redis_request = redis_request::Request {
		ts: util::timestamp::now(),
		state_json: serde_json::to_string(
			&util_party::key::party_config::State::MatchmakerFindingLobby {
				namespace_id,
				query_id,
			},
		)?,
		members: request_members,
	};
	let mut script = REDIS_SCRIPT.prepare_invoke();
	script.arg(serde_json::to_string(&redis_request)?);
	script.key(util_party::key::party_config(party_id));
	for &user_id in &party_member_user_ids {
		script.key(util_party::key::party_member_config(user_id));
	}
	let redis_result = script
		.invoke_async::<_, RedisResult<Vec<String>>>(&mut redis)
		.await?;
	let joining_user_ids = match redis_result.as_ref().map_err(String::as_str) {
		Ok(joining_user_ids) => joining_user_ids
			.iter()
			.map(|x| util::uuid::parse(x))
			.collect::<Result<Vec<_>, _>>()?,
		Err("PARTY_DOES_NOT_EXIST") => {
			tracing::info!("party does not exist, likely removed in race condition");
			return Ok(());
		}
		Err(_) => internal_panic!("unknown redis error"),
	};

	// Filter out the players that are not joining
	let players = players
		.into_iter()
		.filter(|(user_id, _)| joining_user_ids.contains(user_id))
		.map(|(_, player)| player)
		.collect::<Vec<_>>();

	// Find lobby
	//
	// Do this after updating the party to clean up the party appropriately.
	msg!([ctx] mm::msg::lobby_find(namespace_id, query_id) {
		namespace_id: Some(namespace_id.into()),
		query_id: Some(query_id.into()),
		join_kind: backend::matchmaker::query::JoinKind::Party as i32,
		players: players,
		query: Some(find_query),
		..Default::default()
	})
	.await?;

	// Update party & party members
	msg!([ctx] party::msg::update(party_id) {
		party_id: Some(party_id.into()),
	})
	.await?;

	// Send party activity change message
	let chat_message_id = Uuid::new_v4();
	op!([ctx] chat_message_create_with_topic {
		chat_message_id: Some(chat_message_id.into()),
		topic: Some(backend::chat::Topic {
			kind: Some(backend::chat::topic::Kind::Party(
				backend::chat::topic::Party {
					party_id: Some(party_id.into()),
				},
			)),
		}),
		send_ts: util::timestamp::now(),
		body: Some(backend::chat::MessageBody {
			kind: Some(backend::chat::message_body::Kind::PartyActivityChange(
				backend::chat::message_body::PartyActivityChange {
					state: Some(backend::chat::message_body::party_activity_change::State::MatchmakerFindingLobby(
						backend::party::party::StateMatchmakerFindingLobby {
							namespace_id: Some(namespace_id.into()),
							query_id: Some(query_id.into()),
						},
					)),
				}
			)),
		}),
	})
	.await?;

	Ok(())
}
