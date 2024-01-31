use chirp_worker::prelude::*;
use proto::backend::{self};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let region_list_res = op!([ctx] region_list {
		..Default::default()
	})
	.await
	.unwrap();

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

	let _res = op!([ctx] cloud_version_publish {
		game_id: game_res.game_id,
		display_name: "0.0.1".into(),
		config: Some(backend::cloud::VersionConfig {
			cdn: None,
			matchmaker: Some(backend::matchmaker::VersionConfig {
				lobby_groups: vec![
					backend::matchmaker::LobbyGroup {
						name_id: "test".into(),

						regions: region_list_res
							.region_ids
							.iter()
							.cloned()
							.map(|region_id| backend::matchmaker::lobby_group::Region {
								region_id: Some(region_id),
								tier_name_id: util_mm::test::TIER_NAME_ID.to_owned(),
								idle_lobbies: None,
							})
							.collect(),
						max_players_normal: 8,
						max_players_direct: 10,
						max_players_party: 12,
						listable: true,
						taggable: false,
						allow_dynamic_max_players: false,

						runtime: Some(backend::matchmaker::lobby_runtime::Docker {
							build_id: build_res.build_id,
							args: Vec::new(),
							env_vars: vec![
								backend::matchmaker::lobby_runtime::EnvVar {
									key: "HELLo".into(),
									value: "world".into(),
								},
							],
							network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Bridge as i32,
							ports: vec![
								backend::matchmaker::lobby_runtime::Port {
									label: "1234".into(),
									target_port: Some(1234),
									port_range: None,
									proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Https as i32,
									proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
								},
							],
						}.into()),

						actions: None,
					},
				],
				captcha: None,
			}),
			kv: None,
			identity: Some(backend::identity::VersionConfig {
				custom_display_names: vec![backend::identity::CustomDisplayName {
					display_name: "Guest".to_string(),
				}],
				custom_avatars: Vec::new(),
			}),
			module: Some(backend::module::GameVersionConfig {
				dependencies: Vec::new(),
			})
		})
	})
	.await
	.unwrap();

	// TODO: Validate state
}
