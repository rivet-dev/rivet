use chirp_worker::prelude::*;

#[worker_test]
async fn basic(ctx: TestCtx) {
	let _ = op!([ctx] region_config_get {}).await.unwrap();
}
