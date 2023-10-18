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
use tokio::{io::AsyncWriteExt, net::TcpStream, time::Duration};

const LOBBY_GROUP_NAME_ID: &str = "test";
const CHUNK_SIZE: usize = 1;
const BUFFER_SIZE: usize = 2;

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
		connect_socket(&res.player.token, port).await
	})
	.take(CHUNK_SIZE);
	let new_sockets = futures_util::stream::iter(create_lobbies)
		.buffer_unordered(BUFFER_SIZE)
		.try_collect::<Vec<_>>()
		.await?;
	sockets.extend(new_sockets);

	for i in 0..16 {
		tokio::time::sleep(Duration::from_secs(1)).await;

		for (id, socket) in &mut sockets {
			tracing::info!("writing");
			socket
				.write_all(format!("test{i}").as_str().as_bytes())
				.await?;
			socket.flush().await?;
			tracing::info!("written {}", id);

			let mut buffer = [0u8; 64];
			tracing::info!("reading");
			let read = socket.try_read(&mut buffer);
			tracing::info!("read {:?} {:?}", read, buffer);
		}
	}

	// TODO: Very slow
	// tracing::info!("============ Sequential create ==============");
	// for _ in 0..CHUNK_SIZE {
	// 	let res = create_lobby(&test_ctx).await?;
	// 	let (_, port) = internal_unwrap_owned!(res.ports.iter().next());
	// 	let socket = connect_socket(&res.player.token, port).await?;
	// 	sockets.push(socket);
	// }

    return Ok(());

	tracing::info!("============ Batch find ==============");
	let find_lobbies = std::iter::repeat_with(|| async {
		let res = find_lobby(&test_ctx).await?;
		let (_, port) = internal_unwrap_owned!(res.ports.iter().next());
		connect_socket(&res.player.token, port).await
	})
	.take(CHUNK_SIZE * 2);
	let new_sockets = futures_util::stream::iter(find_lobbies)
		.buffer_unordered(BUFFER_SIZE)
		.try_collect::<Vec<_>>()
		.await?;
	sockets.extend(new_sockets);

	// tracing::info!("============ Sequential find ==============");
	// for _ in 0..CHUNK_SIZE {
	// 	let res = find_lobby(&test_ctx).await?;
	// 	let (_, port) = internal_unwrap_owned!(res.ports.iter().next());
	// 	let socket = connect_socket(&res.player.token, port).await?;
	// 	sockets.push(socket);
	// }

	tracing::info!(?sockets);

	tracing::info!("============ Waiting ==============");
	tokio::time::sleep(Duration::from_secs(30)).await;

	for (id, socket) in &mut sockets {
		socket.write_all("test2".as_bytes()).await?;
		tracing::info!("written {}", id);
	}

	// TODO: Write test for mm-gc cleaning up unregistered players

	// Read mm state, verify player and lobby count
	let lobby_list_res = op!([test_ctx.op_ctx] mm_lobby_list_for_namespace {
		namespace_ids: vec![namespace_id.into()],
	})
	.await?;
	let lobby_count = internal_unwrap_owned!(lobby_list_res.namespaces.first())
		.lobby_ids
		.len();
	let player_count_res = op!([test_ctx.op_ctx] mm_player_count_for_namespace {
		namespace_ids: vec![namespace_id.into()],
	})
	.await?;
	let player_count = internal_unwrap_owned!(player_count_res.namespaces.first()).player_count;

	internal_assert_eq!(CHUNK_SIZE, lobby_count as usize, "wrong number of lobbies");
	internal_assert_eq!(
		CHUNK_SIZE * 2,
		player_count as usize,
		"wrong number of players"
	);

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
			headers.insert(
				"x-coords",
				reqwest::header::HeaderValue::from_str("0.0,0.0")?,
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
		skip_namespaces_and_versions: true,
		..Default::default()
	})
	.await?;

	let build_res = op!([ctx] faker_build {
		game_id: game_res.game_id,
		image: faker::build::Image::MmPlayerConnect as i32,
	})
	.await?;

	let game_version_res = op!([ctx] faker_game_version {
		game_id: game_res.game_id,
		override_lobby_groups: Some(faker::game_version::request::OverrideLobbyGroups {
			lobby_groups: vec![
				backend::matchmaker::LobbyGroup {
					name_id: LOBBY_GROUP_NAME_ID.into(),

					regions: vec![backend::matchmaker::lobby_group::Region {
						region_id: Some(region_id.into()),
						tier_name_id: "basic-1d16".into(),
						idle_lobbies: Some(backend::matchmaker::lobby_group::IdleLobbies {
							min_idle_lobbies: 0,
							max_idle_lobbies: CHUNK_SIZE as u32 * 2,
						}),
					}],
					max_players_normal: 4,
					max_players_direct: 4,
					max_players_party: 4,
					listable: true,

					runtime: Some(backend::matchmaker::lobby_runtime::Docker {
						build_id: build_res.build_id,
						args: Vec::new(),
						env_vars: vec![backend::matchmaker::lobby_runtime::EnvVar {
							key: "PORT".to_string(),
							value: "5051".to_string(),
						}],
						network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Bridge as i32,
						ports: vec![
							backend::matchmaker::lobby_runtime::Port {
								label: "test-tcp".into(),
								target_port: Some(5051),
								port_range: None,
								// port_range: Some(backend::matchmaker::lobby_runtime::PortRange {
								// 	min: 26000,
								// 	max: 31999
								// }),
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
	let namespace_id = internal_unwrap!(namespace_res.namespace_id);

	op!([ctx] mm_config_namespace_config_set {
		namespace_id: Some(*namespace_id),
		lobby_count_max: 256,
		max_players_per_client: 512,
		max_players_per_client_vpn: 512,
		max_players_per_client_proxy: 512,
		max_players_per_client_tor: 512,
		max_players_per_client_hosting: 512,
	})
	.await?;

	Ok((
		internal_unwrap!(game_res.game_id).as_uuid(),
		internal_unwrap!(game_version_res.version_id).as_uuid(),
		namespace_id.as_uuid(),
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
	tracing::info!("creating lobby");

	let res = matchmaker_lobbies_api::matchmaker_lobbies_create(
		&ctx.config,
		models::MatchmakerLobbiesCreateRequest {
			game_mode: LOBBY_GROUP_NAME_ID.to_string(),
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
			game_modes: vec![LOBBY_GROUP_NAME_ID.to_string()],
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

async fn connect_socket(
	player_token: &str,
	mm_port: &models::MatchmakerJoinPort,
) -> GlobalResult<(String, TcpStream)> {
	let socket_id = util::faker::ident();
	tracing::info!("connecting {}", socket_id);

	let hostname = mm_port.hostname.as_str();
	let port = internal_unwrap_owned!(mm_port.port) as u16;
	let host = format!("{hostname}:{port}");

	tracing::info!(?host);

	let mut stream = TcpStream::connect(host).await?;
	stream.set_nodelay(true)?;
	tracing::info!("connected {}", socket_id);

	stream.write_all(player_token.as_bytes()).await?;
	stream.flush().await?;
	tracing::info!("written {}", socket_id);

	stream.readable().await?;

	let mut buffer = [0u8; 4];
	stream.try_read(&mut buffer)?;
	tracing::info!("read {} {:?}", socket_id, buffer);

	Ok((socket_id, stream))
}
