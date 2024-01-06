use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let region_res = op!([ctx] faker_region {}).await.unwrap();
	let region_id = region_res.region_id.as_ref().unwrap().as_uuid();

	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let build_res = op!([ctx] faker_build {
		game_id: game_res.game_id,
		image: backend::faker::Image::MmLobbyAutoReady as i32,
	})
	.await
	.unwrap();

	let prepare_res = op!([ctx] mm_config_version_prepare {
		game_id: game_res.game_id,
		config: Some(backend::matchmaker::VersionConfig {
			lobby_groups: vec![
				backend::matchmaker::LobbyGroup {
					name_id: "test".into(),

					regions: vec![backend::matchmaker::lobby_group::Region {
						region_id: Some(region_id.into()),
						tier_name_id: util_mm::test::TIER_NAME_ID.to_owned(),
						idle_lobbies: None,
					}],
					max_players_normal: 8,
					max_players_direct: 8,
					max_players_party: 8,
					listable: true,
					taggable: false,
					allow_dynamic_max_players: false,

					runtime: Some(backend::matchmaker::LobbyRuntime {
						runtime: Some(backend::matchmaker::lobby_runtime::Runtime::Docker(
							backend::matchmaker::lobby_runtime::Docker {
								build_id: build_res.build_id,
								args: Vec::new(),
								env_vars: vec![
									backend::matchmaker::lobby_runtime::EnvVar {
										key: "HELLO".into(),
										value: "world".into(),
									},
								],
								network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Bridge as i32,
								ports: vec![
									backend::matchmaker::lobby_runtime::Port {
										label: "test".to_owned(),
										target_port: Some(1234),
										port_range: None,
										proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Https as i32,
										proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
									},
								],
							}
						 )),
					}),

					actions: None,
				},
			],
			captcha: None,
		}),
	})
	.await
	.unwrap();

	let version_id = Uuid::new_v4();
	op!([ctx] mm_config_version_publish {
		version_id: Some(version_id.into()),
		config: Some(backend::matchmaker::VersionConfig {
			lobby_groups: vec![
				backend::matchmaker::LobbyGroup {
					name_id: "test-1".into(),

					regions: vec![
						backend::matchmaker::lobby_group::Region {
							region_id: Some(region_id.into()),
							tier_name_id: util_mm::test::TIER_NAME_ID.to_owned(),
							idle_lobbies: None,
						},
					],
					max_players_normal: 8,
					max_players_direct: 10,
					max_players_party: 12,
					listable: true,
					taggable: false,
					allow_dynamic_max_players: false,

					runtime: Some(
						backend::matchmaker::lobby_runtime::Docker {
							build_id: build_res.build_id,
							args: Vec::new(),
							env_vars: vec![
								backend::matchmaker::lobby_runtime::EnvVar {
									key: "HELLO".into(),
									value: "world".into(),
								},
							],
							network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Bridge as i32,
							ports: vec![
								backend::matchmaker::lobby_runtime::Port {
									label: "test".to_owned(),
									target_port: Some(1234),
									port_range: None,
									proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Https as i32,
									proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
								},
							],
						}
						.into()
					),

					actions: None,
				},
			],
			captcha: None,
		}),
		config_ctx: prepare_res.config_ctx.clone(),
	})
	.await
	.unwrap();

	// TODO: Validate the outputs
}
