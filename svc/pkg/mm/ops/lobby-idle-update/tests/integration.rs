use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use redis::AsyncCommands;

use std::{collections::HashSet, time::Duration};
use tokio::time::Instant;
use tracing::Instrument;

struct Ctx {
	test_ctx: TestCtx,
	region_id: Uuid,
	game_id: Uuid,
	namespace_id: Uuid,
	version_id: Uuid,
	lobby_group_id: Uuid,
}

impl Ctx {
	/// `previous_ctx` is the previous test context. We use this to preserve the
	/// same game game and namespace.
	#[tracing::instrument(skip(test_ctx, previous_ctx))]
	async fn init(
		test_ctx: &TestCtx,
		previous_ctx: Option<&Ctx>,
		min_idle_lobbies: u32,
		max_idle_lobbies: u32,
	) -> Ctx {
		tracing::info!("init");

		// Create game if needed
		let (region_id, game_id, namespace_id) = if let Some(previous_ctx) = previous_ctx {
			tracing::info!("using previous ctx game");
			(
				previous_ctx.region_id,
				previous_ctx.game_id,
				previous_ctx.namespace_id,
			)
		} else {
			tracing::info!("creating new game");

			let region_res = op!([test_ctx] faker_region {}).await.unwrap();
			let region_id = region_res.region_id.as_ref().unwrap().as_uuid();

			let game_res = op!([test_ctx] faker_game {
				..Default::default()
			})
			.await
			.unwrap();
			let game_id = game_res.game_id.as_ref().unwrap().as_uuid();
			let namespace_id = game_res.namespace_ids.first().unwrap().as_uuid();

			(region_id, game_id, namespace_id)
		};

		let build_res = op!([test_ctx] faker_build {
			game_id: Some(game_id.into()),
			image: faker::build::Image::MmLobbyAutoReady as i32,
		})
		.await
		.unwrap();

		let game_version_res = op!([test_ctx] faker_game_version {
			game_id: Some(game_id.into()),
			override_lobby_groups: Some(faker::game_version::request::OverrideLobbyGroups {
				lobby_groups: vec![backend::matchmaker::LobbyGroup {
					name_id: "test-group".to_owned(),

					regions: vec![backend::matchmaker::lobby_group::Region {
						region_id: Some(region_id.into()),
						tier_name_id: util_mm::test::TIER_NAME_ID.to_owned(),
						idle_lobbies: Some(backend::matchmaker::lobby_group::IdleLobbies {
							min_idle_lobbies,
							max_idle_lobbies,
						}),
					}],
					max_players_normal: 1,
					max_players_direct: 1,
					max_players_party: 1,
					listable: true,

					runtime: Some(backend::matchmaker::lobby_runtime::Docker {
						build_id: build_res.build_id,
						args: Vec::new(),
						env_vars: Vec::new(),
						network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Bridge as i32,
						ports: Vec::new(),
					}.into()),

					find_config: None,
					join_config: None,
					create_config: None,
				}],
			}),
			..Default::default()
		})
		.await
		.unwrap();
		let version_id = game_version_res.version_id.as_ref().unwrap().as_uuid();

		let version_get_res = op!([test_ctx] mm_config_version_get {
			version_ids: vec![version_id.into()],
		})
		.await
		.unwrap();
		let version = version_get_res.versions.first();
		let version = version.as_ref().unwrap();
		let config_meta = version.config_meta.as_ref().unwrap();
		let lobby_group = config_meta.lobby_groups.first();
		let lobby_group = lobby_group.as_ref().unwrap();
		let lobby_group_id = lobby_group.lobby_group_id.as_ref().unwrap().as_uuid();

		tracing::info!(
			?region_id,
			?game_id,
			?namespace_id,
			?version_id,
			?lobby_group_id,
			"new test context"
		);

		let ctx = Ctx {
			test_ctx: (*test_ctx).clone(),
			region_id,
			game_id,
			namespace_id,
			version_id,
			lobby_group_id,
		};

		// Set the namespace's version
		let create_sub = ctx.sub_lobby_create().await;
		op!([test_ctx] game_namespace_version_set {
			namespace_id: Some(namespace_id.into()),
			version_id: Some(version_id.into()),
		})
		.await
		.unwrap();
		ctx.wait_lobby_create(create_sub, min_idle_lobbies).await;

		// Assert the new idle lobbies were created appropriately
		if previous_ctx.is_none() {
			assert_eq!(
				min_idle_lobbies,
				ctx.fetch_idle_lobby_ids().await.len() as u32,
				"initial idle lobbies not correct"
			);
		}

		ctx
	}

	/// Waits for `count` lobbies to be cleaned up before proceeding.
	#[tracing::instrument(skip(self, sub))]
	async fn wait_lobby_create(
		&self,
		mut sub: chirp_client::SubscriptionHandle<mm::msg::lobby_create_complete::Message>,
		count: u32,
	) {
		tracing::info!(%count, "waiting for lobby create");

		// Wait for `count` lobbies to create
		let mut i = 0;
		while i < count {
			let msg = sub.next().await.unwrap();
			let get_res = op!([self.test_ctx] mm_lobby_get {
				lobby_ids: vec![msg.lobby_id.unwrap()],
				include_stopped: true,
			})
			.await
			.unwrap();
			let lobby = get_res.lobbies.first().unwrap();
			if lobby.lobby_group_id.as_ref().unwrap().as_uuid() == self.lobby_group_id {
				i += 1;
				tracing::info!(%i, %count, "lobby created");
			}
		}

		// Check for any excess calls
		tracing::trace!("checking for excess lobby create");
		let check_excess_until = Instant::now() + Duration::from_secs(1);
		loop {
			let msg = match tokio::time::timeout_at(check_excess_until, sub.next()).await {
				Ok(x) => x.unwrap(),
				Err(_) => {
					tracing::info!("no excess found");
					break;
				}
			};

			let get_res = op!([self.test_ctx] mm_lobby_get {
				lobby_ids: vec![msg.lobby_id.unwrap()],
				include_stopped: true,
			})
			.await
			.unwrap();
			let lobby = get_res.lobbies.first().unwrap();
			if lobby.lobby_group_id.as_ref().unwrap().as_uuid() == self.lobby_group_id {
				panic!("received excess lobby create: {:?}", lobby);
			}
		}
	}

	/// Waits for `count` lobbies to be cleaned up before proceeding.
	#[tracing::instrument(skip(self, sub))]
	async fn wait_lobby_cleanup(
		&self,
		mut sub: chirp_client::SubscriptionHandle<mm::msg::lobby_cleanup_complete::Message>,
		count: u32,
	) {
		tracing::info!(%count, "waiting for lobby cleanup");

		let mut cleaned_up_lobbies = HashSet::<Uuid>::new();

		while (cleaned_up_lobbies.len() as u32) < count {
			let msg = sub.next().await.unwrap();
			let get_res = op!([self.test_ctx] mm_lobby_get {
				lobby_ids: vec![msg.lobby_id.unwrap()],
				include_stopped: true,
			})
			.await
			.unwrap();
			let lobby = get_res.lobbies.first().unwrap();
			let lobby_id = lobby.lobby_id.unwrap().as_uuid();
			if !cleaned_up_lobbies.contains(&lobby_id)
				&& lobby.lobby_group_id.as_ref().unwrap().as_uuid() == self.lobby_group_id
			{
				cleaned_up_lobbies.insert(lobby_id);
				tracing::info!(i = cleaned_up_lobbies.len(), %count, "lobby cleaned up");
			}
		}

		// Check for any excess calls
		tracing::info!("checking for excess lobby cleanup");
		let check_excess_until = Instant::now() + Duration::from_secs(1);
		loop {
			let msg = match tokio::time::timeout_at(check_excess_until, sub.next()).await {
				Ok(x) => x.unwrap(),
				Err(_) => {
					tracing::info!("no excess found");
					break;
				}
			};

			let get_res = op!([self.test_ctx] mm_lobby_get {
				lobby_ids: vec![msg.lobby_id.unwrap()],
				include_stopped: true,
			})
			.await
			.unwrap();
			let lobby = get_res.lobbies.first().unwrap();
			let lobby_id = lobby.lobby_id.unwrap().as_uuid();
			if !cleaned_up_lobbies.contains(&lobby_id)
				&& lobby.lobby_group_id.as_ref().unwrap().as_uuid() == self.lobby_group_id
			{
				panic!("received excess lobby cleanup: {:?}", lobby);
			}
		}
	}

	#[tracing::instrument(skip(self))]
	async fn fetch_idle_lobby_ids(&self) -> HashSet<Uuid> {
		let crdb = self.test_ctx.crdb("db-mm-state").await.unwrap();

		// Find lobbies without any players
		let ili_crdb = sqlx::query_as::<_, (Uuid,)>(indoc!(
			"
			SELECT lobby_id
			FROM lobbies
			WHERE
				namespace_id = $1 AND
				region_id = $2 AND
				lobby_group_id = $3 AND
				stop_ts IS NULL AND
				NOT EXISTS (
					SELECT 1 FROM players WHERE players.lobby_id = lobbies.lobby_id
				)
			"
		))
		.bind(self.namespace_id)
		.bind(self.region_id)
		.bind(self.lobby_group_id)
		.fetch_all(&crdb)
		.await
		.unwrap()
		.into_iter()
		.map(|(lobby_id,)| lobby_id)
		.collect::<HashSet<_>>();

		// Assert they match what's in the database
		let ili_redis = self
			.test_ctx
			.redis_mm()
			.await
			.unwrap()
			.zrange::<_, Vec<String>>(
				util_mm::key::idle_lobby_ids(
					self.namespace_id,
					self.region_id,
					self.lobby_group_id,
				),
				0,
				-1,
			)
			.await
			.unwrap()
			.into_iter()
			.map(|x| util::uuid::parse(&x).unwrap())
			.collect::<HashSet<Uuid>>();

		assert_eq!(
			ili_crdb, ili_redis,
			"crdb and redis idle lobby ids do not match"
		);

		ili_crdb
	}

	#[tracing::instrument(skip(self))]
	async fn is_idle(&self, lobby_id: Uuid) -> bool {
		self.test_ctx
			.redis_mm()
			.await
			.unwrap()
			.zscore::<_, _, Option<u32>>(
				util_mm::key::idle_lobby_ids(
					self.namespace_id,
					self.region_id,
					self.lobby_group_id,
				),
				lobby_id.to_string(),
			)
			.await
			.unwrap()
			.is_some()
	}

	/// Calls mm-lobby-idle-update.
	///
	/// Automatically waits for `create_count` lobbies to be created and
	/// `stop_count` lobbies to be stopped.
	#[tracing::instrument(skip(self))]
	async fn call_update(&self, create_count: u32, stop_count: u32) {
		tracing::info!("updating idle lobbies");
		let create_sub = self.sub_lobby_create().await;
		let cleanup_sub = self.sub_lobby_cleanup().await;
		op!([self.test_ctx] mm_lobby_idle_update {
			namespace_id: Some(self.namespace_id.into()),
			region_id: Some(self.region_id.into()),
		})
		.await
		.unwrap();
		self.wait_lobby_create(create_sub, create_count).await;
		self.wait_lobby_cleanup(cleanup_sub, stop_count).await;

		// If there is an incorrect lobby create or destroy message called, then
		// we'll sleep to let that change show up in `fetch_idle_lobby_ids` in
		// the later assertions.
		tokio::time::sleep(Duration::from_secs(1)).await;
	}

	/// Publishes an empty version to the namespace so it stops all running idle
	/// lobbies.
	///
	/// This is useful both for testing and to keep dev environments clean.
	#[tracing::instrument(skip(self))]
	async fn cleanup(&self, stop_count: u32) {
		tracing::info!("cleaning up");

		// Publish a new empty version that doesn't have idle lobbies so it doesn't
		// run a bunch of idle lobbies in the background
		let game_version_res = op!([self.test_ctx] faker_game_version {
			game_id: Some(self.game_id.into()),
			..Default::default()
		})
		.await
		.unwrap();

		let cleanup_sub = self.sub_lobby_cleanup().await;
		op!([self.test_ctx] game_namespace_version_set {
			namespace_id: Some(self.namespace_id.into()),
			version_id: game_version_res.version_id,
		})
		.await
		.unwrap();
		self.wait_lobby_cleanup(cleanup_sub, stop_count).await;

		tracing::info!("finished cleaning up");
	}

	/// Create a subscription to all lobby create events.
	#[tracing::instrument(skip_all)]
	async fn sub_lobby_create(
		&self,
	) -> chirp_client::SubscriptionHandle<mm::msg::lobby_create_complete::Message> {
		subscribe!([self.test_ctx] mm::msg::lobby_create_complete("*"))
			.await
			.unwrap()
	}

	/// Create a subscription to all lobby cleanup events.
	#[tracing::instrument(skip_all)]
	async fn sub_lobby_cleanup(
		&self,
	) -> chirp_client::SubscriptionHandle<mm::msg::lobby_cleanup_complete::Message> {
		subscribe!([self.test_ctx] mm::msg::lobby_cleanup_complete("*"))
			.await
			.unwrap()
	}
}

#[worker_test]
async fn create_idle_lobbies(ctx: TestCtx) {
	// Test context A
	//
	// Test initial idle lobby creation
	let test_ctx_a = Ctx::init(&ctx, None, 2, 4).await;
	async {
		tracing::info!("checking idempotent, nothing should happen");
		let initial_idle_lobbies = test_ctx_a.fetch_idle_lobby_ids().await;
		test_ctx_a.call_update(0, 0).await;
		assert_eq!(
			initial_idle_lobbies,
			test_ctx_a.fetch_idle_lobby_ids().await,
			"lobbies should not have changed"
		);
	}
	.instrument(tracing::info_span!("ctx a"))
	.await;

	// Test context B
	//
	// Switching to a different version
	let cleanup_a_sub = test_ctx_a.sub_lobby_cleanup().await;
	let test_ctx_b = Ctx::init(&ctx, Some(&test_ctx_a), 3, 5).await;
	test_ctx_a.wait_lobby_cleanup(cleanup_a_sub, 2).await;
	async {
		tracing::info!("checking idempotent, nothing should happen");
		test_ctx_b.call_update(0, 0).await;

		assert_eq!(
			0,
			test_ctx_a.fetch_idle_lobby_ids().await.len(),
			"version a lobbies not destroyed"
		);
		assert_eq!(
			3,
			test_ctx_b.fetch_idle_lobby_ids().await.len(),
			"version b lobbies not created"
		);
	}
	.instrument(tracing::info_span!("ctx b"))
	.await;

	// Cleanup
	//
	// Test disabling idle lobbies
	test_ctx_b.cleanup(3).await;
	async {
		tracing::info!("checking idempotent, nothing should happen");
		test_ctx_b.call_update(0, 0).await;

		assert_eq!(
			0,
			test_ctx_a.fetch_idle_lobby_ids().await.len(),
			"version a lobbies not destroyed"
		);
		assert_eq!(
			0,
			test_ctx_b.fetch_idle_lobby_ids().await.len(),
			"version b lobbies not destroyed"
		);
	}
	.instrument(tracing::info_span!("cleanup"))
	.await;
}

#[worker_test]
async fn test_find_idle_lobby(ctx: TestCtx) {
	let test_ctx = Ctx::init(&ctx, None, 2, 4).await;
	let lgi = test_ctx.lobby_group_id;

	tracing::info!("calling");
	test_ctx.call_update(0, 0).await;
	let initial_idle_lobbies = test_ctx.fetch_idle_lobby_ids().await;
	assert_eq!(2, initial_idle_lobbies.len(), "lobbies not created");

	// Wait for lobbies to become ready
	loop {
		let lobby_ids = initial_idle_lobbies
			.iter()
			.cloned()
			.map(Into::<common::Uuid>::into)
			.collect::<Vec<_>>();
		let lobbies_res = op!([ctx] mm_lobby_get {
			lobby_ids: lobby_ids,
			include_stopped: false,
		})
		.await
		.unwrap();

		let all_ready = lobbies_res.lobbies.iter().all(|x| x.ready_ts.is_some());
		if all_ready {
			break;
		} else {
			tokio::time::sleep(Duration::from_millis(500)).await;
		}
	}

	// Attempt to find a lobby in the same lobby group. This will cause
	// mm-lobby-find to create a new idle lobby.
	let lobby_id = {
		tracing::info!("finding lobby");
		let player_id = Uuid::new_v4();
		let query_id = Uuid::new_v4();
		let find_res = msg!([ctx] @notrace mm::msg::lobby_find(test_ctx.namespace_id, query_id)
			-> Result<mm::msg::lobby_find_complete, mm::msg::lobby_find_fail>
		{
			namespace_id: Some(test_ctx.namespace_id.into()),
			query_id: Some(query_id.into()),
			join_kind: backend::matchmaker::query::JoinKind::Normal as i32,
			players: vec![mm::msg::lobby_find::Player {
				player_id: Some(player_id.into()),
				token_session_id: Some(Uuid::new_v4().into()),
				client_info: Some(backend::net::ClientInfo {
					user_agent: Some("Test".into()),
					remote_address: Some(util::faker::ip_addr_v4().to_string()),
				}),
			}],
			query: Some(mm::msg::lobby_find::message::Query::LobbyGroup(backend::matchmaker::query::LobbyGroup {
				lobby_group_ids: vec![test_ctx.lobby_group_id.into()],
				region_ids: vec![test_ctx.region_id.into()],
				auto_create: None,
			})),
			..Default::default()
		})
		.await
		.unwrap()
		.unwrap();
		let find_lobby_id = find_res.lobby_id.as_ref().unwrap().as_uuid();

		assert!(
			initial_idle_lobbies.contains(&find_lobby_id),
			"did not find one of the idle lobbies"
		);

		let lobby_res = op!([ctx] mm_lobby_get {
			lobby_ids: vec![find_lobby_id.into()],
			include_stopped: false,
		})
		.await
		.unwrap();
		let _ = lobby_res.lobbies.first().expect("lobby should exit");
		assert!(!test_ctx.is_idle(find_lobby_id).await);

		find_lobby_id
	};

	// Check that a new idle lobby was booted in its place from mm-lobby-find
	tokio::time::sleep(Duration::from_secs(1)).await;
	tracing::info!("checking new lobby booted in replacement");
	assert_eq!(
		2,
		test_ctx.fetch_idle_lobby_ids().await.len(),
		"wrong idle lobby count"
	);

	// Cleanup both the idle lobbies and the non-idle lobby we created
	test_ctx.cleanup(2).await;
	msg!([ctx] mm::msg::lobby_stop(lobby_id) {
		lobby_id: Some(lobby_id.into()),
	})
	.await
	.unwrap();
}

// #[worker_test]
// async fn smoke_test(ctx: TestCtx) {
// 	let test_ctx = Arc::new(Ctx::init(&ctx, None, 2, 4).await);

// 	let max_duration = 10_000;

// 	// Smoke test calling update
// 	let mut futs = Vec::new();
// 	for i in 0..30 {
// 		let test_ctx = test_ctx.clone();
// 		let duration = rand::thread_rng().gen_range(0..max_duration);
// 		futs.push(tokio::spawn(async move {
// 			tokio::time::sleep(Duration::from_millis(duration)).await;
// 			test_ctx.call_update(0, 0).await;
// 		}));
// 	}

// 	// Smoke test finding lobbies
// 	let mut futs = Vec::new();
// 	for i in 0..10 {
// 		let test_ctx = test_ctx.clone();
// 		let duration = rand::thread_rng().gen_range(0..max_duration);
// 		futs.push(tokio::spawn(async move {
// 			tokio::time::sleep(Duration::from_millis(duration)).await;
// 			tracing::info!("finding lobby");
// 			let player_id = Uuid::new_v4();
// 			let query_id = Uuid::new_v4();
// 			let find_res = msg!([test_ctx.test_ctx] @notrace mm::msg::lobby_find(test_ctx.namespace_id, query_id) -> Result<mm::msg::lobby_find_complete, mm::msg::lobby_find_fail> {
// 				namespace_id: Some(test_ctx.namespace_id.into()),
// 				query_id: Some(query_id.into()),
// 				join_kind: backend::matchmaker::query::JoinKind::Normal as i32,
// 				players: vec![mm::msg::lobby_find::Player {
// 					player_id: Some(player_id.into()),
// 					token_session_id: Some(Uuid::new_v4().into()),
// 					client_info: Some(backend::net::ClientInfo {
// 						user_agent: Some("Test".into()),
// 						remote_address: Some(util::faker::ip_addr_v4().to_string()),
// 					}),
// 				}],
// 				query: Some(mm::msg::lobby_find::message::Query::LobbyGroup(backend::matchmaker::query::LobbyGroup {
// 					lobby_group_ids: vec![test_ctx.lobby_group_id.into()],
// 					region_ids: vec![test_ctx.region_id.into()],
// 					auto_create: None,
// 				})),
// 			})
// 			.await
// 			.unwrap().unwrap();

// 		}));
// 	}

// 	futures_util::future::try_join_all(futs).await.unwrap();

// 	// Wait to let any pending requests resolve
// 	tokio::time::sleep(Duration::from_secs(5)).await;

// 	// Check that the correct number of idle lobbies exist
// 	assert_eq!(
// 		2,
// 		test_ctx.fetch_idle_lobby_ids().await.len(),
// 		"incorrect number of remaining idle lobbies"
// 	);

// 	tracing::info!("complete");
// }
