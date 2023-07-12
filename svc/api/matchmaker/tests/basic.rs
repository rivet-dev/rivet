use std::{collections::HashSet, str::FromStr, sync::Once};

use proto::backend::{self, pkg::*};
use rivet_api::apis::{configuration::Configuration, *};
use rivet_matchmaker::model;
use rivet_operation::prelude::*;

const LOBBY_GROUP_NAME_ID_BRIDGE: &str = "test-bridge";
const LOBBY_GROUP_NAME_ID_HOST: &str = "test-host";

static GLOBAL_INIT: Once = Once::new();

#[allow(unused)]
struct Ctx {
	op_ctx: OperationContext<()>,
	primary_region_id: Uuid,
	primary_region_name_id: String,
	game_id: Uuid,
	game_name_id: String,
	namespace_id: Uuid,
	namespace_name_id: String,
	custom_domain: String,
	version_id: Uuid,
	mm_config: backend::matchmaker::VersionConfig,
	mm_config_meta: backend::matchmaker::VersionConfigMeta,
	ns_auth_token: String,
	ns_dev_auth_token: String,
}

impl Ctx {
	async fn init() -> Ctx {
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

	fn config(&self, bearer_token: String) -> Configuration {
		Configuration {
			base_path: util::env::svc_router_url("api-matchmaker"),
			bearer_access_token: Some(bearer_token),
			..Default::default()
		}
	}

	fn http_client(&self, bearer_token: String) -> rivet_matchmaker::ClientWrapper {
		rivet_matchmaker::Config::builder()
			.set_uri(util::env::svc_router_url("api-matchmaker"))
			.set_bearer_token(bearer_token)
			.build_client()
	}

	async fn setup_region(ctx: &OperationContext<()>) -> (Uuid, String) {
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

	async fn setup_game(
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
							idle_lobbies: None,
						}],
						max_players_normal: 8,
						max_players_direct: 10,
						max_players_party: 12,

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

	async fn setup_public_token(ctx: &OperationContext<()>, namespace_id: Uuid) -> String {
		let token_res = op!([ctx] cloud_namespace_token_public_create {
			namespace_id: Some(namespace_id.into()),
		})
		.await
		.unwrap();

		token_res.token
	}

	async fn setup_dev_token(
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

	fn chirp(&self) -> &chirp_client::Client {
		self.op_ctx.chirp()
	}

	fn op_ctx(&self) -> &OperationContext<()> {
		&self.op_ctx
	}
}

impl Ctx {
	/// Issues a testing lobby token. We use this since we can't access the lobby token issued
	/// on the lobby creation.
	async fn lobby_token(&self, lobby_id: &str) -> String {
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

#[tokio::test(flavor = "multi_thread")]
async fn find_with_regions() {
	let ctx = Ctx::init().await;
	let http_client = ctx.http_client(ctx.ns_auth_token.clone());

	{
		tracing::info!("finding lobby");

		// Create lobby
		create_lobby(&ctx, Uuid::new_v4(), &ctx.mm_config_meta.lobby_groups[0]).await;

		let res = http_client
			.find_lobby()
			.set_game_modes(Some(vec![LOBBY_GROUP_NAME_ID_BRIDGE.into()]))
			.set_regions(Some(vec![ctx.primary_region_name_id.clone()]))
			.captcha(captcha_config())
			.send()
			.await
			.unwrap();

		assert_lobby_state(&ctx, res.lobby().unwrap()).await;
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn find_without_regions() {
	let ctx = Ctx::init().await;
	let http_client = ctx.http_client(ctx.ns_auth_token.clone());

	{
		tracing::info!("finding lobby");

		// Create lobby
		create_lobby(&ctx, Uuid::new_v4(), &ctx.mm_config_meta.lobby_groups[0]).await;

		let res = http_client
			.find_lobby()
			.set_game_modes(Some(vec![LOBBY_GROUP_NAME_ID_BRIDGE.into()]))
			.captcha(captcha_config())
			.send()
			.await
			.unwrap();

		assert_lobby_state(&ctx, res.lobby().unwrap()).await;
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn list_lobbies() {
	let ctx = Ctx::init().await;
	let http_client = ctx.http_client(ctx.ns_auth_token.clone());

	let lobby_group_meta = &ctx.mm_config_meta.lobby_groups[0];

	let mut lobby_ids = HashSet::new();
	for _ in 0..3 {
		let lobby_id = Uuid::new_v4();
		lobby_ids.insert(lobby_id);

		// Create lobby
		create_lobby(&ctx, lobby_id, lobby_group_meta).await;

		// Create players
		let query_id = Uuid::new_v4();
		let primary_player_id = Uuid::new_v4();
		msg!([ctx] @notrace mm::msg::lobby_find(ctx.namespace_id, query_id) -> Result<mm::msg::lobby_find_complete, mm::msg::lobby_find_fail> {
			namespace_id: Some(ctx.namespace_id.into()),
			query_id: Some(query_id.into()),
			join_kind: backend::matchmaker::query::JoinKind::Normal as i32,
			players: vec![
				mm::msg::lobby_find::Player {
					player_id: Some(primary_player_id.into()),
					token_session_id: Some(Uuid::new_v4().into()),
					client_info: None,
				},
				mm::msg::lobby_find::Player {
					player_id: Some(Uuid::new_v4().into()),
					token_session_id: Some(Uuid::new_v4().into()),
					client_info: None,
				},
			],
			query: Some(mm::msg::lobby_find::message::Query::Direct(backend::matchmaker::query::Direct {
				lobby_id: Some(lobby_id.into()),
			})),
			..Default::default()
		})
		.await
		.unwrap().unwrap();
	}

	{
		tracing::info!("listing lobbies");

		let res = http_client.list_lobbies().send().await.unwrap();
		tracing::info!(?res, "lobby list");

		let game_modes = res.game_modes().unwrap();
		assert_eq!(2, game_modes.len(), "wrong game mode count");

		let regions = res.regions().unwrap();
		assert_eq!(1, regions.len(), "wrong region count");
		let region = &regions[0];
		assert_eq!(
			ctx.primary_region_name_id,
			region.region_id().unwrap(),
			"wrong region name"
		);

		let lobbies = res.lobbies().unwrap();
		assert_eq!(lobby_ids.len(), lobbies.len(), "wrong lobby count");
		assert_eq!(
			lobby_ids,
			lobbies
				.iter()
				.map(|l| Uuid::from_str(l.lobby_id().unwrap()).unwrap())
				.collect::<HashSet<_>>(),
			"lobby ids don't match"
		);
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn lobby_lifecycle() {
	let ctx = Ctx::init().await;
	let http_client = ctx.http_client(ctx.ns_auth_token.clone());

	// MARK: POST /matchmaker/lobbies/find (A)
	let (lobby_a, lobby_a_token) = {
		tracing::info!("finding lobby a");

		// Create lobby
		create_lobby(&ctx, Uuid::new_v4(), &ctx.mm_config_meta.lobby_groups[0]).await;

		let res = http_client
			.find_lobby()
			.set_game_modes(Some(vec![LOBBY_GROUP_NAME_ID_BRIDGE.into()]))
			.captcha(captcha_config())
			.send()
			.await
			.unwrap();
		let lobby = res.lobby().unwrap();
		assert_lobby_state(&ctx, lobby).await;

		let lobby_token = ctx.lobby_token(lobby.lobby_id().unwrap()).await;

		(lobby.clone(), lobby_token)
	};

	let http_client_a = ctx.http_client(lobby_a_token.clone());

	// MARK: POST /matchmaker/player/connected (A)
	{
		tracing::info!("connected player a");

		let _res = http_client_a
			.player_connected()
			.player_token(lobby_a.player().unwrap().token().unwrap())
			.send()
			.await
			.unwrap();
	}

	// MARK: POST /matchmaker/lobbies/join (B)
	let (lobby_b, _lobby_b_token) = {
		tracing::info!("finding lobby b");

		let res = http_client
			.join_lobby()
			.lobby_id(lobby_a.lobby_id().unwrap())
			.captcha(captcha_config())
			.send()
			.await
			.unwrap();
		let lobby = res.lobby().unwrap();
		assert_lobby_state(&ctx, lobby).await;

		let lobby_token = ctx.lobby_token(lobby.lobby_id().unwrap()).await;

		(lobby.clone(), lobby_token)
	};

	// MARK: POST /matchmaker/player/connected (B)
	{
		tracing::info!("connected player b");

		let _res = http_client_a
			.player_connected()
			.player_token(lobby_b.player().unwrap().token().unwrap())
			.send()
			.await
			.unwrap();
	}

	// MARK: POST /matchmaker/player/disconnected (A)
	{
		tracing::info!("disconnected player a");

		let _res = http_client_a
			.player_disconnected()
			.player_token(lobby_a.player().unwrap().token().unwrap())
			.send()
			.await
			.unwrap();
	}

	// MARK: POST /matchmaker/player/disconnected (B)
	{
		tracing::info!("disconnected player b");

		let _res = http_client_a
			.player_disconnected()
			.player_token(lobby_a.player().unwrap().token().unwrap())
			.send()
			.await
			.unwrap();
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn lobby_lifecycle_dev() {
	use backend::matchmaker::lobby_runtime::{Port, ProxyKind, ProxyProtocol};

	let ctx = Ctx::init().await;

	// Create token
	let ns_dev_auth_token = Ctx::setup_dev_token(
		&ctx.op_ctx,
		ctx.namespace_id,
		"127.0.0.1".to_owned(),
		vec![
			Port {
				label: "test-80".into(),
				target_port: Some(80),
				port_range: None,
				proxy_protocol: ProxyProtocol::Https as i32,
				proxy_kind: ProxyKind::GameGuard as i32,
			},
			Port {
				label: "test-8080".into(),
				target_port: Some(8080),
				port_range: None,
				proxy_protocol: ProxyProtocol::Https as i32,
				proxy_kind: ProxyKind::GameGuard as i32,
			},
			Port {
				label: "test-5050".into(),
				target_port: Some(5050),
				port_range: None,
				proxy_protocol: ProxyProtocol::Https as i32,
				proxy_kind: ProxyKind::GameGuard as i32,
			},
		],
	)
	.await;

	let http_client = ctx.http_client(ns_dev_auth_token);

	// MARK: POST /matchmaker/lobbies/find (A)
	let lobby_a = {
		tracing::info!("finding lobby a");

		let res = http_client
			.find_lobby()
			.set_game_modes(Some(vec![LOBBY_GROUP_NAME_ID_BRIDGE.into()]))
			.captcha(captcha_config())
			.send()
			.await
			.unwrap();
		let lobby = res.lobby().unwrap();

		let ports = lobby.ports().unwrap();
		assert_eq!(3, ports.len(), "missing dev lobby port");

		{
			let p = ports.get("test-80").unwrap();
			assert_eq!(80, p.port().unwrap());
			assert!(p.is_tls().unwrap());
		}

		{
			let p = ports.get("test-8080").unwrap();
			assert_eq!(8080, p.port().unwrap());
			assert!(p.is_tls().unwrap());
		}

		{
			let p = ports.get("test-5050").unwrap();
			assert_eq!(5050, p.port().unwrap());
			assert!(p.is_tls().unwrap());
		}

		lobby.clone()
	};

	// MARK: POST /matchmaker/player/connected (A)
	{
		tracing::info!("connected player a");

		let _res = http_client
			.player_connected()
			.player_token(lobby_a.player().unwrap().token().unwrap())
			.send()
			.await
			.unwrap();
	}

	// MARK: POST /matchmaker/lobbies/join (B)
	let (lobby_b, _lobby_b_token) = {
		tracing::info!("finding lobby b");

		let res = http_client
			.join_lobby()
			.lobby_id(lobby_a.lobby_id().unwrap())
			.captcha(captcha_config())
			.send()
			.await
			.unwrap();
		let lobby = res.lobby().unwrap();
		let lobby_token = ctx.lobby_token(lobby.lobby_id().unwrap()).await;

		(lobby.clone(), lobby_token)
	};

	// MARK: POST /matchmaker/player/connected (B)
	{
		tracing::info!("connected player b");

		let _res = http_client
			.player_connected()
			.player_token(lobby_b.player().unwrap().token().unwrap())
			.send()
			.await
			.unwrap();
	}

	// MARK: PUT /matchmaker/lobbies/closed
	{
		tracing::info!("closing lobby a");

		let _res = http_client
			.set_lobby_closed()
			.is_closed(true)
			.send()
			.await
			.unwrap();

		tracing::info!("opening lobby a");

		let _res = http_client
			.set_lobby_closed()
			.is_closed(false)
			.send()
			.await
			.unwrap();
	}

	// MARK: POST /matchmaker/player/disconnected (A)
	{
		tracing::info!("disconnected player a");

		let _res = http_client
			.player_disconnected()
			.player_token(lobby_a.player().unwrap().token().unwrap())
			.send()
			.await
			.unwrap();
	}

	// MARK: POST /matchmaker/player/disconnected (B)
	{
		tracing::info!("disconnected player b");

		let _res = http_client
			.player_disconnected()
			.player_token(lobby_a.player().unwrap().token().unwrap())
			.send()
			.await
			.unwrap();
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn list_regions() {
	let ctx = Ctx::init().await;
	let http_client = ctx.http_client(ctx.ns_dev_auth_token.clone());

	// MARK: GET /matchmaker/regions/recommend
	{
		tracing::info!("recommending region");

		let _res = http_client.list_regions().send().await.unwrap();
	}
}

// NOTE: This test is identical to `recommend_region`
#[tokio::test(flavor = "multi_thread")]
async fn list_regions_dev() {
	let ctx = Ctx::init().await;
	let http_client = ctx.http_client(ctx.ns_dev_auth_token.clone());

	// MARK: GET /matchmaker/regions/recommend
	{
		tracing::info!("recommending region dev");

		let _res = http_client.list_regions().send().await.unwrap();
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn find_domain_auth() {
	let ctx = Ctx::init().await;
	let http_client = ctx.http_client(ctx.ns_auth_token.clone());

	// Normal domain
	{
		tracing::info!("finding lobby with domain auth");

		// Create lobby
		create_lobby(&ctx, Uuid::new_v4(), &ctx.mm_config_meta.lobby_groups[0]).await;

		let url = format!(
			"https://{}--{}.{}/hello-world",
			ctx.game_name_id,
			ctx.namespace_name_id,
			util::env::domain_cdn(),
		);

		let res = http_client
			.find_lobby()
			.origin(url)
			.set_game_modes(Some(vec![LOBBY_GROUP_NAME_ID_BRIDGE.into()]))
			.captcha(captcha_config())
			.send()
			.await
			.unwrap();
		assert_lobby_state(&ctx, res.lobby().unwrap()).await;
	}

	// Custom domain
	{
		tracing::info!("finding lobby with custom domain auth");

		let url = format!("https://{}/hello-world", ctx.custom_domain);

		let res = http_client
			.find_lobby()
			.origin(url)
			.set_game_modes(Some(vec![LOBBY_GROUP_NAME_ID_BRIDGE.into()]))
			.captcha(captcha_config())
			.send()
			.await
			.unwrap();
		assert_lobby_state(&ctx, res.lobby().unwrap()).await;
	}
}

#[tokio::test(flavor = "multi_thread")]
async fn player_statistics() {
	let ctx = Ctx::init().await;

	// MARK: GET /players/statistics
	let res = matchmaker_players_api::matchmaker_players_get_statistics(
		&ctx.config(ctx.ns_auth_token.clone()),
	)
	.await
	.unwrap();
	let player_count = res.player_count;
	let game_modes = res.game_modes;

	tracing::info!(?player_count, ?game_modes);
}

async fn assert_lobby_state(
	ctx: &Ctx,
	lobby: &model::MatchmakerLobbyJoinInfo,
) -> backend::matchmaker::Lobby {
	// Fetch lobby data
	let lobby_res = op!([ctx] mm_lobby_get {
		lobby_ids: vec![Uuid::from_str(lobby.lobby_id().unwrap()).unwrap().into()],
		..Default::default()
	})
	.await
	.unwrap();
	let lobby_data = lobby_res.lobbies.first().expect("lobby not created");
	assert!(lobby_data.ready_ts.is_some(), "lobby not ready");
	assert!(lobby_data.run_id.is_some(), "no run id");

	// Validate ports
	{
		tracing::info!(ports = ?lobby.ports().unwrap(), "validating ports");
		assert_eq!(6, lobby.ports().unwrap().len());

		{
			let p = lobby.ports().unwrap().get("test-80-http").unwrap();
			assert_eq!(80, p.port().unwrap());
			assert!(!p.is_tls().unwrap());
		}

		{
			let p = lobby.ports().unwrap().get("test-80-https").unwrap();
			assert_eq!(443, p.port().unwrap());
			assert!(p.is_tls().unwrap());
		}

		{
			let p = lobby.ports().unwrap().get("test-5050-https").unwrap();
			assert_eq!(443, p.port().unwrap());
			assert!(p.is_tls().unwrap());
		}

		{
			let p = lobby.ports().unwrap().get("test-5051-tcp").unwrap();
			assert!(
				p.port().unwrap() >= util_job::consts::MIN_INGRESS_PORT_TCP as i32
					&& p.port().unwrap() <= util_job::consts::MAX_INGRESS_PORT_TCP as i32
			);
			assert!(!p.is_tls().unwrap());
		}

		{
			let p = lobby.ports().unwrap().get("test-5051-tls").unwrap();
			assert!(
				p.port().unwrap() >= util_job::consts::MIN_INGRESS_PORT_TCP as i32
					&& p.port().unwrap() <= util_job::consts::MAX_INGRESS_PORT_TCP as i32
			);
			assert!(p.is_tls().unwrap());
		}

		{
			let p = lobby.ports().unwrap().get("test-5052-udp").unwrap();
			assert!(
				p.port().unwrap() >= util_job::consts::MIN_INGRESS_PORT_UDP as i32
					&& p.port().unwrap() <= util_job::consts::MAX_INGRESS_PORT_UDP as i32
			);
			assert!(!p.is_tls().unwrap());
		}
	}

	lobby_data.clone()
}

fn captcha_config() -> model::CaptchaConfig {
	model::CaptchaConfig::Hcaptcha(
		model::captcha_config_hcaptcha::Builder::default()
			.client_response("10000000-aaaa-bbbb-cccc-000000000001")
			.build(),
	)
}

async fn create_lobby(
	ctx: &Ctx,
	lobby_id: Uuid,
	lobby_group_meta: &backend::matchmaker::LobbyGroupMeta,
) {
	msg!([ctx] mm::msg::lobby_create(lobby_id) -> mm::msg::lobby_create_complete {
		lobby_id: Some(lobby_id.into()),
		namespace_id: Some(ctx.namespace_id.into()),
		lobby_group_id: lobby_group_meta.lobby_group_id,
		region_id: Some(ctx.primary_region_id.into()),
		create_ray_id: None,
		preemptively_created: false,
	})
	.await
	.unwrap();

	msg!([ctx] @wait mm::msg::lobby_ready(lobby_id) {
		lobby_id: Some(lobby_id.into()),
	})
	.await
	.unwrap();
}

// TODO: Conflicts with other tests
// #[tokio::test(flavor = "multi_thread")]
// async fn find_rate_limit() {
// 	let ctx = Ctx::init().await;
// 	let http_client = ctx.http_client(ctx.ns_auth_token.clone());

// 	{
// 		tracing::info!("finding lobby");

// 		// Create lobby
// 		create_lobby(&ctx, Uuid::new_v4(), &ctx.mm_config_meta.lobby_groups[0]).await;

// 		const RATE_LIMIT: usize = 4 * 15; // rate * bucket minutes
// 		for i in 0..RATE_LIMIT {
// 			tracing::info!(i, "req");
// 			http_client.list_lobbies().send().await.unwrap();
// 		}

// 		let err = http_client.list_lobbies().send().await.unwrap_err();

// 		// Assert that rate limit happened
// 		if let aws_smithy_client::SdkError::ServiceError { err, .. } = err {
// 			assert_eq!(err.code().unwrap(), "API_RATE_LIMIT");
// 		} else {
// 			panic!("{}", err);
// 		}
// 	}
// }

// TODO: Validate both player & lobby deleted

// TODO:
// seek lobby
// > check lobby exists
// validate player
// delete player
// > check lobby and player deleted

// TODO: Test forbidden
// TODO: Seek with directly lobby deletion
// TODO: Seek multiple game modes

// TODO: Dev tokens

// TODO: Validate both player & lobby deleted

// TODO:
// seek lobby
// > check lobby exists
// validate player
// delete player
// > check lobby and player deleted

// TODO: Test forbidden
// TODO: Seek with directly lobby deletion
// TODO: Seek multiple game modes

// TODO: Dev tokens
