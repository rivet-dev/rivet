use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let namespace_id = Uuid::new_v4();
	op!([ctx] cloud_namespace_create {
		namespace_id: Some(namespace_id.into()),
	})
	.await
	.unwrap();

	let res = op!([ctx] cloud_namespace_get {
		namespace_ids: vec![namespace_id.into(), Uuid::new_v4().into()],
	})
	.await
	.unwrap();

	assert_eq!(1, res.namespaces.len());
	let _namespace_res = res.namespaces.first().expect("namespace not returned");
}
