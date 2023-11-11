use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn empty(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let run_res = op!([ctx] faker_job_run {
		..Default::default()
	})
	.await
	.unwrap();
	let run_id = run_res.run_id.as_ref().unwrap().as_uuid();

	// Read the run
	let res = op!([ctx] job_run_get {
		run_ids: vec![run_id.into(), Uuid::new_v4().into()],
	})
	.await
	.unwrap();
	assert_eq!(1, res.runs.len());
}
