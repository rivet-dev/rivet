use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[operation(name = "mm-dev-player-token-create")]
async fn handle(
	ctx: OperationContext<mm::dev_player_token_create::Request>,
) -> GlobalResult<mm::dev_player_token_create::Response> {
	let namespace_id = internal_unwrap!(ctx.namespace_id).as_uuid();
	let player_id = internal_unwrap!(ctx.player_id).as_uuid();

	let token_res = op!([ctx] token_create {
			issuer: Self::NAME.into(),
			token_config: Some(token::create::request::TokenConfig {
				// This has to be longer than the player ready timeout since we
				// use this token to disconnect players too. If the token is
				// expired when the player disconnects, the lobby will leak.
				ttl: util::duration::days(365),
			}),
			refresh_token_config: None,
			client: None,
			kind: Some(token::create::request::Kind::New(token::create::request::KindNew {
				entitlements: vec![
					proto::claims::Entitlement {
						kind: Some(
							proto::claims::entitlement::Kind::MatchmakerDevelopmentPlayer(proto::claims::entitlement::MatchmakerDevelopmentPlayer {
								namespace_id: Some(namespace_id.into()),
								player_id: Some(player_id.into()),
							})
						)
					}
				],
			})),
			label: Some("dev_player".into()),
			..Default::default()
		})
		.await?;
	let token = internal_unwrap!(token_res.token);

	Ok(mm::dev_player_token_create::Response {
		player_jwt: token.token.clone(),
	})
}
