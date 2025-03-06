use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;
use serde_json::Value;

#[derive(sqlx::FromRow)]
struct BuildRow {
	build_id: Uuid,
	game_id: Option<Uuid>,
	env_id: Option<Uuid>,
	upload_id: Uuid,
	display_name: String,
	image_tag: String,
	create_ts: i64,
	kind: i64,
	compression: i64,
	tags: Value,
}

#[operation(name = "build-get")]
async fn handle(ctx: OperationContext<build::get::Request>) -> GlobalResult<build::get::Response> {
	let build_ids = ctx
		.build_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

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
		FROM
			db_build.builds
		WHERE
			build_id = ANY($1)
		",
		build_ids,
	)
	.await?
	.into_iter()
	.map(|build| {
		let Value::Object(tags) = build.tags else {
			bail!("tags not a map");
		};

		Ok(backend::build::Build {
			build_id: Some(build.build_id.into()),
			game_id: build.game_id.map(|x| x.into()),
			env_id: build.env_id.map(|x| x.into()),
			upload_id: Some(build.upload_id.into()),
			display_name: build.display_name.clone(),
			image_tag: build.image_tag.clone(),
			create_ts: build.create_ts,
			kind: build.kind as i32,
			compression: build.compression as i32,
			tags: serde_json::from_value(
				tags.into_iter()
					.filter(|(_, v)| !matches!(v, Value::Null))
					.collect(),
			)?,
		})
	})
	.collect::<GlobalResult<Vec<_>>>()?;

	Ok(build::get::Response { builds })
}
