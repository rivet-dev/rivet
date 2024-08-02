use chirp_workflow::prelude::*;

#[derive(Debug)]
pub struct Input {
	pub name_ids: Vec<String>,
}

#[derive(Debug)]
pub struct Output {
	pub clusters: Vec<Cluster>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct Cluster {
	pub cluster_id: Uuid,
	pub name_id: String,
}

#[operation]
pub async fn cluster_resolve_for_name_id(
	ctx: &OperationCtx,
	input: &Input,
) -> GlobalResult<Output> {
	let clusters = sql_fetch_all!(
		[ctx, Cluster]
		"
		SELECT
			cluster_id,
			name_id
		FROM db_cluster.clusters
		WHERE name_id = ANY($1)
		",
		&input.name_ids,
	)
	.await?;

	Ok(Output { clusters })
}
