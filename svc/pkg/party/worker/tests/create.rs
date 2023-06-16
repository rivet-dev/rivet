use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use redis::AsyncCommands;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let party_id = Uuid::new_v4();
	let leader_user_id = Uuid::new_v4();

	let (mut member_create_sub, mut party_updated_sub) = tokio::try_join!(
		subscribe!([ctx] party::msg::member_create(party_id, leader_user_id)),
		subscribe!([ctx] party::msg::update(party_id)),
	)
	.unwrap();
	msg!([ctx] party::msg::create(party_id) -> party::msg::create_complete {
		party_id: Some(party_id.into()),
		leader_user_id: Some(leader_user_id.into()),
		party_size: 4,
		..Default::default()
	})
	.await
	.unwrap();
	tokio::try_join!(member_create_sub.next(), party_updated_sub.next(),).unwrap();

	let party_exists = ctx
		.redis_party()
		.await
		.unwrap()
		.exists::<_, bool>(util_party::key::party_config(party_id))
		.await
		.unwrap();
	assert!(party_exists);
}

#[worker_test]
async fn init_state_mm_lobby(ctx: TestCtx) {
	let party_id = Uuid::new_v4();
	let leader_user_id = Uuid::new_v4();

	// Create fake lobby
	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();

	let player_res = op!([ctx] faker_mm_player {
		namespace_id: lobby_res.namespace_id,
		lobby_id: lobby_res.lobby_id,
	})
	.await
	.unwrap();

	// Create party with leader in lobby
	msg!([ctx] party::msg::create(party_id) -> party::msg::create_complete {
		party_id: Some(party_id.into()),
		leader_user_id: Some(leader_user_id.into()),
		party_size: 4,
		initial_state: Some(party::msg::create::message::InitialState::MatchmakerLobby(party::msg::create::message::StateMatchmakerLobby {
			namespace_id: lobby_res.namespace_id,
			lobby_id: lobby_res.lobby_id,
			leader_player_id: player_res.player_id,
			leader_player_token: player_res.player_token.clone(),
		})),
		..Default::default()
	})
	.await
	.unwrap();

	let party_res = op!([ctx] party_get {
		party_ids: vec![party_id.into()],
	})
	.await
	.unwrap();
	let party = party_res.parties.first().unwrap();
	assert!(
		matches!(
			party.state,
			Some(backend::party::party::State::MatchmakerLobby(_))
		),
		"party not in lobby"
	);

	let party_member_res = op!([ctx] party_member_get {
		user_ids: vec![leader_user_id.into()],
	})
	.await
	.unwrap();
	let party_member = party_member_res.party_members.first().unwrap();
	assert!(
		matches!(
			party_member.state,
			Some(backend::party::party_member::State::MatchmakerLobby(_))
		),
		"party member not in lobby"
	);
}
