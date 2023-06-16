use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "cloud-game-token-create")]
async fn handle(
	ctx: OperationContext<cloud::game_token_create::Request>,
) -> GlobalResult<cloud::game_token_create::Response> {
	let game_id = internal_unwrap!(ctx.game_id).as_uuid();

	let token_res = op!([ctx] token_create {
		issuer: Self::NAME.into(),
		token_config: Some(token::create::request::TokenConfig {
			ttl: util::duration::days(365),
		}),
		refresh_token_config: None,
		client: None,
		kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
			entitlements: vec![
				proto::claims::Entitlement {
					kind: Some(
						proto::claims::entitlement::Kind::GameCloud(proto::claims::entitlement::GameCloud {
							game_id: Some(game_id.into())
						})
					)
				}
			],
		})),
		label: Some("cloud".into()),
		..Default::default()
	})
	.await?;

	let token = internal_unwrap!(token_res.token);
	let token_session_id = internal_unwrap!(token_res.session_id).as_uuid();

	sqlx::query("INSERT INTO game_cloud_tokens (game_id, token_session_id) VALUES ($1, $2)")
		.bind(game_id)
		.bind(token_session_id)
		.execute(&ctx.crdb("db-cloud").await?)
		.await?;

	Ok(cloud::game_token_create::Response {
		token: token.token.clone(),
	})
}
