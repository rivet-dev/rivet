use chirp_workflow::prelude::*;

#[derive(Debug)]
pub struct Input {
	pub cluster_id: Uuid,
	pub name_ids: Vec<String>,
}

#[derive(Debug)]
pub struct Output {
	pub datacenters: Vec<Datacenter>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct Datacenter {
	pub datacenter_id: Uuid,
	pub name_id: String,
}

#[operation]
pub async fn cluster_datacenter_resolve_for_name_id(
	ctx: &OperationCtx,
	input: &Input,
) -> GlobalResult<Output> {
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
		&input.cluster_id,
		&input.name_ids,
	)
	.await?;

	Ok(Output { datacenters })
}
