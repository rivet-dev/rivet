use std::time::Duration;

use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use rivet_api::{
	apis::{configuration::Configuration, matchmaker_lobbies_api},
	models,
};
use tokio::net::TcpStream;

#[worker_test]
async fn player_register(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = lobby_res.namespace_id.as_ref().unwrap().as_uuid();
	let lobby_id = lobby_res.lobby_id.as_ref().unwrap().as_uuid();

	let player_id = Uuid::new_v4();
	let query_id = Uuid::new_v4();
	msg!([ctx] @notrace mm::msg::lobby_find(namespace_id, query_id) -> Result<mm::msg::lobby_find_complete, mm::msg::lobby_find_fail> {
		namespace_id: Some(namespace_id.into()),
		query_id: Some(query_id.into()),
		join_kind: backend::matchmaker::query::JoinKind::Normal as i32,
		players: vec![mm::msg::lobby_find::Player {
			player_id: Some(player_id.into()),
			token_session_id: Some(Uuid::new_v4().into()),
			client_info:None,
		}],
		query: Some(mm::msg::lobby_find::message::Query::Direct(backend::matchmaker::query::Direct {
			lobby_id: Some(lobby_id.into()),
		})),
		..Default::default()
	})
	.await
	.unwrap().unwrap();

	// Register player
	msg!([ctx] mm::msg::player_register(player_id) -> Result<mm::msg::player_register_complete, mm::msg::player_register_fail> {
		player_id: Some(player_id.into()),
		lobby_id: Some(lobby_id.into()),
	})
	.await
	.unwrap().unwrap();

	let player_count_res = op!([ctx] mm_lobby_player_count {
		lobby_ids: vec![
			lobby_id.into()
		],
	})
	.await
	.unwrap();
	let player_count = player_count_res
		.lobbies
		.first()
		.unwrap()
		.registered_player_count;

	assert_eq!(1, player_count, "registered player count not updated");

	let (register_ts,) = sqlx::query_as::<_, (Option<i64>,)>(
		"SELECT register_ts FROM db_mm_state.players WHERE player_id = $1",
	)
	.bind(player_id)
	.fetch_one(&ctx.crdb().await.unwrap())
	.await
	.unwrap();
	assert!(register_ts.is_some());

	// Attempt to register again
	let res = msg!([ctx] mm::msg::player_register(player_id) -> Result<mm::msg::player_register_complete, mm::msg::player_register_fail> {
		player_id: Some(player_id.into()),
		lobby_id: Some(lobby_id.into()),
	})
	.await
	.unwrap().unwrap_err();
	assert_eq!(
		mm::msg::player_register_fail::ErrorCode::PlayerAlreadyRegistered as i32,
		res.error_code
	);
}

#[worker_test]
async fn wrong_lobby(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let lobby_res = op!([ctx] faker_mm_lobby {
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = lobby_res.namespace_id.as_ref().unwrap().as_uuid();
	let lobby_id = lobby_res.lobby_id.as_ref().unwrap().as_uuid();

	let player_id = Uuid::new_v4();
	let query_id = Uuid::new_v4();
	msg!([ctx] @notrace mm::msg::lobby_find(namespace_id, query_id) -> Result<mm::msg::lobby_find_complete, mm::msg::lobby_find_fail> {
		namespace_id: Some(namespace_id.into()),
		query_id: Some(query_id.into()),
		join_kind: backend::matchmaker::query::JoinKind::Normal as i32,
		players: vec![mm::msg::lobby_find::Player {
			player_id: Some(player_id.into()),
			token_session_id: Some(Uuid::new_v4().into()),
			client_info:None,
		}],
		query: Some(mm::msg::lobby_find::message::Query::Direct(backend::matchmaker::query::Direct {
			lobby_id: Some(lobby_id.into()),
		})),
		..Default::default()
	})
	.await
	.unwrap().unwrap();

	// Register player in the wrong lobby
	let mut remove_sub = subscribe!([ctx] mm::msg::player_remove_complete(player_id))
		.await
		.unwrap();
	let res =
		msg!([ctx] mm::msg::player_register(player_id) -> Result<mm::msg::player_register_complete, mm::msg::player_register_fail> {
			player_id: Some(player_id.into()),
			lobby_id: Some(Uuid::new_v4().into()),
		})
		.await
		.unwrap()
		.unwrap_err();
	assert_eq!(
		mm::msg::player_register_fail::ErrorCode::PlayerInDifferentLobby as i32,
		res.error_code
	);

	remove_sub.next().await.unwrap();

	let player_count_res = op!([ctx] mm_lobby_player_count {
		lobby_ids: vec![
			lobby_id.into()
		],
	})
	.await
	.unwrap();
	let player_count = player_count_res.lobbies.first().unwrap().total_player_count;

	assert_eq!(0, player_count, "player not removed");
}

// Tests player connection to a host port end to end
#[worker_test]
async fn host_e2e(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	// Setup
	tracing::info!("setting up");
	let (primary_region_id, primary_region_name_id) = setup_region(ctx.op_ctx()).await.unwrap();
	let (_game_id, version_id, namespace_id, _mm_config, _mm_config_meta) =
		setup_game(ctx.op_ctx(), primary_region_id).await.unwrap();
	let ns_auth_token = setup_public_token(ctx.op_ctx(), namespace_id)
		.await
		.unwrap();
	let config = setup_config(ctx.op_ctx(), ns_auth_token).await.unwrap();

	tracing::info!("finding");
	let res = matchmaker_lobbies_api::matchmaker_lobbies_find(
		&config,
		models::MatchmakerLobbiesFindRequest {
			game_modes: vec!["test".to_string()],
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
	.await
	.unwrap();
	let (_, mm_port) = res.ports.iter().next().unwrap();

	tracing::info!(?mm_port, "found");
	let hostname = mm_port.hostname.as_str();
	let port = mm_port.port_range.as_ref().unwrap().min as u16;
	let host = format!("{hostname}:{port}");

	tracing::info!(?host, "connecting");
	let stream = TcpStream::connect(host).await.unwrap();
	stream.set_nodelay(true).unwrap();

	stream.readable().await.unwrap();

	tokio::time::sleep(Duration::from_secs(5)).await;

	let player_count_res = op!([test_ctx.op_ctx] mm_player_count_for_namespace {
		namespace_ids: vec![namespace_id.into()],
	})
	.await?;
	let player_count = unwrap!(player_count_res.namespaces.first()).player_count;

	assert!(1, player_count, "player not registered");
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
		unwrap_ref!(token_res.token).token.clone()
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
	let region_id = unwrap_ref!(region_res.region_id).as_uuid();

	let get_res = op!([ctx] region_get {
		region_ids: vec![region_id.into()],
	})
	.await?;
	let region_data = unwrap!(get_res.regions.first());

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
					name_id: "test".into(),

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
							value: "26000".to_string(),
						}],
						network_mode: backend::matchmaker::lobby_runtime::NetworkMode::Host as i32,
						ports: vec![
							backend::matchmaker::lobby_runtime::Port {
								label: "test-tcp".into(),
								target_port: None,
								port_range: Some(backend::matchmaker::lobby_runtime::PortRange {
									min: 26000,
									max: 31999
								}),
								proxy_protocol: backend::matchmaker::lobby_runtime::ProxyProtocol::Tcp as i32,
								proxy_kind: backend::matchmaker::lobby_runtime::ProxyKind::None as i32,
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
	let namespace_id = unwrap_ref!(namespace_res.namespace_id);

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
		unwrap_ref!(game_res.game_id).as_uuid(),
		unwrap_ref!(game_version_res.version_id).as_uuid(),
		namespace_id.as_uuid(),
		unwrap_ref!(game_version_res.mm_config).clone(),
		unwrap_ref!(game_version_res.mm_config_meta).clone(),
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
