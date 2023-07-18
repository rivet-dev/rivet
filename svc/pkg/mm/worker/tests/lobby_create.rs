use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

struct Setup {
	namespace_id: Uuid,
	lobby_group_id: Uuid,
	region_id: Uuid,
}

async fn setup(ctx: &TestCtx) -> Setup {
	let region_res = op!([ctx] faker_region {}).await.unwrap();
	let region_id = region_res.region_id.as_ref().unwrap().as_uuid();

	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = game_res.namespace_ids.first().unwrap().clone().as_uuid();

	let build_res = op!([ctx] faker_build {
		game_id: game_res.game_id,
		image: faker::build::Image::MmLobbyAutoReady as i32,
	})
	.await
	.unwrap();

	let game_version_res = op!([ctx] faker_game_version {
		game_id: game_res.game_id,
		override_lobby_groups: Some(faker::game_version::request::OverrideLobbyGroups {
			lobby_groups: vec![backend::matchmaker::LobbyGroup {
				name_id: "test-1".into(),

				regions: vec![backend::matchmaker::lobby_group::Region {
					region_id: Some(region_id.into()),
					tier_name_id: util_mm::test::TIER_NAME_ID.to_owned(),
					idle_lobbies: None,
				}],
				max_players_normal: 8,
				max_players_direct: 10,
				max_players_party: 12,
				listable: true,

				runtime: Some(backend::matchmaker::lobby_runtime::Docker {
					build_id: build_res.build_id,
					args: Vec::new(),
					env_vars: vec![backend::matchmaker::lobby_runtime::EnvVar {
						key: "HELLO".into(),
						value: "world".into(),
					}],
					network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Bridge as i32,
					ports: vec![
						backend::matchmaker::lobby_runtime::Port {
							label: "1234".into(),
							target_port: Some(1234),
							port_range: None,
							proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Http as i32,
							proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
						},
						backend::matchmaker::lobby_runtime::Port {
							label: "1235".into(),
							target_port: Some(1235),
							port_range: None,
							proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Https as i32,
							proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
						},
						backend::matchmaker::lobby_runtime::Port {
							label: "1236".into(),
							target_port: Some(1236),
							port_range: None,
							proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Tcp as i32,
							proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
						},
						backend::matchmaker::lobby_runtime::Port {
							label: "1237".into(),
							target_port: Some(1237),
							port_range: None,
							proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::TcpTls as i32,
							proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
						},
					],

				}.into()),

				find_config: None,
				join_config: None,
				create_config: None,
			},
			backend::matchmaker::LobbyGroup {
				name_id: "test-2".into(),

				regions: vec![backend::matchmaker::lobby_group::Region {
					region_id: Some(region_id.into()),
					tier_name_id: util_mm::test::TIER_NAME_ID.to_owned(),
					idle_lobbies: None,
				}],
				max_players_normal: 8,
				max_players_direct: 10,
				max_players_party: 12,
				listable: true,

				runtime: Some(backend::matchmaker::lobby_runtime::Docker {
					build_id: build_res.build_id,
					args: Vec::new(),
					env_vars: vec![backend::matchmaker::lobby_runtime::EnvVar {
						key: "HELLO".into(),
						value: "world".into(),
					}],
					network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Host as i32,
					ports: vec![
						backend::matchmaker::lobby_runtime::Port {
							label: "1234".into(),
							target_port: Some(1234),
							port_range: None,
							proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Http as i32,
							proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
						},
						backend::matchmaker::lobby_runtime::Port {
							label: "26000-27000".into(),
							target_port: None,
							port_range: Some(backend::matchmaker::lobby_runtime::PortRange {
								min: 26000,
								max: 27000,
							}),
							proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Udp as i32,
							proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::None as i32,
						},
					],

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

	let version_get_res = op!([ctx] mm_config_version_get {
		version_ids: vec![game_version_res.version_id.unwrap()],
	})
	.await
	.unwrap();
	let lobby_group_id = version_get_res
		.versions
		.first()
		.unwrap()
		.config_meta
		.as_ref()
		.unwrap()
		.lobby_groups
		.first()
		.unwrap()
		.lobby_group_id
		.as_ref()
		.unwrap()
		.as_uuid();

	op!([ctx] game_namespace_version_set {
		namespace_id: Some(namespace_id.into()),
		version_id: game_version_res.version_id,
	})
	.await
	.unwrap();

	Setup {
		namespace_id,
		lobby_group_id,
		region_id,
	}
}

#[worker_test]
async fn lobby_create(ctx: TestCtx) {
	let setup = setup(&ctx).await;

	let lobby_id = Uuid::new_v4();
	msg!([ctx] mm::msg::lobby_create(lobby_id) -> mm::msg::lobby_create_complete {
		lobby_id: Some(lobby_id.into()),
		namespace_id: Some(setup.namespace_id.into()),
		lobby_group_id: Some(setup.lobby_group_id.into()),
		region_id: Some(setup.region_id.into()),
		create_ray_id: None,
		preemptively_created: false,

		creator_user_id: None,
		is_custom: false,
		publicity: None,
		lobby_config_json: None,
	})
	.await
	.unwrap();

	let (sql_mpn, sql_mpd, sql_mpp) = sqlx::query_as::<_, (i64, i64, i64)>(indoc!(
		"
		SELECT max_players_normal, max_players_direct, max_players_party
		FROM lobbies
		WHERE lobby_id = $1
		"
	))
	.bind(lobby_id)
	.fetch_one(&ctx.crdb("db-mm-state").await.unwrap())
	.await
	.unwrap();
	assert_eq!(8, sql_mpn);
	assert_eq!(10, sql_mpd);
	assert_eq!(12, sql_mpp);
}

#[worker_test]
async fn custom_private_lobby_create(ctx: TestCtx) {
	let setup = setup(&ctx).await;

	let lobby_id = Uuid::new_v4();
	msg!([ctx] mm::msg::lobby_create(lobby_id) -> mm::msg::lobby_create_complete {
		lobby_id: Some(lobby_id.into()),
		namespace_id: Some(setup.namespace_id.into()),
		lobby_group_id: Some(setup.lobby_group_id.into()),
		region_id: Some(setup.region_id.into()),
		create_ray_id: None,
		preemptively_created: false,

		creator_user_id: None,
		is_custom: true,
		publicity: Some(backend::matchmaker::lobby::Publicity::Private as i32),
		lobby_config_json: Some(r#"{ "foo": "bar" }"#.to_string()),
	})
	.await
	.unwrap();

	let (is_custom, publicity) = sqlx::query_as::<_, (bool, i64)>(indoc!(
		"
		SELECT is_custom, publicity 
		FROM lobbies
		WHERE lobby_id = $1
		"
	))
	.bind(lobby_id)
	.fetch_one(&ctx.crdb("db-mm-state").await.unwrap())
	.await
	.unwrap();

	assert!(is_custom);
	assert_eq!(
		backend::matchmaker::lobby::Publicity::Private as i32 as i64,
		publicity
	);
}

#[worker_test]
async fn lobby_create_max_lobby_count(ctx: TestCtx) {
	let setup = setup(&ctx).await;

	let lobby_count_max = 3;
	op!([ctx] mm_config_namespace_config_set {
		namespace_id: Some(setup.namespace_id.into()),
		lobby_count_max: lobby_count_max,
		max_players_per_client: 3,
		max_players_per_client_vpn: 3,
		max_players_per_client_proxy: 3,
		max_players_per_client_tor: 3,
		max_players_per_client_hosting: 3,
	})
	.await
	.unwrap();

	// Create lobbies to fill up all existing slots
	for _ in 0..lobby_count_max {
		let lobby_id = Uuid::new_v4();
		msg!([ctx] mm::msg::lobby_create(lobby_id) -> mm::msg::lobby_create_complete {
			lobby_id: Some(lobby_id.into()),
			namespace_id: Some(setup.namespace_id.into()),
			lobby_group_id: Some(setup.lobby_group_id.into()),
			region_id: Some(setup.region_id.into()),
			create_ray_id: None,
			preemptively_created: false,

			creator_user_id: None,
			is_custom: false,
			publicity: None,
			lobby_config_json: None,
		})
		.await
		.unwrap();
	}

	// Bump it over the max
	let lobby_id = Uuid::new_v4();
	let fail_msg = msg!([ctx] mm::msg::lobby_create(lobby_id) -> mm::msg::lobby_create_fail {
		lobby_id: Some(lobby_id.into()),
		namespace_id: Some(setup.namespace_id.into()),
		lobby_group_id: Some(setup.lobby_group_id.into()),
		region_id: Some(setup.region_id.into()),
		create_ray_id: None,
		preemptively_created: false,

		creator_user_id: None,
		is_custom: false,
		publicity: None,
		lobby_config_json: None,
	})
	.await
	.unwrap();
	assert_eq!(
		mm::msg::lobby_create_fail::ErrorCode::LobbyCountOverMax as i32,
		fail_msg.error_code
	);
}

#[worker_test]
async fn lobby_create_reuse_job_id(ctx: TestCtx) {
	let setup = setup(&ctx).await;

	let lobby_id_a = Uuid::new_v4();
	msg!([ctx] @notrace mm::msg::lobby_create(lobby_id_a) -> mm::msg::lobby_ready_complete {
		lobby_id: Some(lobby_id_a.into()),
		namespace_id: Some(setup.namespace_id.into()),
		lobby_group_id: Some(setup.lobby_group_id.into()),
		region_id: Some(setup.region_id.into()),
		create_ray_id: None,
		preemptively_created: false,

		creator_user_id: None,
		is_custom: false,
		publicity: None,
		lobby_config_json: None,
	})
	.await
	.unwrap();

	let lobby_id_b = Uuid::new_v4();
	msg!([ctx] @notrace mm::msg::lobby_create(lobby_id_b) -> mm::msg::lobby_ready_complete {
		lobby_id: Some(lobby_id_b.into()),
		namespace_id: Some(setup.namespace_id.into()),
		lobby_group_id: Some(setup.lobby_group_id.into()),
		region_id: Some(setup.region_id.into()),
		create_ray_id: None,
		preemptively_created: false,

		creator_user_id: None,
		is_custom: false,
		publicity: None,
		lobby_config_json: None,
	})
	.await
	.unwrap();

	let lobbies = op!([ctx] mm_lobby_get {
		lobby_ids: vec![lobby_id_a.into(), lobby_id_b.into()],
	})
	.await
	.unwrap();
	let run_ids = lobbies
		.lobbies
		.iter()
		.flat_map(|x| x.run_id)
		.collect::<Vec<_>>();

	let runs = op!([ctx] job_run_get {
		run_ids: run_ids,
	})
	.await
	.unwrap();

	let Some(backend::job::RunMeta { kind: Some(backend::job::run_meta::Kind::Nomad(backend::job::run_meta::Nomad { dispatched_job_id: dispatch_a, .. }))}) = &runs.runs[0].run_meta else {
		panic!()
	};
	let Some(backend::job::RunMeta { kind: Some(backend::job::run_meta::Kind::Nomad(backend::job::run_meta::Nomad { dispatched_job_id: dispatch_b, .. }))}) = &runs.runs[1].run_meta else {
		panic!()
	};

	let job_a = dispatch_a.as_ref().unwrap().split_once('/').unwrap().0;
	let job_b = dispatch_b.as_ref().unwrap().split_once('/').unwrap().0;

	assert_eq!(job_a, job_b, "these runs were dispatched from different jobs. this means the nomad job is not deterministic.");
}

// TODO: Write test verifying that lobby config data shows up in lobby env

// #[worker_test]
// async fn lobby_create_stress_test(ctx: TestCtx) {
// 	let setup = setup(&ctx).await;

// 	loop {
// 		let lobby_id = Uuid::new_v4();
// 		msg!([ctx] mm::msg::lobby_create(lobby_id) -> mm::msg::lobby_create_complete {
// 			lobby_id: Some(lobby_id.into()),
// 			namespace_id: Some(setup.namespace_id.into()),
// 			lobby_group_id: Some(setup.lobby_group_id.into()),
// 			region_id: Some(setup.region_id.into()),
// 			create_ray_id: None,
// 			preemptively_created: false,

// 			creator_user_id: None,
// 			is_custom: false,
// 			publicity: None,
// 			lobby_config_json: None,
// 		})
// 		.await
// 		.unwrap();

// 		tokio::time::sleep(std::time::Duration::from_secs(1)).await;
// 	}
// }
