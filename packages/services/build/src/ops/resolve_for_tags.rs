use std::{collections::HashMap, convert::TryInto};

use chirp_workflow::prelude::*;

use super::get::BuildRow;
use crate::types;

#[derive(Debug)]
pub struct Input {
	pub game_id: Option<Uuid>,
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
		WHERE
			($2 IS NULL OR game_id = $2) AND
			tags @> $1
		",
		input.game_id,
		serde_json::to_string(&input.tags)?,
	)
	.await?
	.into_iter()
	.map(|build| build.try_into())
	.collect::<GlobalResult<Vec<_>>>()?;

	Ok(Output { builds })
}
