use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_create_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();
	let namespace_id = game_create_res.namespace_ids.first().unwrap().as_uuid();

	let version_create_res = op!([ctx] game_version_create {
		game_id: game_create_res.game_id,
		display_name: util::faker::display_name(),
	})
	.await
	.unwrap();

	op!([ctx] game_namespace_version_set {
		namespace_id: Some(namespace_id.into()),
		version_id: version_create_res.version_id,
	})
	.await
	.unwrap();

	let (sql_version_id,): (Uuid,) =
		sqlx::query_as("SELECT version_id FROM db_game.game_namespaces WHERE namespace_id = $1")
			.bind(namespace_id)
			.fetch_one(&ctx.crdb().await.unwrap())
			.await
			.unwrap();
	assert_eq!(
		version_create_res.version_id.unwrap().as_uuid(),
		sql_version_id,
		"version did not update"
	);
}
