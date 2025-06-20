use std::{collections::HashMap, convert::TryInto};

use chirp_workflow::prelude::*;

use super::get::BuildRow;
use crate::types;

#[derive(Debug)]
pub struct Input {
	pub env_id: Uuid,
	pub tags: HashMap<String, String>,
	pub bypass_cache: bool,
}

#[derive(Debug)]
pub struct Output {
	pub builds: Vec<types::Build>,
}

#[operation]
pub async fn build_resolve_for_tags(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let tags_str = unwrap!(cjson::to_string(&input.tags));

	let builds = if input.bypass_cache {
		get_builds(&ctx, input.env_id, &tags_str).await?
	} else {
		unwrap!(
			ctx.cache()
				.ttl(util::duration::seconds(15))
				.fetch_one_json(
					"build",
					(input.env_id, tags_str.as_str()),
					{
						let ctx = ctx.clone();
						let tags_str = tags_str.clone();
						move |mut cache, key| {
							let ctx = ctx.clone();
							let tags_str = tags_str.clone();
							async move {
								let builds = get_builds(&ctx, input.env_id, &tags_str).await?;

								cache.resolve(&key, builds);

								Ok(cache)
							}
						}
					}
				)
				.await?
		)
	};

	Ok(Output { builds })
}

async fn get_builds(
	ctx: &OperationCtx,
	env_id: Uuid,
	tags_str: &str,
) -> GlobalResult<Vec<types::Build>> {
	sql_fetch_all!(
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
		env_id,
		tags_str,
	)
	.await?
	.into_iter()
	.map(TryInto::try_into)
	.collect::<GlobalResult<Vec<_>>>()
}
