use std::{collections::HashSet, convert::TryInto, net::IpAddr};

use chirp_workflow::prelude::*;
use linode::util::client;
use reqwest::header;
use serde_json::json;

use super::get::ServerRow;
use crate::types::{Filter, Provider, Server};

#[derive(Deserialize)]
struct GetLinodesResponse {
	data: Vec<Linode>,
}

#[derive(Deserialize)]
struct Linode {
	created: chrono::NaiveDateTime,
	label: String,
}

#[derive(Debug)]
pub struct Input {
	pub filter: Filter,
}

#[derive(Debug)]
pub struct Output {
	pub servers: Vec<Server>,
}

/// Fetches deleted servers directly from the cloud providers own APIs and returns existing servers older
/// than 12 hours.
#[operation]
pub async fn cluster_server_lost_list(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let linode_token = util::env::read_secret(&["linode", "token"]).await?;

	let accounts = sql_fetch_all!(
		[ctx, (sqlx::types::Json<Provider>, String)]
		"
		SELECT provider2, provider_api_token
		FROM db_cluster.datacenters
		WHERE
			provider_api_token IS NOT NULL AND
			($1 IS NULL OR cluster_id = ANY($1))
		",
		&input.filter.cluster_ids,
	)
	.await?
	.into_iter()
	.map(|(provider, provider_api_token)| (provider.0, provider_api_token))
	.chain(std::iter::once((
		Provider::Linode,
		util::env::read_secret(&["linode", "token"]).await?,
	)))
	.collect::<HashSet<_>>();

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

	let mut servers = Vec::new();
	for (provider, api_token) in accounts {
		match provider {
			Provider::Linode => {
				servers.extend(
					run_for_linode_account(ctx, &input.filter, &api_token, &headers).await?,
				);
			}
		}
	}

	Ok(Output { servers })
}

async fn run_for_linode_account(
	ctx: &OperationCtx,
	filter: &Filter,
	api_token: &str,
	headers: &header::HeaderMap,
) -> GlobalResult<Vec<Server>> {
	// Build HTTP client
	let client =
		client::Client::new_with_headers(Some(api_token.to_string()), headers.clone()).await?;

	let req = client
		.inner()
		.get("https://api.linode.com/v4/linode/instances")
		.query(&[("page_size", 500)]);

	let res = client
		.request(req, None, false)
		.await?
		.json::<GetLinodesResponse>()
		.await?;

	tracing::info!("{} servers in account", res.data.len());

	let server_ids = res
		.data
		.into_iter()
		// Filter out servers younger than 12 hours
		.filter(|linode| {
			linode.created.and_utc().timestamp_millis() < ctx.ts() - util::duration::hours(12)
		})
		// Parse server ID from linode label
		.filter_map(|linode| {
			linode
				.label
				.get(util::env::namespace().len() + 1..)
				.map(util::uuid::parse)
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	// Select deleted servers that match the linode api call
	let servers = sql_fetch_all!(
		[ctx, ServerRow]
		"
		SELECT
			s.server_id,
			s.datacenter_id,
			s.pool_type,
			s.pool_type2,
			s.provider_server_id,
			s.vlan_ip,
			s.public_ip,
			s.cloud_destroy_ts
		FROM db_cluster.servers AS s
		JOIN db_cluster.datacenters AS d
		ON s.datacenter_id = d.datacenter_id
		WHERE
			server_id = ANY($1) AND
			cloud_destroy_ts IS NOT NULL AND

			($2 IS NULL OR s.server_id = ANY($2)) AND
			($3 IS NULL OR s.datacenter_id = ANY($3)) AND
			($4 IS NULL OR d.cluster_id = ANY($4)) AND
			($5 IS NULL OR s.pool_type2 = ANY($5::JSONB[])) AND
			($6 IS NULL OR s.public_ip = ANY($6))
		",
		server_ids,
		&filter.server_ids,
		&filter.datacenter_ids,
		&filter.cluster_ids,
		filter.pool_types
			.as_ref()
			.map(|x| x.iter()
				.map(serde_json::to_string)
				.collect::<Result<Vec<_>, _>>()
			).transpose()?,
		filter.public_ips
			.as_ref()
			.map(|x| x.iter()
				.cloned()
				.map(IpAddr::V4)
				.collect::<Vec<_>>()
			),
	)
	.await?
	.into_iter()
	.map(TryInto::try_into)
	.collect::<GlobalResult<Vec<_>>>()?;

	Ok(servers)
}
