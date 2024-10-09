use std::{collections::HashSet, convert::TryInto};

use chirp_workflow::prelude::*;
use linode::util::{api, client};
use reqwest::header;
use serde_json::json;

use crate::types::{Filter, Provider, Server};

#[derive(Debug)]
pub struct Input {
	pub filter: Filter,
}

#[derive(Debug)]
pub struct Output {}

#[operation]
pub async fn cluster_server_prune_with_filter(
	ctx: &OperationCtx,
	input: &Input,
) -> GlobalResult<Output> {
	let linode_token = util::env::read_secret(&["linode", "token"]).await?;

	let servers_res = ctx
		.op(crate::ops::server::lost_list::Input {
			filter: input.filter.clone(),
		})
		.await?;

	let dc_ids = servers_res
		.servers
		.iter()
		.map(|x| x.datacenter_id)
		.collect::<Vec<_>>();
	let accounts = sql_fetch_all!(
		[ctx, (i64, String)]
		"
		SELECT provider, provider_api_token
		FROM db_cluster.datacenters
		WHERE
			provider_api_token IS NOT NULL AND
			datacenter_id = ANY($1)
		",
		dc_ids,
	)
	.await?
	.into_iter()
	.map(|(provider, provider_api_token)| {
		Ok((
			unwrap!(Provider::from_repr(provider.try_into()?)),
			provider_api_token,
		))
	})
	.chain(std::iter::once(Ok((Provider::Linode, linode_token))))
	.collect::<GlobalResult<HashSet<_>>>()?;

	// Filter by namespace
	let filter = json!({
		"label": {
			"+contains": format!("{}-", util::env::namespace()),
		}
	});
	let mut headers = header::HeaderMap::new();
	headers.insert(
		"X-Filter",
		header::HeaderValue::from_str(&serde_json::to_string(&filter)?)?,
	);

	for (provider, api_token) in accounts {
		match provider {
			Provider::Linode => {
				run_for_linode_account(&servers_res.servers, &api_token, &headers).await?;
			}
		}
	}

	Ok(Output {})
}

async fn run_for_linode_account(
	servers: &[Server],
	api_token: &str,
	headers: &header::HeaderMap,
) -> GlobalResult<()> {
	// Build HTTP client
	let client =
		client::Client::new_with_headers(Some(api_token.to_string()), headers.clone()).await?;

	tracing::info!("pruning {} servers", servers.len());

	for server in servers {
		let Some(linode_id) = &server.provider_server_id else {
			tracing::warn!(server_id = ?server.server_id, "provider_server_ide is none");
			continue;
		};
		let linode_id = linode_id.parse()?;

		tracing::info!("pruning {} (linode_id {})", server.server_id, linode_id);

		let firewalls = api::list_linode_firewalls(&client, linode_id).await?;

		for firewall in firewalls {
			api::delete_firewall(&client, firewall.id).await?;
		}

		api::delete_instance(&client, linode_id).await?;

		// NOTE: Does not delete ssh keys
	}

	Ok(())
}
