use std::{io::Write, net::TcpStream};

use futures_util::{StreamExt, TryStreamExt};
use proto::{
	backend::{self, pkg::*},
	common,
};
use rivet_api::{
	apis::{configuration::Configuration, matchmaker_lobbies_api},
	models,
};
use rivet_operation::prelude::*;
use serde_json::json;
use tokio::time::{interval, Duration, Instant};

pub const LOBBY_GROUP_NAME_ID_BRIDGE: &str = "test-bridge";
pub const LOBBY_GROUP_NAME_ID_HOST: &str = "test-host";

struct Ctx {
	pub op_ctx: OperationContext<()>,
	pub primary_region_id: Uuid,
	pub primary_region_name_id: String,
	config: Configuration,
}

#[tracing::instrument(skip_all)]
pub async fn run_from_env(ts: i64) -> GlobalResult<()> {
	let pools = rivet_pools::from_env("load-test-mm").await?;
	let client = chirp_client::SharedClient::from_env(pools.clone())?.wrap_new("load-test-mm");
	let cache = rivet_cache::CacheInner::from_env(pools.clone())?;
	let ctx = OperationContext::new(
		"load-test-mm".into(),
		std::time::Duration::from_secs(60),
		rivet_connection::Connection::new(client, pools, cache),
		Uuid::new_v4(),
		Uuid::new_v4(),
		util::timestamp::now(),
		util::timestamp::now(),
		(),
		Vec::new(),
	);

	// Setup
	let (primary_region_id, primary_region_name_id) = setup_region(&ctx).await?;
	let (game_id, version_id, namespace_id, mm_config, mm_config_meta) =
		setup_game(&ctx, primary_region_id).await?;
	let ns_auth_token = setup_public_token(&ctx, namespace_id).await?;
	let config = setup_config(&ctx, ns_auth_token).await?;

	let test_ctx = Ctx {
		op_ctx: ctx,
		primary_region_id,
		primary_region_name_id,
		config,
	};

	// Tests
	tracing::info!("============ Starting tests ==============");

	let mut sockets = Vec::new();

	tracing::info!("============ Batch create ==============");
	let create_lobbies = std::iter::repeat_with(|| async {
		let res = create_lobby(&test_ctx).await?;
		let (_, port) = internal_unwrap_owned!(res.ports.iter().next());
		connect_socket(port).await
	})
	.take(64);
	let new_sockets = futures_util::stream::iter(create_lobbies)
		.buffer_unordered(16)
		.try_collect::<Vec<_>>()
		.await?;
	sockets.extend(new_sockets);

	tracing::info!("============ Sequential create ==============");

	for _ in 0..64 {
		let res = create_lobby(&test_ctx).await?;
		let (_, port) = internal_unwrap_owned!(res.ports.iter().next());
		let socket = connect_socket(port).await?;
		sockets.push(socket);
	}

	tracing::info!("============ Batch find ==============");
	let find_lobbies = std::iter::repeat_with(|| async {
		let res = find_lobby(&test_ctx).await?;
		let (_, port) = internal_unwrap_owned!(res.ports.iter().next());
		connect_socket(port).await
	})
	.take(64);
	let new_sockets = futures_util::stream::iter(find_lobbies)
		.buffer_unordered(16)
		.try_collect::<Vec<_>>()
		.await?;
	sockets.extend(new_sockets);

	tracing::info!("============ Sequential find ==============");
	for _ in 0..64 {
		let res = find_lobby(&test_ctx).await?;
		let (_, port) = internal_unwrap_owned!(res.ports.iter().next());
		let socket = connect_socket(port).await?;
		sockets.push(socket);
	}

	tracing::info!("============ Waiting ==============");
	tokio::time::sleep(Duration::from_secs(30)).await;

	// TODO: Read mm state, verify player count. Then wait again and verify players were cleaned up

	Ok(())
}

async fn setup_config(
	ctx: &OperationContext<()>,
	bearer_token: String,
) -> GlobalResult<Configuration> {
	let bypass_token = {
		let token_res = op!([ctx] token_create {
			token_config: Some(token::create::request::TokenConfig {
				ttl: util::duration::hours(1)
			}),
			refresh_token_config: None,
			issuer: "api-status".to_owned(),
			client: None,
			kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
				entitlements: vec![
					proto::claims::Entitlement {
						kind: Some(
							proto::claims::entitlement::Kind::Bypass(proto::claims::entitlement::Bypass { })
						)
					}
				],
			})),
			label: Some("byp".to_owned()),
			..Default::default()
		})
		.await?;
		internal_unwrap!(token_res.token).token.clone()
	};

	let client = reqwest::Client::builder()
		.default_headers({
			let mut headers = reqwest::header::HeaderMap::new();
			headers.insert(
				"x-bypass-token",
				reqwest::header::HeaderValue::from_str(&bypass_token)?,
			);
			headers.insert(
				"Host",
				reqwest::header::HeaderValue::from_str("api.max3.gameinc.io")?,
			);
			headers.insert(
				"cf-connecting-ip",
				reqwest::header::HeaderValue::from_str("127.0.0.1")?,
			);
			headers
		})
		.build()?;

	Ok(Configuration {
		client,
		base_path: "http://traefik.traefik.svc.cluster.local:80".into(),
		bearer_access_token: Some(bearer_token),
		..Default::default()
	})
}

async fn setup_region(ctx: &OperationContext<()>) -> GlobalResult<(Uuid, String)> {
	tracing::info!("setup region");

	let region_res = op!([ctx] faker_region {}).await?;
	let region_id = internal_unwrap!(region_res.region_id).as_uuid();

	let get_res = op!([ctx] region_get {
		region_ids: vec![region_id.into()],
	})
	.await?;
	let region_data = internal_unwrap_owned!(get_res.regions.first());

	Ok((region_id, region_data.name_id.clone()))
}

async fn setup_game(
	ctx: &OperationContext<()>,
	region_id: Uuid,
) -> GlobalResult<(
	Uuid,
	Uuid,
	Uuid,
	backend::matchmaker::VersionConfig,
	backend::matchmaker::VersionConfigMeta,
)> {
	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await?;

	let build_res = op!([ctx] faker_build {
		game_id: game_res.game_id,
		image: faker::build::Image::MmLobbyAutoReady as i32,
	})
	.await?;

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
								label: "test-5051-tcp".into(),
								target_port: Some(5051),
								port_range: None,
								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Tcp as i32,
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
			],
		}),
		..Default::default()
	})
	.await?;

	let namespace_res = op!([ctx] faker_game_namespace {
		game_id: game_res.game_id,
		version_id: game_version_res.version_id,
		..Default::default()
	})
	.await?;

	Ok((
		internal_unwrap!(game_res.game_id).as_uuid(),
		internal_unwrap!(game_version_res.version_id).as_uuid(),
		internal_unwrap!(namespace_res.namespace_id).as_uuid(),
		internal_unwrap!(game_version_res.mm_config).clone(),
		internal_unwrap!(game_version_res.mm_config_meta).clone(),
	))
}

async fn setup_public_token(
	ctx: &OperationContext<()>,
	namespace_id: Uuid,
) -> GlobalResult<String> {
	let token_res = op!([ctx] cloud_namespace_token_public_create {
		namespace_id: Some(namespace_id.into()),
	})
	.await?;

	Ok(token_res.token)
}

async fn create_lobby(ctx: &Ctx) -> GlobalResult<models::MatchmakerCreateLobbyResponse> {
	let res = matchmaker_lobbies_api::matchmaker_lobbies_create(
		&ctx.config,
		models::MatchmakerLobbiesCreateRequest {
			game_mode: LOBBY_GROUP_NAME_ID_BRIDGE.to_string(),
			region: Some(ctx.primary_region_name_id.clone()),
			publicity: models::MatchmakerCustomLobbyPublicity::Public,
			lobby_config: Some(Some(json!({ "foo": "bar" }))),
			verification_data: None,
			captcha: Some(Box::new(models::CaptchaConfig {
				hcaptcha: Some(Box::new(models::CaptchaConfigHcaptcha {
					client_response: "10000000-aaaa-bbbb-cccc-000000000001".to_string(),
				})),
				turnstile: None,
			})),
		},
	)
	.await?;

	Ok(res)
}

async fn find_lobby(ctx: &Ctx) -> GlobalResult<models::MatchmakerFindLobbyResponse> {
	let res = matchmaker_lobbies_api::matchmaker_lobbies_find(
		&ctx.config,
		models::MatchmakerLobbiesFindRequest {
			game_modes: Vec::new(),
			prevent_auto_create_lobby: None,
			regions: None,
			verification_data: None,
			captcha: Some(Box::new(models::CaptchaConfig {
				hcaptcha: Some(Box::new(models::CaptchaConfigHcaptcha {
					client_response: "10000000-aaaa-bbbb-cccc-000000000001".to_string(),
				})),
				turnstile: None,
			})),
		},
		None,
	)
	.await?;

	Ok(res)
}

async fn connect_socket(mm_port: &models::MatchmakerJoinPort) -> GlobalResult<TcpStream> {
	let socket_id = util::faker::ident();

	let hostname = mm_port.hostname.as_str();
	let port = internal_unwrap_owned!(mm_port.port) as u16;

	let mut stream = TcpStream::connect((hostname, port))?;
	tracing::info!("connected {}", socket_id);

	// Write an empty package to get the response
	stream.write_all(b"Hello, world!")?;
	stream.flush()?;
	tracing::info!("written {}", socket_id);

	Ok(stream)
}
