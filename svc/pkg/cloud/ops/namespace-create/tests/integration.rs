use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let namespace_id = Uuid::new_v4();

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
