use chirp_worker::prelude::*;

#[worker_test]
async fn upsert(ctx: TestCtx) {
	if !util::feature::cf_custom_hostname() {
		return;
	}

	let game_res = op!([ctx] faker_game { }).await.unwrap();
	let namespace_id = *game_res.namespace_ids.first().unwrap();
	let domain = format!("{}.com", util::faker::ident());

	// This should upsert the domain
	op!([ctx] cdn_namespace_domain_create {
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
			WHERE namespace_id = $1 AND domain = $2
		)
		"
	))
	.bind(namespace_id.as_uuid())
	.bind(domain)
	.fetch_one(&ctx.crdb().await.unwrap())
	.await
	.unwrap();
	assert!(sql_exists);
}

#[worker_test]
async fn invalid_domain(ctx: TestCtx) {
	if !util::feature::cf_custom_hostname() {
		return;
	}

	let game_res = op!([ctx] faker_game { }).await.unwrap();
	let namespace_id = *game_res.namespace_ids.first().unwrap();

	op!([ctx] cdn_namespace_domain_create {
		namespace_id: Some(namespace_id),
		domain: util::env::domain_main().unwrap().to_owned(),
	})
	.await
	.unwrap_err();

	op!([ctx] cdn_namespace_domain_create {
		namespace_id: Some(namespace_id),
		domain: util::env::domain_cdn().unwrap().to_owned(),
	})
	.await
	.unwrap_err();

	op!([ctx] cdn_namespace_domain_create {
		namespace_id: Some(namespace_id),
		domain: format!("test.{}", util::env::domain_main().unwrap()),
	})
	.await
	.unwrap_err();

	op!([ctx] cdn_namespace_domain_create {
		namespace_id: Some(namespace_id),
		domain: format!("test.{}", util::env::domain_cdn().unwrap()),
	})
	.await
	.unwrap_err();

	op!([ctx] cdn_namespace_domain_create {
		namespace_id: Some(namespace_id),
		domain: "".to_owned(),
	})
	.await
	.unwrap_err();

	op!([ctx] cdn_namespace_domain_create {
		namespace_id: Some(namespace_id),
		domain: (0..257).map(|_| 'a').collect(),
	})
	.await
	.unwrap_err();
}
