use std::{collections::HashMap, convert::TryInto};

use chirp_workflow::prelude::*;

use super::get::BuildRow;
use crate::types;

#[derive(Debug)]
pub struct Input {
	pub env_id: Uuid,
	pub tags: HashMap<String, String>,
}

#[derive(Debug)]
pub struct Output {
	pub builds: Vec<types::Build>,
}

#[operation]
pub async fn get(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let builds = sql_fetch_all!(
		[ctx, BuildRow]
		"
		SELECT
			build_id,
			game_id,
			env_id,
			upload_id,
			display_name,
			image_tag,
			create_ts,
			kind,
			compression,
			tags
		FROM db_build.builds
		WHERE env_id = $1 AND tags @> $2
		",
		input.env_id,
		serde_json::to_string(&input.tags)?,
	)
	.await?
	.into_iter()
	.map(|build| build.try_into())
	.collect::<GlobalResult<Vec<_>>>()?;

	Ok(Output { builds })
}
