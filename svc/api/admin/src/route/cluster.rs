use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};

use proto::backend;
use rivet_api::models;
use rivet_convert::ApiInto;
use rivet_operation::prelude::*;
use serde::Deserialize;

use crate::auth::Auth;

// MARK: GET /cluster/server_ips
#[derive(Debug, Clone, Deserialize)]
pub struct ServerIpsQuery {
	server_id: Option<Uuid>,
	pool: Option<models::AdminPoolType>,
}

pub async fn server_ips(
	ctx: Ctx<Auth>,
	watch_index: WatchIndexQuery,
	query: ServerIpsQuery,
) -> GlobalResult<models::AdminClusterGetServerIpsResponse> {
	if query.server_id.is_none() && query.pool.is_none() {
		bail_with!(
			API_BAD_QUERY,
			error = "expected one of: `server_id`, `pool`"
		);
	}

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

			let cluster_id = util::env::default_cluster_id();
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
		// Handled earlier
		(None, None) => unreachable!(),
	};

	Ok(models::AdminClusterGetServerIpsResponse { ips })
}
