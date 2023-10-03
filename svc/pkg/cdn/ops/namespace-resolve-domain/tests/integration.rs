use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	if !util::feature::cf_custom_hostname() {
		return;
	}

	let namespace_id = Uuid::new_v4();
	let domain = format!("{}.com", util::faker::ident());

	op!([ctx] cdn_namespace_create {
		namespace_id: Some(namespace_id.into()),
	})
	.await
	.unwrap();

	op!([ctx] cdn_namespace_domain_create {
		namespace_id: Some(namespace_id.into()),
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
