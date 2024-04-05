use std::collections::HashMap;

use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Datacenter {
	cluster_id: Uuid,
	datacenter_id: Uuid,
}

#[operation(name = "cluster-datacenter-list")]
pub async fn handle(
	ctx: OperationContext<cluster::datacenter_list::Request>,
) -> GlobalResult<cluster::datacenter_list::Response> {
	let cluster_ids = ctx
		.cluster_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let datacenters = sql_fetch_all!(
		[ctx, Datacenter]
		"
		SELECT
			cluster_id,
			datacenter_id
		FROM db_cluster.datacenters
		WHERE cluster_id = ANY($1)
		",
		&cluster_ids
	)
	.await?;

	// Fill in empty clusters
	let mut dcs_by_cluster_id = cluster_ids
		.iter()
		.map(|cluster_id| (*cluster_id, Vec::new()))
		.collect::<HashMap<_, Vec<Uuid>>>();

	for dc in datacenters {
		dcs_by_cluster_id
			.entry(dc.cluster_id)
			.or_default()
			.push(dc.datacenter_id);
	}

	Ok(cluster::datacenter_list::Response {
		clusters: dcs_by_cluster_id
			.into_iter()
			.map(
				|(cluster_id, datacenter_ids)| cluster::datacenter_list::response::Cluster {
					cluster_id: Some(cluster_id.into()),
					datacenter_ids: datacenter_ids
						.into_iter()
						.map(Into::into)
						.collect::<Vec<_>>(),
				},
			)
			.collect::<Vec<_>>(),
	})
}
