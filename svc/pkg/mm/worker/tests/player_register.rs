use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = lobby_res.namespace_id.as_ref().unwrap().as_uuid();
	let lobby_id = lobby_res.lobby_id.as_ref().unwrap().as_uuid();

	let player_id = Uuid::new_v4();
	let query_id = Uuid::new_v4();
	msg!([ctx] @notrace mm::msg::lobby_find(namespace_id, query_id) -> Result<mm::msg::lobby_find_complete, mm::msg::lobby_find_fail> {
		namespace_id: Some(namespace_id.into()),
		query_id: Some(query_id.into()),
		join_kind: backend::matchmaker::query::JoinKind::Normal as i32,
		players: vec![mm::msg::lobby_find::Player {
			player_id: Some(player_id.into()),
			token_session_id: Some(Uuid::new_v4().into()),
			client_info:None,
		}],
		query: Some(mm::msg::lobby_find::message::Query::Direct(backend::matchmaker::query::Direct {
			lobby_id: Some(lobby_id.into()),
		})),
		..Default::default()
	})
	.await
	.unwrap().unwrap();

	// Register player
	msg!([ctx] mm::msg::player_register(player_id) -> Result<mm::msg::player_register_complete, mm::msg::player_register_fail> {
		player_id: Some(player_id.into()),
		lobby_id: Some(lobby_id.into()),
	})
	.await
	.unwrap().unwrap();

	let player_count_res = op!([ctx] mm_lobby_player_count {
		lobby_ids: vec![
			lobby_id.into()
		],
	})
	.await
	.unwrap();
	let player_count = player_count_res
		.lobbies
		.first()
		.unwrap()
		.registered_player_count;

	assert_eq!(1, player_count, "registered player count not updated");

	let (register_ts,) = sqlx::query_as::<_, (Option<i64>,)>(
		"SELECT register_ts FROM db_mm_state.players WHERE player_id = $1",
	)
	.bind(player_id)
	.fetch_one(&ctx.crdb().await.unwrap())
	.await
	.unwrap();
	assert!(register_ts.is_some());

	// Attempt to register again
	let res = msg!([ctx] mm::msg::player_register(player_id) -> Result<mm::msg::player_register_complete, mm::msg::player_register_fail> {
		player_id: Some(player_id.into()),
		lobby_id: Some(lobby_id.into()),
	})
	.await
	.unwrap().unwrap_err();
	assert_eq!(
		mm::msg::player_register_fail::ErrorCode::PlayerAlreadyRegistered as i32,
		res.error_code
	);
}

#[worker_test]
async fn wrong_lobby(ctx: TestCtx) {
	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = lobby_res.namespace_id.as_ref().unwrap().as_uuid();
	let lobby_id = lobby_res.lobby_id.as_ref().unwrap().as_uuid();

	let player_id = Uuid::new_v4();
	let query_id = Uuid::new_v4();
	msg!([ctx] @notrace mm::msg::lobby_find(namespace_id, query_id) -> Result<mm::msg::lobby_find_complete, mm::msg::lobby_find_fail> {
		namespace_id: Some(namespace_id.into()),
		query_id: Some(query_id.into()),
		join_kind: backend::matchmaker::query::JoinKind::Normal as i32,
		players: vec![mm::msg::lobby_find::Player {
			player_id: Some(player_id.into()),
			token_session_id: Some(Uuid::new_v4().into()),
			client_info:None,
		}],
		query: Some(mm::msg::lobby_find::message::Query::Direct(backend::matchmaker::query::Direct {
			lobby_id: Some(lobby_id.into()),
		})),
		..Default::default()
	})
	.await
	.unwrap().unwrap();

	// Register player in the wrong lobby
	let mut remove_sub = subscribe!([ctx] mm::msg::player_remove_complete(player_id))
		.await
		.unwrap();
	let res =
		msg!([ctx] mm::msg::player_register(player_id) -> Result<mm::msg::player_register_complete, mm::msg::player_register_fail> {
			player_id: Some(player_id.into()),
			lobby_id: Some(Uuid::new_v4().into()),
		})
		.await
		.unwrap()
		.unwrap_err();
	assert_eq!(
		mm::msg::player_register_fail::ErrorCode::PlayerInDifferentLobby as i32,
		res.error_code
	);

	remove_sub.next().await.unwrap();

	let player_count_res = op!([ctx] mm_lobby_player_count {
		lobby_ids: vec![
			lobby_id.into()
		],
	})
	.await
	.unwrap();
	let player_count = player_count_res.lobbies.first().unwrap().total_player_count;

	assert_eq!(0, player_count, "player not removed");
}
