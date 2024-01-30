use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::backend;
use rivet_api::models;
use rivet_convert::ApiTryInto;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: GET /games/{}/builds
pub async fn get_builds(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::CloudGamesListGameBuildsResponse> {
	ctx.auth()
		.check_game_read_or_admin(ctx.op_ctx(), game_id)
		.await?;

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

			GlobalResult::Ok((
				build.create_ts,
				models::CloudBuildSummary {
					build_id: unwrap_ref!(build.build_id).as_uuid(),
					upload_id: unwrap_ref!(build.upload_id).as_uuid(),
					display_name: build.display_name.clone(),
					create_ts: util::timestamp::to_string(build.create_ts)?,
					content_length: upload
						.map_or(0, |upload| upload.content_length)
						.try_into()?,
					complete: upload.map_or(true, |upload| upload.complete_ts.is_some()),
				},
			))
		})
		.collect::<Result<Vec<_>, _>>()?;

	// Sort by date desc
	builds.sort_by_key(|(create_ts, _)| *create_ts);
	builds.reverse();

	Ok(models::CloudGamesListGameBuildsResponse {
		builds: builds.into_iter().map(|(_, x)| x).collect::<Vec<_>>(),
	})
}

// MARK: POST /games/{}/versions/builds
pub async fn create_build(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	body: models::CloudGamesCreateGameBuildRequest,
) -> GlobalResult<models::CloudGamesCreateGameBuildResponse> {
	ctx.auth()
		.check_game_write_or_admin(ctx.op_ctx(), game_id)
		.await?;

	// TODO: Read and validate image file

	let multipart_upload = body.multipart_upload.unwrap_or(false);

	let kind = match body.kind {
		None | Some(models::CloudGamesBuildKind::DockerImage) => {
			backend::build::BuildKind::DockerImage
		}
		Some(models::CloudGamesBuildKind::OciBundle) => backend::build::BuildKind::OciBundle,
	};

	let compression = match body.compression {
		None | Some(models::CloudGamesBuildCompression::None) => {
			backend::build::BuildCompression::None
		}
		Some(models::CloudGamesBuildCompression::Lz4) => backend::build::BuildCompression::Lz4,
	};

	let create_res = op!([ctx] build_create {
		game_id: Some(game_id.into()),
		display_name: body.display_name,
		image_tag: Some(body.image_tag),
		image_file: Some((*body.image_file).try_into()?),
		multipart: multipart_upload,
		kind: kind as i32,
		compression: compression as i32,
		..Default::default()
	})
	.await?;

	let image_presigned_request = if !multipart_upload {
		Some(Box::new(
			unwrap!(create_res.image_presigned_requests.first())
				.clone()
				.try_into()?,
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
				.map(ApiTryInto::try_into)
				.collect::<GlobalResult<Vec<_>>>()?,
		)
	} else {
		None
	};

	Ok(models::CloudGamesCreateGameBuildResponse {
		build_id: unwrap_ref!(create_res.build_id).as_uuid(),
		upload_id: unwrap_ref!(create_res.upload_id).as_uuid(),
		image_presigned_request,
		image_presigned_requests,
	})
}
