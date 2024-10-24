use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let display_name = "1.2.3".to_owned();

	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	let game_id = game_res.game_id.unwrap().as_uuid();

	let res = op!([ctx] game_version_create {
		game_id: Some(game_id.into()),
		display_name: display_name.clone(),
	})
	.await
	.unwrap();
	let version_id = res.version_id.unwrap().as_uuid();

	let (sql_display_name,): (String,) =
		sqlx::query_as("SELECT display_name FROM db_game.game_versions WHERE version_id = $1")
			.bind(version_id)
			.fetch_one(&ctx.crdb().await.unwrap())
			.await
			.unwrap();
	assert_eq!(display_name, sql_display_name);
}

#[worker_test]
async fn non_unique_name(ctx: TestCtx) {
	let display_name = "1.2.3".to_owned();

	let game_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	let game_id = game_res.game_id.unwrap().as_uuid();

	op!([ctx] game_version_create {
		game_id: Some(game_id.into()),
		display_name: display_name.clone(),
	})
	.await
	.unwrap();

	// Create another version with the same name
	op!([ctx] game_version_create {
		game_id: Some(game_id.into()),
		display_name: display_name.clone(),
	})
	.await
	.unwrap_err();
}
