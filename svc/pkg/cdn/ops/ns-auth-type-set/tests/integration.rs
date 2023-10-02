use chirp_worker::prelude::*;
use proto::backend;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let namespace_id = Uuid::new_v4();

	op!([ctx] cdn_namespace_create {
		namespace_id: Some(namespace_id.into()),
	})
	.await
	.unwrap();

	op!([ctx] cdn_ns_auth_type_set {
		namespace_id: Some(namespace_id.into()),
		auth_type: backend::cdn::namespace_config::AuthType::Basic as i32,
	})
	.await
	.unwrap();

	let (sql_exists,) = sqlx::query_as::<_, (bool,)>(indoc!(
		"
		SELECT EXISTS (
			SELECT 1
			FROM db_cdn.game_namespaces
			WHERE namespace_id = $1 AND auth_type = 1
		)
		"
	))
	.bind(namespace_id)
	.fetch_one(&ctx.crdb().await.unwrap())
	.await
	.unwrap();

	assert!(sql_exists);
}
