use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_id = Uuid::new_v4();

	op!([ctx] cloud_game_config_create {
		game_id: Some(game_id.into()),
	})
	.await
	.unwrap();

	let (_,): (i64,) = sqlx::query_as("SELECT 1 FROM db_cloud.game_configs WHERE game_id = $1")
		.bind(game_id)
		.fetch_one(&ctx.crdb().await.unwrap())
		.await
		.unwrap();
}
