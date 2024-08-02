use chirp_workflow::prelude::*;

#[derive(Debug)]
pub struct Input {}

#[derive(Debug)]
pub struct Output {
	pub cluster_ids: Vec<Uuid>,
}

#[operation]
pub async fn cluster_list(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let cluster_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		SELECT cluster_id
		FROM db_cluster.clusters
		",
	)
	.await?
	.into_iter()
	.map(|(cluster_id,)| cluster_id)
	.collect::<Vec<_>>();

	Ok(Output { cluster_ids })
}
