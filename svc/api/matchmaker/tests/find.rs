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
