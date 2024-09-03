use chirp_workflow::prelude::*;
use nomad_client::{
	apis::{configuration::Configuration, nodes_api},
	models,
};
use rivet_operation::prelude::proto::backend::pkg::mm;

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
pub(crate) async fn cluster_server_undrain(
	ctx: &mut WorkflowCtx,
	input: &Input,
) -> GlobalResult<()> {
	match input.pool_type {
		PoolType::Job => {
			ctx.activity(UndrainNodeInput {
				datacenter_id: input.datacenter_id,
				server_id: input.server_id,
			})
			.await?;
		}
		PoolType::Gg => {
			ctx.signal(crate::workflows::server::DnsCreate {})
				.tag("server_id", input.server_id)
				.send()
				.await?;
		}
		PoolType::Ats => {}
	}

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UndrainNodeInput {
	datacenter_id: Uuid,
	server_id: Uuid,
}

#[activity(UndrainNode)]
async fn undrain_node(ctx: &ActivityCtx, input: &UndrainNodeInput) -> GlobalResult<()> {
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
				eligibility: Some("eligible".to_string()),
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

		// Allow new matchmaker requests to the node running on this server
		msg!([ctx] mm::msg::nomad_node_closed_set(&nomad_node_id) {
			datacenter_id: Some(input.datacenter_id.into()),
			nomad_node_id: nomad_node_id.clone(),
			is_closed: false,
		})
		.await?;
	}

	Ok(())
}
