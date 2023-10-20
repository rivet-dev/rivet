use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "game-user-create")]
async fn handle(
	ctx: OperationContext<game_user::create::Request>,
) -> GlobalResult<game_user::create::Response> {
	let crdb = ctx.crdb().await?;

	let game_user_id = Uuid::new_v4();
	let namespace_id = unwrap_ref!(ctx.namespace_id).as_uuid();
	let user_id = unwrap_ref!(ctx.user_id).as_uuid();

	let token_res = op!([ctx] token_create {
		issuer: Self::NAME.into(),
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
							game_user_id: Some(game_user_id.into()),
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
	let token_session_id = unwrap_ref!(token_res.session_id).as_uuid();

	sqlx::query(indoc!(
		"
		INSERT INTO db_game_user.game_users (game_user_id, user_id, token_session_id, namespace_id, create_ts)
		VALUES ($1, $2, $3, $4, $5)
		"
	))
	.bind(game_user_id)
	.bind(user_id)
	.bind(token_session_id)
	.bind(namespace_id)
	.bind(ctx.ts())
	.execute(&crdb)
	.await?;

	Ok(game_user::create::Response {
		token: game_user_token.token.to_owned(),
		token_session_id: Some(token_session_id.into()),
		game_user_id: Some(game_user_id.into()),
		user_id: Some(user_id.into()),
	})
}
