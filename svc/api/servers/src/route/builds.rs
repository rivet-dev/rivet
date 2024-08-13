use std::collections::HashMap;

use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::backend;
use rivet_api::models;
use rivet_claims::ClaimsDecode;
use rivet_convert::ApiTryInto;
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::Auth;

// MARK: GET /games/{}/builds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetQuery {
	tags: Option<String>,
}

pub async fn get_builds(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	_watch_index: WatchIndexQuery,
	query: GetQuery,
) -> GlobalResult<models::GamesServersListBuildsResponse> {
	ctx.auth().check_game(ctx.op_ctx(), game_id, true).await?;

	let list_res = op!([ctx] build_list_for_game {
		game_id: Some(game_id.into()),
		tags: query.tags.as_deref().map_or(Ok(HashMap::new()), serde_json::from_str)?,
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

			GlobalResult::Ok((
				build.create_ts,
				models::CloudBuildSummary {
					build_id: unwrap_ref!(build.build_id).as_uuid(),
					upload_id: unwrap_ref!(build.upload_id).as_uuid(),
					display_name: build.display_name.clone(),
					create_ts: util::timestamp::to_string(build.create_ts)?,
					content_length: upload
						.map_or(0, |upload| upload.content_length)
						.api_try_into()?,
					complete: upload.map_or(true, |upload| upload.complete_ts.is_some()),
					tags: build.tags.clone(),
				},
			))
		})
		.collect::<Result<Vec<_>, _>>()?;

	// Sort by date desc
	builds.sort_by_key(|(create_ts, _)| *create_ts);
	builds.reverse();

	Ok(models::GamesServersListBuildsResponse {
		builds: builds.into_iter().map(|(_, x)| x).collect::<Vec<_>>(),
	})
}

// MARK: POST /games/{}/builds/prepare
pub async fn create_build(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	body: models::GamesServersCreateBuildRequest,
) -> GlobalResult<models::GamesServersCreateBuildResponse> {
	ctx.auth().check_game(ctx.op_ctx(), game_id, false).await?;

	// TODO: Read and validate image file

	let multipart_upload = body.multipart_upload.unwrap_or(false);

	let kind = match body.kind {
		None | Some(models::GamesServersBuildKind::DockerImage) => {
			backend::build::BuildKind::DockerImage
		}
		Some(models::GamesServersBuildKind::OciBundle) => backend::build::BuildKind::OciBundle,
	};

	let compression = match body.compression {
		None | Some(models::GamesServersBuildCompression::None) => {
			backend::build::BuildCompression::None
		}
		Some(models::GamesServersBuildCompression::Lz4) => backend::build::BuildCompression::Lz4,
	};

	// Verify that tags are valid
	let tags: HashMap<String, String> = body
		.tags
		.map_or(Ok(HashMap::new()), serde_json::from_value)?;

	let create_res = op!([ctx] build_create {
		game_id: Some(game_id.into()),
		display_name: body.display_name,
		image_tag: Some(body.image_tag),
		image_file: Some((*body.image_file).api_try_into()?),
		multipart: multipart_upload,
		kind: kind as i32,
		compression: compression as i32,
		tags: tags,
	})
	.await?;

	let image_presigned_request = if !multipart_upload {
		Some(Box::new(
			unwrap!(create_res.image_presigned_requests.first())
				.clone()
				.api_try_into()?,
		))
	} else {
		None
	};

	let image_presigned_requests = if multipart_upload {
		Some(
			create_res
				.image_presigned_requests
				.iter()
				.cloned()
				.map(ApiTryInto::api_try_into)
				.collect::<GlobalResult<Vec<_>>>()?,
		)
	} else {
		None
	};

	Ok(models::GamesServersCreateBuildResponse {
		build_id: unwrap_ref!(create_res.build_id).as_uuid(),
		upload_id: unwrap_ref!(create_res.upload_id).as_uuid(),
		image_presigned_request,
		image_presigned_requests,
	})
}

// MARK: POST /games/{}/builds/{}/complete
pub async fn complete_build(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	build_id: Uuid,
	_body: serde_json::Value,
) -> GlobalResult<serde_json::Value> {
	ctx.auth().check_game(ctx.op_ctx(), game_id, false).await?;

	let build_res = op!([ctx] build_get {
		build_ids: vec![build_id.into()],
	})
	.await?;
	let build = unwrap_with!(build_res.builds.first(), BUILDS_BUILD_NOT_FOUND);

	ensure_with!(
		unwrap!(build.game_id).as_uuid() == game_id,
		BUILDS_BUILD_NOT_FOUND
	);

	op!([ctx] @dont_log_body upload_complete {
		upload_id: build.upload_id,
		bucket: None,
	})
	.await?;

	Ok(json!({}))
}
