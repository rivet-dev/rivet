use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_api::models as new_models;
use rivet_cloud_server::models;
use rivet_convert::ApiTryInto;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: GET /games/{}/builds
pub async fn get_builds(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::ListGameBuildsResponse> {
	ctx.auth().check_game_read(ctx.op_ctx(), game_id).await?;

	let list_res = op!([ctx] build_list_for_game {
		game_id: Some(game_id.into()),
	})
	.await?;

	let builds_res = op!([ctx] build_get {
		build_ids: list_res.build_ids.clone(),
	})
	.await?;

	let uploads_res = op!([ctx] upload_get {
		upload_ids: builds_res
			.builds
			.iter()
			.flat_map(|build| build.upload_id)
			.collect::<Vec<_>>(),
	})
	.await?;

	// Convert the build data structures
	let mut builds = builds_res
		.builds
		.iter()
		.map(|build| {
			let upload = uploads_res
				.uploads
				.iter()
				.find(|u| u.upload_id == build.upload_id);
			if upload.is_none() {
				tracing::warn!("unable to find upload for build");
			}

			GlobalResult::Ok(models::BuildSummary {
				build_id: internal_unwrap!(build.build_id).as_uuid().to_string(),
				upload_id: internal_unwrap!(build.upload_id).as_uuid().to_string(),
				display_name: build.display_name.clone(),
				create_ts: util::timestamp::to_chrono(build.create_ts)?,
				content_length: upload.map_or(0, |upload| upload.content_length) as i64,
				complete: upload.map_or(true, |upload| upload.complete_ts.is_some()),
			})
		})
		.collect::<Result<Vec<_>, _>>()?;

	// Sort by date desc
	builds.sort_by_key(|b| b.create_ts);
	builds.reverse();

	Ok(models::ListGameBuildsResponse { builds })
}

// MARK: POST /games/{}/versions/builds
pub async fn create_build(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	body: new_models::CloudGamesCreateGameBuildRequest,
) -> GlobalResult<new_models::CloudGamesCreateGameBuildResponse> {
	ctx.auth().check_game_write(ctx.op_ctx(), game_id).await?;

	// TODO: Read and validate image file

	let create_res = op!([ctx] build_create {
		game_id: Some(game_id.into()),
		display_name: body.display_name,
		image_tag: Some(body.image_tag),
		image_file: Some((*body.image_file).try_into()?),
		..Default::default()
	})
	.await?;

	Ok(new_models::CloudGamesCreateGameBuildResponse {
		build_id: internal_unwrap!(create_res.build_id).as_uuid(),
		upload_id: internal_unwrap!(create_res.upload_id).as_uuid(),
		// Deprecated
		image_presigned_request: Box::new(
			internal_unwrap_owned!(create_res.image_presigned_requests.first())
				.clone()
				.try_into()?,
		),
		image_presigned_requests: create_res
			.image_presigned_requests
			.iter()
			.cloned()
			.map(ApiTryInto::try_into)
			.collect::<GlobalResult<Vec<_>>>()?,
	})
}
