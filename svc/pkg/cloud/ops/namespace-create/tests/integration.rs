use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let game_create_res = op!([ctx] faker_game {
		..Default::default()
	})
	.await
	.unwrap();

	let version_create_res = op!([ctx] game_version_create {
		game_id: game_create_res.game_id,
		display_name: util::faker::ident(),
	})
	.await
	.unwrap();

	let res = op!([ctx] game_namespace_create {
		game_id: game_create_res.game_id,
		display_name: util::faker::display_name(),
		version_id: version_create_res.version_id,
		name_id: util::faker::ident(),
	})
	.await
	.unwrap();
	let namespace_id = res.namespace_id.unwrap().as_uuid();

	op!([ctx] cloud_namespace_create {
		namespace_id: Some(namespace_id.into()),
	})
	.await
	.unwrap();

	let (_,): (i64,) =
		sqlx::query_as("SELECT 1 FROM db_cloud.game_namespaces WHERE namespace_id = $1")
			.bind(namespace_id)
			.fetch_one(&ctx.crdb().await.unwrap())
			.await
			.unwrap();
}
