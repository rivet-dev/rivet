use chirp_worker::prelude::*;

#[worker_test]
async fn upsert(ctx: TestCtx) {
	let namespace_id = Uuid::new_v4();
	let domain = format!("{}.com", util::faker::ident());

	op!([ctx] cdn_namespace_create {
		namespace_id: Some(namespace_id.into()),
	})
	.await
	.unwrap();

	// Create the domain
	op!([ctx] cdn_namespace_domain_create {
		namespace_id: Some(namespace_id.into()),
		domain: domain.clone(),
	})
	.await
	.unwrap();

	// This should upsert the domain
	op!([ctx] cdn_namespace_domain_create {
		namespace_id: Some(namespace_id.into()),
		domain: domain.clone(),
	})
	.await
	.unwrap();

	let (sql_exists,) = sqlx::query_as::<_, (bool,)>(indoc!(
		"
		SELECT EXISTS (
			SELECT 1
			FROM game_namespace_domains
			WHERE namespace_id = $1 AND domain = $2
		)
		"
	))
	.bind(namespace_id)
	.bind(domain)
	.fetch_one(&ctx.crdb("db-cdn").await.unwrap())
	.await
	.unwrap();
	assert!(sql_exists);
}

#[worker_test]
async fn invalid_domain(ctx: TestCtx) {
	let namespace_id = Uuid::new_v4();

	op!([ctx] cdn_namespace_create {
		namespace_id: Some(namespace_id.into()),
	})
	.await
	.unwrap();

	op!([ctx] cdn_namespace_domain_create {
		namespace_id: Some(namespace_id.into()),
		domain: util::env::domain_main().to_owned(),
	})
	.await
	.unwrap_err();

	op!([ctx] cdn_namespace_domain_create {
		namespace_id: Some(namespace_id.into()),
		domain: util::env::domain_cdn().to_owned(),
	})
	.await
	.unwrap_err();

	op!([ctx] cdn_namespace_domain_create {
		namespace_id: Some(namespace_id.into()),
		domain: format!("test.{}", util::env::domain_main()),
	})
	.await
	.unwrap_err();

	op!([ctx] cdn_namespace_domain_create {
		namespace_id: Some(namespace_id.into()),
		domain: format!("test.{}", util::env::domain_cdn()),
	})
	.await
	.unwrap_err();

	op!([ctx] cdn_namespace_domain_create {
		namespace_id: Some(namespace_id.into()),
		domain: "".to_owned(),
	})
	.await
	.unwrap_err();

	op!([ctx] cdn_namespace_domain_create {
		namespace_id: Some(namespace_id.into()),
		domain: (0..257).map(|_| 'a').collect(),
	})
	.await
	.unwrap_err();
}
