use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker_test]
async fn empty(ctx: TestCtx) {
	if !util::feature::job_run() {
		return;
	}

	let nomad_config = nomad_util::config_from_env().unwrap();

	let run_res = op!([ctx] faker_job_run {
		..Default::default()
	})
	.await
	.unwrap();
	let run_id = run_res.run_id.unwrap().as_uuid();

	// Read the alloc ID
	let (alloc_id,) = sqlx::query_as::<_, (String,)>(
		"SELECT alloc_id FROM db_job_state.run_meta_nomad WHERE run_id = $1",
	)
	.bind(run_id)
	.fetch_one(&ctx.crdb().await.unwrap())
	.await
	.unwrap();
	tracing::info!(%alloc_id, "alloc id");

	// Check allocation created
	nomad_client::apis::allocations_api::get_allocation(
		&nomad_config,
		&alloc_id,
		None,
		None,
		None,
		None,
	)
	.await
	.expect("failed to fetch allocation, this might be a 404");

	// Stop the allocation
	let mut finished_sub = subscribe!([ctx] job_run::msg::finished(run_id))
		.await
		.unwrap();
	let mut cleanup_sub = subscribe!([ctx] job_run::msg::cleanup(run_id))
		.await
		.unwrap();
	msg!([ctx] job_run::msg::stop(run_id) {
		run_id: Some(run_id.into()),
		..Default::default()
	})
	.await
	.unwrap();
	finished_sub.next().await.unwrap();
	cleanup_sub.next().await.unwrap();

	// Check if allocation still exists
	match nomad_client::apis::allocations_api::get_allocation(
		&nomad_config,
		&alloc_id,
		None,
		None,
		None,
		None,
	)
	.await
	{
		Ok(status) => {
			let task_state = status
				.task_states
				.as_ref()
				.unwrap()
				.get("test-server")
				.expect("missing test-server task state");
			tracing::info!(?task_state, "task state");
			assert!(!task_state.failed.unwrap(), "task failed");
			assert_eq!("stop", status.desired_status.as_ref().unwrap());
			assert_eq!("dead", task_state.state.as_ref().unwrap());
		}
		Err(err) => {
			tracing::info!(?err, "allocation no longer exists");
		}
	}
}
