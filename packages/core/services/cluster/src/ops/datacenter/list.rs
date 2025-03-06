use std::collections::HashMap;

use chirp_workflow::prelude::*;

#[derive(Debug)]
pub struct Input {
	pub cluster_ids: Vec<Uuid>,
}

#[derive(Debug)]
pub struct Output {
	pub clusters: Vec<Cluster>,
}

#[derive(Debug)]
pub struct Cluster {
	pub cluster_id: Uuid,
	pub datacenter_ids: Vec<Uuid>,
}

#[operation]
pub async fn cluster_datacenter_list(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let rows = sql_fetch_all!(
		[ctx, (Uuid, Uuid)]
		"
		SELECT
			cluster_id,
			datacenter_id
		FROM db_cluster.datacenters
		WHERE cluster_id = ANY($1)
		",
		&input.cluster_ids,
	)
	.await?;

	// Fill in empty clusters
	let mut dcs_by_cluster_id = input
		.cluster_ids
		.iter()
		.map(|cluster_id| (*cluster_id, Vec::new()))
		.collect::<HashMap<_, Vec<Uuid>>>();

	for (cluster_id, datacenter_id) in rows {
		dcs_by_cluster_id
			.entry(cluster_id)
			.or_default()
			.push(datacenter_id);
	}

	Ok(Output {
		clusters: dcs_by_cluster_id
			.into_iter()
			.map(|(cluster_id, datacenter_ids)| Cluster {
				cluster_id,
				datacenter_ids,
			})
			.collect::<Vec<_>>(),
	})
}
