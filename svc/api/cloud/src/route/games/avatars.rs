use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::backend;
use rivet_api::models as new_models;
use rivet_claims::ClaimsDecode;
use rivet_cloud_server::models;
use rivet_convert::ApiTryInto;
use rivet_operation::prelude::*;

use crate::auth::Auth;

const MAX_AVATAR_UPLOAD_SIZE: i64 = util::file_size::megabytes(2) as i64;

// MARK: GET /games/{}/avatars
pub async fn get_custom_avatars(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::ListGameCustomAvatarsResponse> {
	ctx.auth().check_game_read(ctx.op_ctx(), game_id).await?;

	let custom_avatars_res = op!([ctx] custom_user_avatar_list_for_game {
		game_id: Some(game_id.into()),
	})
	.await?;

	let upload_ids = custom_avatars_res
		.custom_avatars
		.iter()
		.flat_map(|custom_avatar| custom_avatar.upload_id)
		.collect::<Vec<_>>();
	let (uploads_res, upload_files_res) = tokio::try_join!(
		op!([ctx] upload_get {
			upload_ids: upload_ids.clone(),
		}),
		op!([ctx] upload_file_list {
			upload_ids: upload_ids,
		})
	)?;

	// Convert the avatar data structures
	let mut custom_avatars = custom_avatars_res
		.custom_avatars
		.iter()
		.filter_map(|custom_avatar| {
			// Fetch upload and file for custom avatar
			if let (Some(upload), Some(file)) = (
				uploads_res
					.uploads
					.iter()
					.find(|upload| upload.upload_id == custom_avatar.upload_id),
				upload_files_res
					.files
					.iter()
					.find(|file| file.upload_id == custom_avatar.upload_id),
			) {
				Some((custom_avatar, upload, file))
			} else {
				None
			}
		})
		.map(|(custom_avatar, upload, file)| {
			let upload_id = unwrap_ref!(custom_avatar.upload_id).as_uuid();
			let profile_file_name = file
				.path
				.rsplit_once('/')
				.map(|(_, file_name)| file_name.to_owned())
				.unwrap_or(file.path.clone());

			GlobalResult::Ok(models::CustomAvatarSummary {
				upload_id: upload_id.to_string(),
				display_name: profile_file_name.clone(),
				url: upload.complete_ts.map(|_| {
					util::route::custom_avatar(upload_id, &profile_file_name, upload.provider)
				}),
				create_ts: util::timestamp::to_chrono(upload.create_ts)?,
				content_length: upload.content_length as i64,
				complete: upload.complete_ts.is_some(),
			})
		})
		.collect::<Result<Vec<_>, _>>()?;

	// Sort by date desc
	custom_avatars.sort_by_key(|b| b.create_ts);
	custom_avatars.reverse();

	Ok(models::ListGameCustomAvatarsResponse { custom_avatars })
}

// MARK: POST /games/{}/avatar-upload/prepare
pub async fn prepare_avatar_upload(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	body: new_models::CloudGamesPrepareCustomAvatarUploadRequest,
) -> GlobalResult<new_models::CloudGamesPrepareCustomAvatarUploadResponse> {
	ctx.auth().check_game_write(ctx.op_ctx(), game_id).await?;

	let user_id = ctx.auth().claims()?.as_user().ok().map(|x| x.user_id);

	ensure_with!(
		body.content_length >= 0,
		CLOUD_INVALID_CONFIG,
		error = "`content_length` out of bounds"
	);
	ensure_with!(
		body.content_length < MAX_AVATAR_UPLOAD_SIZE,
		UPLOAD_TOO_LARGE
	);

	let ext = if body.path.ends_with(".png") {
		"png"
	} else if body.path.ends_with(".jpg") || body.path.ends_with(".jpeg") {
		"jpeg"
	} else {
		bail!("invalid file type (allowed: .png, .jpg)");
	};

	// Create the upload
	let upload_prepare_res = op!([ctx] upload_prepare {
		bucket: "bucket-user-avatar".to_owned(),
		files: vec![
			backend::upload::PrepareFile {
				path: format!("image.{ext}"),
				mime: Some(format!("image/{ext}")),
				content_length: body.content_length.try_into()?,
				nsfw_score_threshold: Some(util_nsfw::score_thresholds::USER_AVATAR),
				..Default::default()
			},
		],
		user_id: user_id.map(Into::into),
	})
	.await?;

	let upload_id = unwrap_ref!(upload_prepare_res.upload_id).as_uuid();
	let presigned_request = unwrap!(upload_prepare_res.presigned_requests.first());

	Ok(new_models::CloudGamesPrepareCustomAvatarUploadResponse {
		upload_id,
		presigned_request: Box::new(presigned_request.clone().try_into()?),
	})
}

// MARK: POST /games/{}/avatar-upload/{}/complete
pub async fn complete_avatar_upload(
	ctx: Ctx<Auth>,
	game_id: Uuid,
	upload_id: Uuid,
	_body: models::CompleteCustomAvatarUploadRequest,
) -> GlobalResult<models::CompleteCustomAvatarUploadResponse> {
	ctx.auth().check_game_write(ctx.op_ctx(), game_id).await?;

	op!([ctx] custom_user_avatar_upload_complete {
		game_id: Some(game_id.into()),
		upload_id: Some(upload_id.into()),
	})
	.await?;

	Ok(models::CompleteCustomAvatarUploadResponse {})
}
