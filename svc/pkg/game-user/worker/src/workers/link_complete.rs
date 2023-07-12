use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use serde_json::json;

#[worker(name = "game-user-link-complete")]
async fn worker(
	ctx: &OperationContext<game_user::msg::link_complete::Message>,
) -> GlobalResult<()> {
	let crdb = ctx.crdb("db-game-user").await?;

	let user_id = internal_unwrap!(ctx.user_id).as_uuid();
	let link_id = internal_unwrap!(ctx.link_id).as_uuid();

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

	let (old_game_user_id, namespace_id) = sqlx::query_as::<_, (Uuid, Uuid)>(indoc!(
		"
		SELECT current_game_user_id, namespace_id
		FROM links
		WHERE link_id = $1
		"
	))
	.bind(link_id)
	.fetch_one(&crdb)
	.await?;

	let game_user_token = match internal_unwrap!(
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
			let game_user_token = internal_unwrap_owned!(token_res.token.clone());
			let game_user_token_session_id = internal_unwrap!(token_res.session_id).as_uuid();

			// Flag as linked
			let (updated,) = sqlx::query_as::<_, (bool,)>(indoc!(
				"
				WITH
					update_links AS (
						UPDATE links
						SET complete_ts = $1, new_game_user_id = $3, new_game_user_token = $4
						WHERE link_id = $2 AND complete_ts IS NULL AND cancelled_ts IS NULL
						RETURNING 1
					),
					insert_users AS (
						INSERT INTO game_users (game_user_id, user_id, token_session_id, namespace_id, create_ts)
						SELECT $5, $6, $7, $8, $1
						WHERE EXISTS (SELECT 1 FROM update_links)
						RETURNING 1
					)
				SELECT EXISTS (SELECT 1 FROM update_links)
				"
			))
			.bind(ctx.ts())
			// links
			.bind(link_id)
			.bind(new_game_user_id)
			.bind(&game_user_token.token)
			// game_users
			.bind(new_game_user_id)
			.bind(user_id)
			.bind(game_user_token_session_id)
			.bind(namespace_id)
			.fetch_one(&crdb)
			.await?;

			// Catch race condition
			if !updated {
				tracing::info!("game link complete in race condition");

				op!([ctx] token_revoke {
					jtis: vec![game_user_token_session_id.into()],
				})
				.await?;

				panic_with!(GAME_USER_LINK_FAILED);
			}

			msg!([ctx] game_user::msg::switch(old_game_user_id, new_game_user_id) {
				old_game_user_id: Some(old_game_user_id.into()),
				new_game_user_id: Some(new_game_user_id.into()),
			})
			.await?;

			msg!([ctx] analytics::msg::event_create() {
				events: vec![
					analytics::msg::event_create::Event {
						name: "game_user_link.complete".into(),
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

			Some(game_user_token.token)
		}
		game_user::msg::link_complete::GameUserLinkCompleteResolution::Cancel => {
			// Flag as cancelled
			let update_query = sqlx::query(indoc!(
				"
				UPDATE links
				SET cancelled_ts = $2
				WHERE link_id = $1 AND complete_ts IS NULL AND cancelled_ts IS NULL
				"
			))
			.bind(link_id)
			.bind(ctx.ts())
			.execute(&crdb)
			.await?;

			// Catch race condition
			if update_query.rows_affected() == 0 {
				tracing::info!("game link complete in race condition");

				panic_with!(GAME_USER_LINK_FAILED);
			}

			msg!([ctx] analytics::msg::event_create() {
				events: vec![
					analytics::msg::event_create::Event {
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
