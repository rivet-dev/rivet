use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde_json::json;

#[worker(name = "game-user-link-complete")]
async fn worker(
	ctx: &OperationContext<game_user::msg::link_complete::Message>,
) -> GlobalResult<()> {
	let _crdb = ctx.crdb().await?;

	let user_id = unwrap_ref!(ctx.user_id).as_uuid();
	let link_id = unwrap_ref!(ctx.link_id).as_uuid();

	// Exchange token
	let exchange_res = op!([ctx] token_exchange {
		jti: ctx.user_link_jti,
	})
	.await;
	match exchange_res {
		Ok(_) => {}
		Err(err) if err.is(formatted_error::code::TOKEN_EXCHANGE_FAILED) => {
			msg!([ctx] game_user::msg::link_complete_fail(link_id) {
				link_id: Some(link_id.into()),
				error_code: game_user::msg::link_complete_fail::ErrorCode::TokenExchangeFailed as i32,
			})
			.await?;
			return Ok(());
		}
		Err(err) => {
			return Err(err);
		}
	}

	let (old_game_user_id, namespace_id) = sql_fetch_one!(
		[ctx, (Uuid, Uuid)]
		"
		SELECT current_game_user_id, namespace_id
		FROM db_game_user.links
		WHERE link_id = $1
		",
		link_id,
	)
	.await?;

	let game_user_token = match unwrap_ref!(
		game_user::msg::link_complete::GameUserLinkCompleteResolution::from_i32(ctx.resolution)
	) {
		game_user::msg::link_complete::GameUserLinkCompleteResolution::Complete => {
			// Create game user token
			let new_game_user_id = Uuid::new_v4();
			let token_res = op!([ctx] token_create {
				issuer: "game-user-link-complete".into(),
				token_config: Some(token::create::request::TokenConfig {
					ttl: util_game_user::GAME_USER_TOKEN_TTL,
				}),
				refresh_token_config: None,
				client: None,
				kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
					entitlements: vec![
						proto::claims::Entitlement {
							kind: Some(
								proto::claims::entitlement::Kind::GameUser(proto::claims::entitlement::GameUser {
									game_user_id: Some(new_game_user_id.into()),
								})
							)
						}
					],
				})),
				label: Some("game_user".into()),
				combine_refresh_token: true,
				..Default::default()
			})
			.await?;
			let game_user_token = unwrap!(token_res.token.clone());
			let game_user_token_session_id = unwrap_ref!(token_res.session_id).as_uuid();

			// Flag as linked
			let (updated,) = sql_fetch_one!(
				[ctx, (bool,)]
				"
				WITH
					update_links AS (
						UPDATE db_game_user.links
						SET complete_ts = $1, new_game_user_id = $3, new_game_user_token = $4
						WHERE link_id = $2 AND complete_ts IS NULL AND cancelled_ts IS NULL
						RETURNING 1
					),
					insert_users AS (
						INSERT INTO db_game_user.game_users (game_user_id, user_id, token_session_id, namespace_id, create_ts)
						SELECT $5, $6, $7, $8, $1
						WHERE EXISTS (SELECT 1 FROM update_links)
						RETURNING 1
					)
				SELECT EXISTS (SELECT 1 FROM update_links)
				",
				ctx.ts(),
				link_id,
				new_game_user_id,
				&game_user_token.token,
				new_game_user_id,
				user_id,
				game_user_token_session_id,
				namespace_id,
			)
			.await?;

			// Catch race condition
			if !updated {
				tracing::info!("game link complete in race condition");

				op!([ctx] token_revoke {
					jtis: vec![game_user_token_session_id.into()],
				})
				.await?;

				bail_with!(GAME_USER_LINK_FAILED);
			}

			msg!([ctx] game_user::msg::switch(old_game_user_id, new_game_user_id) {
				old_game_user_id: Some(old_game_user_id.into()),
				new_game_user_id: Some(new_game_user_id.into()),
			})
			.await?;

			Some(game_user_token.token)
		}
		game_user::msg::link_complete::GameUserLinkCompleteResolution::Cancel => {
			// Flag as cancelled
			let update_query = sql_execute!(
				[ctx]
				"
				UPDATE db_game_user.links
				SET cancelled_ts = $2
				WHERE link_id = $1 AND complete_ts IS NULL AND cancelled_ts IS NULL
				",
				link_id,
				ctx.ts(),
			)
			.await?;

			// Catch race condition
			if update_query.rows_affected() == 0 {
				tracing::info!("game link complete in race condition");

				bail_with!(GAME_USER_LINK_FAILED);
			}

			msg!([ctx] analytics::msg::event_create() {
				events: vec![
					analytics::msg::event_create::Event {
						event_id: Some(Uuid::new_v4().into()),
						name: "game_user_link.cancel".into(),
						user_id: Some(user_id.into()),
						namespace_id: Some(namespace_id.into()),
						properties_json: Some(serde_json::to_string(&json!({
							"link_id": link_id,
						}))?),
						..Default::default()
					}
				],
			})
			.await?;

			None
		}
	};

	msg!([ctx] game_user::msg::link_complete_complete(link_id) {
		user_id: Some(user_id.into()),
		link_id: Some(link_id.into()),
		game_user_token,
	})
	.await?;

	Ok(())
}
