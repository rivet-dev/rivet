use chirp_worker::prelude::*;
use proto::backend;

use ::job_gc::run_from_env;

lazy_static::lazy_static! {
	static ref NOMAD_CONFIG: nomad_client::apis::configuration::Configuration =
		nomad_util::config_from_env().unwrap();
}

#[tokio::test]
async fn all() {
	// Run tests sequentially so the they don't interfere with each other

	tracing_subscriber::fmt()
		.json()
		.with_max_level(tracing::Level::INFO)
		.with_span_events(tracing_subscriber::fmt::format::FmtSpan::CLOSE)
		.init();

	let ctx = TestCtx::from_env("all").await.unwrap();

	test_kill_orphaned_job(ctx).await;
}

async fn test_kill_orphaned_job(ctx: TestCtx) {
	let pools = rivet_pools::from_env("job-gc-test").await.unwrap();

	// Run the job
	let run_res = op!([ctx] faker_job_run {}).await.unwrap();
	let run_id = run_res.run_id.as_ref().unwrap().as_uuid();

	let run_get_res = op!([ctx] job_run_get {
		run_ids: vec![run_id.into()],
	})
	.await
	.unwrap();
	let run_data = run_get_res.runs.first().unwrap();
	let _dispatched_job_id = match &run_data.run_meta.as_ref().unwrap().kind {
		Some(backend::job::run_meta::Kind::Nomad(run_meta_nomad)) => {
			run_meta_nomad.dispatched_job_id.clone()
		}
		_ => panic!("expected nomad run meta"),
	};

	// This should do nothing
	run_from_env(util::timestamp::now(), pools).await.unwrap();

	// TODO: Stop the job without nomad-monitor picking it up somehow then call
	// run_from_env again
}
