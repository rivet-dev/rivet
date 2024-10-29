use chirp_workflow::prelude::*;
use tokio::sync::OnceCell;

// The cluster ID will never change, so store it in memory.
static CLUSTER_ID_ONCE: OnceCell<Uuid> = OnceCell::const_new();

#[derive(Debug, Default)]
pub struct Input {}

#[derive(Debug)]
pub struct Output {
	pub cluster_id: Uuid,
}

#[operation]
pub async fn get_cluster_id(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	// Pick a cluster ID to insert if none exists. If this is specified in the config. fall back to
	// this.
	let default_cluster_id =
		if let Some(cluster_id) = ctx.config().server()?.rivet.default_cluster_id {
			cluster_id
		} else {
			Uuid::new_v4()
		};

	let cluster_id = CLUSTER_ID_ONCE
		.get_or_try_init(|| async {
			sql_fetch_one!(
				[ctx, (Uuid,)]
				"
				WITH new_row AS (
					INSERT INTO db_dynamic_config.config (id, cluster_id)
					VALUES (1, $1)
					ON CONFLICT (id) DO NOTHING
					RETURNING cluster_id
				)
				SELECT cluster_id 
				FROM new_row
				UNION ALL
				SELECT cluster_id 
				FROM db_dynamic_config.config
				WHERE NOT EXISTS (SELECT 1 FROM new_row)

				",
				default_cluster_id
			)
			.await
			.map(|x| x.0)
		})
		.await?;

	Ok(Output {
		cluster_id: *cluster_id,
	})
}
