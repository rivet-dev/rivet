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

#[operation(name = "cluster-list")]
pub async fn handle(
	ctx: OperationContext<cluster::list::Request>,
) -> GlobalResult<cluster::list::Response> {
	let crdb = ctx.crdb().await?;

	let cluster_ids = sql_fetch_all!(
		[ctx, Cluster, &crdb]
		"
		SELECT
			cluster_id,
			name_id,
			owner_team_id,
			create_ts
		FROM db_cluster.clusters
		",
	)
	.await?
	.into_iter()
	.map(|cluster| cluster.cluster_id.into())
	.collect::<Vec<_>>();

	Ok(cluster::list::Response { cluster_ids })
}
