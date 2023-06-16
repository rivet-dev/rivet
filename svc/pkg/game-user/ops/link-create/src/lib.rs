use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde_json::json;

pub const TOKEN_TTL: i64 = util::duration::minutes(15);

#[operation(name = "game-user-link-create")]
async fn handle(
	ctx: OperationContext<game_user::link_create::Request>,
) -> GlobalResult<game_user::link_create::Response> {
	let crdb = ctx.crdb("db-game-user").await?;

	let game_user_id = internal_unwrap!(ctx.game_user_id).as_uuid();
	let link_id = Uuid::new_v4();

	let game_user_res = op!([ctx] game_user_get {
		game_user_ids: vec![game_user_id.into()],
	})
	.await?;
	let game_user = internal_unwrap_owned!(game_user_res.game_users.first());
	let namespace_id = internal_unwrap!(game_user.namespace_id).as_uuid();

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

	let user_link_token = internal_unwrap_owned!(token_res.token.clone());
	let token_session_id = internal_unwrap!(token_res.session_id).as_uuid();

	sqlx::query(indoc!(
		"
		INSERT INTO links (link_id, namespace_id, token_session_id, current_game_user_id, create_ts)
		VALUES ($1, $2, $3, $4, $5)
		"
	))
	.bind(link_id)
	.bind(namespace_id)
	.bind(token_session_id)
	.bind(game_user_id)
	.bind(ctx.ts())
	.execute(&crdb)
	.await?;

	msg!([ctx] analytics::msg::event_create() {
		events: vec![
			analytics::msg::event_create::Event {
				name: "game_user_link.create".into(),
				user_id: game_user.user_id,
				namespace_id: game_user.namespace_id,
				properties_json: Some(serde_json::to_string(&json!({
					"link_id": link_id,
				}))?),
				..Default::default()
			}
		],
	})
	.await?;

	Ok(game_user::link_create::Response {
		link_id: Some(link_id.into()),
		user_link_token: user_link_token.token.to_owned(),
		token_session_id: Some(token_session_id.into()),
	})
}
