use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Datacenter {
	datacenter_id: Uuid,
	name_id: String,
}

#[operation(name = "cluster-datacenter-resolve-for-name-id")]
pub async fn handle(
	ctx: OperationContext<cluster::datacenter_resolve_for_name_id::Request>,
) -> GlobalResult<cluster::datacenter_resolve_for_name_id::Response> {
	let cluster_id = unwrap_ref!(ctx.cluster_id).as_uuid();

	let datacenters = sql_fetch_all!(
		[ctx, Datacenter]
		"
		SELECT
			datacenter_id,
			name_id
		FROM db_cluster.datacenters
		WHERE
			cluster_id = $1 AND
			name_id = ANY($2)
		",
		&cluster_id,
		&ctx.name_ids,
	)
	.await?
	.into_iter()
	.map(
		|dc| cluster::datacenter_resolve_for_name_id::response::Datacenter {
			datacenter_id: Some(dc.datacenter_id.into()),
			name_id: dc.name_id,
		},
	)
	.collect::<Vec<_>>();

	Ok(cluster::datacenter_resolve_for_name_id::Response { datacenters })
}
