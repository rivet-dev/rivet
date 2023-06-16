use futures_util::StreamExt;
use rivet_operation::prelude::*;

use crate::NOMAD_CONFIG;

/// Number of idle nodes to keep.
const KEEP_IDLE_NODES: usize = 8;

#[tracing::instrument(skip_all)]
pub async fn run(pools: rivet_pools::Pools, ctx: OperationContext<()>) -> GlobalResult<()> {
	let region_list_res = op!([ctx] region_list {
		..Default::default()
	})
	.await?;
	let regions_res = op!([ctx] region_get {
		region_ids: region_list_res.region_ids.clone(),
	})
	.await?;

	for region in &regions_res.regions {
		let mut empty_nodes = Vec::new();
		let mut active_nodes = Vec::new();

		// Find all empty nodes
		let nodes = nomad_client::apis::nodes_api::get_nodes(
			&NOMAD_CONFIG,
			None,
			Some(&region.nomad_region),
			None,
			None,
			None,
		)
		.await?;
		for node in &nodes {
			if internal_unwrap!(node.node_class) != "job" {
				continue;
			}

			let node_id = internal_unwrap_owned!(node.ID.clone());
			let allocs = nomad_client::apis::nodes_api::get_allocations_for_node(
				&NOMAD_CONFIG,
				node_id.as_str(),
				None,
				Some(&region.nomad_region),
				None,
				None,
			)
			.await?;
			if allocs.is_empty() {
				empty_nodes.push(node_id);
			} else {
				active_nodes.push(node_id);
			}
		}

		empty_nodes.sort();
		active_nodes.sort();
		tracing::info!(
			region = %region.nomad_region,
			empty_len = empty_nodes.len(),
			active_len = active_nodes.len(),
			"nodes"
		);

		if empty_nodes.len() > KEEP_IDLE_NODES {
			let nodes_to_remove = &empty_nodes[KEEP_IDLE_NODES..];
			tracing::info!(len = nodes_to_remove.len(), nodes = ?nodes_to_remove, "nodes to remove");

			for node in nodes_to_remove {
				let allocs = nomad_client::apis::nodes_api::update_drain_mode_for_node(
					&NOMAD_CONFIG,
					node.as_str(),
					None,
					Some(&region.nomad_region),
					None,
					None,
					Some(nomad_client::models::NodeUpdateDrainRequest {
						node_id: Some(node.clone()),
						drain_spec: Some(Box::new(nomad_client::models::DrainSpec {
							deadline: 1800000000000,
							ignore_system_jobs: Some(false),
						})),
						mark_eligible: Some(false),
					}),
				)
				.await?;
			}
		}
	}

	Ok(())
}
