use std::collections::HashSet;

use api_helper::{
	anchor::{WatchIndexQuery, WatchResponse},
	ctx::Ctx,
};
use chirp_client::TailAnchorResponse;
use futures_util::FutureExt;
use proto::{
	backend::{self, pkg::*},
	common,
};
use rivet_api::models;
use rivet_claims::ClaimsDecode;
use rivet_convert::{convert, fetch, ApiInto, ApiTryInto};
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{assert, auth::Auth, utils};

const MAX_AVATAR_UPLOAD_SIZE: i64 = util::file_size::megabytes(2) as i64;

// MARK: POST /identities
pub async fn setup_identity(
	ctx: Ctx<Auth>,
	body: models::IdentitySetupRequest,
) -> GlobalResult<models::IdentitySetupResponse> {
	let namespace_id = utils::get_namespace_id(&ctx).await?;
	utils::validate_config(ctx.op_ctx(), namespace_id).await?;

	// Attempt to setup an existing identity token. If does not succeed, a new
	// token will be created.
	if let Some(response) = attempt_setup_existing_identity_token(
		&ctx,
		namespace_id,
		body.existing_identity_token.as_deref(),
	)
	.await?
	{
		tracing::info!("setup existing identity");
		Ok(response)
	} else {
		tracing::info!("creating new identity");

		// Create a new user
		let user_id = Uuid::new_v4();
		msg!([ctx] user::msg::create(user_id) -> user::msg::create_complete {
			user_id: Some(user_id.into()),
			namespace_id: Some(namespace_id),
		})
		.await?;
		tracing::info!(%user_id, "created new user");

		// Create game user
		let game_user_res = op!([ctx] game_user_create {
			namespace_id: Some(namespace_id),
			user_id: Some(user_id.into()),
		})
		.await?;

		utils::touch_user_presence(ctx.op_ctx().base(), user_id, true);

		// Decode the tokens for the expiration ts
		let user_token_claims = rivet_claims::decode(&game_user_res.token)??;
		let game_user_id = *unwrap!(game_user_res.game_user_id);
		let refresh_jti = unwrap!(user_token_claims.jti);

		// Create a game session row
		msg!([ctx] game_user::msg::session_create(game_user_id) {
			game_user_id: Some(game_user_id.into()),
			refresh_jti: Some(refresh_jti),
		})
		.await?;

		// Fetch the user data
		let (identities, game_resolve_res) = tokio::try_join!(
			fetch::identity::profiles(ctx.op_ctx(), user_id, Some(game_user_id), vec![user_id]),
			op!([ctx] game_resolve_namespace_id {
				namespace_ids: vec![namespace_id],
			})
		)?;
		let identity = unwrap!(identities.into_iter().next());
		let game_id = unwrap!(unwrap!(game_resolve_res.games.first()).game_id).as_uuid();

		Ok(models::IdentitySetupResponse {
			identity_token: game_user_res.token.clone(),
			identity_token_expire_ts: util::timestamp::to_string(
				user_token_claims.exp.unwrap_or(0),
			)?,
			identity: Box::new(identity),
			game_id,
		})
	}
}

/// Attempt to use the existing refresh token to return a new user token. Any
/// failure will fall through and a new game user will be created & returned.
async fn attempt_setup_existing_identity_token(
	ctx: &Ctx<Auth>,
	namespace_id: common::Uuid,
	existing_identity_token: Option<&str>,
) -> GlobalResult<Option<models::IdentitySetupResponse>> {
	let req_user_token = if let Some(x) = &existing_identity_token {
		x
	} else {
		return Ok(None);
	};

	let req_user_token_claims = match rivet_claims::decode(req_user_token) {
		Ok(Ok(x)) => x,
		Ok(Err(err)) => {
			tracing::warn!(?err, "failed to validate token, probably expired");
			return Ok(None);
		}
		Err(err) => {
			tracing::warn!(?err, "failed to decode token");
			return Ok(None);
		}
	};

	// Check if the token is revoked
	let req_user_token_jti = unwrap!(req_user_token_claims.jti);
	let req_token_res = op!([ctx] token_get {
		jtis: vec![req_user_token_jti],
	})
	.await?;
	if req_token_res.tokens.is_empty() {
		tracing::warn!(
			?req_user_token_jti,
			"token that we've signed should exist in the database"
		);
	}
	// TODO: Add back
	// let req_token = unwrap!(
	// 	req_token_res.tokens.first(),
	// 	"token that we've signed should exist in the database"
	// );
	// if req_token.revoke_ts.is_some() {
	// 	tracing::warn!("identity token revoked, likely due to race condition with saving token");
	// 	return Ok(None);
	// }

	// Refresh the token if needed
	let (user_token, user_token_claims) = if util::timestamp::now()
		> req_user_token_claims.iat + util_game_user::GAME_USER_TOKEN_REFRESH_AGE
	{
		tracing::info!("been a while since the token, refreshing token");

		// This will return TOKEN_REFRESH_NOT_FOUND if an invalid token is
		// provided.
		let token_create_res = op!([ctx] token_create {
			token_config: Some(token::create::request::TokenConfig {
				ttl: util_game_user::GAME_USER_TOKEN_TTL,
			}),
			refresh_token_config: None,
			issuer: "api-identity".to_owned(),
			client: Some(ctx.client_info()),
			kind: Some(token::create::request::Kind::Refresh(
				token::create::request::KindRefresh { refresh_token: req_user_token.to_string() },
			)),
			label: Some("game_user".into()),
			combine_refresh_token: true,
			..Default::default()
		})
		.await;
		let token_create_res = match token_create_res {
			Ok(x) => x,
			// Catch TOKEN_REVOKED and create a new user.
			//
			// This is usually a side effect of a client calling
			// SetupIdentity and not saving the returning identity_token
			// or calling SetupIdentity twice and one refresh token gets
			// overwritten.
			Err(err) if err.is(formatted_error::code::TOKEN_REVOKED) => {
				tracing::warn!(
					"identity token revoked, likely due to race condition with saving token"
				);
				return Ok(None);
			}
			Err(err) => {
				return Err(err);
			}
		};

		let user_token = unwrap_ref!(token_create_res.token);

		// Decode the token
		let user_token_claims = rivet_claims::decode(&user_token.token)??;

		(user_token.token.to_owned(), user_token_claims)
	} else {
		(req_user_token.to_string(), req_user_token_claims)
	};

	tracing::info!(?user_token_claims, "fetching user");
	let game_user_ent = user_token_claims.as_game_user()?;
	let refresh_jti = unwrap!(user_token_claims.jti);

	// Create a game session row
	msg!([ctx] game_user::msg::session_create(game_user_ent.game_user_id) {
		game_user_id: Some(game_user_ent.game_user_id.into()),
		refresh_jti: Some(refresh_jti),
	})
	.await?;

	// Fetch user data
	let Some(user_id) =
		utils::resolve_user_with_game_user_id(ctx, game_user_ent.game_user_id).await?
	else {
		tracing::info!("game user not found");
		return Ok(None);
	};
	utils::touch_user_presence(ctx.op_ctx().base(), user_id, false);

	let (identities, game_resolve_res) = tokio::try_join!(
		fetch::identity::profiles(
			ctx.op_ctx(),
			user_id,
			Some(game_user_ent.game_user_id),
			vec![user_id],
		),
		op!([ctx] game_resolve_namespace_id {
			namespace_ids: vec![namespace_id],
		})
	)?;
	let identity = unwrap!(identities.into_iter().next());
	let game_id = unwrap!(unwrap!(game_resolve_res.games.first()).game_id).as_uuid();

	Ok(Some(models::IdentitySetupResponse {
		identity_token: user_token.clone(),
		identity_token_expire_ts: util::timestamp::to_string(
			user_token_claims.exp.unwrap_or_default(),
		)?,
		identity: Box::new(identity),
		game_id,
	}))
}

// MARK: GET /identities/{}/profile
pub async fn profile(
	ctx: Ctx<Auth>,
	identity_id: Uuid,
	watch_index: WatchIndexQuery,
) -> GlobalResult<models::IdentityGetProfileResponse> {
	let (current_user_id, game_user) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	let (identity, update_ts) =
		get_profile(&ctx, current_user_id, game_user, identity_id, watch_index).await?;

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
	let (current_user_id, game_user) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	let (identity, update_ts) = get_profile(
		&ctx,
		current_user_id,
		game_user,
		current_user_id,
		watch_index,
	)
	.await?;

	Ok(models::IdentityGetProfileResponse {
		identity: Box::new(identity),
		watch: WatchResponse::new_as_model(update_ts),
	})
}

// TODO: Use information from messages instead of re-fetching everything
async fn get_profile(
	ctx: &Ctx<Auth>,
	current_user_id: Uuid,
	game_user: Option<game_user::get::response::GameUser>,
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

	let identities = fetch::identity::profiles(
		ctx.op_ctx(),
		current_user_id,
		game_user.and_then(|x| x.game_user_id.map(|id| *id)),
		vec![identity_id],
	)
	.await?;

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
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

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
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

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
		account_number: body.account_number.map(|n| n.try_into()).transpose()?,
		bio: body.bio.clone(),
	})
	.await?;

	Ok(serde_json::json!({}))
}

// MARK: GET /identities/search
#[derive(Debug, Serialize, Deserialize)]
pub struct SearchQuery {
	query: String,
	limit: Option<u32>,
	anchor: Option<String>,
}

pub async fn search(
	ctx: Ctx<Auth>,
	_watch_index: WatchIndexQuery,
	query: SearchQuery,
) -> GlobalResult<models::IdentitySearchResponse> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	ensure_with!(
		query.limit.map(|v| v != 0).unwrap_or(true),
		API_BAD_QUERY_PARAMETER,
		parameter = "limit",
		error = "Must be greater than 0",
	);

	let user_search_res = op!([ctx] user_search {
		query: query.query,
		limit: query.limit.unwrap_or(32),
		anchor: query.anchor
			.map(|anchor| anchor.parse::<i64>())
			.transpose()?,
	})
	.await?;

	let res = op!([ctx] user_get {
		user_ids: user_search_res.user_ids.clone(),
	})
	.await?;

	let user_handles = res
		.users
		.iter()
		.map(|user| convert::identity::handle_without_presence(current_user_id, user))
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok(models::IdentitySearchResponse {
		identities: user_handles,
		anchor: user_search_res.anchor.as_ref().map(ToString::to_string),
	})
}

// MARK: POST /identities/self/activity/
pub async fn set_game_activity(
	ctx: Ctx<Auth>,
	body: models::IdentitySetGameActivityRequest,
) -> GlobalResult<serde_json::Value> {
	let game_user = ctx.auth().fetch_game_user(ctx.op_ctx()).await?;

	msg!([ctx] user_presence::msg::game_activity_set(game_user.user_id) {
		user_id: Some(game_user.user_id.into()),
		game_activity: Some(backend::user::presence::GameActivity {
			game_id: Some(game_user.game_id.into()),
			message: body.game_activity.message.unwrap_or_default(),
			public_metadata: body.game_activity
				.public_metadata
				.map(|public_metadata| serde_json::to_string(&public_metadata))
				.transpose()?,
			friend_metadata: body.game_activity
				.mutual_metadata
				.map(|mutual_metadata| serde_json::to_string(&mutual_metadata))
				.transpose()?,
		}),
	})
	.await?;

	Ok(serde_json::json!({}))
}

// MARK: DELETE /identities/self/activity/
pub async fn remove_game_activity(ctx: Ctx<Auth>) -> GlobalResult<serde_json::Value> {
	let game_user = ctx.auth().fetch_game_user(ctx.op_ctx()).await?;

	msg!([ctx] user_presence::msg::game_activity_set(game_user.user_id) {
		user_id: Some(game_user.user_id.into()),
		game_activity: None,
	})
	.await?;

	Ok(serde_json::json!({}))
}

// MARK: POST /identities/self/status
pub async fn update_status(
	ctx: Ctx<Auth>,
	body: models::IdentityUpdateStatusRequest,
) -> GlobalResult<serde_json::Value> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	msg!([ctx] user_presence::msg::status_set(current_user_id) {
		user_id: Some(current_user_id.into()),
		status: ApiInto::<backend::user::Status>::api_into(body.status) as i32,
		user_set_status: true,
		silent: false,
	})
	.await?;

	Ok(serde_json::json!({}))
}

// MARK: POST /identities/{}/follow
pub async fn follow_identity(
	ctx: Ctx<Auth>,
	identity_id: Uuid,
	_body: serde_json::Value,
) -> GlobalResult<serde_json::Value> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	op!([ctx] user_follow_toggle {
		follower_user_id: Some(current_user_id.into()),
		following_user_id: Some(identity_id.into()),
		active: true,
	})
	.await?;

	Ok(serde_json::json!({}))
}

// MARK: DELETE /identities/{}/follow
pub async fn unfollow_identity(
	ctx: Ctx<Auth>,
	identity_id: Uuid,
) -> GlobalResult<serde_json::Value> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	op!([ctx] user_follow_toggle {
		follower_user_id: Some(current_user_id.into()),
		following_user_id: Some(identity_id.into()),
		active: false,
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
				.map(|n| n.try_into())
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
				content_length: body.content_length.try_into()?,
				nsfw_score_threshold: Some(util_nsfw::score_thresholds::USER_AVATAR),
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
		presigned_request: Box::new(presigned_request.clone().api_into()),
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

// MARK: POST /identities/self/beta-signup
pub async fn beta_signup(
	ctx: Ctx<Auth>,
	_body: models::IdentitySignupForBetaRequest,
) -> GlobalResult<serde_json::Value> {
	let _user_ent = ctx.auth().user(ctx.op_ctx()).await?;

	bail_with!(ERROR, error = "deprecated");
}

// MARK: POST /identities/{}/report
pub async fn report_identity(
	ctx: Ctx<Auth>,
	identity_id: Uuid,
	body: models::IdentityReportRequest,
) -> GlobalResult<serde_json::Value> {
	let game_user = ctx.auth().fetch_game_user(ctx.op_ctx()).await?;

	if let Some(reason) = &body.reason {
		ensure!(reason.len() <= 300, "`reason` too long");
	}

	msg!([ctx] user_report::msg::create(identity_id) {
		reporter_user_id: Some(game_user.user_id.into()),
		subject_user_id: Some(identity_id.into()),
		namespace_id: Some(game_user.namespace_id.into()),
		reason: body.reason.clone(),
	})
	.await?;

	Ok(serde_json::json!({}))
}

// MARK: GET /identities/{}/followers
enum FollowConsumerUpdate {
	FollowCreate(common::Uuid),
	FollowRemove(common::Uuid),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FollowsQuery {
	count: Option<u32>,
	anchor: Option<String>,
}

pub async fn followers(
	ctx: Ctx<Auth>,
	identity_id: Uuid,
	watch_index: WatchIndexQuery,
	query: FollowsQuery,
) -> GlobalResult<models::IdentityListFollowersResponse> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	// Fetch follows
	let followers_res = op!([ctx] user_follow_list {
		kind: user_follow::list::request::Kind::Following as i32,
		user_ids: vec![identity_id.into()],
		limit: query.count.unwrap_or(32),
		anchor: query.anchor
			.map(|anchor| anchor.parse::<i64>())
			.transpose()?,
	})
	.await?;
	let followers_res = unwrap!(followers_res.follows.first());
	let follows = &followers_res.follows;

	// Get user ids
	let mut user_ids = follows
		.iter()
		.filter_map(|f| f.user_id.as_ref())
		.map(common::Uuid::as_uuid)
		.collect::<HashSet<_>>();

	// Wait for an update if needed
	let (update, update_ts) = if let Some(anchor) = watch_index.to_consumer()? {
		let follow_sub = tail_anchor!([ctx, anchor] user_follow::msg::create("*", identity_id));
		let unfollow_sub = tail_anchor!([ctx, anchor] user_follow::msg::delete("*", identity_id));

		// User presence subs
		let user_presence_subs_select =
			util::future::select_all_or_wait(user_ids.iter().cloned().map(|user_id| {
				tail_anchor!([ctx, anchor] user_presence::msg::update(user_id)).boxed()
			}));

		util::macros::select_with_timeout!({
			event = follow_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(msg.follower_user_id.map(FollowConsumerUpdate::FollowCreate), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = unfollow_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(msg.follower_user_id.map(FollowConsumerUpdate::FollowRemove), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = user_presence_subs_select => {
				if let TailAnchorResponse::Message(msg) = event? {
					(None, Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
		})
	} else {
		Default::default()
	};
	let update_ts = update_ts.unwrap_or_else(util::timestamp::now);

	// Remove/add new user
	match update {
		Some(FollowConsumerUpdate::FollowCreate(user_id)) => {
			user_ids.insert(user_id.as_uuid());
		}
		Some(FollowConsumerUpdate::FollowRemove(user_id)) => {
			user_ids.remove(&user_id.as_uuid());
		}
		_ => {}
	}

	let identities = fetch::identity::handles(
		ctx.op_ctx(),
		current_user_id,
		user_ids.into_iter().collect::<Vec<_>>(),
	)
	.await?;

	Ok(models::IdentityListFollowersResponse {
		identities,
		anchor: followers_res.anchor.map(|a| a.to_string()),
		watch: WatchResponse::new_as_model(update_ts),
	})
}

// MARK: GET /identities/{}/following
pub async fn following(
	ctx: Ctx<Auth>,
	identity_id: Uuid,
	watch_index: WatchIndexQuery,
	query: FollowsQuery,
) -> GlobalResult<models::IdentityListFollowingResponse> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	// Fetch follows
	let followers_res = op!([ctx] user_follow_list {
		kind: user_follow::list::request::Kind::Follower as i32,
		user_ids: vec![identity_id.into()],
		limit: query.count.unwrap_or(32),
		anchor: query.anchor
			.map(|anchor| anchor.parse::<i64>())
			.transpose()?,
	})
	.await?;
	let followers_res = &unwrap!(followers_res.follows.first());
	let follows = &followers_res.follows;

	// Get user ids
	let mut user_ids = follows
		.iter()
		.filter_map(|f| f.user_id.as_ref())
		.map(common::Uuid::as_uuid)
		.collect::<HashSet<_>>();

	// Wait for an update if needed
	let (update, update_ts) = if let Some(anchor) = watch_index.to_consumer()? {
		let follow_sub = tail_anchor!([ctx, anchor] user_follow::msg::create(identity_id, "*"));
		let unfollow_sub = tail_anchor!([ctx, anchor] user_follow::msg::delete(identity_id, "*"));

		// User presence subs
		let user_presence_subs_select =
			util::future::select_all_or_wait(user_ids.iter().cloned().map(|user_id| {
				tail_anchor!([ctx, anchor] user_presence::msg::update(user_id)).boxed()
			}));

		util::macros::select_with_timeout!({
			event = follow_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(msg.following_user_id.map(FollowConsumerUpdate::FollowCreate), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = unfollow_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(msg.following_user_id.map(FollowConsumerUpdate::FollowRemove), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = user_presence_subs_select => {
				if let TailAnchorResponse::Message(msg) = event? {
					(None, Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
		})
	} else {
		Default::default()
	};
	let update_ts = update_ts.unwrap_or_else(util::timestamp::now);

	// Remove/add new user
	match update {
		Some(FollowConsumerUpdate::FollowCreate(user_id)) => {
			user_ids.insert(user_id.as_uuid());
		}
		Some(FollowConsumerUpdate::FollowRemove(user_id)) => {
			user_ids.remove(&user_id.as_uuid());
		}
		_ => {}
	}

	let identities = fetch::identity::handles(
		ctx.op_ctx(),
		current_user_id,
		user_ids.into_iter().collect::<Vec<_>>(),
	)
	.await?;

	Ok(models::IdentityListFollowingResponse {
		identities,
		anchor: followers_res.anchor.map(|a| a.to_string()),
		watch: WatchResponse::new_as_model(update_ts),
	})
}

// MARK: GET /identities/self/friends
pub async fn friends(
	ctx: Ctx<Auth>,
	watch_index: WatchIndexQuery,
	query: FollowsQuery,
) -> GlobalResult<models::IdentityListFriendsResponse> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	// Fetch follows
	let followers_res = op!([ctx] user_follow_list {
		kind: user_follow::list::request::Kind::Mutual as i32,
		user_ids: vec![current_user_id.into()],
		limit: query.count.unwrap_or(32),
		anchor: query.anchor
			.map(|anchor| anchor.parse::<i64>())
			.transpose()?,
	})
	.await?;
	let followers_res = &unwrap!(followers_res.follows.first());
	let follows = &followers_res.follows;

	// Get user ids
	let mut user_ids = follows
		.iter()
		.filter_map(|f| f.user_id.as_ref())
		.map(common::Uuid::as_uuid)
		.collect::<HashSet<_>>();

	// Wait for an update if needed
	let (update, update_ts) = if let Some(anchor) = watch_index.to_consumer()? {
		let follow_sub =
			tail_anchor!([ctx, anchor] user::msg::mutual_follow_create(current_user_id));
		let unfollow_sub =
			tail_anchor!([ctx, anchor] user::msg::mutual_follow_delete(current_user_id));

		// User presence subs
		let user_presence_subs_select =
			util::future::select_all_or_wait(user_ids.iter().cloned().map(|user_id| {
				tail_anchor!([ctx, anchor] user_presence::msg::update(user_id)).boxed()
			}));

		util::macros::select_with_timeout!({
			event = follow_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(msg.user_b_id.map(FollowConsumerUpdate::FollowCreate), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = unfollow_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(msg.user_b_id.map(FollowConsumerUpdate::FollowRemove), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = user_presence_subs_select => {
				if let TailAnchorResponse::Message(msg) = event? {
					(None, Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
		})
	} else {
		Default::default()
	};
	let update_ts = update_ts.unwrap_or_else(util::timestamp::now);

	// Remove/add new user
	match update {
		Some(FollowConsumerUpdate::FollowCreate(user_id)) => {
			user_ids.insert(user_id.as_uuid());
		}
		Some(FollowConsumerUpdate::FollowRemove(user_id)) => {
			user_ids.remove(&user_id.as_uuid());
		}
		_ => {}
	}

	let identities = fetch::identity::handles(
		ctx.op_ctx(),
		current_user_id,
		user_ids.into_iter().collect::<Vec<_>>(),
	)
	.await?;

	Ok(models::IdentityListFriendsResponse {
		identities,
		anchor: followers_res.anchor.map(|a| a.to_string()),
		watch: WatchResponse::new_as_model(update_ts),
	})
}

// MARK: GET /identities/{}/mutual-friends
pub async fn mutual_friends(
	ctx: Ctx<Auth>,
	identity_id: Uuid,
	_watch_index: WatchIndexQuery,
	query: FollowsQuery,
) -> GlobalResult<models::IdentityListMutualFriendsResponse> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	// Fetch follows
	let mutual_friends_res = op!([ctx] user_mutual_friend_list {
		user_a_id: Some(current_user_id.into()),
		user_b_id: Some(identity_id.into()),
		limit: query.count.unwrap_or(32),
		anchor: query.anchor
			.map(|anchor| anchor.parse::<i64>())
			.transpose()?,
	})
	.await?;
	let anchor = mutual_friends_res.anchor.map(|a| a.to_string());

	// Get user ids
	let user_ids = mutual_friends_res
		.mutual_friends
		.iter()
		.map(|f| Ok(unwrap_ref!(f.user_id).as_uuid()))
		.collect::<GlobalResult<Vec<_>>>()?;

	let identities = fetch::identity::handles(ctx.op_ctx(), current_user_id, user_ids).await?;

	Ok(models::IdentityListMutualFriendsResponse { identities, anchor })
}

// MARK: GET /identities/self/recent-followers
enum RecentFollowConsumerUpdate {
	FollowCreate(common::Uuid),
	FollowRemove(common::Uuid),
}

pub async fn recent_followers(
	ctx: Ctx<Auth>,
	watch_index: WatchIndexQuery,
	query: FollowsQuery,
) -> GlobalResult<models::IdentityListRecentFollowersResponse> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	// Fetch follow requests
	let follow_requests_res = op!([ctx] user_follow_request_list {
		user_ids: vec![current_user_id.into()],
		limit: query.count.unwrap_or(32),
		anchor: query.anchor
			.map(|anchor| anchor.parse::<i64>())
			.transpose()?,
	})
	.await?;
	let follow_requests_res = &unwrap!(follow_requests_res.follows.first());
	let follows = &follow_requests_res.follows;

	// Get user ids
	let mut user_ids = follows
		.iter()
		.filter_map(|f| f.user_id.as_ref())
		.map(common::Uuid::as_uuid)
		.collect::<HashSet<_>>();

	// Wait for an update if needed
	let (update, update_ts) = if let Some(anchor) = watch_index.to_consumer()? {
		let follow_sub = tail_anchor!([ctx, anchor] user_follow::msg::create("*", current_user_id));
		let unfollow_sub =
			tail_anchor!([ctx, anchor] user_follow::msg::delete("*", current_user_id));
		let ignore_sub = tail_anchor!([ctx, anchor] user_follow::msg::request_ignore_complete("*", current_user_id));
		let mutual_sub =
			tail_anchor!([ctx, anchor] user::msg::mutual_follow_create(current_user_id));

		// User presence subs
		let user_presence_subs_select =
			util::future::select_all_or_wait(user_ids.iter().cloned().map(|user_id| {
				tail_anchor!([ctx, anchor] user_presence::msg::update(user_id)).boxed()
			}));

		util::macros::select_with_timeout!({
			event = follow_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(msg.follower_user_id.map(|user_id| {
						if msg.is_mutual {
							RecentFollowConsumerUpdate::FollowRemove(user_id)
						} else {
							RecentFollowConsumerUpdate::FollowCreate(user_id)
						}
					}), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = unfollow_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(msg.follower_user_id.map(RecentFollowConsumerUpdate::FollowRemove), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = ignore_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(msg.follower_user_id.map(RecentFollowConsumerUpdate::FollowRemove), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = mutual_sub => {
				if let TailAnchorResponse::Message(msg) = event? {
					(msg.user_b_id.map(RecentFollowConsumerUpdate::FollowRemove), Some(msg.msg_ts()))
				} else {
					Default::default()
				}
			}
			event = user_presence_subs_select => {
				let event = event?;

				(None, event.msg_ts())
			}
		})
	} else {
		Default::default()
	};
	let update_ts = update_ts.unwrap_or_else(util::timestamp::now);

	// Remove/add new user
	match update {
		Some(RecentFollowConsumerUpdate::FollowCreate(user_id)) => {
			user_ids.insert(user_id.as_uuid());
		}
		Some(RecentFollowConsumerUpdate::FollowRemove(user_id)) => {
			user_ids.remove(&user_id.as_uuid());
		}
		_ => {}
	}

	let identities = fetch::identity::handles(
		ctx.op_ctx(),
		current_user_id,
		user_ids.into_iter().collect::<Vec<_>>(),
	)
	.await?;

	Ok(models::IdentityListRecentFollowersResponse {
		identities,
		anchor: follow_requests_res.anchor.map(|a| a.to_string()),
		watch: WatchResponse::new_as_model(update_ts),
	})
}

// MARK: POST /identities/self/recent-followers/{}/ignore
pub async fn recent_follower_ignore(
	ctx: Ctx<Auth>,
	identity_id: Uuid,
	_body: serde_json::Value,
) -> GlobalResult<serde_json::Value> {
	let (current_user_id, _) = ctx.auth().dual_user(ctx.op_ctx()).await?;

	msg!([ctx] user_follow::msg::request_ignore(identity_id, current_user_id) -> user_follow::msg::request_ignore_complete {
		follower_user_id: Some(identity_id.into()),
		following_user_id: Some(current_user_id.into()),
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
