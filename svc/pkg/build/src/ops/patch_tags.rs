use futures_util::FutureExt;
use std::collections::HashMap;

use chirp_workflow::prelude::*;

#[derive(Debug)]
pub struct Input {
	pub build_id: Uuid,
	pub tags: HashMap<String, String>,
	pub exclusive_tags: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct Output {}

#[operation]
pub async fn patch_tags(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	// Validate tags don't overlap
	if let Some(exclusive_tags) = &input.exclusive_tags {
		ensure_with!(
			exclusive_tags.iter().all(|k| input.tags.contains_key(k)),
			BUILDS_TAGS_MISSING_EXCLUSIVE_KEY
		);
	}

	let tags_json = serde_json::to_value(&input.tags)?;

	rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		let ctx = ctx.clone();
		let build_id = input.build_id;
		let tags_json = tags_json.clone();
		let exclusive_tags = input.exclusive_tags.clone();

		async move {
			// Remove the exclusive tag from other builds
			if let Some(exclusive_tags) = &exclusive_tags {
				sql_execute!(
					[ctx, @tx tx]
					"
					UPDATE db_build.builds b
					SET tags = (
						SELECT jsonb_object_agg(key, value)
						FROM jsonb_each(tags)
						WHERE NOT (key = ANY($2::TEXT[]))
					)
					WHERE
						b.env_id = (
							SELECT env_id
							FROM db_build.builds
							WHERE build_id = $1
						)
						AND tags ?| $2::TEXT[]
					",
					&build_id,
					&exclusive_tags,
				)
				.await?;
			}

			// Add tag to current build
			sql_execute!(
				[ctx, @tx tx]
				"
				UPDATE db_build.builds
				SET tags = tags || $2
				WHERE build_id = $1
				",
				&build_id,
				&tags_json,
			)
			.await?;

			Ok(())
		}
		.boxed()
	})
	.await?;

	Ok(Output {})
}
