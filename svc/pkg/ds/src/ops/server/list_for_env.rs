use std::collections::HashMap;

use chirp_workflow::prelude::*;

#[derive(Debug, Default)]
pub struct Input {
	pub env_id: Uuid,
	pub tags: HashMap<String, String>,
	pub include_destroyed: bool,
	pub cursor: Option<Uuid>,
}

#[derive(Debug)]
pub struct Output {
	pub server_ids: Vec<Uuid>,
}

#[operation]
pub async fn list_for_env(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let server_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		WITH
			after_server AS (
				SELECT create_ts, server_id
				FROM db_ds.servers
				WHERE server_id = $4
			)
		SELECT server_id
		FROM db_ds.servers
		WHERE
			env_id = $1 AND 
			tags @> $2 AND 
			($3 OR destroy_ts IS NOT NULL) AND 
			(
				$4 IS NULL OR
				(create_ts, server_id) < (SELECT create_ts, server_id FROM after_server)
			)
		ORDER BY create_ts DESC, server_id DESC
		LIMIT 64
		",
		input.env_id,
		serde_json::to_value(&input.tags)?,
		input.include_destroyed,
		input.cursor,
	)
	.await?
	.into_iter()
	.map(|(id,)| id)
	.collect::<Vec<_>>();

	Ok(Output { server_ids })
}
