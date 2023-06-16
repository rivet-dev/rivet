use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use util_party::key::party_config::PublicityLevel;

#[worker(name = "party-create")]
async fn worker(ctx: OperationContext<party::msg::create::Message>) -> GlobalResult<()> {
	// TODO:
	return Ok(());

	let party_id = internal_unwrap!(ctx.party_id).as_uuid();
	let leader_user_id = internal_unwrap!(ctx.leader_user_id).as_uuid();

	internal_assert!(ctx.party_size >= 2);
	internal_assert!(ctx.party_size <= 16);

	let (state, party_member_state) = match &ctx.initial_state {
		None => (util_party::key::party_config::State::Idle {}, None),
		Some(party::msg::create::message::InitialState::MatchmakerLobby(state)) => (
			util_party::key::party_config::State::MatchmakerLobby {
				namespace_id: internal_unwrap!(state.namespace_id).as_uuid(),
				lobby_id: internal_unwrap!(state.lobby_id).as_uuid(),
			},
			Some(
				party::msg::member_create::message::InitialState::MatchmakerLobby(
					party::msg::member_create::message::StateMatchmakerLobby {
						player_id: state.leader_player_id,
						player_token: state.leader_player_token.clone(),
					},
				),
			),
		),
	};

	// Derive default publicity
	let req_publicity = ctx.publicity.clone().unwrap_or_default();
	let mut publicity = util_party::key::party_config::Publicity::default();
	if let Some(public) = req_publicity.public {
		publicity.public = convert_publicity(public);
	}
	if let Some(friends) = req_publicity.friends {
		publicity.friends = convert_publicity(friends);
	}
	if let Some(teams) = req_publicity.teams {
		publicity.teams = convert_publicity(teams);
	}

	let config = util_party::key::party_config::Config {
		party_id,
		create_ts: ctx.ts(),
		leader_user_id: Some(leader_user_id),
		party_size: ctx.party_size,
		state_change_ts: util::timestamp::now(),
		state,
		publicity,
	};

	redis::cmd("JSON.SET")
		.arg(util_party::key::party_config(party_id))
		.arg("$")
		.arg(serde_json::to_string(&config)?)
		.query_async::<_, ()>(&mut ctx.redis_party().await?)
		.await?;

	// Create new chat with party
	op!([ctx] chat_message_create_with_topic {
		chat_message_id: Some(Uuid::new_v4().into()),
		topic: Some(backend::chat::Topic {
			kind: Some(backend::chat::topic::Kind::Party(
				backend::chat::topic::Party {
					party_id: Some(party_id.into()),
				},
			)),
		}),
		send_ts: util::timestamp::now(),
		body: Some(backend::chat::MessageBody {
			kind: Some(backend::chat::message_body::Kind::ChatCreate(
				backend::chat::message_body::ChatCreate {},
			)),
		}),
	})
	.await?;

	// Create party member
	msg!([ctx] party::msg::member_create(party_id, leader_user_id) {
		party_id: Some(party_id.into()),
		user_id: Some(leader_user_id.into()),
		initial_state: party_member_state,
		client: ctx.client.clone(),
	})
	.await?;

	// Send complete message only after all tasks have completed
	msg!([ctx] party::msg::create_complete(party_id) {
		party_id: Some(party_id.into()),
	})
	.await?;

	Ok(())
}

fn convert_publicity(level: i32) -> PublicityLevel {
	match backend::party::party::PublicityLevel::from_i32(level) {
		None | Some(backend::party::party::PublicityLevel::None) => PublicityLevel::None,
		Some(backend::party::party::PublicityLevel::View) => PublicityLevel::View,
		Some(backend::party::party::PublicityLevel::Join) => PublicityLevel::Join,
	}
}
