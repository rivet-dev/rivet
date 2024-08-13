use std::collections::HashMap;

use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::backend;
use rivet_api::models;
use rivet_convert::ApiTryInto;
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;
use util::timestamp;

use crate::auth::Auth;

// MARK: GET /games/{}/builds/{}
pub async fn get(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	build_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::ServersGetBuildResponse> {
	ctx.auth().check_game(ctx.op_ctx(), game_id, true).await?;

	let builds_res = op!([ctx] build_get {
		build_ids: vec![build_id.into()],
	})
	.await?;
	let build = unwrap!(builds_res.builds.first());

	let uploads_res = op!([ctx] upload_get {
		upload_ids: builds_res
			.builds
			.iter()
			.filter_map(|build| build.upload_id)
			.collect::<Vec<_>>(),
	})
	.await?;
	let upload = unwrap!(uploads_res.uploads.first());

	let build = models::ServersBuild {
		id: unwrap!(build.build_id).as_uuid(),
		name: build.display_name.clone(),
		created_at: timestamp::to_string(build.create_ts)?,
		content_length: upload.content_length.api_try_into()?,
		tags: build.tags.clone(),
	};

	Ok(models::ServersGetBuildResponse {
		build: Box::new(build),
	})
}

// MARK: GET /games/{}/builds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetQuery {
	tags_json: Option<String>,
}

pub async fn list(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	_watch_index: WatchIndexQuery,
	query: GetQuery,
) -> GlobalResult<models::ServersListBuildsResponse> {
	ctx.auth().check_game(ctx.op_ctx(), game_id, true).await?;

	let list_res = op!([ctx] build_list_for_game {
		game_id: Some(game_id.into()),
		tags: query.tags_json.as_deref().map_or(Ok(HashMap::new()), serde_json::from_str)?,
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
		.filter_map(|build| {
			if let Some(upload) = uploads_res
				.uploads
				.iter()
				.find(|u| u.upload_id == build.upload_id)
			{
				Some((build, upload))
			} else {
				None
			}
		})
		.map(|(build, upload)| {
			GlobalResult::Ok((
				build.create_ts,
				models::ServersBuild {
					id: unwrap!(build.build_id).as_uuid(),
					name: build.display_name.clone(),
					created_at: timestamp::to_string(build.create_ts)?,
					content_length: upload.content_length.api_try_into()?,
					tags: build.tags.clone(),
				},
			))
		})
		.collect::<Result<Vec<_>, _>>()?;

	// Sort by date desc
	builds.sort_by_key(|(create_ts, _)| *create_ts);
	builds.reverse();

	Ok(models::ServersListBuildsResponse {
		builds: builds.into_iter().map(|(_, x)| x).collect::<Vec<_>>(),
	})
}

// MARK: PATCH /games/{}/builds/{}/tags
pub async fn patch_tags(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	build_id: Uuid,
	body: models::ServersPatchBuildTagsRequest,
) -> GlobalResult<serde_json::Value> {
	ctx.auth().check_game(ctx.op_ctx(), game_id, false).await?;

	let tags: HashMap<String, String> =
		serde_json::from_value::<HashMap<String, String>>(unwrap!(body.tags))?;

	ctx.op(build::ops::get::Input {
		build_ids: vec![build_id],
	})
	.await?;

	ctx.op(build::ops::patch_tags::Input {
		build_id,
		tags,
		exclusive_tags: body.exclusive_tags,
	})
	.await?;

	Ok(json!({}))
}

// MARK: POST /games/{}/builds/prepare
pub async fn create_build(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	body: models::ServersCreateBuildRequest,
) -> GlobalResult<models::ServersCreateBuildResponse> {
	ctx.auth().check_game(ctx.op_ctx(), game_id, false).await?;

	// TODO: Read and validate image file

	let multipart_upload = body.multipart_upload.unwrap_or(false);

	let kind = match body.kind {
		None | Some(models::ServersBuildKind::DockerImage) => {
			backend::build::BuildKind::DockerImage
		}
		Some(models::ServersBuildKind::OciBundle) => backend::build::BuildKind::OciBundle,
	};

	let compression = match body.compression {
		None | Some(models::ServersBuildCompression::None) => {
			backend::build::BuildCompression::None
		}
		Some(models::ServersBuildCompression::Lz4) => backend::build::BuildCompression::Lz4,
	};

	let create_res = op!([ctx] build_create {
		game_id: Some(game_id.into()),
		display_name: body.name,
		image_tag: Some(body.image_tag),
		image_file: Some((*body.image_file).api_try_into()?),
		multipart: multipart_upload,
		kind: kind as i32,
		compression: compression as i32,
	})
	.await?;
	let build_id = unwrap_ref!(create_res.build_id).as_uuid();

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

	Ok(models::ServersCreateBuildResponse {
		build: build_id,
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
