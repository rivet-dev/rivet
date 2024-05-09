use std::time::Duration;

use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[worker_test]
async fn cpu_stress(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let template_res = op!([ctx] faker_job_template {
		kind: Some(
			faker::job_template::request::Kind::Stress(
				faker::job_template::request::Stress {
					flags: "-c 1 -l 50".into(),
				}
			)
		),
		..Default::default()
	})
	.await
	.unwrap();

	let run_res = op!([ctx] faker_job_run {
		job_spec_json: Some(template_res.job_spec_json.clone()),
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
				task: util_job::RUN_MAIN_TASK_NAME.to_owned(),
			}],
		})
		.await
		.unwrap();

		let metrics = metrics_res.metrics.first().unwrap();
		let memory = *metrics.memory.last().unwrap();
		let cpu = *metrics.cpu.last().unwrap();
		if memory != 0 {
			tracing::info!(?memory, "received valid memory metrics");
			if cpu > 0.45 {
				tracing::info!(?cpu, "received valid cpu metrics");
				break;
			} else {
				tracing::info!("cpu metrics not high enough");
			}
		} else {
			tracing::info!("received zeroed metrics, either Prometheus has not polled this client yet or requests are failing");
		}
	}
}

#[worker_test]
async fn memory_stress(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let template_res = op!([ctx] faker_job_template {
		kind: Some(
			faker::job_template::request::Kind::Stress(
				faker::job_template::request::Stress {
					flags: "--vm 1 --vm-bytes 40M --vm-hang 0".into(),
				},
			)
		),
		..Default::default()
	})
	.await
	.unwrap();

	let run_res = op!([ctx] faker_job_run {
		job_spec_json: Some(template_res.job_spec_json.clone()),
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
				task: util_job::RUN_MAIN_TASK_NAME.to_owned(),
			}],
		})
		.await
		.unwrap();

		let metrics = metrics_res.metrics.first().unwrap();
		let memory = *metrics.memory.last().unwrap();
		if memory > 80_000_000 {
			tracing::info!(?memory, "received valid memory metrics");
			break;
		} else {
			tracing::info!("memory metrics not high enough");
		}
	}
}
