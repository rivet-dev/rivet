use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};

use proto::backend;
use rivet_api::models;
use rivet_convert::{ApiInto, ApiTryFrom};
use rivet_operation::prelude::{proto::backend::pkg::cluster, *};
use serde::Deserialize;

use crate::auth::Auth;

pub mod datacenters;

// MARK: GET /cluster/server_ips
#[derive(Debug, Clone, Deserialize)]
pub struct ServerIpsQuery {
	server_id: Option<Uuid>,
	pool: Option<models::AdminPoolType>,
}

pub async fn server_ips(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	query: ServerIpsQuery,
) -> GlobalResult<models::AdminClustersGetServerIpsResponse> {
	let ips = match (query.server_id, query.pool) {
		(Some(server_id), _) => {
			let servers_res = op!([ctx] cluster_server_get {
				server_ids: vec![server_id.into()],
			})
			.await?;
			let public_ip = servers_res
				.servers
				.first()
				.and_then(|server| server.public_ip.clone());

			public_ip.into_iter().collect::<Vec<_>>()
		}
		(_, Some(pool)) => {
			let pool_type = Some(ApiInto::<backend::cluster::PoolType>::api_into(pool));

			let cluster_id = util_cluster::default_cluster_id();
			let server_list_res = op!([ctx] cluster_server_list {
				cluster_ids: vec![cluster_id.into()],
			})
			.await?;
			let cluster = unwrap!(server_list_res.clusters.first());

			let servers_res = op!([ctx] cluster_server_get {
				server_ids: cluster.server_ids.clone(),
			})
			.await?;

			servers_res
				.servers
				.iter()
				.filter(|server| {
					backend::cluster::PoolType::from_i32(server.pool_type) == pool_type
				})
				.filter_map(|server| server.public_ip.clone())
				.collect::<Vec<_>>()
		}
		(None, None) => {
			bail_with!(
				API_BAD_QUERY,
				error = "expected one of: `server_id`, `pool`"
			);
		}
	};

	Ok(models::AdminClustersGetServerIpsResponse { ips })
}

// MARK: GET /clusters
pub async fn list(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::AdminClustersListResponse> {
	let cluster_ids = op!([ctx] cluster_list {}).await?.cluster_ids;

	let clusters = op!([ctx] cluster_get {
		cluster_ids: cluster_ids.into_iter().map(Into::into).collect()
	})
	.await?
	.clusters
	.into_iter()
	.map(models::AdminCluster::api_try_from)
	.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::AdminClustersListResponse { clusters })
}

// MARK: POST /cluster
pub async fn create(
	ctx: Ctx<Auth>,
	body: models::AdminClustersCreateRequest,
) -> GlobalResult<models::AdminClustersCreateResponse> {
	let cluster_id = Uuid::new_v4();

	msg!([ctx] cluster::msg::create(cluster_id) -> cluster::msg::create_complete {
		cluster_id: Some(cluster_id.into()),
		owner_team_id: body.owner_team_id.map(Into::into),
		name_id: body.name_id,
	})
	.await?;

	Ok(models::AdminClustersCreateResponse { cluster_id })
}
