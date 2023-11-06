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
		image: faker::build::Image::MmLobbyAutoReady as i32,
	})
	.await
	.unwrap();

	op!([ctx] mm_config_version_prepare {
		game_id: game_res.game_id,
		config: Some(backend::matchmaker::VersionConfig {
			lobby_groups: vec![
				backend::matchmaker::LobbyGroup {
					name_id: "test-bridge".into(),

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

					runtime: Some(backend::matchmaker::lobby_runtime::Docker {
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
								label: "test-https".to_owned(),
								target_port: Some(1234),
								port_range: None,
								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Https as i32,
								proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
							},
						],
					}.into()),

					actions: None,
				},
				backend::matchmaker::LobbyGroup {
					name_id: "test-host".into(),

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

					runtime: Some(backend::matchmaker::lobby_runtime::Docker {
						build_id: build_res.build_id,
						args: Vec::new(),
						env_vars: vec![
							backend::matchmaker::lobby_runtime::EnvVar {
								key: "HELLO".into(),
								value: "world".into(),
							},
						],
						network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Host as i32,
						ports: vec![
							backend::matchmaker::lobby_runtime::Port {
								label: "test-https".to_owned(),
								target_port: Some(1234),
								port_range: None,
								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Https as i32,
								proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
							},
							backend::matchmaker::lobby_runtime::Port {
								label: "test-tcp".to_owned(),
								target_port: Some(1235),
								port_range: None,
								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Tcp as i32,
								proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
							},
							backend::matchmaker::lobby_runtime::Port {
								label: "test-tcp-tls".to_owned(),
								target_port: Some(1235),
								port_range: None,
								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Tcp as i32,
								proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
							},
							backend::matchmaker::lobby_runtime::Port {
								label: "test-tcp-tls".to_owned(),
								target_port: Some(1236),
								port_range: None,
								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Udp as i32,
								proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
							},
							backend::matchmaker::lobby_runtime::Port {
								label: "test-tcp-host".to_owned(),
								target_port: None,
								port_range: Some(backend::matchmaker::lobby_runtime::PortRange {
									min: 26000,
									max: 27000,
								}),
								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Udp as i32,
								proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::None as i32,
							},
							backend::matchmaker::lobby_runtime::Port {
								label: "test-udp-host".to_owned(),
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

					actions: None,
				},
			],
			captcha: None,
		}),
	})
	.await
	.unwrap();

	// TODO: Validate the outputs
}
