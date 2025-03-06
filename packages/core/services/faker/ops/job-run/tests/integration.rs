use chirp_worker::prelude::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	if ctx.config().server().unwrap().rivet.job_run.is_none() {
		return;
	}

	op!([ctx] faker_job_run {
		..Default::default()
	})
	.await
	.unwrap();
}
