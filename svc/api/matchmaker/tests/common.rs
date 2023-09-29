use std::{str::FromStr, sync::Once};

use proto::backend::{self, pkg::*};
use rivet_api::apis::configuration::Configuration;
use rivet_operation::prelude::*;

pub const LOBBY_GROUP_NAME_ID_BRIDGE: &str = "test-bridge";
pub const LOBBY_GROUP_NAME_ID_HOST: &str = "test-host";

pub static GLOBAL_INIT: Once = Once::new();

#[allow(unused)]
pub struct Ctx {
	pub op_ctx: OperationContext<()>,
	pub primary_region_id: Uuid,
	pub primary_region_name_id: String,
	pub game_id: Uuid,
	pub game_name_id: String,
	pub namespace_id: Uuid,
	pub namespace_name_id: String,
	pub custom_domain: String,
	pub version_id: Uuid,
	pub mm_config: backend::matchmaker::VersionConfig,
	pub mm_config_meta: backend::matchmaker::VersionConfigMeta,
	pub ns_auth_token: String,
	pub ns_dev_auth_token: String,
}

impl Ctx {
	pub async fn init() -> Ctx {
		GLOBAL_INIT.call_once(|| {
			tracing_subscriber::fmt()
				.pretty()
				.with_max_level(tracing::Level::INFO)
				.with_target(false)
				.without_time()
				.init();
		});

		let pools = rivet_pools::from_env("api-matchmaker-test").await.unwrap();
		let cache = rivet_cache::CacheInner::new(
			"api-matchmaker-test".to_string(),
			std::env::var("RIVET_SOURCE_HASH").unwrap(),
			pools.redis_cache().unwrap(),
		);
		let client = chirp_client::SharedClient::from_env(pools.clone())
			.expect("create client")
			.wrap_new("api-matchmaker-test");
		let conn = rivet_connection::Connection::new(client, pools, cache);
		let op_ctx = OperationContext::new(
			"api-matchmaker-test".to_string(),
			std::time::Duration::from_secs(60),
			conn,
			Uuid::new_v4(),
			Uuid::new_v4(),
			util::timestamp::now(),
			util::timestamp::now(),
			(),
			Vec::new(),
		);

		let (primary_region_id, primary_region_name_id) = Self::setup_region(&op_ctx).await;
		let (game_id, version_id, namespace_id, mm_config, mm_config_meta) =
			Self::setup_game(&op_ctx, primary_region_id).await;
		let ns_auth_token = Self::setup_public_token(&op_ctx, namespace_id).await;
		let ns_dev_auth_token =
			Self::setup_dev_token(&op_ctx, namespace_id, "127.0.0.1".to_owned(), Vec::new()).await;

		let game_get_res = op!([op_ctx] game_get {
			game_ids: vec![game_id.into()],
		})
		.await
		.unwrap();
		let game_data = game_get_res.games.first().unwrap();

		let custom_domain = format!("{}.com", util::faker::ident());
		op!([op_ctx] cdn_namespace_domain_create {
			namespace_id: Some(namespace_id.into()),
			domain: custom_domain.clone(),
		})
		.await
		.unwrap();

		let ns_get_res = op!([op_ctx] game_namespace_get {
			namespace_ids: vec![namespace_id.into()],
		})
		.await
		.unwrap();
		let ns_data = ns_get_res.namespaces.first().unwrap();

		Ctx {
			op_ctx,
			primary_region_id,
			primary_region_name_id,
			game_id,
			game_name_id: game_data.name_id.clone(),
			namespace_id,
			namespace_name_id: ns_data.name_id.clone(),
			custom_domain,
			version_id,
			mm_config,
			mm_config_meta,
			ns_auth_token,
			ns_dev_auth_token,
		}
	}

	pub fn config(&self, bearer_token: String) -> Configuration {
		Configuration {
			base_path: util::env::domain_main().into(),
			bearer_access_token: Some(bearer_token),
			..Default::default()
		}
	}

	pub async fn setup_region(ctx: &OperationContext<()>) -> (Uuid, String) {
		tracing::info!("setup region");

		let region_res = op!([ctx] faker_region {}).await.unwrap();
		let region_id = region_res.region_id.as_ref().unwrap().as_uuid();

		let get_res = op!([ctx] region_get {
			region_ids: vec![region_id.into()],
		})
		.await
		.unwrap();
		let region_data = get_res.regions.first().unwrap();

		(region_id, region_data.name_id.clone())
	}

	pub async fn setup_game(
		ctx: &OperationContext<()>,
		region_id: Uuid,
	) -> (
		Uuid,
		Uuid,
		Uuid,
		backend::matchmaker::VersionConfig,
		backend::matchmaker::VersionConfigMeta,
	) {
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

		let game_version_res = op!([ctx] faker_game_version {
			game_id: game_res.game_id,
			override_lobby_groups: Some(faker::game_version::request::OverrideLobbyGroups {
				lobby_groups: vec![
					backend::matchmaker::LobbyGroup {
						name_id: LOBBY_GROUP_NAME_ID_BRIDGE.into(),

						regions: vec![backend::matchmaker::lobby_group::Region {
							region_id: Some(region_id.into()),
							tier_name_id: "basic-1d8".into(),
							idle_lobbies: Some(backend::matchmaker::lobby_group::IdleLobbies {
								min_idle_lobbies: 0,
								// Set a high max lobby count in case this is
								// coming from a test that test mm-lobby-create
								// without creating an associated player
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
							env_vars: Vec::new(),
							network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Bridge as i32,
							ports: vec![
								backend::matchmaker::lobby_runtime::Port {
									label: "test-80-http".into(),
									target_port: Some(80),
									port_range: None,
									proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Http as i32,
									proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
								},
								backend::matchmaker::lobby_runtime::Port {
									label: "test-80-https".into(),
									target_port: Some(80),
									port_range: None,
									proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Https as i32,
										proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
								},
								backend::matchmaker::lobby_runtime::Port {
									label: "test-5050-https".into(),
									target_port: Some(5050),
									port_range: None,
									proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Https as i32,
									proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
								},
								backend::matchmaker::lobby_runtime::Port {
									label: "test-5051-tcp".into(),
									target_port: Some(5051),
									port_range: None,
									proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Tcp as i32,
									proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
								},
								backend::matchmaker::lobby_runtime::Port {
									label: "test-5051-tls".into(),
									target_port: Some(5051),
									port_range: None,
									proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::TcpTls as i32,
									proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
								},
								backend::matchmaker::lobby_runtime::Port {
									label: "test-5052-udp".into(),
									target_port: Some(5052),
									port_range: None,
									proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Udp as i32,
									proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
								},
							],
						}.into()),

						find_config: None,
						join_config: None,
						create_config: Some(backend::matchmaker::CreateConfig {
							identity_requirement: backend::matchmaker::IdentityRequirement::None as i32,
							verification_config: None,

							enable_public: true,
							enable_private: true,
							max_lobbies_per_identity: Some(1),
						}),
					},
					backend::matchmaker::LobbyGroup {
						name_id: LOBBY_GROUP_NAME_ID_HOST.into(),

						regions: vec![backend::matchmaker::lobby_group::Region {
							region_id: Some(region_id.into()),
							tier_name_id: "basic-1d8".into(),
							idle_lobbies: None,
						}],
						max_players_normal: 8,
						max_players_direct: 10,
						max_players_party: 12,
						listable: true,

						runtime: Some(backend::matchmaker::lobby_runtime::Docker {
							build_id: build_res.build_id,
							args: Vec::new(),
							env_vars: Vec::new(),
							network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Host as i32,
							ports: vec![
								backend::matchmaker::lobby_runtime::Port {
									label: "test-80-http".into(),
									target_port: Some(80),
									port_range: None,
									proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Http as i32,
									proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
								},
								backend::matchmaker::lobby_runtime::Port {
									label: "test-80-https".into(),
									target_port: Some(80),
									port_range: None,
									proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Https as i32,
									proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
								},
								backend::matchmaker::lobby_runtime::Port {
									label: "test-5050-https".into(),
									target_port: Some(5050),
									port_range: None,
									proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Https as i32,
									proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::GameGuard as i32,
								},
								backend::matchmaker::lobby_runtime::Port {
									label: "test-26000-udp".into(),
									target_port: None,
									port_range: Some(backend::matchmaker::lobby_runtime::PortRange {
										min: 26000,
										max: 27000,
									}),
									proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Udp as i32,
									proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::None as i32,
								},
								backend::matchmaker::lobby_runtime::Port {
									label: "test-28000-udp".into(),
									target_port: None,
									port_range: Some(backend::matchmaker::lobby_runtime::PortRange {
										min: 28000,
										max: 28000,
									}),
									proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Udp as i32,
									proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::None as i32,
								},
							],
						}.into()),

						find_config: None,
						join_config: None,
						create_config: None,
					},
				],
			}),
			..Default::default()
		})
		.await
		.unwrap();

		let namespace_res = op!([ctx] faker_game_namespace {
			game_id: game_res.game_id,
			version_id: game_version_res.version_id,
			..Default::default()
		})
		.await
		.unwrap();

		(
			game_res.game_id.as_ref().unwrap().as_uuid(),
			game_version_res.version_id.as_ref().unwrap().as_uuid(),
			namespace_res.namespace_id.as_ref().unwrap().as_uuid(),
			game_version_res.mm_config.clone().unwrap(),
			game_version_res.mm_config_meta.clone().unwrap(),
		)
	}

	pub async fn setup_public_token(ctx: &OperationContext<()>, namespace_id: Uuid) -> String {
		let token_res = op!([ctx] cloud_namespace_token_public_create {
			namespace_id: Some(namespace_id.into()),
		})
		.await
		.unwrap();

		token_res.token
	}

	pub async fn setup_dev_token(
		ctx: &OperationContext<()>,
		namespace_id: Uuid,
		hostname: String,
		lobby_ports: Vec<backend::matchmaker::lobby_runtime::Port>,
	) -> String {
		let token_res = op!([ctx] cloud_namespace_token_development_create {
			hostname: hostname.to_owned(),
			namespace_id: Some(namespace_id.into()),
			lobby_ports: lobby_ports,
		})
		.await
		.unwrap();

		token_res.token
	}

	pub fn chirp(&self) -> &chirp_client::Client {
		self.op_ctx.chirp()
	}

	pub fn op_ctx(&self) -> &OperationContext<()> {
		&self.op_ctx
	}
}

impl Ctx {
	/// Issues a testing lobby token. We use this since we can't access the lobby token issued
	/// on the lobby creation.
	pub async fn lobby_token(&self, lobby_id: &str) -> String {
		let token_res = op!([self] token_create {
			issuer: "test".into(),
			token_config: Some(token::create::request::TokenConfig {
				ttl: util::duration::days(365),
			}),
			refresh_token_config: None,
			client: None,
			kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
				entitlements: vec![
					proto::claims::Entitlement {
						kind: Some(
							proto::claims::entitlement::Kind::MatchmakerLobby(proto::claims::entitlement::MatchmakerLobby {
								lobby_id: Some(Uuid::from_str(lobby_id).unwrap().into()),
							})
						)
					}
				],
			})),
			label: Some("lobby".into()),
			..Default::default()
		})
		.await
		.unwrap();

		token_res.token.as_ref().unwrap().token.clone()
	}
}
