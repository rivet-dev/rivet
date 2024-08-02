use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_api::models;
use rivet_convert::ApiTryInto;
use rivet_operation::prelude::*;
use serde_json::json;

use crate::auth::Auth;

pub mod datacenters;
pub mod servers;

// MARK: GET /clusters
pub async fn list(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::AdminClustersListClustersResponse> {
	let cluster_ids = ctx.op(cluster::ops::list::Input {}).await?.cluster_ids;

	let clusters = ctx
		.op(cluster::ops::get::Input { cluster_ids })
		.await?
		.clusters
		.into_iter()
		.map(ApiTryInto::<models::AdminClustersCluster>::api_try_into)
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::AdminClustersListClustersResponse { clusters })
}

// MARK: POST /cluster
pub async fn create(
	ctx: Ctx<Auth>,
	body: models::AdminClustersCreateClusterRequest,
) -> GlobalResult<models::AdminClustersCreateClusterResponse> {
	let cluster_id = Uuid::new_v4();

	let tags = json!({
		"cluster_id": cluster_id,
	});
	let mut sub = ctx
		.subscribe::<cluster::workflows::cluster::CreateComplete>(&tags)
		.await?;

	ctx.dispatch_tagged_workflow(
		&tags,
		cluster::workflows::cluster::Input {
			cluster_id,
			owner_team_id: body.owner_team_id,
			name_id: body.name_id,
		},
	)
	.await?;

	sub.next().await?;

	Ok(models::AdminClustersCreateClusterResponse { cluster_id })
}
