use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_api::models;
use rivet_convert::ApiFrom;
use rivet_operation::prelude::{
	proto::backend::{self, pkg::*},
	*,
};
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
	async fn convert_to_proto(
		&self,
		ctx: &Ctx<Auth>,
		cluster_id: Uuid,
	) -> GlobalResult<backend::cluster::ServerFilter> {
		let mut filter = backend::cluster::ServerFilter::default();

		filter.filter_cluster_ids = true;
		filter.cluster_ids = vec![cluster_id.into()];

		if let Some(server_id) = self.server_id {
			filter.filter_server_ids = true;
			filter.server_ids = vec![server_id.into()];
		}

		if let Some(name_id) = &self.datacenter {
			// Look up datacenter
			let resolve_res = op!([ctx] cluster_datacenter_resolve_for_name_id {
				cluster_id: Some(cluster_id.into()),
				name_ids: vec![name_id.clone()],
			})
			.await?;
			let datacenter = unwrap!(resolve_res.datacenters.first(), "datacenter not found");

			// Filter datacenters
			filter.filter_datacenter_ids = true;
			filter.datacenter_ids = vec![unwrap!(datacenter.datacenter_id)];
		}

		if let Some(pool) = self.pool {
			let pool_type = <backend::cluster::PoolType as ApiFrom<_>>::api_from(pool);

			filter.filter_pool_types = true;
			filter.pool_types = vec![pool_type as i32];
		}

		if let Some(public_ip) = &self.public_ip {
			filter.filter_public_ips = true;
			filter.public_ips = vec![public_ip.clone()];
		}

		Ok(filter)
	}
}

// MARK: GET /clusters/{cluster_id}/servers
pub async fn list(
	ctx: Ctx<Auth>,
	cluster_id: Uuid,
	_watch_index: WatchIndexQuery,
	query: ServerFilterQuery,
) -> GlobalResult<models::AdminClustersListServersResponse> {
	let filter = query.convert_to_proto(&ctx, cluster_id).await?;

	let servers_res = op!([ctx] cluster_server_list {
		filter: Some(filter)
	})
	.await?;

	let servers = servers_res
		.servers
		.iter()
		.map(|x| {
			GlobalResult::Ok(models::AdminClustersServer {
				server_id: unwrap!(x.server_id).as_uuid(),
				public_ip: x.public_ip.clone().unwrap_or_default(),
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
	let filter = query.convert_to_proto(&ctx, cluster_id).await?;

	let request_id = Uuid::new_v4();
	msg!([ctx] cluster::msg::server_taint(request_id) {
		filter: Some(filter),
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
	let filter = query.convert_to_proto(&ctx, cluster_id).await?;

	op!([ctx] cluster_server_destroy_with_filter {
		filter: Some(filter)
	})
	.await?;

	Ok(json!({}))
}
