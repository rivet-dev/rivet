use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde_json::json;

pub const TOKEN_TTL: i64 = util::duration::minutes(15);

#[operation(name = "game-user-link-create")]
async fn handle(
	ctx: OperationContext<game_user::link_create::Request>,
) -> GlobalResult<game_user::link_create::Response> {
	let crdb = ctx.crdb().await?;

	let game_user_id = unwrap_ref!(ctx.game_user_id).as_uuid();
	let link_id = Uuid::new_v4();

	let game_user_res = op!([ctx] game_user_get {
		game_user_ids: vec![game_user_id.into()],
	})
	.await?;
	let game_user = unwrap!(game_user_res.game_users.first());
	let namespace_id = unwrap_ref!(game_user.namespace_id).as_uuid();

	let token_res = op!([ctx] token_create {
		issuer: Self::NAME.into(),
		token_config: Some(token::create::request::TokenConfig {
			ttl: TOKEN_TTL,
		}),
		refresh_token_config: None,
		client: None,
		kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
			entitlements: vec![
				proto::claims::Entitlement {
					kind: Some(
						proto::claims::entitlement::Kind::GameUserLink(proto::claims::entitlement::GameUserLink {
							link_id: Some(link_id.into()),
						})
					)
				}
			],
		})),
		label: Some("game_user_link".into()),
		..Default::default()
	})
	.await?;

	let user_link_token = unwrap!(token_res.token);
	let token_session_id = unwrap_ref!(token_res.session_id).as_uuid();

	sql_execute!(
		[ctx]
		"
		INSERT INTO db_game_user.links (link_id, namespace_id, token_session_id, current_game_user_id, create_ts)
		VALUES ($1, $2, $3, $4, $5)
		",
		link_id,
		namespace_id,
		token_session_id,
		game_user_id,
		ctx.ts(),
	)
	.await?;

	Ok(game_user::link_create::Response {
		link_id: Some(link_id.into()),
		user_link_token: user_link_token.token.to_owned(),
		token_session_id: Some(token_session_id.into()),
	})
}
