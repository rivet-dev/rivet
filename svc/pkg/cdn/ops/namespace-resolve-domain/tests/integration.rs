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

	let res = op!([ctx] cdn_namespace_resolve_domain {
		domains: vec![
			domain.clone(),
			format!("{}.com", util::faker::ident()),
		],
	})
	.await
	.unwrap();
	assert_eq!(1, res.namespaces.len());
}
