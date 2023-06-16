use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

use std::collections::HashSet;

struct TestVersion {
	region_id: Uuid,
	version_id: Uuid,
}

impl TestVersion {
	async fn create(ctx: &TestCtx, name: &str) -> Self {
		let version_id = Uuid::new_v4();

		let region_res = op!([ctx] faker_region {}).await.unwrap();
		let region_id = region_res.region_id.as_ref().unwrap().as_uuid();

		let game_res = op!([ctx] faker_game {
			..Default::default()
		})
		.await
		.unwrap();

		let build_res = op!([ctx] faker_build {
			game_id: game_res.game_id,
			image: faker::build::Image::MmLobbyAutoReady as i32,
		})
		.await
		.unwrap();

		let config = backend::matchmaker::VersionConfig {
			lobby_groups: vec![backend::matchmaker::LobbyGroup {
				name_id: format!("test-{}", name),

				regions: vec![backend::matchmaker::lobby_group::Region {
					region_id: Some(region_id.into()),
					tier_name_id: util_mm::test::TIER_NAME_ID.to_owned(),
					idle_lobbies: None,
				}],
				max_players_normal: 8,
				max_players_direct: 10,
				max_players_party: 12,

				runtime: Some(
					backend::matchmaker::lobby_runtime::Docker {
						build_id: build_res.build_id,
						args: Vec::new(),
						env_vars: vec![backend::matchmaker::lobby_runtime::EnvVar {
							key: "HELLO".into(),
							value: "world".into(),
						}],
						network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Bridge
							as i32,
						ports: vec![backend::matchmaker::lobby_runtime::Port {
							label: "1234".into(),
							target_port: Some(1234),
							port_range: None,
							proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Http
								as i32,
							proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard
								as i32,
						}],
					}
					.into(),
				),
			}],
			captcha: None,
		};

		let version_prepare_res = op!([ctx] mm_config_version_prepare {
			game_id: game_res.game_id,
			config: Some(config.clone()),
		})
		.await
		.unwrap();

		op!([ctx] mm_config_version_publish {
			version_id: Some(version_id.into()),
			config: Some(config.clone()),
			config_ctx: version_prepare_res.config_ctx.clone(),
		})
		.await
		.unwrap();

		TestVersion {
			region_id,
			version_id,
		}
	}
}

#[worker_test]
async fn single(ctx: TestCtx) {
	let version = TestVersion::create(&ctx, "a").await;

	let queries = vec![
		vec![version.version_id.into()],
		vec![
			version.version_id.into(),
			Uuid::new_v4().into(),
			Uuid::new_v4().into(),
		],
		vec![
			Uuid::new_v4().into(),
			version.version_id.into(),
			Uuid::new_v4().into(),
		],
	];
	for query in queries {
		let res = op!([ctx] mm_config_version_get {
			version_ids: query
		})
		.await
		.unwrap();
		let version_res = res.versions.first().expect("version not returned");
		let version_config_res = version_res.config.as_ref().unwrap();
		let lobby_group_res = version_config_res
			.lobby_groups
			.first()
			.expect("missing lobby group");
		assert_eq!("test-a", lobby_group_res.name_id);
		assert_eq!(8, lobby_group_res.max_players_normal);
		assert_eq!(10, lobby_group_res.max_players_direct);
		assert_eq!(12, lobby_group_res.max_players_party);

		let region_res = lobby_group_res.regions.first().expect("missing version");
		assert_eq!(
			version.region_id,
			region_res.region_id.as_ref().unwrap().as_uuid()
		);

		let runtime = lobby_group_res
			.runtime
			.as_ref()
			.unwrap()
			.runtime
			.as_ref()
			.unwrap();
		match runtime {
			backend::matchmaker::lobby_runtime::Runtime::Docker(runtime) => {
				let port_res = runtime.ports.first().expect("missing port");
				assert_eq!(1234, port_res.target_port.unwrap());

				let env_var_res = runtime.env_vars.first().expect("missing env var");
				assert_eq!("HELLO", env_var_res.key);
				assert_eq!("world", env_var_res.value);
			}
		}
	}
}

#[worker_test]
async fn multiple(ctx: TestCtx) {
	let version_a = TestVersion::create(&ctx, "a").await;
	let version_b = TestVersion::create(&ctx, "b").await;
	let version_c = TestVersion::create(&ctx, "c").await;

	let queries: Vec<(Vec<common::Uuid>, Vec<&'static str>)> = vec![
		(
			vec![
				version_a.version_id.into(),
				version_b.version_id.into(),
				version_c.version_id.into(),
				Uuid::new_v4().into(),
			],
			vec!["test-a", "test-b", "test-c"],
		),
		(
			vec![
				version_a.version_id.into(),
				Uuid::new_v4().into(),
				Uuid::new_v4().into(),
			],
			vec!["test-a"],
		),
		(
			vec![
				Uuid::new_v4().into(),
				version_c.version_id.into(),
				version_a.version_id.into(),
				Uuid::new_v4().into(),
			],
			vec!["test-c", "test-a"],
		),
	];
	for (query, lobby_group_names) in queries {
		let res = op!([ctx] mm_config_version_get {
			version_ids: query
		})
		.await
		.unwrap();

		assert_eq!(
			lobby_group_names.len(),
			res.versions.len(),
			"wrong version count"
		);

		// Validate lobby gropu names
		let actual_lobby_group_names = lobby_group_names
			.iter()
			.map(|x| x.to_string())
			.collect::<HashSet<String>>();
		let res_lobby_group_names = res
			.versions
			.iter()
			.map(|x| x.config.as_ref().unwrap().lobby_groups[0].name_id.clone())
			.collect::<HashSet<String>>();
		assert_eq!(
			actual_lobby_group_names, res_lobby_group_names,
			"mismatched lobby group names"
		)
	}
}
