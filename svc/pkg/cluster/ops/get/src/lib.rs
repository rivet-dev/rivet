use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Cluster {
	cluster_id: Uuid,
	name_id: String,
	owner_team_id: Option<Uuid>,
	create_ts: i64,
}

impl From<Cluster> for backend::cluster::Cluster {
	fn from(value: Cluster) -> Self {
		backend::cluster::Cluster {
			cluster_id: Some(value.cluster_id.into()),
			name_id: value.name_id,
			owner_team_id: value.owner_team_id.map(Into::into),
			create_ts: value.create_ts,
		}
	}
}

#[operation(name = "cluster-get")]
pub async fn handle(
	ctx: OperationContext<cluster::get::Request>,
) -> GlobalResult<cluster::get::Response> {
	let crdb = ctx.crdb().await?;
	let cluster_ids = ctx
		.cluster_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let clusters = sql_fetch_all!(
		[ctx, Cluster, &crdb]
		"
		SELECT
			cluster_id,
			name_id,
			owner_team_id,
			create_ts
		FROM db_cluster.clusters
		WHERE cluster_id = ANY($1)
		",
		cluster_ids
	)
	.await?
	.into_iter()
	.map(Into::into)
	.collect::<Vec<_>>();

	Ok(cluster::get::Response { clusters })
}
