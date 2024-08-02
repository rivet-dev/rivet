use std::{net::Ipv4Addr, str::FromStr};

use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_api::models;
use rivet_convert::ApiInto;
use rivet_operation::prelude::*;
use serde::Deserialize;
use serde_json::{json, Value};

use crate::auth::Auth;

#[derive(Debug, Clone, Deserialize)]
pub struct ServerFilterQuery {
	server_id: Option<Uuid>,
	datacenter: Option<String>,
	pool: Option<models::AdminClustersPoolType>,
	public_ip: Option<String>,
}

impl ServerFilterQuery {
	async fn convert(
		&self,
		ctx: &Ctx<Auth>,
		cluster_id: Uuid,
	) -> GlobalResult<cluster::types::Filter> {
		Ok(cluster::types::Filter {
			cluster_ids: Some(vec![cluster_id]),
			server_ids: self.server_id.map(|x| vec![x]),
			datacenter_ids: if let Some(name_id) = &self.datacenter {
				// Look up datacenter
				let resolve_res = ctx
					.op(cluster::ops::datacenter::resolve_for_name_id::Input {
						cluster_id,
						name_ids: vec![name_id.clone()],
					})
					.await?;
				let datacenter = unwrap!(resolve_res.datacenters.first(), "datacenter not found");

				// Filter datacenters
				Some(vec![datacenter.datacenter_id])
			} else {
				None
			},
			pool_types: self.pool.map(ApiInto::api_into).map(|x| vec![x]),
			public_ips: self
				.public_ip
				.as_deref()
				.map(Ipv4Addr::from_str)
				.transpose()?
				.map(|x| vec![x]),
		})
	}
}

// MARK: GET /clusters/{cluster_id}/servers
pub async fn list(
	ctx: Ctx<Auth>,
	cluster_id: Uuid,
	_watch_index: WatchIndexQuery,
	query: ServerFilterQuery,
) -> GlobalResult<models::AdminClustersListServersResponse> {
	let servers_res = ctx
		.op(cluster::ops::server::list::Input {
			filter: query.convert(&ctx, cluster_id).await?,
			include_destroyed: false,
		})
		.await?;

	let servers = servers_res
		.servers
		.iter()
		.map(|server| {
			GlobalResult::Ok(models::AdminClustersServer {
				server_id: server.server_id,
				public_ip: server.public_ip.map(|ip| ip.to_string()),
			})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::AdminClustersListServersResponse { servers })
}

// MARK: GET /clusters/{cluster_id}/servers/taint
pub async fn taint(
	ctx: Ctx<Auth>,
	cluster_id: Uuid,
	_body: serde_json::Value,
	query: ServerFilterQuery,
) -> GlobalResult<Value> {
	ctx.op(cluster::ops::server::taint_with_filter::Input {
		filter: query.convert(&ctx, cluster_id).await?,
	})
	.await?;

	Ok(json!({}))
}

// MARK: GET /clusters/{cluster_id}/servers/destroy
pub async fn destroy(
	ctx: Ctx<Auth>,
	cluster_id: Uuid,
	_body: serde_json::Value,
	query: ServerFilterQuery,
) -> GlobalResult<Value> {
	ctx.op(cluster::ops::server::destroy_with_filter::Input {
		filter: query.convert(&ctx, cluster_id).await?,
	})
	.await?;

	Ok(json!({}))
}
