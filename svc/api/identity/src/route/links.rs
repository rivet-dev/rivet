use api_helper::{
	anchor::{WatchIndexQuery, WatchResponse},
	ctx::Ctx,
};
use proto::backend::pkg::*;
use rivet_api::models;
use rivet_convert::{convert, fetch, ApiInto};
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{auth::Auth, utils};

// MARK: POST /game-links
pub async fn prepare_game_link(
	ctx: Ctx<Auth>,
	_body: serde_json::Value,
) -> GlobalResult<models::IdentityPrepareGameLinkResponse> {
	let game_user_ent = ctx.auth().fetch_game_user(ctx.op_ctx()).await?;

	let game_user_link_create_res = op!([ctx] game_user_link_create {
		game_user_id: Some(game_user_ent.game_user_id.into()),
	})
	.await?;

	let _link_id = internal_unwrap!(game_user_link_create_res.link_id).as_uuid();
	let identity_link_token = &game_user_link_create_res.user_link_token;
	let identity_link_url = util::route::identity_game_link(identity_link_token);
	let claims = rivet_claims::decode(identity_link_token)??;

	Ok(models::IdentityPrepareGameLinkResponse {
		identity_link_token: identity_link_token.clone(),
		identity_link_url,
		expire_ts: util::timestamp::to_string(internal_unwrap_owned!(claims.exp))?,
	})
}

// MARK: GET /game-links
#[derive(Debug, Serialize, Deserialize)]
pub struct GameLinkQuery {
	identity_link_token: String,
}

pub async fn get_game_link(
	ctx: Ctx<Auth>,
	watch_index: WatchIndexQuery,
	query: GameLinkQuery,
) -> GlobalResult<models::IdentityGetGameLinkResponse> {
	if let Ok((current_user_id, _)) = ctx.auth().dual_user(ctx.op_ctx()).await {
		utils::touch_user_presence(ctx.op_ctx().base(), current_user_id, false);
	}

	let (game_user_link_ent, _) = ctx
		.auth()
		.game_user_link_ent(query.identity_link_token.to_owned())?;
	let link_id = game_user_link_ent.link_id;

	// Wait for an update if needed
	let update_ts = if let Some(anchor) = watch_index.to_consumer()? {
		let game_link_complete_sub =
			tail_anchor!([ctx, anchor] game_user::msg::link_complete_complete(link_id));

		util::macros::select_with_timeout!({
			event = game_link_complete_sub => {
				event?.msg_ts()
			}
		})
	} else {
		Default::default()
	};
	let update_ts = update_ts.unwrap_or_else(util::timestamp::now);

	let game_user_link_status_res = op!([ctx] game_user_link_get {
		link_ids: vec![link_id.into()],
	})
	.await?;

	let game_user_link = internal_unwrap_owned!(game_user_link_status_res.game_user_links.first());
	let current_game_user_id = internal_unwrap!(game_user_link.current_game_user_id).as_uuid();
	let namespace_id = internal_unwrap!(game_user_link.namespace_id).as_uuid();
	let status = internal_unwrap_owned!(
		game_user::link_get::response::GameUserLinkStatus::from_i32(game_user_link.status)
	);

	let (namespace_res, current_game_user) = tokio::try_join!(
		op!([ctx] game_namespace_get {
			namespace_ids: vec![namespace_id.into()]
		}),
		async {
			let game_user_res = op!([ctx] game_user_get {
				game_user_ids: vec![current_game_user_id.into()]
			})
			.await?;
			let game_user = internal_unwrap_owned!(game_user_res.game_users.first());
			let user_id = internal_unwrap!(game_user.user_id).as_uuid();

			// Fetch current identity
			let identities = fetch::identity::handles(ctx.op_ctx(), user_id, vec![user_id]).await?;
			let identity = internal_unwrap_owned!(identities.into_iter().next());

			Ok(identity)
		}
	)?;

	// Fetch game
	let game = {
		let namespace = internal_unwrap_owned!(namespace_res.namespaces.first());
		let game_id = *internal_unwrap!(namespace.game_id);

		let game_res = op!([ctx] game_get {
			game_ids: vec![game_id]
		})
		.await?;
		let game = internal_unwrap_owned!(game_res.games.first());

		convert::game::handle(game)?
	};

	// Fetch new identity
	let new_identity = if let (Some(new_game_user_id), Some(new_game_user_token)) = (
		game_user_link.new_game_user_id,
		&game_user_link.new_game_user_token,
	) {
		let new_game_user_id = new_game_user_id.as_uuid();

		let new_game_user_token_claims = rivet_claims::decode(new_game_user_token)??;

		// Fetch game user
		let game_user_res = op!([ctx] game_user_get {
			game_user_ids: vec![new_game_user_id.into()],
		})
		.await?;
		let game_user = internal_unwrap_owned!(game_user_res.game_users.first());
		let user_id = internal_unwrap!(game_user.user_id).as_uuid();

		// Fetch identity
		let identities =
			fetch::identity::profiles(ctx.op_ctx(), user_id, Some(new_game_user_id), vec![user_id])
				.await?;
		let identity = internal_unwrap_owned!(identities.into_iter().next());

		Some(models::IdentityGetGameLinkNewIdentity {
			identity_token: new_game_user_token.clone(),
			identity_token_expire_ts: util::timestamp::to_string(
				new_game_user_token_claims.exp.unwrap_or(0),
			)?,
			identity: Box::new(identity),
		})
	} else {
		None
	};

	Ok(models::IdentityGetGameLinkResponse {
		status: status.api_into(),
		game: Box::new(game),
		current_identity: Box::new(current_game_user),
		new_identity: new_identity.map(Box::new),
		watch: WatchResponse::new_as_model(update_ts),
	})
}

// MARK: POST /game-links/complete
pub async fn complete_game_link(
	ctx: Ctx<Auth>,
	body: models::IdentityCompleteGameLinkRequest,
) -> GlobalResult<serde_json::Value> {
	let user_ent = ctx.auth().user(ctx.op_ctx()).await?;

	utils::touch_user_presence(ctx.op_ctx().base(), user_ent.user_id, false);

	let (game_user_link_ent, token_jti) = ctx
		.auth()
		.game_user_link_ent(body.identity_link_token.to_owned())?;

	let complete_res = msg!([ctx] game_user::msg::link_complete(game_user_link_ent.link_id) -> Result<game_user::msg::link_complete_complete, game_user::msg::link_complete_fail> {
		user_id: Some(user_ent.user_id.into()),
		link_id: Some(game_user_link_ent.link_id.into()),
		user_link_jti: Some(token_jti.into()),
		resolution: game_user::msg::link_complete::GameUserLinkCompleteResolution::Complete as i32,
	})
	.await?;
	match complete_res {
		Ok(_) => {}
		Err(msg) => {
			use game_user::msg::link_complete_fail::ErrorCode::*;

			let code = game_user::msg::link_complete_fail::ErrorCode::from_i32(msg.error_code);
			match internal_unwrap_owned!(code) {
				Unknown => internal_panic!("unknown link complete error code"),
				TokenExchangeFailed => panic_with!(TOKEN_EXCHANGE_FAILED),
			}
		}
	};

	Ok(serde_json::json!({}))
}

// MARK: POST /game-links/cancel
pub async fn cancel_game_link(
	ctx: Ctx<Auth>,
	body: models::IdentityCancelGameLinkRequest,
) -> GlobalResult<serde_json::Value> {
	let user_ent = ctx.auth().user(ctx.op_ctx()).await?;

	utils::touch_user_presence(ctx.op_ctx().base(), user_ent.user_id, false);

	let (game_user_link_ent, token_jti) = ctx
		.auth()
		.game_user_link_ent(body.identity_link_token.to_owned())?;

	let complete_res = msg!([ctx] game_user::msg::link_complete(game_user_link_ent.link_id) -> Result<game_user::msg::link_complete_complete, game_user::msg::link_complete_fail> {
		user_id: Some(user_ent.user_id.into()),
		link_id: Some(game_user_link_ent.link_id.into()),
		user_link_jti: Some(token_jti.into()),
		resolution: game_user::msg::link_complete::GameUserLinkCompleteResolution::Cancel as i32,
	})
	.await?;
	match complete_res {
		Ok(_) => {}
		Err(msg) => {
			use game_user::msg::link_complete_fail::ErrorCode::*;

			let code = game_user::msg::link_complete_fail::ErrorCode::from_i32(msg.error_code);
			match internal_unwrap_owned!(code) {
				Unknown => internal_panic!("unknown link complete error code"),
				TokenExchangeFailed => panic_with!(TOKEN_EXCHANGE_FAILED),
			}
		}
	};

	Ok(serde_json::json!({}))
}
