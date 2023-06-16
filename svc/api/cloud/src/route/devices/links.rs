use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use chirp_client::TailAnchorResponse;
use proto::backend::pkg::*;
use rivet_api::models;
use rivet_claims::ClaimsDecode;
use rivet_operation::prelude::*;
use serde::{Deserialize, Serialize};

use crate::auth::Auth;

// MARK: POST /devices/links
pub async fn prepare(
	ctx: Ctx<Auth>,
	_body: serde_json::Value,
) -> GlobalResult<models::CloudDevicesPrepareDeviceLinkResponse> {
	// No auth required since the device doesn't have a token yet

	let create_res = op!([ctx] cloud_device_link_create {}).await?;

	Ok(models::CloudDevicesPrepareDeviceLinkResponse {
		device_link_id: internal_unwrap!(create_res.device_link_id).as_uuid(),
		device_link_token: create_res.token.clone(),
		device_link_url: util::route::cloud_device_link(&create_res.token),
	})
}

// MARK: GET /devices/links
#[derive(Debug, Serialize, Deserialize)]
pub struct GetQuery {
	device_link_token: String,
}

pub async fn get(
	ctx: Ctx<Auth>,
	watch_index: WatchIndexQuery,
	query: GetQuery,
) -> GlobalResult<models::CloudDevicesGetDeviceLinkResponse> {
	// No auth required since the device doesn't have a token yet

	// Decode device link token
	let claims = rivet_claims::decode(&query.device_link_token)??.as_cloud_device_link()?;
	let link_id = claims.device_link_id;

	// Check for complete message
	let complete_msg = if let Some(anchor) = watch_index.to_consumer()? {
		let complete_sub =
			tail_anchor!([ctx, anchor] cloud::msg::device_link_complete_complete(link_id));

		util::macros::select_with_timeout!({
			msg = complete_sub => {
				if let TailAnchorResponse::Message(msg) = msg? {
					Some(msg)
				} else {
					None
				}
			}
		})
	} else {
		tail_read!([ctx] cloud::msg::device_link_complete_complete(link_id)).await?
	};
	let update_ts = complete_msg
		.as_ref()
		.map_or_else(util::timestamp::now, |x| x.msg_ts());

	// Get cloud token from the message
	let cloud_token = complete_msg.as_ref().map(|x| x.cloud_token.clone());

	Ok(models::CloudDevicesGetDeviceLinkResponse {
		cloud_token,
		watch: Box::new(models::WatchResponse {
			index: (update_ts + 1).to_string(),
		}),
	})
}

// MARK: POST /devices/links/complete
pub async fn complete(
	ctx: Ctx<Auth>,
	body: models::CloudDevicesCompleteDeviceLinkRequest,
) -> GlobalResult<serde_json::Value> {
	// Verify completer is a user. Cloud tokens should not be able to link other
	// cloud tokens.
	let rivet_claims::ent::User { .. } = ctx.auth().claims()?.as_user()?;

	// Verify has access to game
	ctx.auth()
		.check_game_write(ctx.op_ctx(), body.game_id)
		.await?;

	// Decode device link token
	let claims = rivet_claims::decode(&body.device_link_token)??.as_cloud_device_link()?;
	let link_id = claims.device_link_id;

	// Check the link isn't complete already
	let link_complete_msg = tail_read!([ctx] cloud::msg::device_link_complete(link_id)).await?;
	if link_complete_msg.is_some() {
		panic_with!(CLOUD_DEVICE_LINK_ALREADY_COMPLETE)
	}

	// Publish link complete message
	msg!([ctx] cloud::msg::device_link_complete(link_id) {
		device_link_id: Some(link_id.into()),
		game_id: Some(body.game_id.into()),
	})
	.await?;

	Ok(serde_json::json!({}))
}
