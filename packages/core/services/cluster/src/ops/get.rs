use chirp_workflow::prelude::*;

use crate::types::Cluster;

#[derive(Debug)]
pub struct Input {
	pub cluster_ids: Vec<Uuid>,
}

#[derive(Debug)]
pub struct Output {
	pub clusters: Vec<Cluster>,
}

#[operation]
pub async fn cluster_get(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let clusters = sql_fetch_all!(
		[ctx, Cluster]
		"
		SELECT
			cluster_id,
			name_id,
			owner_team_id,
			create_ts
		FROM db_cluster.clusters
		WHERE cluster_id = ANY($1)
		",
		&input.cluster_ids,
	)
	.await?;

	Ok(Output { clusters })
}
