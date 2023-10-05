use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let res = op!([ctx] game_get {
		game_ids: Vec::new(),
	})
	.await
	.unwrap();
	assert!(res.games.is_empty());
}

#[worker_test]
async fn fetch(ctx: TestCtx) {
	let team_res = op!([ctx] faker_team {
		is_dev: true,
		..Default::default()
	})
	.await
	.unwrap();
	let team_id = team_res.team_id.as_ref().unwrap().as_uuid();

	struct TestGame {
		game_id: Option<Uuid>,
		name_id: String,
		display_name: String,
		developer_team_id: Uuid,
	}

	let mut games = std::iter::repeat_with(|| TestGame {
		game_id: None,
		name_id: util::faker::ident(),
		display_name: util::faker::display_name(),
		developer_team_id: team_id,
	})
	.take(8)
	.collect::<Vec<_>>();

	for game in &mut games {
		let res = op!([ctx] game_create {
			name_id: game.name_id.clone(),
			display_name: game.display_name.clone(),
			developer_team_id: Some(game.developer_team_id.into()),
		})
		.await
		.unwrap();
		game.game_id = res.game_id.map(|id| id.as_uuid());
	}

	let res = op!([ctx] game_get {
		game_ids: games.iter().filter_map(|u| u.game_id).map(Into::<common::Uuid>::into).collect(),
	})
	.await
	.unwrap();

	assert_eq!(games.len(), res.games.len());
	for game in &games {
		let game_res = res
			.games
			.iter()
			.find(|u| Some(u.game_id.as_ref().unwrap().as_uuid()) == game.game_id)
			.expect("game not returned");
		assert_eq!(game.name_id, game_res.name_id);
		assert_eq!(game.display_name, game_res.display_name);
		assert_eq!(
			Some(game.developer_team_id.into()),
			game_res.developer_team_id
		);
	}
}

// #[worker_test]
// async fn stress(ctx: TestCtx) {
// 	let team_res = op!([ctx] faker_team {
// 		is_dev: true,
// 		..Default::default()
// 	})
// 	.await
// 	.unwrap();
// 	let team_id = team_res.team_id.clone().unwrap();

// 	struct TestGame {
// 		game_id: Option<Uuid>,
// 		name_id: String,
// 		display_name: String,
// 		url: String,
// 		developer_team_id: Uuid,
// 		description: String,
// 		tags: Vec<String>,
// 	}

// 	let game_res = op!([ctx] game_create {
// 		name_id: util::faker::ident(),
// 		display_name: util::faker::display_name(),
// 		developer_team_id: Some(team_id.clone()),
// 	})
// 	.await
// 	.unwrap();
// 	let game_id = game_res.game_id.clone().unwrap();

// 	let mut handles = Vec::new();
// 	for i in 0..2000 {
// 		tracing::info!(?i, "iter");

// 		let client = ctx.chirp().clone();
// 		let game_id = game_id.clone();
// 		let handle = tokio::spawn(async move {
// 			let res = op!([ctx] game_get {
// 				game_ids: vec![game_id]
// 			})
// 			.await
// 			.unwrap();
// 		});
// 		handles.push(handle);

// 		tokio::time::sleep(std::time::Duration::from_millis(50)).await;
// 	}

// 	futures_util::future::try_join_all(handles).await.unwrap();
// }

// #[worker_test]
// async fn stress_serial(ctx: TestCtx) {
// 	let team_res = op!([ctx] faker_team {
// 		is_dev: true,
// 		..Default::default()
// 	})
// 	.await
// 	.unwrap();
// 	let team_id = team_res.team_id.clone().unwrap();

// 	struct TestGame {
// 		game_id: Option<Uuid>,
// 		name_id: String,
// 		display_name: String,
// 		url: String,
// 		developer_team_id: Uuid,
// 		description: String,
// 		tags: Vec<String>,
// 	}

// 	let game_res = op!([ctx] game_create {
// 		name_id: util::faker::ident(),
// 		display_name: util::faker::display_name(),
// 		developer_team_id: Some(team_id.clone()),
// 	})
// 	.await
// 	.unwrap();
// 	let game_id = game_res.game_id.clone().unwrap();

// 	for i in 0..10000 {
// 		tracing::info!(?i, "iter");

// 		let client = ctx.chirp().clone();
// 		let game_id = game_id.clone();
// 		let res = op!([ctx] game_get {
// 			game_ids: vec![game_id]
// 		})
// 		.await
// 		.unwrap();
// 	}
// }
