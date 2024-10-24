use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	op!([ctx] perf_log_get {
		ray_ids: Vec::new()
	})
	.await
	.unwrap();
}
