use std::time::Duration;

use chirp_worker::prelude::*;
use indoc::indoc;
use proto::backend::{self, pkg::*};

use ::mm_gc::run_from_env;

#[tokio::test(flavor = "multi_thread")]
async fn all() {
	// TODO: interferes with other mm tests
	return;

	if !util::feature::job_run() {
		return;
	}

	tracing_subscriber::fmt()
		.json()
		.with_max_level(tracing::Level::INFO)
		.with_span_events(tracing_subscriber::fmt::format::FmtSpan::NONE)
		.init();

	let ctx = TestCtx::from_env("all").await.unwrap();

	// Run tests sequentially so the gc's don't interfere with each other
	// remove_unready_lobbies(ctx.clone()).await;
	// remove_unregistered_players(ctx.clone()).await;
	remove_auto_remove_players(ctx.clone()).await;
}

async fn remove_unready_lobbies(ctx: TestCtx) {
	let _pools = rivet_pools::from_env("mm-gc-test").await.unwrap();

	let lobby = op!([ctx] faker_mm_lobby {
		skip_set_ready: true,
		..Default::default()
	})
	.await
	.unwrap();
	let lobby_id = lobby.lobby_id.as_ref().unwrap().as_uuid();

	// Check that it didn't remove lobbies it shouldn't
	{
		run_from_env(util::timestamp::now(), ctx.op_ctx().base())
			.await
			.unwrap();
		tokio::time::sleep(Duration::from_secs(1)).await;

		let get_res = op!([ctx] mm_lobby_get {
			lobby_ids: vec![lobby_id.into()],
			include_stopped: true,
		})
		.await
		.unwrap();
		assert_eq!(1, get_res.lobbies.len());
		assert!(
			get_res.lobbies[0].ready_ts.is_none(),
			"lobby should not be ready"
		);
		assert!(get_res.lobbies[0].stop_ts.is_none());
	}

	// Simulate timestamp long in the future to remove lobby
	{
		let mut cleanup_sub = subscribe!([ctx] mm::msg::lobby_cleanup(lobby_id))
			.await
			.unwrap();
		run_from_env(
			util::timestamp::now()
				+ util_mm::consts::LOBBY_READY_TIMEOUT
				+ util::duration::seconds(1),
			ctx.op_ctx().base(),
		)
		.await
		.unwrap();
		cleanup_sub.next().await.unwrap();

		let get_res = op!([ctx] mm_lobby_get {
			lobby_ids: vec![lobby_id.into()],
			include_stopped: true,
		})
		.await
		.unwrap();
		assert!(
			get_res.lobbies.is_empty() || get_res.lobbies[0].stop_ts.is_some(),
			"lobby should be stopped"
		);
	}
}

async fn remove_unregistered_players(ctx: TestCtx) {
	let _pools = rivet_pools::from_env("mm-gc-test").await.unwrap();

	let lobby = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = lobby.namespace_id.as_ref().unwrap().as_uuid();
	let lobby_id = lobby.lobby_id.as_ref().unwrap().as_uuid();

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

	// Check that it didn't remove players it shouldn't
	{
		run_from_env(util::timestamp::now(), ctx.op_ctx().base())
			.await
			.unwrap();
		tokio::time::sleep(Duration::from_secs(1)).await;

		let (crdb_remove_ts,) = sqlx::query_as::<_, (Option<i64>,)>(indoc!(
			"
			SELECT remove_ts
			FROM db_mm_state.players
			WHERE player_id = $1
			"
		))
		.bind(player_id)
		.fetch_one(&ctx.crdb().await.unwrap())
		.await
		.unwrap();
		assert!(
			crdb_remove_ts.is_none(),
			"player is removed when it shouldn't be"
		);
	}

	// Simulate timestamp long in the future to remove player
	{
		let mut player_remove_sub = subscribe!([ctx] mm::msg::player_remove_complete(player_id))
			.await
			.unwrap();

		run_from_env(
			util::timestamp::now()
				+ util_mm::consts::PLAYER_READY_TIMEOUT
				+ util::duration::seconds(1),
			ctx.op_ctx().base(),
		)
		.await
		.unwrap();

		player_remove_sub.next().await.unwrap();

		let crdb_player_exists = sqlx::query_as::<_, (Option<i64>,)>(
			"SELECT remove_ts FROM db_mm_state.players WHERE player_id = $1",
		)
		.bind(player_id)
		.fetch_one(&ctx.crdb().await.unwrap())
		.await
		.unwrap()
		.0
		.is_none();
		assert!(!crdb_player_exists, "player not removed");
	}

	// TODO: Test max(create_ts, lobby_ready_ts) logic
}

async fn remove_auto_remove_players(ctx: TestCtx) {
	let _pools = rivet_pools::from_env("mm-gc-test").await.unwrap();

	let lobby = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = lobby.namespace_id.as_ref().unwrap().as_uuid();
	let lobby_id = lobby.lobby_id.as_ref().unwrap().as_uuid();

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

	msg!([ctx] mm::msg::player_register(player_id) -> mm::msg::player_register_complete {
		player_id: Some(player_id.into()),
		lobby_id: Some(lobby_id.into()),
	})
	.await
	.unwrap();

	// Check that it didn't remove players it shouldn't
	{
		run_from_env(util::timestamp::now(), ctx.op_ctx().base())
			.await
			.unwrap();
		tokio::time::sleep(Duration::from_secs(1)).await;

		run_from_env(
			util::timestamp::now()
				+ util_mm::consts::PLAYER_READY_TIMEOUT
				+ util::duration::seconds(1),
			ctx.op_ctx().base(),
		)
		.await
		.unwrap();
		tokio::time::sleep(Duration::from_secs(1)).await;

		let (crdb_remove_ts,) = sqlx::query_as::<_, (Option<i64>,)>(indoc!(
			"
			SELECT remove_ts
			FROM db_mm_state.players
			WHERE player_id = $1
			"
		))
		.bind(player_id)
		.fetch_one(&ctx.crdb().await.unwrap())
		.await
		.unwrap();
		assert!(
			crdb_remove_ts.is_none(),
			"player is removed when it shouldn't be"
		);
	}

	// Simulate timestamp long in the future to remove player
	{
		let mut player_remove_sub = subscribe!([ctx] mm::msg::player_remove_complete(player_id))
			.await
			.unwrap();

		run_from_env(
			util::timestamp::now()
				+ util_mm::consts::PLAYER_AUTO_REMOVE_TIMEOUT
				+ util::duration::seconds(1),
			ctx.op_ctx().base(),
		)
		.await
		.unwrap();

		player_remove_sub.next().await.unwrap();

		let crdb_player_exists = sqlx::query_as::<_, (Option<i64>,)>(
			"SELECT remove_ts FROM db_mm_state.players WHERE player_id = $1",
		)
		.bind(player_id)
		.fetch_one(&ctx.crdb().await.unwrap())
		.await
		.unwrap()
		.0
		.is_none();
		assert!(!crdb_player_exists, "player not removed");
	}
}
