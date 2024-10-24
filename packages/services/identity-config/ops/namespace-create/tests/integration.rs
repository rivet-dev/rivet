use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	let _res = op!([ctx] identity_config_namespace_create {
		namespace_id: Some(Uuid::new_v4().into()),
	})
	.await
	.unwrap();
}
