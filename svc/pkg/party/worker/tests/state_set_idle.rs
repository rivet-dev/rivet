use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn basic(ctx: TestCtx) {
	// Create fake lobby
	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = lobby_res.namespace_id.as_ref().unwrap().as_uuid();
	let lobby_id = lobby_res.lobby_id.as_ref().unwrap().as_uuid();

	// Create party
	let party_id = Uuid::new_v4();
	let leader_user_id = Uuid::new_v4();
	msg!([ctx] party::msg::create(party_id) -> party::msg::create_complete {
		party_id: Some(party_id.into()),
		leader_user_id: Some(leader_user_id.into()),
		party_size: 4,
		..Default::default()
	})
	.await
	.unwrap();

	// Validate member in party
	let member_get_res = op!([ctx] party_member_get {
		user_ids: vec![leader_user_id.into()],
	})
	.await
	.unwrap();
	assert_eq!(1, member_get_res.party_members.len());
	let member = member_get_res.party_members.first().unwrap();
	assert!(member.state.is_none(), "member should be idle");

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
	tracing::info!("waiting for lobby found");
	loop {
		let _msg = update_sub.next().await.unwrap();

		// Wait until party is in lobby
		let get_res = op!([ctx] party_get {
			party_ids: vec![party_id.into()],
		})
		.await
		.unwrap();
		let party = get_res.parties.first().unwrap();

		let member_get_res = op!([ctx] party_member_get {
			user_ids: vec![leader_user_id.into()],
		})
		.await
		.unwrap();
		assert_eq!(1, member_get_res.party_members.len());
		let member = member_get_res.party_members.first().unwrap();

		let party_in_lobby = matches!(
			party.state,
			Some(backend::party::party::State::MatchmakerLobby(_))
		);
		let party_member_in_lobby = matches!(
			member.state,
			Some(backend::party::party_member::State::MatchmakerLobby(_))
		);

		tracing::info!(?party_in_lobby, ?party_member_in_lobby);
		if party_in_lobby && party_member_in_lobby {
			tracing::info!("ready to set idle");
			break;
		}
	}

	// Set idle
	msg!([ctx] party::msg::state_set_idle(party_id) -> party::msg::update {
		party_id: Some(party_id.into()),
	})
	.await
	.unwrap();

	let get_res = op!([ctx] party_get {
		party_ids: vec![party_id.into()],
	})
	.await
	.unwrap();
	let party = get_res.parties.first().expect("party not found");
	assert!(party.state.is_none(), "party should have empty state");

	let member_get_res = op!([ctx] party_member_get {
		user_ids: vec![leader_user_id.into()],
	})
	.await
	.unwrap();
	assert_eq!(1, member_get_res.party_members.len());
	let member = member_get_res.party_members.first().unwrap();
	assert!(
		matches!(
			member.state,
			Some(backend::party::party_member::State::MatchmakerReady(_))
		),
		"member should be pending"
	);
}
