mod common;

use common::*;

use rivet_api::{apis::*, models};
use rivet_operation::prelude::*;
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn create_custom_lobby() {
	let ctx = Ctx::init().await;

	{
		tracing::info!("creating custom lobby");

		let res = matchmaker_lobbies_api::matchmaker_lobbies_create(
			&ctx.config(ctx.ns_auth_token.clone()),
			models::MatchmakerLobbiesCreateRequest {
				game_mode: LOBBY_GROUP_NAME_ID_BRIDGE.to_string(),
				region: Some(ctx.primary_region_name_id.clone()),
				publicity: Some(models::MatchmakerCustomLobbyPublicity::Public),
				lobby_config: Some(Some(json!({ "foo": "bar" }))),
				verification_data: None,
				tags: None,
				max_players: None,
				captcha: Some(Box::new(models::CaptchaConfig {
					hcaptcha: Some(Box::new(models::CaptchaConfigHcaptcha {
						client_response: "10000000-aaaa-bbbb-cccc-000000000001".to_string(),
					})),
					turnstile: None,
				})),
			},
		)
		.await
		.unwrap();

		assert_lobby_state(&ctx, &res.lobby).await;
	}
}
