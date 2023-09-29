mod common;

use common::*;
use proto::backend::{self, pkg::*};
use rivet_api::{apis::*, models};
use rivet_operation::prelude::*;
use serde_json::json;

// #[tokio::test(flavor = "multi_thread")]
// async fn find_with_regions() {
// 	let ctx = Ctx::init().await;
// 	let http_client = ctx.http_client(ctx.ns_auth_token.clone());

// 	{
// 		tracing::info!("finding lobby");

// 		// Create lobby
// 		create_lobby(&ctx, Uuid::new_v4(), &ctx.mm_config_meta.lobby_groups[0]).await;

// 		let res = http_client
// 			.find_lobby()
// 			.set_game_modes(Some(vec![LOBBY_GROUP_NAME_ID_BRIDGE.into()]))
// 			.set_regions(Some(vec![ctx.primary_region_name_id.clone()]))
// 			.captcha(captcha_config())
// 			.send()
// 			.await
// 			.unwrap();

// 		assert_lobby_state_smithy(&ctx, res.lobby().unwrap()).await;
// 	}
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn find_without_regions() {
// 	let ctx = Ctx::init().await;
// 	let http_client = ctx.http_client(ctx.ns_auth_token.clone());

// 	{
// 		tracing::info!("finding lobby");

// 		// Create lobby
// 		create_lobby(&ctx, Uuid::new_v4(), &ctx.mm_config_meta.lobby_groups[0]).await;

// 		let res = http_client
// 			.find_lobby()
// 			.set_game_modes(Some(vec![LOBBY_GROUP_NAME_ID_BRIDGE.into()]))
// 			.captcha(captcha_config())
// 			.send()
// 			.await
// 			.unwrap();

// 		assert_lobby_state_smithy(&ctx, res.lobby().unwrap()).await;
// 	}
// }

#[tokio::test(flavor = "multi_thread")]
async fn create_custom_lobby() {
	let ctx = Ctx::init().await;

	{
		tracing::info!("creating custom lobby");

		let res = matchmaker_lobbies_api::matchmaker_lobbies_create(
			&ctx.config(ctx.ns_auth_token.clone()),
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
		.await
		.unwrap();

		assert_lobby_state(&ctx, &res.lobby).await;
	}
}

// #[tokio::test(flavor = "multi_thread")]
// async fn list_lobbies() {
// 	let ctx = Ctx::init().await;
// 	let http_client = ctx.http_client(ctx.ns_auth_token.clone());

// 	let lobby_group_meta = &ctx.mm_config_meta.lobby_groups[0];

// 	let mut lobby_ids = HashSet::new();
// 	for _ in 0..3 {
// 		let lobby_id = Uuid::new_v4();
// 		lobby_ids.insert(lobby_id);

// 		// Create lobby
// 		create_lobby(&ctx, lobby_id, lobby_group_meta).await;

// 		// Create players
// 		let query_id = Uuid::new_v4();
// 		let primary_player_id = Uuid::new_v4();
// 		msg!([ctx] @notrace mm::msg::lobby_find(ctx.namespace_id, query_id) -> Result<mm::msg::lobby_find_complete, mm::msg::lobby_find_fail> {
// 			namespace_id: Some(ctx.namespace_id.into()),
// 			query_id: Some(query_id.into()),
// 			join_kind: backend::matchmaker::query::JoinKind::Normal as i32,
// 			players: vec![
// 				mm::msg::lobby_find::Player {
// 					player_id: Some(primary_player_id.into()),
// 					token_session_id: Some(Uuid::new_v4().into()),
// 					client_info: None,
// 				},
// 				mm::msg::lobby_find::Player {
// 					player_id: Some(Uuid::new_v4().into()),
// 					token_session_id: Some(Uuid::new_v4().into()),
// 					client_info: None,
// 				},
// 			],
// 			query: Some(mm::msg::lobby_find::message::Query::Direct(backend::matchmaker::query::Direct {
// 				lobby_id: Some(lobby_id.into()),
// 			})),
// 			..Default::default()
// 		})
// 		.await
// 		.unwrap().unwrap();
// 	}

// 	{
// 		tracing::info!("listing lobbies");

// 		let res = http_client.list_lobbies().send().await.unwrap();
// 		tracing::info!(?res, "lobby list");

// 		let game_modes = res.game_modes().unwrap();
// 		assert_eq!(2, game_modes.len(), "wrong game mode count");

// 		let regions = res.regions().unwrap();
// 		assert_eq!(1, regions.len(), "wrong region count");
// 		let region = &regions[0];
// 		assert_eq!(
// 			ctx.primary_region_name_id,
// 			region.region_id().unwrap(),
// 			"wrong region name"
// 		);

// 		let lobbies = res.lobbies().unwrap();
// 		assert_eq!(lobby_ids.len(), lobbies.len(), "wrong lobby count");
// 		assert_eq!(
// 			lobby_ids,
// 			lobbies
// 				.iter()
// 				.map(|l| Uuid::from_str(l.lobby_id().unwrap()).unwrap())
// 				.collect::<HashSet<_>>(),
// 			"lobby ids don't match"
// 		);
// 	}
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn lobby_lifecycle() {
// 	let ctx = Ctx::init().await;
// 	let http_client = ctx.http_client(ctx.ns_auth_token.clone());

// 	// MARK: POST /matchmaker/lobbies/find (A)
// 	let (lobby_a, lobby_a_token) = {
// 		tracing::info!("finding lobby a");

// 		// Create lobby
// 		create_lobby(&ctx, Uuid::new_v4(), &ctx.mm_config_meta.lobby_groups[0]).await;

// 		let res = http_client
// 			.find_lobby()
// 			.set_game_modes(Some(vec![LOBBY_GROUP_NAME_ID_BRIDGE.into()]))
// 			.captcha(captcha_config())
// 			.send()
// 			.await
// 			.unwrap();
// 		let lobby = res.lobby().unwrap();
// 		assert_lobby_state_smithy(&ctx, lobby).await;

// 		let lobby_token = ctx.lobby_token(lobby.lobby_id().unwrap()).await;

// 		(lobby.clone(), lobby_token)
// 	};

// 	let http_client_a = ctx.http_client(lobby_a_token.clone());

// 	// MARK: POST /matchmaker/player/connected (A)
// 	{
// 		tracing::info!("connected player a");

// 		let _res = http_client_a
// 			.player_connected()
// 			.player_token(lobby_a.player().unwrap().token().unwrap())
// 			.send()
// 			.await
// 			.unwrap();
// 	}

// 	// MARK: POST /matchmaker/lobbies/join (B)
// 	let (lobby_b, _lobby_b_token) = {
// 		tracing::info!("finding lobby b");

// 		let res = http_client
// 			.join_lobby()
// 			.lobby_id(lobby_a.lobby_id().unwrap())
// 			.captcha(captcha_config())
// 			.send()
// 			.await
// 			.unwrap();
// 		let lobby = res.lobby().unwrap();
// 		assert_lobby_state_smithy(&ctx, lobby).await;

// 		let lobby_token = ctx.lobby_token(lobby.lobby_id().unwrap()).await;

// 		(lobby.clone(), lobby_token)
// 	};

// 	// MARK: POST /matchmaker/player/connected (B)
// 	{
// 		tracing::info!("connected player b");

// 		let _res = http_client_a
// 			.player_connected()
// 			.player_token(lobby_b.player().unwrap().token().unwrap())
// 			.send()
// 			.await
// 			.unwrap();
// 	}

// 	// MARK: POST /matchmaker/player/disconnected (A)
// 	{
// 		tracing::info!("disconnected player a");

// 		let _res = http_client_a
// 			.player_disconnected()
// 			.player_token(lobby_a.player().unwrap().token().unwrap())
// 			.send()
// 			.await
// 			.unwrap();
// 	}

// 	// MARK: POST /matchmaker/player/disconnected (B)
// 	{
// 		tracing::info!("disconnected player b");

// 		let _res = http_client_a
// 			.player_disconnected()
// 			.player_token(lobby_a.player().unwrap().token().unwrap())
// 			.send()
// 			.await
// 			.unwrap();
// 	}
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn lobby_lifecycle_dev() {
// 	use backend::matchmaker::lobby_runtime::{Port, ProxyKind, ProxyProtocol};

// 	let ctx = Ctx::init().await;

// 	// Create token
// 	let ns_dev_auth_token = Ctx::setup_dev_token(
// 		&ctx.op_ctx,
// 		ctx.namespace_id,
// 		"127.0.0.1".to_owned(),
// 		vec![
// 			Port {
// 				label: "test-80".into(),
// 				target_port: Some(80),
// 				port_range: None,
// 				proxy_protocol: ProxyProtocol::Https as i32,
// 				proxy_kind: ProxyKind::GameGuard as i32,
// 			},
// 			Port {
// 				label: "test-8080".into(),
// 				target_port: Some(8080),
// 				port_range: None,
// 				proxy_protocol: ProxyProtocol::Https as i32,
// 				proxy_kind: ProxyKind::GameGuard as i32,
// 			},
// 			Port {
// 				label: "test-5050".into(),
// 				target_port: Some(5050),
// 				port_range: None,
// 				proxy_protocol: ProxyProtocol::Https as i32,
// 				proxy_kind: ProxyKind::GameGuard as i32,
// 			},
// 		],
// 	)
// 	.await;

// 	let http_client = ctx.http_client(ns_dev_auth_token);

// 	// MARK: POST /matchmaker/lobbies/find (A)
// 	let lobby_a = {
// 		tracing::info!("finding lobby a");

// 		let res = http_client
// 			.find_lobby()
// 			.set_game_modes(Some(vec![LOBBY_GROUP_NAME_ID_BRIDGE.into()]))
// 			.captcha(captcha_config())
// 			.send()
// 			.await
// 			.unwrap();
// 		let lobby = res.lobby().unwrap();

// 		let ports = lobby.ports().unwrap();
// 		assert_eq!(3, ports.len(), "missing dev lobby port");

// 		{
// 			let p = ports.get("test-80").unwrap();
// 			assert_eq!(80, p.port().unwrap());
// 			assert!(p.is_tls().unwrap());
// 		}

// 		{
// 			let p = ports.get("test-8080").unwrap();
// 			assert_eq!(8080, p.port().unwrap());
// 			assert!(p.is_tls().unwrap());
// 		}

// 		{
// 			let p = ports.get("test-5050").unwrap();
// 			assert_eq!(5050, p.port().unwrap());
// 			assert!(p.is_tls().unwrap());
// 		}

// 		lobby.clone()
// 	};

// 	// MARK: POST /matchmaker/player/connected (A)
// 	{
// 		tracing::info!("connected player a");

// 		let _res = http_client
// 			.player_connected()
// 			.player_token(lobby_a.player().unwrap().token().unwrap())
// 			.send()
// 			.await
// 			.unwrap();
// 	}

// 	// MARK: POST /matchmaker/lobbies/join (B)
// 	let (lobby_b, _lobby_b_token) = {
// 		tracing::info!("finding lobby b");

// 		let res = http_client
// 			.join_lobby()
// 			.lobby_id(lobby_a.lobby_id().unwrap())
// 			.captcha(captcha_config())
// 			.send()
// 			.await
// 			.unwrap();
// 		let lobby = res.lobby().unwrap();
// 		let lobby_token = ctx.lobby_token(lobby.lobby_id().unwrap()).await;

// 		(lobby.clone(), lobby_token)
// 	};

// 	// MARK: POST /matchmaker/player/connected (B)
// 	{
// 		tracing::info!("connected player b");

// 		let _res = http_client
// 			.player_connected()
// 			.player_token(lobby_b.player().unwrap().token().unwrap())
// 			.send()
// 			.await
// 			.unwrap();
// 	}

// 	// MARK: PUT /matchmaker/lobbies/closed
// 	{
// 		tracing::info!("closing lobby a");

// 		let _res = http_client
// 			.set_lobby_closed()
// 			.is_closed(true)
// 			.send()
// 			.await
// 			.unwrap();

// 		tracing::info!("opening lobby a");

// 		let _res = http_client
// 			.set_lobby_closed()
// 			.is_closed(false)
// 			.send()
// 			.await
// 			.unwrap();
// 	}

// 	// MARK: POST /matchmaker/player/disconnected (A)
// 	{
// 		tracing::info!("disconnected player a");

// 		let _res = http_client
// 			.player_disconnected()
// 			.player_token(lobby_a.player().unwrap().token().unwrap())
// 			.send()
// 			.await
// 			.unwrap();
// 	}

// 	// MARK: POST /matchmaker/player/disconnected (B)
// 	{
// 		tracing::info!("disconnected player b");

// 		let _res = http_client
// 			.player_disconnected()
// 			.player_token(lobby_a.player().unwrap().token().unwrap())
// 			.send()
// 			.await
// 			.unwrap();
// 	}
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn list_regions() {
// 	let ctx = Ctx::init().await;
// 	let http_client = ctx.http_client(ctx.ns_dev_auth_token.clone());

// 	// MARK: GET /matchmaker/regions/recommend
// 	{
// 		tracing::info!("recommending region");

// 		let _res = http_client.list_regions().send().await.unwrap();
// 	}
// }

// // NOTE: This test is identical to `recommend_region`
// #[tokio::test(flavor = "multi_thread")]
// async fn list_regions_dev() {
// 	let ctx = Ctx::init().await;
// 	let http_client = ctx.http_client(ctx.ns_dev_auth_token.clone());

// 	// MARK: GET /matchmaker/regions/recommend
// 	{
// 		tracing::info!("recommending region dev");

// 		let _res = http_client.list_regions().send().await.unwrap();
// 	}
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn find_domain_auth() {
// 	let ctx = Ctx::init().await;
// 	let http_client = ctx.http_client(ctx.ns_auth_token.clone());

// 	// Normal domain
// 	{
// 		tracing::info!("finding lobby with domain auth");

// 		// Create lobby
// 		create_lobby(&ctx, Uuid::new_v4(), &ctx.mm_config_meta.lobby_groups[0]).await;

// 		let url = format!(
// 			"https://{}--{}.{}/hello-world",
// 			ctx.game_name_id,
// 			ctx.namespace_name_id,
// 			util::env::domain_cdn(),
// 		);

// 		let res = http_client
// 			.find_lobby()
// 			.origin(url)
// 			.set_game_modes(Some(vec![LOBBY_GROUP_NAME_ID_BRIDGE.into()]))
// 			.captcha(captcha_config())
// 			.send()
// 			.await
// 			.unwrap();
// 		assert_lobby_state_smithy(&ctx, res.lobby().unwrap()).await;
// 	}

// 	// Custom domain
// 	{
// 		tracing::info!("finding lobby with custom domain auth");

// 		let url = format!("https://{}/hello-world", ctx.custom_domain);

// 		let res = http_client
// 			.find_lobby()
// 			.origin(url)
// 			.set_game_modes(Some(vec![LOBBY_GROUP_NAME_ID_BRIDGE.into()]))
// 			.captcha(captcha_config())
// 			.send()
// 			.await
// 			.unwrap();
// 		assert_lobby_state_smithy(&ctx, res.lobby().unwrap()).await;
// 	}
// }

// #[tokio::test(flavor = "multi_thread")]
// async fn player_statistics() {
// 	let ctx = Ctx::init().await;

// 	// MARK: GET /players/statistics
// 	let res = matchmaker_players_api::matchmaker_players_get_statistics(
// 		&ctx.config(ctx.ns_auth_token.clone()),
// 	)
// 	.await
// 	.unwrap();
// 	let player_count = res.player_count;
// 	let game_modes = res.game_modes;

// 	tracing::info!(?player_count, ?game_modes);
// }

// async fn assert_lobby_state_smithy(
// 	ctx: &Ctx,
// 	lobby: &model::MatchmakerLobbyJoinInfo,
// ) -> backend::matchmaker::Lobby {
// 	// Fetch lobby data
// 	let lobby_res = op!([ctx] mm_lobby_get {
// 		lobby_ids: vec![Uuid::from_str(lobby.lobby_id().unwrap()).unwrap().into()],
// 		..Default::default()
// 	})
// 	.await
// 	.unwrap();
// 	let lobby_data = lobby_res.lobbies.first().expect("lobby not created");
// 	assert!(lobby_data.ready_ts.is_some(), "lobby not ready");
// 	assert!(lobby_data.run_id.is_some(), "no run id");

// 	// Validate ports
// 	{
// 		let ports = lobby.ports().unwrap();
// 		tracing::info!(?ports, "validating ports");
// 		assert_eq!(6, ports.len());

// 		{
// 			let p = ports.get("test-80-http").unwrap();
// 			assert_eq!(80, p.port().unwrap());
// 			assert!(!p.is_tls().unwrap());
// 		}

// 		{
// 			let p = ports.get("test-80-https").unwrap();
// 			assert_eq!(443, p.port().unwrap());
// 			assert!(p.is_tls().unwrap());
// 		}

// 		{
// 			let p = ports.get("test-5050-https").unwrap();
// 			assert_eq!(443, p.port().unwrap());
// 			assert!(p.is_tls().unwrap());
// 		}

// 		{
// 			let p = ports.get("test-5051-tcp").unwrap();
// 			assert!(
// 				p.port().unwrap() >= util_job::consts::MIN_INGRESS_PORT_TCP as i32
// 					&& p.port().unwrap() <= util_job::consts::MAX_INGRESS_PORT_TCP as i32
// 			);
// 			assert!(!p.is_tls().unwrap());
// 		}

// 		{
// 			let p = ports.get("test-5051-tls").unwrap();
// 			assert!(
// 				p.port().unwrap() >= util_job::consts::MIN_INGRESS_PORT_TCP as i32
// 					&& p.port().unwrap() <= util_job::consts::MAX_INGRESS_PORT_TCP as i32
// 			);
// 			assert!(p.is_tls().unwrap());
// 		}

// 		{
// 			let p = ports.get("test-5052-udp").unwrap();
// 			assert!(
// 				p.port().unwrap() >= util_job::consts::MIN_INGRESS_PORT_UDP as i32
// 					&& p.port().unwrap() <= util_job::consts::MAX_INGRESS_PORT_UDP as i32
// 			);
// 			assert!(!p.is_tls().unwrap());
// 		}
// 	}

// 	lobby_data.clone()
// }

async fn assert_lobby_state(
	ctx: &Ctx,
	lobby: &models::MatchmakerJoinLobby,
) -> backend::matchmaker::Lobby {
	// Fetch lobby data
	let lobby_res = op!([ctx] mm_lobby_get {
		lobby_ids: vec![lobby.lobby_id.into()],
		..Default::default()
	})
	.await
	.unwrap();
	let lobby_data = lobby_res.lobbies.first().expect("lobby not created");
	assert!(lobby_data.ready_ts.is_some(), "lobby not ready");
	assert!(lobby_data.run_id.is_some(), "no run id");

	// Validate ports
	{
		let ports = &lobby.ports;
		tracing::info!(?ports, "validating ports");
		assert_eq!(6, ports.len());

		{
			let p = ports.get("test-80-http").unwrap();
			assert_eq!(80, p.port.unwrap());
			assert!(!p.is_tls);
		}

		{
			let p = ports.get("test-80-https").unwrap();
			assert_eq!(443, p.port.unwrap());
			assert!(p.is_tls);
		}

		{
			let p = ports.get("test-5050-https").unwrap();
			assert_eq!(443, p.port.unwrap());
			assert!(p.is_tls);
		}

		{
			let p = ports.get("test-5051-tcp").unwrap();
			assert!(
				p.port.unwrap() >= util_job::consts::MIN_INGRESS_PORT_TCP as i32
					&& p.port.unwrap() <= util_job::consts::MAX_INGRESS_PORT_TCP as i32
			);
			assert!(!p.is_tls);
		}

		{
			let p = ports.get("test-5051-tls").unwrap();
			assert!(
				p.port.unwrap() >= util_job::consts::MIN_INGRESS_PORT_TCP as i32
					&& p.port.unwrap() <= util_job::consts::MAX_INGRESS_PORT_TCP as i32
			);
			assert!(p.is_tls);
		}

		{
			let p = ports.get("test-5052-udp").unwrap();
			assert!(
				p.port.unwrap() >= util_job::consts::MIN_INGRESS_PORT_UDP as i32
					&& p.port.unwrap() <= util_job::consts::MAX_INGRESS_PORT_UDP as i32
			);
			assert!(!p.is_tls);
		}
	}

	lobby_data.clone()
}

// fn captcha_config() -> model::CaptchaConfig {
// 	model::CaptchaConfig::Hcaptcha(
// 		model::captcha_config_hcaptcha::Builder::default()
// 			.client_response("10000000-aaaa-bbbb-cccc-000000000001")
// 			.build(),
// 	)
// }

// async fn create_lobby(
// 	ctx: &Ctx,
// 	lobby_id: Uuid,
// 	lobby_group_meta: &backend::matchmaker::LobbyGroupMeta,
// ) {
// 	msg!([ctx] mm::msg::lobby_create(lobby_id) -> mm::msg::lobby_create_complete {
// 		lobby_id: Some(lobby_id.into()),
// 		namespace_id: Some(ctx.namespace_id.into()),
// 		lobby_group_id: lobby_group_meta.lobby_group_id,
// 		region_id: Some(ctx.primary_region_id.into()),
// 		create_ray_id: None,
// 		preemptively_created: false,

// 		creator_user_id: None,
// 		is_custom: false,
// 		publicity: None,
// 		lobby_config_json: None,
// 	})
// 	.await
// 	.unwrap();

// 	msg!([ctx] @wait mm::msg::lobby_ready(lobby_id) {
// 		lobby_id: Some(lobby_id.into()),
// 	})
// 	.await
// 	.unwrap();
// }

// // TODO: Conflicts with other tests
// // #[tokio::test(flavor = "multi_thread")]
// // async fn find_rate_limit() {
// // 	let ctx = Ctx::init().await;
// // 	let http_client = ctx.http_client(ctx.ns_auth_token.clone());

// // 	{
// // 		tracing::info!("finding lobby");

// // 		// Create lobby
// // 		create_lobby(&ctx, Uuid::new_v4(), &ctx.mm_config_meta.lobby_groups[0]).await;

// // 		const RATE_LIMIT: usize = 4 * 15; // rate * bucket minutes
// // 		for i in 0..RATE_LIMIT {
// // 			tracing::info!(i, "req");
// // 			http_client.list_lobbies().send().await.unwrap();
// // 		}

// // 		let err = http_client.list_lobbies().send().await.unwrap_err();

// // 		// Assert that rate limit happened
// // 		if let aws_smithy_client::SdkError::ServiceError { err, .. } = err {
// // 			assert_eq!(err.code().unwrap(), "API_RATE_LIMIT");
// // 		} else {
// // 			panic!("{}", err);
// // 		}
// // 	}
// // }

// // TODO: Validate both player & lobby deleted

// // TODO:
// // seek lobby
// // > check lobby exists
// // validate player
// // delete player
// // > check lobby and player deleted

// // TODO: Test forbidden
// // TODO: Seek with directly lobby deletion
// // TODO: Seek multiple game modes

// // TODO: Dev tokens

// // TODO: Validate both player & lobby deleted

// // TODO:
// // seek lobby
// // > check lobby exists
// // validate player
// // delete player
// // > check lobby and player deleted

// // TODO: Test forbidden
// // TODO: Seek with directly lobby deletion
// // TODO: Seek multiple game modes

// // TODO: Dev tokens
