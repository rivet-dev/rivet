use chirp_workflow::prelude::*;
use nomad_client::{
	apis::{configuration::Configuration, nodes_api},
	models,
};
use rivet_operation::prelude::proto::backend::pkg::*;
use serde_json::json;

use crate::types::PoolType;

lazy_static::lazy_static! {
	static ref NOMAD_CONFIG: Configuration = nomad_util::new_config_from_env().unwrap();
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Input {
	pub datacenter_id: Uuid,
	pub server_id: Uuid,
	pub pool_type: PoolType,
}

#[workflow]
pub(crate) async fn cluster_server_drain(ctx: &mut WorkflowCtx, input: &Input) -> GlobalResult<()> {
	let drain_timeout = ctx
		.activity(GetDrainTimeoutInput {
			datacenter_id: input.datacenter_id,
			pool_type: input.pool_type.clone(),
		})
		.await?;

	match input.pool_type {
		PoolType::Job => {
			ctx.activity(DrainNodeInput {
				datacenter_id: input.datacenter_id,
				server_id: input.server_id,
				drain_timeout,
			})
			.await?;
		}
		PoolType::Gg => {
			ctx.tagged_signal(
				&json!({
					"server_id": input.server_id,
				}),
				crate::workflows::server::DnsDelete {},
			)
			.await?;
		}
		PoolType::Ats => {}
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
pub(crate) struct GetDrainTimeoutInput {
	pub datacenter_id: Uuid,
	pub pool_type: PoolType,
}

#[activity(GetDrainTimeout)]
pub(crate) async fn get_drain_timeout(
	ctx: &ActivityCtx,
	input: &GetDrainTimeoutInput,
) -> GlobalResult<u64> {
	let dcs_res = ctx
		.op(crate::ops::datacenter::get::Input {
			datacenter_ids: vec![input.datacenter_id],
		})
		.await?;
	let dc = unwrap!(dcs_res.datacenters.into_iter().next());

	let pool = unwrap!(
		dc.pools.iter().find(|p| p.pool_type == input.pool_type),
		"datacenter does not have this type of pool configured"
	);

	Ok(pool.drain_timeout)
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct DrainNodeInput {
	datacenter_id: Uuid,
	server_id: Uuid,
	drain_timeout: u64,
}

#[activity(DrainNode)]
async fn drain_node(ctx: &ActivityCtx, input: &DrainNodeInput) -> GlobalResult<()> {
	let (nomad_node_id,) = sql_fetch_one!(
		[ctx, (Option<String>,)]
		"
		SELECT nomad_node_id
		FROM db_cluster.servers
		WHERE server_id = $1
		",
		input.server_id,
	)
	.await?;

	if let Some(nomad_node_id) = nomad_node_id {
		let res = nodes_api::update_node_eligibility(
			&NOMAD_CONFIG,
			&nomad_node_id,
			models::NodeUpdateEligibilityRequest {
				eligibility: Some("ineligible".to_string()),
				node_id: Some(nomad_node_id.clone()),
			},
			None,
			None,
			None,
			None,
			None,
			None,
			None,
			None,
			None,
		)
		.await;

		// Catch "node not found" error
		if let Err(nomad_client::apis::Error::ResponseError(
			nomad_client::apis::ResponseContent { content, .. },
		)) = res
		{
			if content == "node not found" {
				tracing::warn!("node does not exist, not draining");
			}
		}

		// Prevent new matchmaker requests to the node running on this server
		msg!([ctx] mm::msg::nomad_node_closed_set(&nomad_node_id) {
			datacenter_id: Some(input.datacenter_id.into()),
			nomad_node_id: nomad_node_id.clone(),
			is_closed: true,
		})
		.await?;

		msg!([ctx] job_run::msg::drain_all(&nomad_node_id) {
			nomad_node_id: nomad_node_id.clone(),
			drain_timeout: input.drain_timeout,
		})
		.await?;

		msg!([ctx] ds::msg::drain_all(&nomad_node_id) {
			nomad_node_id: nomad_node_id.clone(),
			drain_timeout: input.drain_timeout,
		})
		.await?;
	}

	Ok(())
}
