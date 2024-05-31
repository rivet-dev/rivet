use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_api::models;
use rivet_convert::ApiTryFrom;
use rivet_operation::prelude::{proto::backend::pkg::cluster, *};

use crate::auth::Auth;

pub mod datacenters;
pub mod servers;

// MARK: GET /clusters
pub async fn list(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::AdminClustersListClustersResponse> {
	let cluster_ids = op!([ctx] cluster_list {}).await?.cluster_ids;

	let clusters = op!([ctx] cluster_get {
		cluster_ids: cluster_ids.into_iter().map(Into::into).collect()
	})
	.await?
	.clusters
	.into_iter()
	.map(models::AdminClustersCluster::api_try_from)
	.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::AdminClustersListClustersResponse { clusters })
}

// MARK: POST /cluster
pub async fn create(
	ctx: Ctx<Auth>,
	body: models::AdminClustersCreateClusterRequest,
) -> GlobalResult<models::AdminClustersCreateClusterResponse> {
	let cluster_id = Uuid::new_v4();

	msg!([ctx] cluster::msg::create(cluster_id) -> cluster::msg::create_complete {
		cluster_id: Some(cluster_id.into()),
		owner_team_id: body.owner_team_id.map(Into::into),
		name_id: body.name_id,
	})
	.await?;

	Ok(models::AdminClustersCreateClusterResponse { cluster_id })
}
