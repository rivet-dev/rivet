use std::sync::Arc;

use rivet_operation::prelude::*;

mod monitors;
use monitors::*;

pub async fn run_from_env(pools: rivet_pools::Pools) -> GlobalResult<()> {
	let shared_client = chirp_client::SharedClient::from_env(pools.clone())?;
	let redis_job = pools.redis("persistent")?;

	// Start nomad event monitor
	let redis_index_key = "nomad:monitor_index";
	let configuration = nomad_util::new_config_from_env().unwrap();

	nomad_util::monitor::Monitor::run(
		configuration,
		redis_job,
		redis_index_key,
		&["Allocation", "Evaluation", "Node"],
		|event| handle(shared_client.clone(), event),
	)
	.await?;

	Ok(())
}

async fn handle(
	shared_client: Arc<chirp_client::SharedClient>,
	event: nomad_util::monitor::NomadEvent,
) {
	// TODO: Figure out how to abstract the branches
	if let Some(payload) = event
		.decode::<alloc_plan::PlanResult>("Allocation", "PlanResult")
		.unwrap()
	{
		let client = shared_client.wrap_new("nomad-alloc-plan-monitor");
		let spawn_res = tokio::task::Builder::new()
			.name("nomad_alloc_plan_monitor::handle_event")
			.spawn(async move {
				match alloc_plan::handle(client, &payload, event.payload.to_string()).await {
					Ok(_) => {}
					Err(err) => {
						tracing::error!(?err, ?payload, "error handling event");
					}
				}
			});
		if let Err(err) = spawn_res {
			tracing::error!(?err, "failed to spawn handle_event task");
		}
	} else if let Some(payload) = event
		.decode::<alloc_update::AllocationUpdated>("Allocation", "AllocationUpdated")
		.unwrap()
	{
		let client = shared_client.wrap_new("nomad-alloc-updated-monitor");
		let spawn_res = tokio::task::Builder::new()
			.name("nomad_alloc_update_monitor::handle_event")
			.spawn(async move {
				match alloc_update::handle(client, &payload, event.payload.to_string()).await {
					Ok(_) => {}
					Err(err) => {
						tracing::error!(?err, ?payload, "error handling event");
					}
				}
			});
		if let Err(err) = spawn_res {
			tracing::error!(?err, "failed to spawn handle_event task");
		}
	} else if let Some(payload) = event
		.decode::<eval_update::PlanResult>("Evaluation", "EvaluationUpdated")
		.unwrap()
	{
		let client = shared_client.wrap_new("nomad-eval-update-monitor");
		let spawn_res = tokio::task::Builder::new()
			.name("nomad_eval_update_monitor::handle_event")
			.spawn(async move {
				match eval_update::handle(client, &payload, event.payload.to_string()).await {
					Ok(_) => {}
					Err(err) => {
						tracing::error!(?err, ?payload, "error handling event");
					}
				}
			});
		if let Err(err) = spawn_res {
			tracing::error!(?err, "failed to spawn handle_event task");
		}
	} else if let Some(payload) = event
		.decode::<node_registration::NodeRegistration>("Node", "NodeRegistration")
		.unwrap()
	{
		let client = shared_client.wrap_new("nomad-node-registration-monitor");
		let spawn_res = tokio::task::Builder::new()
			.name("nomad_node_registration_monitor::handle")
			.spawn(async move {
				match node_registration::handle(client, &payload).await {
					Ok(_) => {}
					Err(err) => {
						tracing::error!(?err, ?payload, "error handling event");
					}
				}
			});
		if let Err(err) = spawn_res {
			tracing::error!(?err, "failed to spawn handle_event task");
		}
	} else if let Some(payload) = event
		.decode::<node_drain::NodeDrain>("Node", "NodeDrain")
		.unwrap()
	{
		let client = shared_client.wrap_new("nomad-node-drain-monitor");
		let spawn_res = tokio::task::Builder::new()
			.name("nomad_node_drain_monitor::handle")
			.spawn(async move {
				match node_drain::handle(client, &payload).await {
					Ok(_) => {}
					Err(err) => {
						tracing::error!(?err, ?payload, "error handling event");
					}
				}
			});
		if let Err(err) = spawn_res {
			tracing::error!(?err, "failed to spawn handle_event task");
		}
	}
}
