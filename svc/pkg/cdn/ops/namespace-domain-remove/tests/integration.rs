use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	if !util::feature::cf_custom_hostname() {
		return;
	}

	let game_res = op!([ctx] faker_game { }).await.unwrap();
	let namespace_id = *game_res.namespace_ids.first().unwrap();
	let domain = format!("{}.com", util::faker::ident());

	op!([ctx] cdn_namespace_domain_create {
		namespace_id: Some(namespace_id),
		domain: domain.clone(),
	})
	.await
	.unwrap();

	op!([ctx] cdn_namespace_domain_remove {
		namespace_id: Some(namespace_id),
		domain: domain.clone(),
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
	.bind(namespace_id.as_uuid())
	.bind(domain)
	.fetch_one(&ctx.crdb().await.unwrap())
	.await
	.unwrap();
	assert!(!sql_exists);
}
