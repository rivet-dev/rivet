use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	op!([ctx] faker_job_run {
		..Default::default()
	})
	.await
	.unwrap();
}
