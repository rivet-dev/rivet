use chirp_worker::prelude::*;
use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

pub struct Setup {
	pub namespace_id: Uuid,
	pub lobby_group_id: Uuid,
	pub region_id: Uuid,
	pub region: backend::region::Region,
}

impl Setup {
	pub async fn init(ctx: &TestCtx) -> Self {
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
			image: faker::build::Image::MmLobbyEcho as i32,
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
						idle_lobbies: Some(backend::matchmaker::lobby_group::IdleLobbies {
							min_idle_lobbies: 0,
							// Don't auto-destory lobbies from tests
							max_idle_lobbies: 32,
						}),
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
							backend::matchmaker::lobby_runtime::Port {
								label: "test-http".into(),
								target_port: Some(8001),
								port_range: None,
								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Http as i32,
								proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
							},
							backend::matchmaker::lobby_runtime::Port {
								label: "test-tcp".into(),
								target_port: Some(8002),
								port_range: None,
								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Tcp as i32,
								proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
							},
							backend::matchmaker::lobby_runtime::Port {
								label: "test-udp".into(),
								target_port: Some(8003),
								port_range: None,
								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Udp as i32,
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
						idle_lobbies: Some(backend::matchmaker::lobby_group::IdleLobbies {
							min_idle_lobbies: 0,
							// See above
							max_idle_lobbies: 32,
						}),
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
			region: region_res.region.clone().unwrap(),
		}
	}

	pub async fn create_lobby(&self, ctx: &TestCtx) -> Uuid {
		let lobby_id = Uuid::new_v4();
		msg!([ctx] @notrace mm::msg::lobby_create(lobby_id) -> mm::msg::lobby_ready_complete(lobby_id) {
			lobby_id: Some(lobby_id.into()),
			namespace_id: Some(self.namespace_id.into()),
			lobby_group_id: Some(self.lobby_group_id.into()),
			region_id: Some(self.region_id.into()),
			create_ray_id: None,
			preemptively_created: false,

			creator_user_id: None,
			is_custom: false,
			publicity: None,
			lobby_config_json: None,
		})
		.await
		.unwrap();

		lobby_id
	}
}
