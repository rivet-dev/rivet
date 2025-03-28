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
		PoolType::Pegboard | PoolType::PegboardIsolate => {
			ctx.activity(UndrainPegboardClientInput {
				server_id: input.server_id,
			})
			.await?;
		}
		PoolType::Ats | PoolType::Fdb | PoolType::Worker | PoolType::Nats => {}
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
		let nomad_config = nomad_util::new_build_config(ctx.config()).unwrap();
		let res = nodes_api::update_node_eligibility(
			&nomad_config,
			&nomad_node_id,
			nomad_client::models::NodeUpdateEligibilityRequest {
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

#[derive(Debug, Serialize, Deserialize, Hash)]
struct UndrainPegboardClientInput {
	server_id: Uuid,
}

#[activity(UndrainPegboardClient)]
#[max_retries = 15]
async fn undrain_pegboard_client(
	ctx: &ActivityCtx,
	input: &UndrainPegboardClientInput,
) -> GlobalResult<()> {
	let (pegboard_client_id, dc_name_id) = sql_fetch_one!(
		[ctx, (Option<Uuid>, String)]
		"
		SELECT pegboard_client_id, dc.name_id
		FROM db_cluster.servers AS s
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
		client: rivet_pools::reqwest::client().await?,
		base_path: ctx.config().server()?.rivet.edge_api_url_str(&dc_name_id)?,
		bearer_access_token: Some(token),
		..Default::default()
	};

	edge_intercom_pegboard_toggle_client_drain(
		&config,
		&pegboard_client_id.to_string(),
		models::EdgeIntercomPegboardToggleClientDrainRequest {
			draining: false,
			drain_complete_ts: None,
		},
	)
	.await?;

	Ok(())
}
