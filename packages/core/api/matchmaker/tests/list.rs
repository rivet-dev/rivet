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
//
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
