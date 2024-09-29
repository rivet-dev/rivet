use chirp_workflow::prelude::*;
use serde_json::Value;
use std::convert::TryInto;

use crate::types;

#[derive(Debug)]
pub struct Input {
	pub build_ids: Vec<Uuid>,
}

#[derive(Debug)]
pub struct Output {
	pub builds: Vec<types::Build>,
}

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
	tags: sqlx::types::Json<Box<serde_json::value::RawValue>>,
}

impl TryInto<types::Build> for BuildRow {
	type Error = GlobalError;

	fn try_into(self) -> GlobalResult<types::Build> {
		Ok(types::Build {
			build_id: self.build_id,
			game_id: self.game_id,
			env_id: self.env_id,
			upload_id: self.upload_id,
			display_name: self.display_name,
			image_tag: self.image_tag,
			create_ts: self.create_ts,
			kind: unwrap!(types::BuildKind::from_repr(self.kind.try_into()?)),
			compression: unwrap!(types::BuildCompression::from_repr(
				self.compression.try_into()?
			)),
			tags: serde_json::from_str(self.tags.0.get())?,
		})
	}
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
		FROM
			db_build.builds
		WHERE
			build_id = ANY($1)
		",
		&input.build_ids,
	)
	.await?
	.into_iter()
	.map(|build| build.try_into())
	.collect::<GlobalResult<Vec<_>>>()?;

	Ok(Output { builds })
}
