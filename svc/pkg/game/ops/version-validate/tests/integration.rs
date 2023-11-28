use chirp_worker::prelude::*;
use proto::backend::{self, cdn::*, cloud, matchmaker};

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let res = op!([ctx] game_version_validate {
		game_id: Some(game_res.game_id.unwrap()),
		display_name: "   bad name".to_owned(),
		config: Some(cloud::VersionConfig {
			cdn: Some(VersionConfig {
				site_id: None,
				routes: vec![
					Route {
						glob: Some(util::glob::Glob::parse("test-glob").unwrap().into()),
						priority: 0,
						middlewares: vec![Middleware {
							kind: Some(middleware::Kind::CustomHeaders(CustomHeadersMiddleware {
								headers: vec![custom_headers_middleware::Header {
									name: "a".repeat(513).to_string(),
									value: "b".repeat(1025).to_string(),
								}],
							})),
						},
						Middleware {
							kind: Some(middleware::Kind::CustomHeaders(CustomHeadersMiddleware {
								headers: vec![],
							})),
						}],
					},
					Route {
						glob: Some(util::glob::Glob::parse("test-glob").unwrap().into()),
						priority: 0,
						middlewares: vec![Middleware {
							kind: Some(middleware::Kind::CustomHeaders(CustomHeadersMiddleware {
								headers: vec![],
							})),
						}],
					},
				],
			}),
			matchmaker: Some(matchmaker::VersionConfig {
				lobby_groups: vec![
					matchmaker::LobbyGroup {
						name_id: "name".to_owned(),
						regions: Vec::new(),
						max_players_normal: 33,
						max_players_direct: 0,
						max_players_party: 16,
						listable: true,
						taggable: false,
						allow_dynamic_max_players: false,

						runtime: Some(matchmaker::LobbyRuntime {
							runtime: Some(matchmaker::lobby_runtime::Runtime::Docker(
								matchmaker::lobby_runtime::Docker {
									build_id: None,
									args: Vec::new(),
									env_vars: vec![
										matchmaker::lobby_runtime::EnvVar {
											key: "key".to_owned(),
											value: "value".to_owned(),
										},
										matchmaker::lobby_runtime::EnvVar {
											key: "key".to_owned(),
											value: "value".to_owned(),
										},
									],
									network_mode: matchmaker::lobby_runtime::NetworkMode::Bridge
										as i32,
									ports: vec![
										matchmaker::lobby_runtime::Port {
											label: "80".into(),
											target_port: Some(80),
											port_range: None,
											proxy_protocol:
												matchmaker::lobby_runtime::ProxyProtocol::Http
													as i32,
											proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
										},
										matchmaker::lobby_runtime::Port {
											label: "80".into(),
											target_port: Some(80),
											port_range: None,
											proxy_protocol:
												matchmaker::lobby_runtime::ProxyProtocol::Http
													as i32,
											proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
										},
										matchmaker::lobby_runtime::Port {
											label: "80".into(),
											target_port: Some(80),
											port_range: None,
											proxy_protocol:
												matchmaker::lobby_runtime::ProxyProtocol::Https
													as i32,
											proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
										},
										matchmaker::lobby_runtime::Port {
											label: "udp1".into(),
											target_port: None,
											port_range: Some(
												matchmaker::lobby_runtime::PortRange {
													min: 26000,
													max: 26001,
												},
											),
											proxy_protocol:
												matchmaker::lobby_runtime::ProxyProtocol::Udp as i32,
											proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::None as i32,
										},
										matchmaker::lobby_runtime::Port {
											label: "udp2".into(),
											target_port: None,
											port_range: Some(
												matchmaker::lobby_runtime::PortRange {
													min: 26001,
													max: 26003,
												},
											),
											proxy_protocol:
												matchmaker::lobby_runtime::ProxyProtocol::Udp as i32,
											proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::None as i32,
										},
									],
								},
							)),
						}),

						actions: None,
					},
					matchmaker::LobbyGroup {
						name_id: "name".to_owned(),
						regions: vec![matchmaker::lobby_group::Region {
							region_id: Some(Uuid::new_v4().into()),
							tier_name_id: util::faker::ident(),
							idle_lobbies: None,
						}],
						max_players_normal: 33,
						max_players_direct: 0,
						max_players_party: 16,
						listable: true,
						taggable: false,
						allow_dynamic_max_players: false,

						runtime: Some(matchmaker::LobbyRuntime {
							runtime: Some(matchmaker::lobby_runtime::Runtime::Docker(
								matchmaker::lobby_runtime::Docker {
									build_id: None,
									args: Vec::new(),
									env_vars: Vec::new(),
									network_mode: matchmaker::lobby_runtime::NetworkMode::Bridge
										as i32,
									ports: Vec::new(),
								},
							)),
						}),

						actions: None,
					},
				],
				captcha: Some(backend::captcha::CaptchaConfig {
					requests_before_reverify: 601,
					verification_ttl: util::duration::hours(13),
					hcaptcha: Some(backend::captcha::captcha_config::Hcaptcha {
						level: backend::captcha::captcha_config::hcaptcha::Level::Easy as i32,
					}),
					..Default::default()
				}),
			}),
			kv: None,
			identity: Some(backend::identity::VersionConfig {
				custom_display_names: vec![backend::identity::CustomDisplayName {
					display_name: "some bad name".to_string(),
				}],
				custom_avatars: Vec::new(),
			}),
			module: Some(backend::module::GameVersionConfig {
				dependencies: Vec::new(),
				..Default::default()
			}),
		}),
	})
	.await
	.unwrap();

	assert_eq!(res.errors.len(), 20, "validation failed");
}
