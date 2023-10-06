use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let region_res = op!([ctx] faker_region {}).await.unwrap();
	let region_id = region_res.region_id.unwrap();

	// Create logging template
	let template_res = op!([ctx] faker_job_template {
		kind: Some(faker::job_template::request::Kind::Counter(faker::job_template::request::Counter {
			interval_ms: 1000,
		})),
		..Default::default()
	})
	.await
	.unwrap();

	// Run the job
	let run_id = Uuid::new_v4();
	let alloc_res =
		msg!([ctx] @notrace job_run::msg::create(run_id) -> Result<job_run::msg::alloc_planned, job_run::msg::fail> {
			run_id: Some(run_id.into()),
			region_id: Some(region_id),
			parameters: vec![],
			job_spec_json: template_res.job_spec_json.clone(),
			..Default::default()
		})
		.await
		.unwrap()
		.unwrap();
	let alloc_id = if let job_run::msg::alloc_planned::message::RunMeta::Nomad(nomad) =
		alloc_res.run_meta.as_ref().unwrap()
	{
		nomad.alloc_id.clone()
	} else {
		unreachable!()
	};

	// Listen for messages
	let mut log_sub = subscribe!([ctx] nomad_log::msg::entries(alloc_id, "test-counter", "stdout"))
		.await
		.unwrap();
	log_sub.next().await.unwrap();
	log_sub.next().await.unwrap();
}

// TODO: Migrate this test to ClickHouse
// #[worker_test]
// async fn e2e_stress(ctx: TestCtx) {
// 	let scylla = ctx.scylla().await.unwrap();

// 	let region_res = op!([ctx] faker_region {}).await.unwrap();
// 	let region_id = region_res.region_id.unwrap();

// 	// Create logging template
// 	let template_res = op!([ctx] faker_job_template {
// 		kind: Some(faker::job_template::request::Kind::Counter(faker::job_template::request::Counter {
// 			interval_ms: 0,
// 		})),
// 		..Default::default()
// 	})
// 	.await
// 	.unwrap();

// 	// Run the job
// 	let run_id = Uuid::new_v4();
// 	let alloc_res =
// 		msg!([ctx] @notrace job_run::msg::create(run_id) -> Result<job_run::msg::alloc_planned, job_run::msg::fail> {
// 			run_id: Some(run_id.into()),
// 			region_id: Some(region_id),
// 			parameters: vec![],
// 			job_spec_json: template_res.job_spec_json.clone(),
// 			..Default::default()
// 		})
// 		.await
// 		.unwrap()
// 		.unwrap();
// 	let alloc_id = if let job_run::msg::alloc_planned::message::RunMeta::Nomad(nomad) =
// 		alloc_res.run_meta.as_ref().unwrap()
// 	{
// 		nomad.alloc_id.clone()
// 	} else {
// 		unreachable!()
// 	};

// 	// Test that the logs successfully truncate three times before passing
// 	let mut last_count = 0i64;
// 	let mut truncate_count = 0usize;
// 	let mut truncate_history = Vec::new();
// 	loop {
// 		tokio::time::sleep(Duration::from_secs(1)).await;

// 		let (count,) = scylla
// 			.query(
// 				"SELECT COUNT(*) FROM logs WHERE alloc = ? AND task = 'test-counter' AND stream_type = 0",
// 				(&alloc_id,),
// 			)
// 			.await
// 			.unwrap()
// 			.first_row_typed::<(i64,)>()
// 			.unwrap();

// 		tracing::info!(?count, ?last_count, "count");

// 		if count < last_count {
// 			truncate_count += 1;
// 			truncate_history.push((last_count, count));
// 			tracing::info!(?truncate_count, "successfully truncated");

// 			if truncate_count > 10 {
// 				break;
// 			}
// 		}

// 		last_count = count;
// 	}

// 	tracing::info!(?truncate_history, "truncate history");
// }
