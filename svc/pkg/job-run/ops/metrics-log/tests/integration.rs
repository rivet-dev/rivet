use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

use std::time::Duration;

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
	let run_id = run_res.run_id.as_ref().unwrap();

	let job_res = op!([ctx] job_run_get {
		run_ids: vec![*run_id],
	})
	.await
	.unwrap();
	let job_run = job_res.runs.first().unwrap();
	let nomad_job_id = if let Some(backend::job::RunMeta {
		kind: Some(backend::job::run_meta::Kind::Nomad(nomad)),
	}) = job_run.run_meta.as_ref()
	{
		nomad.dispatched_job_id.as_ref().unwrap()
	} else {
		unreachable!()
	};

	// Poll metrics until they return non-0 results
	loop {
		tokio::time::sleep(Duration::from_secs(2)).await;

		let now = util::timestamp::now();
		let metrics_res = op!([ctx] job_run_metrics_log {
			start: (now - util::duration::minutes(15)),
			end: now,
			step: 15000,
			metrics: vec![job_run::metrics_log::request::Metric {
				job: nomad_job_id.clone(),
				task: "test-server".to_owned(),
			}],
		})
		.await
		.unwrap();

		let metrics = metrics_res.metrics.first().unwrap();
		if *metrics.memory.last().unwrap() != 0 {
			tracing::info!("received non-0 metrics");
			break;
		} else {
			tracing::info!("received zeroed metrics, either Prometheus has not polled this client yet or requests are failing");
		}
	}
}
