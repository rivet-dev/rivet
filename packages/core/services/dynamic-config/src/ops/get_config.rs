use chirp_workflow::prelude::*;
use tokio::sync::OnceCell;

// The instance ID will never change, so store it in memory.
static INSTANCE_ID_ONCE: OnceCell<Uuid> = OnceCell::const_new();

#[derive(Debug, Default)]
pub struct Input {}

#[derive(Debug)]
pub struct Output {
	pub instance_id: Uuid,
}

#[operation]
pub async fn get_config(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	// IMPORTANT: This is not the same as the cluster ID from the `cluster` package. This is used
	// for uniquely identifying the entire Rivet cluster.

	// Pick an instance ID to insert if none exists. If this is specified in the config. fall back to
	// this.
	let default_instance_id = if let Some(instance_id) = ctx.config().server()?.rivet.instance_id {
		instance_id
	} else {
		Uuid::new_v4()
	};

	let instance_id = INSTANCE_ID_ONCE
		.get_or_try_init(|| async {
			sql_fetch_one!(
				[ctx, (Uuid,)]
				"
				WITH new_row AS (
					INSERT INTO db_dynamic_config.config (id, rivet_instance_id)
					VALUES (1, $1)
					ON CONFLICT (id) DO NOTHING
					RETURNING rivet_instance_id
				)
				SELECT rivet_instance_id 
				FROM new_row
				UNION ALL
				SELECT rivet_instance_id 
				FROM db_dynamic_config.config
				WHERE NOT EXISTS (SELECT 1 FROM new_row)

				",
				default_instance_id
			)
			.await
			.map(|x| x.0)
		})
		.await?;

	Ok(Output {
		instance_id: *instance_id,
	})
}
