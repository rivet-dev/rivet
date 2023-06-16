use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn basic(ctx: TestCtx) {
	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = lobby_res.namespace_id.as_ref().unwrap().as_uuid();
	let lobby_id = lobby_res.lobby_id.as_ref().unwrap().as_uuid();

	let party_id = Uuid::new_v4();
	let leader_user_id = Uuid::new_v4();

	// Create party
	let mut member_sub =
		subscribe!([ctx] party::msg::member_create_complete(party_id, leader_user_id))
			.await
			.unwrap();
	msg!([ctx] party::msg::create(party_id) -> party::msg::create_complete {
		party_id: Some(party_id.into()),
		leader_user_id: Some(leader_user_id.into()),
		party_size: 4,
		..Default::default()
	})
	.await
	.unwrap();
	member_sub.next().await.unwrap();

	// Find lobby
	let mut update_sub = subscribe!([ctx] party::msg::update(party_id))
		.await
		.unwrap();
	msg!([ctx] party::msg::state_mm_lobby_find(party_id) {
		party_id: Some(party_id.into()),
		namespace_id: Some(namespace_id.into()),
		query: Some(party::msg::state_mm_lobby_find::message::Query::Direct(backend::matchmaker::query::Direct {
			lobby_id: Some(lobby_id.into()),
		})),
	})
	.await
	.unwrap();

	// Wait for lobby update with assigned lobby ID
	loop {
		let _msg = update_sub.next().await.unwrap();

		let get_res = op!([ctx] party_get {
			party_ids: vec![party_id.into()],
		})
		.await
		.unwrap();
		let party = get_res.parties.first().unwrap();
		if matches!(
			party.state,
			Some(backend::party::party::State::MatchmakerLobby(_))
		) {
			tracing::info!("matchmaker in lobby");
			break;
		} else {
			tracing::info!(state = ?party.state, "still not in lobby");
		}
	}

	let get_res = op!([ctx] party_get {
		party_ids: vec![party_id.into()],
	})
	.await
	.unwrap();
	let party = get_res.parties.first().unwrap();
	match &party.state {
		Some(backend::party::party::State::MatchmakerLobby(state)) => {
			assert_eq!(
				lobby_id,
				state.lobby_id.as_ref().unwrap().as_uuid(),
				"party in wrong lobby"
			);
		}
		_ => panic!("party in wrong state"),
	}

	let member_get_res = op!([ctx] party_member_get {
		user_ids: vec![leader_user_id.into()]
	})
	.await
	.unwrap();
	let member = member_get_res.party_members.first().unwrap();
	assert!(
		matches!(
			member.state,
			Some(backend::party::party_member::State::MatchmakerLobby(_))
		),
		"party member not in lobby state"
	);

	// TODO: Add new member, wait for it to join
	// TODO: Set lobby idle
	// TODO: Find lobby again (with both players)
}
