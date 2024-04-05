use chirp_worker::prelude::*;
use proto::backend::pkg::*;

#[worker(name = "cluster-create")]
async fn worker(ctx: &OperationContext<cluster::msg::create::Message>) -> GlobalResult<()> {
	let cluster_id = unwrap_ref!(ctx.cluster_id).as_uuid();
	let owner_team_id = ctx.owner_team_id.map(|id| id.as_uuid());

	sql_execute!(
		[ctx]
		"
		INSERT INTO db_cluster.clusters (
			cluster_id,
			name_id,
			owner_team_id,
			create_ts
		)
		VALUES ($1, $2, $3, $4)
		",
		cluster_id,
		&ctx.name_id,
		owner_team_id,
		util::timestamp::now(),
	)
	.await?;

	msg!([ctx] cluster::msg::create_complete(cluster_id) {
		cluster_id: ctx.cluster_id
	})
	.await?;

	Ok(())
}
