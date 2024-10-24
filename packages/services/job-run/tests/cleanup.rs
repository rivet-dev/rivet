use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn raw(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let run_res = op!([ctx] faker_job_run {
		..Default::default()
	})
	.await
	.unwrap();
	let run_id = run_res.run_id.as_ref().unwrap().as_uuid();

	msg!([ctx] job_run::msg::cleanup(run_id) -> job_run::msg::cleanup_complete {
		run_id: Some(run_id.into()),
		..Default::default()
	})
	.await
	.unwrap();

	let get_res = op!([ctx] job_run::ops::get {
		run_ids: vec![run_id.into()],
	})
	.await
	.unwrap();
	let run = get_res.runs.first().unwrap();
	assert!(run.cleanup_ts.is_some());
}
