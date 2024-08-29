use std::collections::HashMap;

use chirp_workflow::prelude::*;
use nomad_client::apis::{allocations_api, configuration::Configuration, nodes_api};

lazy_static::lazy_static! {
	static ref NOMAD_CONFIG: Configuration = nomad_util::new_config_from_env().unwrap();
}

#[derive(sqlx::FromRow)]
struct ServerRow {
	server_id: Uuid,
	datacenter_id: Uuid,
	nomad_node_id: String,
}

#[derive(Debug)]
pub struct Input {
	pub datacenter_ids: Vec<Uuid>,
}

#[derive(Debug)]
pub struct Output {
	pub datacenters: Vec<Datacenter>,
}

#[derive(Debug)]
pub struct Datacenter {
	pub datacenter_id: Uuid,
	pub servers: Vec<Server>,
}

#[derive(Debug)]
pub struct Server {
	pub server_id: Uuid,
	pub node_id: String,
	pub usage: Stats,
	pub limits: Stats,
}

#[derive(Debug)]
pub struct Stats {
	pub cpu: u64,
	pub memory: u64,
	pub disk: u64,
}

#[operation]
pub async fn cluster_datacenter_topology_get(
	ctx: &OperationCtx,
	input: &Input,
) -> GlobalResult<Output> {
	let servers = sql_fetch_all!(
		[ctx, ServerRow]
		"
		SELECT
			server_id, datacenter_id, nomad_node_id
		FROM db_cluster.servers
		WHERE
			datacenter_id = ANY($1) AND
			nomad_node_id IS NOT NULL AND
			cloud_destroy_ts IS NULL AND
			taint_ts IS NULL
		",
		&input.datacenter_ids,
	)
	.await?;

	// Fetch batch data from nomad
	let (allocation_info, node_info) = tokio::try_join!(
		async {
			// Request is not paginated
			allocations_api::get_allocations(
				&NOMAD_CONFIG,
				None,
				None,
				None,
				None,
				None,
				None,
				None,
				None,
				None,
				Some(true),
				None,
			)
			.await
			.map_err(Into::<GlobalError>::into)
		},
		async {
			// Request is not paginated
			nodes_api::get_nodes(
				&NOMAD_CONFIG,
				None,
				None,
				None,
				None,
				None,
				None,
				None,
				None,
				None,
				Some(true),
			)
			.await
			.map_err(Into::<GlobalError>::into)
		},
	)?;

	// Preempt datacenters
	let mut datacenters = input
		.datacenter_ids
		.iter()
		.map(|datacenter_id| {
			(
				*datacenter_id,
				Datacenter {
					datacenter_id: *datacenter_id,
					servers: Vec::new(),
				},
			)
		})
		.collect::<HashMap<_, _>>();

	for server in servers {
		let mut usage = Stats {
			cpu: 0,
			memory: 0,
			disk: 0,
		};

		// Aggregate all allocated resources for this node
		for alloc in &allocation_info {
			let alloc_node_id = unwrap_ref!(alloc.node_id);

			if alloc_node_id == &server.nomad_node_id {
				let resources = unwrap_ref!(alloc.allocated_resources);
				let shared_resources = unwrap_ref!(resources.shared);

				// Task states don't exist until a task starts
				if let Some(task_states) = &alloc.task_states {
					let tasks = unwrap_ref!(resources.tasks);

					for (task_name, task) in tasks {
						let task_state = unwrap!(task_states.get(task_name));
						let state = unwrap_ref!(task_state.state);

						// Only count pending, running, or failed tasks. In a "failed" allocation, all of the
						// tasks are have a "dead" state
						if state != "pending" && state != "running" && state != "failed" {
							continue;
						}

						let cpu = unwrap_ref!(task.cpu);
						let memory = unwrap_ref!(task.memory);

						usage.cpu += unwrap!(cpu.cpu_shares) as u64;
						usage.memory += unwrap!(memory.memory_mb) as u64;
					}
				}

				usage.disk += unwrap!(shared_resources.disk_mb) as u64;
			}
		}

		// Gracefully handle if node does not exist in API response
		let Some(node) = node_info.iter().find(|node| {
			node.ID
				.as_ref()
				.map_or(false, |node_id| node_id == &server.nomad_node_id)
		}) else {
			tracing::error!(%server.nomad_node_id, "node not found in nomad response");

			continue;
		};

		// Get node resource limits
		let resources = unwrap_ref!(node.node_resources);
		let limits = Stats {
			cpu: unwrap!(unwrap_ref!(resources.cpu).cpu_shares) as u64,
			memory: unwrap!(unwrap_ref!(resources.memory).memory_mb) as u64,
			disk: unwrap!(unwrap_ref!(resources.disk).disk_mb) as u64,
		};

		let datacenter = unwrap!(datacenters.get_mut(&server.datacenter_id));

		datacenter.servers.push(Server {
			server_id: server.server_id,
			node_id: server.nomad_node_id,
			usage,
			limits,
		});
	}

	Ok(Output {
		datacenters: datacenters.into_values().collect(),
	})
}
