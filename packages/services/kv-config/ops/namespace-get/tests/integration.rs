use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let namespace_id = Uuid::new_v4();

	op!([ctx] kv_config_namespace_create {
		namespace_id: Some(namespace_id.into()),
	})
	.await
	.unwrap();

	let res = op!([ctx] kv_config_namespace_get {
		namespace_ids: vec![namespace_id.into()],
	})
	.await
	.unwrap();
	let _ns_data = res.namespaces.first().unwrap();
}
