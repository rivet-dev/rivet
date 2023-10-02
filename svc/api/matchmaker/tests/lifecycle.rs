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
