use chirp_workflow::prelude::*;
use cloudflare::{endpoints as cf, framework as cf_framework};
use linode::util::{api, client};
use reqwest::header;
use serde_json::json;
use std::{collections::HashSet, convert::TryInto};

use crate::{
	types::{Filter, Provider, Server},
	util::cf_client,
};

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
	let servers_res = ctx
		.op(crate::ops::server::lost_list::Input {
			filter: input.filter.clone(),
		})
		.await?;

	prune_cloudflare(ctx, &servers_res).await?;

	prune_linode(ctx, &servers_res).await?;

	Ok(Output {})
}

#[tracing::instrument(skip_all)]
async fn prune_cloudflare(
	ctx: &OperationCtx,
	servers_res: &crate::ops::server::lost_list::Output,
) -> GlobalResult<()> {
	let zone_id = unwrap!(
		ctx.config().server()?.cloudflare()?.zone.job.clone(),
		"job cloudflare zone not configured"
	);
	let cf_token = ctx.config().server()?.cloudflare()?.auth_token.read();
	let client = cf_client(ctx.config(), Some(cf_token)).await?;

	for server in &servers_res.servers {
		let Some(wan_ip) = &server.wan_ip else {
			continue;
		};

		// Lookup DNS record
		let list_records_res = reqwest::Client::new()
			.get(format!(
				"https://api.cloudflare.com/client/v4/zones/{zone_id}/dns_records"
			))
			.bearer_auth(cf_token)
			.query(&[("type", "A"), ("content.exact", &wan_ip.to_string())])
			.send()
			.await?;

		let status = list_records_res.status();
		let result = if status.is_success() {
			match list_records_res
				.json::<cf_framework::response::ApiSuccess<Vec<cf::dns::DnsRecord>>>()
				.await
			{
				Ok(api_resp) => api_resp.result,
				Err(e) => return Err(cf_framework::response::ApiFailure::Invalid(e).into()),
			}
		} else {
			let parsed: Result<cf_framework::response::ApiErrors, reqwest::Error> =
				list_records_res.json().await;
			let errors = parsed.unwrap_or_default();
			return Err(cf_framework::response::ApiFailure::Error(status, errors).into());
		};

		// Delete DNS record
		// TODO: Do this for multiple records
		for record in result {
			tracing::info!(server_id = ?server.server_id, record_id = ?record.id, name = ?record.name, content = ?record.content, "pruning record");

			client
				.request(&cf::dns::DeleteDnsRecord {
					zone_identifier: &zone_id,
					identifier: &record.id,
				})
				.await?;
		}
	}

	Ok(())
}

#[tracing::instrument(skip_all)]
async fn prune_linode(
	ctx: &OperationCtx,
	servers_res: &crate::ops::server::lost_list::Output,
) -> GlobalResult<()> {
	let linode_token = ctx.config().server()?.linode()?.api_token.read().clone();

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
			"+contains": format!("{}-", ctx.config().server()?.rivet.namespace),
		}
	});
	let mut headers = header::HeaderMap::new();
	headers.insert(
		"X-Filter",
		header::HeaderValue::from_str(&serde_json::to_string(&filter)?)?,
	);

	for (provider, api_token) in accounts {
		match provider {
			Provider::Manual => {
				// Noop
			}
			Provider::Linode => {
				run_for_linode_account(&servers_res.servers, &api_token, &headers).await?;
			}
		}
	}

	Ok(())
}

#[tracing::instrument(skip_all)]
async fn run_for_linode_account(
	servers: &[Server],
	api_token: &str,
	headers: &header::HeaderMap,
) -> GlobalResult<()> {
	// Build HTTP client
	let client = client::Client::new_with_headers(api_token.to_string(), headers.clone()).await?;

	tracing::info!("pruning {} linode servers", servers.len());

	for server in servers {
		let Some(linode_id) = &server.provider_server_id else {
			tracing::warn!(server_id = ?server.server_id, "provider_server_ide is none");
			continue;
		};
		let linode_id = linode_id.parse()?;

		tracing::info!(server_id = ?server.server_id, ?linode_id, "pruning linode");

		let firewalls = api::list_linode_firewalls(&client, linode_id).await?;

		for firewall in firewalls {
			api::delete_firewall(&client, firewall.id).await?;
		}

		api::delete_instance(&client, linode_id).await?;

		// NOTE: Does not delete ssh keys
	}

	Ok(())
}
