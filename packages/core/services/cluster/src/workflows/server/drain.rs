use chirp_workflow::prelude::*;
use nomad_client::apis::nodes_api;
use rivet_api::{
	apis::{
		configuration::Configuration,
		edge_intercom_pegboard_api::edge_intercom_pegboard_toggle_client_drain,
	},
	models,
};
use rivet_operation::prelude::proto::{self, backend::pkg::*};

use crate::types::PoolType;

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
			pool_type: input.pool_type,
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
			ctx.signal(crate::workflows::server::DnsDelete {})
				.tag("server_id", input.server_id)
				.send()
				.await?;
		}
		PoolType::Pegboard | PoolType::PegboardIsolate => {
			ctx.activity(DrainPegboardClientInput {
				datacenter_id: input.datacenter_id,
				server_id: input.server_id,
				drain_timeout,
			})
			.await?;
		}
		PoolType::Ats | PoolType::Fdb | PoolType::Worker => {}
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
	// Fetch drain ts from db to make the drain wf consistent if called more than once (in the event that
	// nomad/pb register after the drain started and this wf needs to be run again).
	let (drain_ts, nomad_node_id) = sql_fetch_one!(
		[ctx, (Option<i64>, Option<String>,)]
		"
		SELECT drain_ts, nomad_node_id
		FROM db_cluster.servers
		WHERE server_id = $1
		",
		input.server_id,
	)
	.await?;

	let Some(nomad_node_id) = nomad_node_id else {
		return Ok(());
	};

	let nomad_config = nomad_util::new_build_config(ctx.config())?;
	let res = nodes_api::update_node_eligibility(
		&nomad_config,
		&nomad_node_id,
		nomad_client::models::NodeUpdateEligibilityRequest {
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
	if let Err(nomad_client::apis::Error::ResponseError(nomad_client::apis::ResponseContent {
		content,
		..
	})) = res
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

	let drain_timeout = (drain_ts.unwrap_or_else(util::timestamp::now) as u64
		+ input.drain_timeout)
		.saturating_sub(util::timestamp::now() as u64);

	msg!([ctx] job_run::msg::drain_all(&nomad_node_id) {
		nomad_node_id: nomad_node_id.clone(),
		drain_timeout,
	})
	.await?;

	Ok(())
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct DrainPegboardClientInput {
	server_id: Uuid,
	datacenter_id: Uuid,
	drain_timeout: u64,
}

#[activity(DrainPegboardClient)]
async fn drain_pegboard_client(
	ctx: &ActivityCtx,
	input: &DrainPegboardClientInput,
) -> GlobalResult<()> {
	// Fetch drain ts from db to make the drain wf consistent if called more than once (in the event that
	// nomad/pb register after the drain started and this wf needs to be run again).
	let (drain_ts, pegboard_client_id, dc_name_id) = sql_fetch_one!(
		[ctx, (Option<i64>, Option<Uuid>, String)]
		"
		SELECT drain_ts, pegboard_client_id, dc.name_id
		FROM db_cluster.servers
		JOIN db_cluster.datacenters AS dc
		ON s.datacenter_id = dc.datacenter_id
		WHERE server_id = $1
		",
		input.server_id,
	)
	.await?;

	let Some(pegboard_client_id) = pegboard_client_id else {
		return Ok(());
	};

	let drain_complete_ts =
		drain_ts.unwrap_or_else(util::timestamp::now) + input.drain_timeout as i64;

	// Create ephemeral token to authenticate with edge
	let token_res = op!([ctx] token_create {
		token_config: Some(token::create::request::TokenConfig {
			ttl: util::duration::minutes(5),
		}),
		refresh_token_config: None,
		issuer: "cluster_server".to_owned(),
		client: None,
		kind: Some(token::create::request::Kind::New(
			token::create::request::KindNew { entitlements: vec![proto::claims::Entitlement {
				kind: Some(proto::claims::entitlement::Kind::Bypass(
					proto::claims::entitlement::Bypass { }
				)),
			}]},
		)),
		label: Some("byp".to_owned()),
		ephemeral: true,
	})
	.await?;
	let token = unwrap!(token_res.token).token;

	let config = Configuration {
		base_path: ctx.config().server()?.rivet.edge_api_url_str(&dc_name_id)?,
		bearer_access_token: Some(token),
		..Default::default()
	};

	edge_intercom_pegboard_toggle_client_drain(
		&config,
		&pegboard_client_id.to_string(),
		models::EdgeIntercomPegboardToggleClientDrainRequest {
			draining: true,
			drain_complete_ts: Some(util::timestamp::to_string(drain_complete_ts)?),
		},
	)
	.await?;

	Ok(())
}
