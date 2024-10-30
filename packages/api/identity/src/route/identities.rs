use api_helper::{
	anchor::{WatchIndexQuery, WatchResponse},
	ctx::Ctx,
};
use futures_util::FutureExt;
use proto::backend::{self, pkg::*};
use rivet_api::models;
use rivet_convert::{convert, fetch, ApiInto, ApiTryInto};
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{assert, auth::Auth};

const MAX_AVATAR_UPLOAD_SIZE: i64 = util::file_size::megabytes(2) as i64;

// MARK: GET /identities/{}/profile
pub async fn profile(
	ctx: Ctx<Auth>,
	identity_id: Uuid,
	watch_index: WatchIndexQuery,
) -> GlobalResult<models::IdentityGetProfileResponse> {
	let current_user_id = ctx.auth().user(ctx.op_ctx()).await?.user_id;

	let (identity, update_ts) =
		get_profile(&ctx, current_user_id, identity_id, watch_index).await?;

	Ok(models::IdentityGetProfileResponse {
		identity: Box::new(identity),
		watch: WatchResponse::new_as_model(update_ts),
	})
}

// MARK: GET /identities/self/profile
pub async fn profile_self(
	ctx: Ctx<Auth>,
	watch_index: WatchIndexQuery,
) -> GlobalResult<models::IdentityGetProfileResponse> {
	let current_user_id = ctx.auth().user(ctx.op_ctx()).await?.user_id;

	let (identity, update_ts) =
		get_profile(&ctx, current_user_id, current_user_id, watch_index).await?;

	Ok(models::IdentityGetProfileResponse {
		identity: Box::new(identity),
		watch: WatchResponse::new_as_model(update_ts),
	})
}

// TODO: Use information from messages instead of re-fetching everything
async fn get_profile(
	ctx: &Ctx<Auth>,
	current_user_id: Uuid,
	identity_id: Uuid,
	watch_index: WatchIndexQuery,
) -> GlobalResult<(models::IdentityProfile, i64)> {
	// Wait for an update if needed
	let update_ts = if let Some(anchor) = watch_index.to_consumer()? {
		let user_updated_sub = tail_anchor!([ctx, anchor] user::msg::updated(identity_id));

		util::macros::select_with_timeout!({
			event = user_updated_sub => {
				event?.msg_ts()
			}
		})
	} else {
		Default::default()
	};
	let update_ts = update_ts.unwrap_or_else(util::timestamp::now);

	let identities =
		fetch::identity::profiles(ctx.op_ctx(), current_user_id, vec![identity_id]).await?;

	Ok((
		unwrap_with!(identities.into_iter().next(), IDENTITY_NOT_FOUND),
		update_ts,
	))
}

// MARK: GET /identities/batch/handle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityIdsQuery {
	identity_ids: Vec<Uuid>,
}
pub async fn handles(
	ctx: Ctx<Auth>,
	watch_index: WatchIndexQuery,
	query: IdentityIdsQuery,
) -> GlobalResult<models::IdentityGetHandlesResponse> {
	let current_user_id = ctx.auth().user(ctx.op_ctx()).await?.user_id;

	ensure_with!(
		!query.identity_ids.is_empty(),
		API_BAD_QUERY_PARAMETER,
		parameter = "identity_ids",
		error = "cannot be empty",
	);
	ensure_with!(
		query.identity_ids.len() <= 64,
		API_BAD_QUERY_PARAMETER,
		parameter = "identity_ids",
		error = "too many ids (max 64)",
	);

	// Wait for an update if needed
	let update_ts =
		if let Some(anchor) = watch_index.to_consumer()? {
			// User update subs
			let user_updated_subs_select =
				util::future::select_all_or_wait(query.identity_ids.iter().cloned().map(
					|user_id| tail_anchor!([ctx, anchor] user::msg::updated(user_id)).boxed(),
				));

			util::macros::select_with_timeout!({
				event = user_updated_subs_select => {
					let event = event?;
					event.msg_ts()
				}
			})
		} else {
			Default::default()
		};
	let update_ts = update_ts.unwrap_or_else(util::timestamp::now);

	let identities =
		fetch::identity::handles(ctx.op_ctx(), current_user_id, query.identity_ids).await?;

	Ok(models::IdentityGetHandlesResponse {
		identities,
		watch: WatchResponse::new_as_model(update_ts),
	})
}

// TODO: Add tailing
// MARK: GET /identities/batch/summary
pub async fn summaries(
	ctx: Ctx<Auth>,
	watch_index: WatchIndexQuery,
	query: IdentityIdsQuery,
) -> GlobalResult<models::IdentityGetSummariesResponse> {
	let current_user_id = ctx.auth().user(ctx.op_ctx()).await?.user_id;

	ensure_with!(
		!query.identity_ids.is_empty(),
		API_BAD_QUERY_PARAMETER,
		parameter = "identity_ids",
		error = "cannot be empty",
	);
	ensure_with!(
		query.identity_ids.len() <= 64,
		API_BAD_QUERY_PARAMETER,
		parameter = "identity_ids",
		error = "too many ids (max 64)",
	);

	// Wait for an update if needed
	let update_ts =
		if let Some(anchor) = watch_index.to_consumer()? {
			// User update subs
			let user_updated_subs_select =
				util::future::select_all_or_wait(query.identity_ids.iter().cloned().map(
					|user_id| tail_anchor!([ctx, anchor] user::msg::updated(user_id)).boxed(),
				));

			util::macros::select_with_timeout!({
				event = user_updated_subs_select => {
					let event = event?;
					event.msg_ts()
				}
			})
		} else {
			Default::default()
		};
	let update_ts = update_ts.unwrap_or_else(util::timestamp::now);

	let identities =
		fetch::identity::summaries(ctx.op_ctx(), current_user_id, query.identity_ids).await?;

	Ok(models::IdentityGetSummariesResponse {
		identities,
		watch: WatchResponse::new_as_model(update_ts),
	})
}

// MARK: POST /identities/self/profile
pub async fn update_profile(
	ctx: Ctx<Auth>,
	body: models::IdentityUpdateProfileRequest,
) -> GlobalResult<serde_json::Value> {
	let user_ent = ctx.auth().user(ctx.op_ctx()).await?;
	assert::user_registered(&ctx, user_ent.user_id).await?;

	ensure!(
		body.account_number.unwrap_or_default() >= 0,
		"invalid parameter account_number`"
	);

	msg!([ctx] user::msg::profile_set(user_ent.user_id) -> user::msg::update {
		user_id: Some(user_ent.user_id.into()),
		display_name: body.display_name.clone(),
		account_number: body.account_number.map(|n| n.api_try_into()).transpose()?,
		bio: body.bio.clone(),
	})
	.await?;

	Ok(serde_json::json!({}))
}

// MARK: POST /identities/self/profile/validate
pub async fn validate_profile(
	ctx: Ctx<Auth>,
	body: models::IdentityUpdateProfileRequest,
) -> GlobalResult<models::IdentityValidateProfileResponse> {
	let user_ent = ctx.auth().user(ctx.op_ctx()).await?;

	ensure!(
		body.account_number.unwrap_or_default() >= 0,
		"invalid parameter account_number`"
	);

	let res = op!([ctx] user_profile_validate {
		user_id: Some(user_ent.user_id.into()),
		display_name: body.display_name.clone(),
		account_number:
			body.account_number
				.map(|n| n.api_try_into())
				.transpose()?,
		bio: body.bio.clone(),
	})
	.await?;

	Ok(models::IdentityValidateProfileResponse {
		errors: res
			.errors
			.into_iter()
			.map(ApiInto::api_into)
			.collect::<Vec<_>>(),
	})
}

// MARK: POST /identities/avatar-upload/prepare
pub async fn prepare_avatar_upload(
	ctx: Ctx<Auth>,
	body: models::IdentityPrepareAvatarUploadRequest,
) -> GlobalResult<models::IdentityPrepareAvatarUploadResponse> {
	let user_ent = ctx.auth().user(ctx.op_ctx()).await?;
	assert::user_registered(&ctx, user_ent.user_id).await?;

	ensure!(body.content_length >= 0, "upload invalid");
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
				path: format!("image.{}", ext),
				mime: Some(format!("image/{}", ext)),
				content_length: body.content_length.api_try_into()?,
				..Default::default()
			},
		],
		user_id: Some(user_ent.user_id.into()),
	})
	.await?;

	let upload_id = unwrap_ref!(upload_prepare_res.upload_id).as_uuid();
	let presigned_request = unwrap!(upload_prepare_res.presigned_requests.first());

	Ok(models::IdentityPrepareAvatarUploadResponse {
		upload_id,
		presigned_request: Box::new(presigned_request.clone().api_try_into()?),
	})
}

// MARK: POST /identities/avatar-upload/{}/complete
pub async fn complete_avatar_upload(
	ctx: Ctx<Auth>,
	upload_id: Uuid,
	_body: serde_json::Value,
) -> GlobalResult<serde_json::Value> {
	let user_ent = ctx.auth().user(ctx.op_ctx()).await?;

	op!([ctx] user_avatar_upload_complete {
		user_id: Some(user_ent.user_id.into()),
		upload_id: Some(upload_id.into()),
	})
	.await?;

	Ok(serde_json::json!({}))
}

// MARK: POST /identities/self/delete-request
pub async fn mark_deletion(
	ctx: Ctx<Auth>,
	_body: serde_json::Value,
) -> GlobalResult<serde_json::Value> {
	let user_ent = ctx.auth().user(ctx.op_ctx()).await?;

	op!([ctx] user_pending_delete_toggle {
		user_id: Some(user_ent.user_id.into()),
		active: true,
	})
	.await?;

	Ok(serde_json::json!({}))
}

// MARK: DELETE /identities/self/delete-request
pub async fn unmark_deletion(ctx: Ctx<Auth>) -> GlobalResult<serde_json::Value> {
	let user_ent = ctx.auth().user(ctx.op_ctx()).await?;

	op!([ctx] user_pending_delete_toggle {
		user_id: Some(user_ent.user_id.into()),
		active: false,
	})
	.await?;

	Ok(serde_json::json!({}))
}
