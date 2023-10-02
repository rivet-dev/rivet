use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use redis::AsyncCommands;

#[worker_test]
async fn player_remove(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_res = op!([ctx] faker_mm_lobby {
		max_players_normal: 50,
		max_players_party: 100,
		max_players_direct: 200,
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = lobby_res.namespace_id.as_ref().unwrap().as_uuid();
	let lobby_id = lobby_res.lobby_id.as_ref().unwrap().as_uuid();
	let player_ids = std::iter::repeat_with(Uuid::new_v4)
		.take(3)
		.collect::<Vec<_>>();

	let query_id = Uuid::new_v4();
	msg!([ctx] @notrace mm::msg::lobby_find(namespace_id, query_id) -> Result<mm::msg::lobby_find_complete, mm::msg::lobby_find_fail> {
		namespace_id: Some(namespace_id.into()),
		query_id: Some(query_id.into()),
		join_kind: backend::matchmaker::query::JoinKind::Normal as i32,
		players: player_ids
			.iter()
			.map(|&player_id| mm::msg::lobby_find::Player {
				player_id: Some(player_id.into()),
				token_session_id: Some(Uuid::new_v4().into()),
				client_info:None,
			})
			.collect(),
		query: Some(mm::msg::lobby_find::message::Query::Direct(backend::matchmaker::query::Direct {
			lobby_id: Some(lobby_id.into()),
		})),
		..Default::default()
	})
	.await
	.unwrap().unwrap();

	assert_eq!(3, fetch_player_count(&ctx, lobby_id).await);

	tracing::info!("attempting to remove from the wrong lobby");
	let res = msg!([ctx] mm::msg::player_remove(player_ids[0]) -> Result<mm::msg::player_remove_complete, mm::msg::player_remove_fail> {
		player_id: Some(player_ids[0].into()),
		lobby_id: Some(Uuid::new_v4().into()),
		..Default::default()
	})
	.await
	.unwrap().unwrap_err();
	assert_eq!(
		mm::msg::player_remove_fail::ErrorCode::PlayerInDifferentLobby as i32,
		res.error_code
	);

	tracing::info!("removing player 0");
	msg!([ctx] mm::msg::player_remove(player_ids[0]) -> Result<mm::msg::player_remove_complete, mm::msg::player_remove_fail> {
		player_id: Some(player_ids[0].into()),
		lobby_id: Some(lobby_id.into()),
		..Default::default()
	})
	.await
	.unwrap().unwrap();
	assert!(!does_player_exist(&ctx, lobby_id, player_ids[0]).await);
	assert_eq!(2, fetch_player_count(&ctx, lobby_id).await);

	tracing::info!("removing player 1");
	msg!([ctx] mm::msg::player_remove(player_ids[1]) -> Result<mm::msg::player_remove_complete, mm::msg::player_remove_fail> {
		player_id: Some(player_ids[1].into()),
		lobby_id: Some(lobby_id.into()),
		..Default::default()
	})
	.await
	.unwrap().unwrap();
	assert!(!does_player_exist(&ctx, lobby_id, player_ids[1]).await);
	assert_eq!(1, fetch_player_count(&ctx, lobby_id).await);

	tracing::info!("removing player 2");
	msg!([ctx] mm::msg::player_remove(player_ids[2]) -> Result<mm::msg::player_remove_complete, mm::msg::player_remove_fail> {
		player_id: Some(player_ids[2].into()),
		lobby_id: Some(lobby_id.into()),
		..Default::default()
	})
	.await
	.unwrap().unwrap();
	assert!(!does_player_exist(&ctx, lobby_id, player_ids[2]).await);
	assert_eq!(0, fetch_player_count(&ctx, lobby_id).await);
}

async fn does_player_exist(ctx: &TestCtx, lobby_id: Uuid, player_id: Uuid) -> bool {
	let redis_exists = ctx
		.redis_mm()
		.await
		.unwrap()
		.zscore::<_, _, Option<u64>>(
			util_mm::key::lobby_player_ids(lobby_id),
			player_id.to_string(),
		)
		.await
		.unwrap()
		.is_some();

	let crdb_exists = sqlx::query_as::<_, (Option<i64>,)>(
		"SELECT remove_ts FROM db_mm_state.players WHERE player_id = $1",
	)
	.bind(player_id)
	.fetch_one(&ctx.crdb().await.unwrap())
	.await
	.unwrap()
	.0
	.is_none();

	assert_eq!(redis_exists, crdb_exists);

	crdb_exists
}

async fn fetch_player_count(ctx: &TestCtx, lobby_id: Uuid) -> u32 {
	let redis_player_count = ctx
		.redis_mm()
		.await
		.unwrap()
		.zcard::<_, i64>(util_mm::key::lobby_player_ids(lobby_id))
		.await
		.unwrap();

	let crdb_player_count = sqlx::query_as::<_, (Option<i64>,)>(
		"SELECT remove_ts FROM db_mm_state.players WHERE lobby_id = $1",
	)
	.bind(lobby_id)
	.fetch_all(&ctx.crdb().await.unwrap())
	.await
	.unwrap()
	.into_iter()
	.filter(|(x,)| x.is_none())
	.count() as i64;

	assert_eq!(redis_player_count, crdb_player_count);

	if crdb_player_count > 0 {
		assert_lobby_state(ctx, lobby_id, crdb_player_count).await;
	}

	crdb_player_count as u32
}

async fn assert_lobby_state(ctx: &TestCtx, lobby_id: Uuid, player_count: i64) {
	// Fetch the lobby
	let (namespace_id, region_id, lobby_group_id, max_players_normal, max_players_party) =
		sqlx::query_as::<_, (Uuid, Uuid, Uuid, i64, i64)>(indoc!(
			"
				SELECT namespace_id, region_id, lobby_group_id, max_players_normal, max_players_party
				FROM db_mm_state.lobbies
				WHERE lobby_id = $1
				"
		))
		.bind(lobby_id)
		.fetch_one(&ctx.crdb().await.unwrap())
		.await
		.unwrap();
	let crdb_available_spots_normal = max_players_normal - player_count;
	let crdb_available_spots_party = max_players_party - player_count;

	// Get the Redis counts
	let (redis_available_spots_normal, redis_available_spots_party) = redis::pipe()
		.zscore(
			util_mm::key::lobby_available_spots(
				namespace_id,
				region_id,
				lobby_group_id,
				util_mm::JoinKind::Normal,
			),
			lobby_id.to_string(),
		)
		.zscore(
			util_mm::key::lobby_available_spots(
				namespace_id,
				region_id,
				lobby_group_id,
				util_mm::JoinKind::Party,
			),
			lobby_id.to_string(),
		)
		.query_async::<_, (i64, i64)>(&mut ctx.redis_mm().await.unwrap())
		.await
		.unwrap();

	assert_eq!(crdb_available_spots_normal, redis_available_spots_normal);
	assert_eq!(crdb_available_spots_party, redis_available_spots_party);
}
