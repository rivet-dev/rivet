use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Cluster {
	cluster_id: Uuid,
	name_id: String,
}

#[operation(name = "cluster-resolve-for-name-id")]
pub async fn handle(
	ctx: OperationContext<cluster::resolve_for_name_id::Request>,
) -> GlobalResult<cluster::resolve_for_name_id::Response> {
	let clusters = sql_fetch_all!(
		[ctx, Cluster]
		"
		SELECT
			cluster_id,
			name_id
		FROM db_cluster.clusters
		WHERE
			name_id = ANY($1)
		",
		&ctx.name_ids,
	)
	.await?
	.into_iter()
	.map(|dc| cluster::resolve_for_name_id::response::Cluster {
		cluster_id: Some(dc.cluster_id.into()),
		name_id: dc.name_id,
	})
	.collect::<Vec<_>>();

	Ok(cluster::resolve_for_name_id::Response { clusters })
}
