use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let namespace_id = Uuid::new_v4();
	let domain = "test.com";

	op!([ctx] cdn_namespace_create {
		namespace_id: Some(namespace_id.into()),
	})
	.await
	.unwrap();

	op!([ctx] cdn_namespace_domain_create {
		namespace_id: Some(namespace_id.into()),
		domain: domain.into(),
	})
	.await
	.unwrap();

	op!([ctx] cdn_namespace_domain_remove {
		namespace_id: Some(namespace_id.into()),
		domain: domain.into(),
	})
	.await
	.unwrap();

	let (sql_exists,) = sqlx::query_as::<_, (bool,)>(indoc!(
		"
		SELECT EXISTS (
			SELECT 1
			FROM db_cdn.game_namespace_domains
			WHERE
				namespace_id = $1 AND
				domain = $2
		)
		"
	))
	.bind(namespace_id)
	.bind(domain)
	.fetch_one(&ctx.crdb().await.unwrap())
	.await
	.unwrap();
	assert!(!sql_exists);
}
