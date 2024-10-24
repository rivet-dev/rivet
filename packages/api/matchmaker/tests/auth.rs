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
