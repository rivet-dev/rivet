use futures_util::FutureExt;
use std::collections::HashMap;

use chirp_workflow::prelude::*;

#[derive(Debug)]
pub struct Input {
	pub build_id: Uuid,
	pub tags: HashMap<String, Option<String>>,
	pub exclusive_tags: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct Output {}

#[operation]
pub async fn patch_tags(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let exclusive_tags_json = if let Some(exclusive_tags) = &input.exclusive_tags {
		// Validate tags don't overlap
		let exclusive_tags_map = exclusive_tags
			.iter()
			.map(|tag| {
				let value = unwrap_with!(
					unwrap_with!(input.tags.get(tag), BUILD_TAGS_MISSING_EXCLUSIVE_KEY),
					BUILD_TAGS_NULL_EXCLUSIVE_KEY
				);

				Ok((tag.clone(), value.clone()))
			})
			.collect::<GlobalResult<HashMap<_, _>>>()?;

		// TODO: This can just use a raw value instead of `util::serde::Raw` but only after sqlx is updated
		// past 0.8. Otherwise `RawValue` doesn't implement `sqlx::Encode`
		Some(util::serde::Raw::new(&exclusive_tags_map)?)
	} else {
		None
	};

	let tags_json = util::serde::Raw::new(&input.tags)?;

	rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		let ctx = ctx.clone();
		let build_id = input.build_id;
		let tags_json = tags_json.clone();
		let exclusive_tags_json = exclusive_tags_json.clone();

		async move {
			// Remove the exclusive tag from other builds of the same owner (same game id OR same env id)
			if let Some(exclusive_tags_json) = exclusive_tags_json {
				sql_execute!(
					[ctx, @tx tx]
					"
					WITH
						build_data AS (
							SELECT game_id, env_id
							FROM db_build.builds
							WHERE build_id = $1
						),
						split_exclusive_tags AS (
							SELECT key, value
							FROM jsonb_each($2)
						),
						filter_tags AS (
							-- Combine the now filtered kv pairs back into an object
							SELECT build_id, jsonb_object_agg(key, value) AS tags
							FROM db_build.builds AS b
							-- Split out each kv pair into a row
							JOIN LATERAL jsonb_each(b.tags) AS t(key, value)
							-- Filter out kv pairs that match any pair from the exclusive tags
							ON NOT jsonb_build_object(key, value) <@ $2
							WHERE
								-- Check that game id and env id match 
								(
									(
										b.game_id IS NULL AND
										(SELECT game_id FROM build_data) IS NULL
									) OR
									b.game_id = (SELECT game_id FROM build_data)
								) AND
								(
									(
										b.env_id IS NULL AND
										(SELECT env_id FROM build_data) IS NULL
									) OR
									b.env_id = (SELECT env_id FROM build_data)
								) AND
								-- Check that build has any of the exclusive tags to begin with (pre-filtering)
								(
									SELECT EXISTS (
										SELECT 1
										FROM split_exclusive_tags
										WHERE b.tags @> jsonb_build_object(key, value)
										LIMIT 1
									)
								)
							GROUP BY build_id
						)
					UPDATE db_build.builds AS b
					SET tags = f2.tags
					FROM (
						-- Fetch filtered tags or default to an empty object (there won't be a row from
						-- filter_tags if all tags were removed)
						SELECT b.build_id, COALESCE(f.tags, '{}'::JSONB) AS tags
						FROM db_build.builds AS b
						LEFT JOIN filter_tags AS f
						ON b.build_id = f.build_id
						-- Same as the above where clause but we have to do it again because of the LEFT JOIN
						WHERE
							(
								(
									b.game_id IS NULL AND
									(SELECT game_id FROM build_data) IS NULL
								) OR
								b.game_id = (SELECT game_id FROM build_data)
							) AND
							(
								(
									b.env_id IS NULL AND
									(SELECT env_id FROM build_data) IS NULL
								) OR 
								b.env_id = (SELECT env_id FROM build_data)
							) AND
							-- Check that build has any of the exclusive tags
							(
								SELECT EXISTS (
									SELECT 1
									FROM split_exclusive_tags
									WHERE b.tags @> jsonb_build_object(key, value)
									LIMIT 1
								)
							)
					) AS f2
					WHERE b.build_id = f2.build_id
					",
					build_id,
					exclusive_tags_json,
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
				build_id,
				tags_json,
			)
			.await?;

			Ok(())
		}
		.boxed()
	})
	.await?;

	Ok(Output {})
}
