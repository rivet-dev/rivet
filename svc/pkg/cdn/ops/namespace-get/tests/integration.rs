use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let namespace_id = Uuid::new_v4();

	op!([ctx] cdn_namespace_create {
		namespace_id: Some(namespace_id.into()),
	})
	.await
	.unwrap();

	// TODO: Find a good way to mock this
	// for i in 0..3 {
	// 	op!([ctx] cdn_namespace_domain_create {
	// 		namespace_id: Some(namespace_id.into()),
	// 		domain: format!("{}.com", util::faker::ident()),
	// 	})
	// 	.await
	// 	.unwrap();
	// }

	let res = op!([ctx] cdn_namespace_get {
		namespace_ids: vec![namespace_id.into()],
	})
	.await
	.unwrap();
	let _ns_data = res.namespaces.first().unwrap();

	// TODO: Find a good way to mock this
	// assert_eq!(3, ns_data.config.as_ref().unwrap().domains.len());
}
