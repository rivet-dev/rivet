use futures_util::{StreamExt, TryStreamExt};
use proto::backend::{self, pkg::*};
use rivet_api::{
	apis::{configuration::Configuration, matchmaker_lobbies_api},
	models,
};
use rivet_claims::ClaimsDecode;
use rivet_operation::prelude::*;
use serde_json::json;
use tokio::{io::AsyncWriteExt, net::TcpStream, time::Duration};

const LOBBY_GROUP_NAME_ID: &str = "test";
const CHUNK_SIZE: usize = 64;
const BUFFER_SIZE: usize = 16;

struct Ctx {
	op_ctx: OperationContext<()>,
	conns: Vec<Conn>,
	namespace_id: Uuid,
	primary_region_name_id: String,
	config: Configuration,
}

struct Conn {
	id: Uuid,
	socket: TcpStream,
}

impl Conn {
	async fn new(id: Uuid, host: String) -> GlobalResult<Self> {
		tracing::info!("connecting {} {}", id, host);

		let stream = TcpStream::connect(host).await?;
		stream.set_nodelay(true)?;
		tracing::info!("connected {}", id);

		stream.readable().await?;

		Ok(Conn {
			id,
			socket: stream,
		})
	}

	async fn write(&mut self, data: &[u8]) -> GlobalResult<()> {
		self.socket.write_all(data).await?;
		self.socket.flush().await?;
		tracing::info!("written {}", self.id);

		Ok(())
	}

	// Read 4 bytes
	fn read(&self) -> GlobalResult<[u8; 4]> {
		let mut buffer = [0u8; 4];

		self.socket.try_read(&mut buffer)?;
		tracing::info!("read {} {:?}", self.id, buffer);

		Ok(buffer)
	}
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
	let (_game_id, version_id, namespace_id, _mm_config, _mm_config_meta) =
		setup_game(&ctx, primary_region_id).await?;
	let ns_auth_token = setup_public_token(&ctx, namespace_id).await?;
	let config = setup_config(&ctx, ns_auth_token).await?;

	let mut test_ctx = Ctx {
		op_ctx: ctx,
		conns: Vec::new(),
		namespace_id,
		primary_region_name_id,
		config,
	};

	// Tests
	tracing::info!("============ Starting tests ==============");

	tracing::info!("============ Batch create ==============");

	let create_lobbies = std::iter::repeat_with(|| async {
		let res = create_lobby(&test_ctx).await?;
		let (_, port) = internal_unwrap_owned!(res.ports.iter().next());
		connect_to_lobby(&res.player.token, port).await
	})
	.take(CHUNK_SIZE);
	let new_conns = futures_util::stream::iter(create_lobbies)
		.buffer_unordered(BUFFER_SIZE)
		.try_collect::<Vec<_>>()
		.await?;
	test_ctx.conns.extend(new_conns);

	tracing::info!("============ Batch find ==============");
	let find_lobbies = std::iter::repeat_with(|| async {
		let res = find_lobby(&test_ctx).await?;
		let (_, port) = internal_unwrap_owned!(res.ports.iter().next());
		connect_to_lobby(&res.player.token, port).await
	})
	.take(CHUNK_SIZE);
	let new_conns = futures_util::stream::iter(find_lobbies)
		.buffer_unordered(BUFFER_SIZE)
		.try_collect::<Vec<_>>()
		.await?;
	test_ctx.conns.extend(new_conns);

	tracing::info!("============ Batch find (no connection) ==============");
	let find_no_connect_lobbies = std::iter::repeat_with(|| find_lobby(&test_ctx)).take(CHUNK_SIZE);
	futures_util::stream::iter(find_no_connect_lobbies)
		.buffer_unordered(BUFFER_SIZE)
		.try_collect::<Vec<_>>()
		.await?;

	tracing::info!("============ Waiting ==============");
	tokio::time::sleep(Duration::from_secs(30)).await;

	// Prevent idle cleanup
	for conn in &mut test_ctx.conns {
		conn.write("active".as_bytes()).await?;
	}

	verify_state(&test_ctx, CHUNK_SIZE, CHUNK_SIZE * 3, CHUNK_SIZE * 2).await?;

	tracing::info!("============ Waiting for MM GC ==============");
	tokio::time::sleep(Duration::from_secs(105)).await;

	verify_state(&test_ctx, CHUNK_SIZE, CHUNK_SIZE * 2, CHUNK_SIZE * 2).await?;

	Ok(())
}

// Read mm state, verify player and lobby count
async fn verify_state(
	test_ctx: &Ctx,
	expected_lobby_count: usize,
	expected_player_count: usize,
	expected_registered_player_count: usize,
) -> GlobalResult<()> {
	tracing::info!("============ Verifying State ==============");

	let lobby_list_res = op!([test_ctx.op_ctx] mm_lobby_list_for_namespace {
		namespace_ids: vec![test_ctx.namespace_id.into()],
	})
	.await?;
	let lobby_ids = &internal_unwrap_owned!(lobby_list_res.namespaces.first()).lobby_ids;

	let lobby_players_res = op!([test_ctx.op_ctx] mm_lobby_player_count {
		lobby_ids: lobby_ids.clone(),
	})
	.await?;
	let lobby_player_count = lobby_players_res
		.lobbies
		.iter()
		.fold(0, |a, l| a + l.total_player_count);
	let registered_player_count = lobby_players_res
		.lobbies
		.iter()
		.fold(0, |a, l| a + l.registered_player_count);

	let player_count_res = op!([test_ctx.op_ctx] mm_player_count_for_namespace {
		namespace_ids: vec![test_ctx.namespace_id.into()],
	})
	.await?;
	let player_count = internal_unwrap_owned!(player_count_res.namespaces.first()).player_count;

	tracing::info!(?expected_registered_player_count, ?registered_player_count, ?lobby_player_count, conns=?test_ctx.conns.len());

	internal_assert_eq!(
		expected_lobby_count,
		lobby_ids.len(),
		"wrong number of lobbies"
	);
	internal_assert_eq!(
		expected_player_count,
		player_count as usize,
		"wrong number of players"
	);
	internal_assert_eq!(
		expected_registered_player_count,
		registered_player_count as usize,
		"wrong number of registered players"
	);
	internal_assert_eq!(
		lobby_player_count,
		player_count,
		"player counts don't match"
	);
	internal_assert_eq!(
		test_ctx.conns.len(),
		registered_player_count as usize,
		"registered player count doesn't match connection count"
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
							max_idle_lobbies: 32,
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

async fn connect_to_lobby(token: &str, mm_port: &models::MatchmakerJoinPort) -> GlobalResult<Conn> {
	let hostname = mm_port.hostname.as_str();
	let port = internal_unwrap_owned!(mm_port.port) as u16;
	let host = format!("{hostname}:{port}");

	// Get player id
	let player_claims = rivet_claims::decode(token)
		.map_err(|_| err_code!(TOKEN_ERROR, error = "Malformed player token"))?
		.map_err(|_| err_code!(TOKEN_ERROR, error = "Invalid player token"))?;
	let player_ent = player_claims.as_matchmaker_player()?;
	let id = player_ent.player_id;
	// let id = util::faker::ident();

	let mut conn = Conn::new(id, host).await?;
	conn.write(token.as_bytes()).await?;
	conn.read()?;

	Ok(conn)
}
