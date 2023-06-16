use api_helper::{
	auth::{ApiAuth, AuthRateLimitCtx},
	util::{as_auth_expired, basic_rate_limit},
};
use proto::{backend::pkg::*, claims::Claims};
use rivet_claims::ClaimsDecode;
use rivet_operation::prelude::*;

/// Information derived from the authentication middleware.
pub struct Auth {
	claims: Option<Claims>,
}

#[async_trait]
impl ApiAuth for Auth {
	async fn new(
		api_token: Option<String>,
		rate_limit_ctx: AuthRateLimitCtx<'_>,
	) -> GlobalResult<Auth> {
		Self::rate_limit(rate_limit_ctx).await?;

		Ok(Auth {
			claims: if let Some(api_token) = api_token {
				Some(as_auth_expired(rivet_claims::decode(&api_token)?)?)
			} else {
				None
			},
		})
	}

	async fn rate_limit(rate_limit_ctx: AuthRateLimitCtx<'_>) -> GlobalResult<()> {
		basic_rate_limit(rate_limit_ctx).await
	}
}

impl Auth {
	pub fn claims(&self) -> GlobalResult<&Claims> {
		self.claims
			.as_ref()
			.ok_or_else(|| err_code!(API_UNAUTHORIZED))
	}

	pub async fn user(&self, ctx: &OperationContext<()>) -> GlobalResult<rivet_claims::ent::User> {
		let claims = self.claims()?;
		let user_ent = claims.as_user()?;

		let user_res = op!([ctx] user_get {
			user_ids: vec![user_ent.user_id.into()],
		})
		.await?;
		let user = internal_unwrap_owned!(user_res.users.first());

		// Verify that user is not deleted
		if user.delete_complete_ts.is_some() {
			let jti = internal_unwrap_owned!(claims.jti);
			op!([ctx] token_revoke {
				jtis: vec![jti],
			})
			.await?;

			panic_with!(TOKEN_REVOKED);
		}

		Ok(user_ent)
	}

	pub async fn fetch_game_user(
		&self,
		ctx: &OperationContext<()>,
	) -> GlobalResult<(Uuid, game_user::get::response::GameUser)> {
		let claims = self.claims()?;
		let game_user_ent = claims.as_game_user()?;

		let game_user_res = op!([ctx] game_user_get {
			game_user_ids: vec![game_user_ent.game_user_id.into()]
		})
		.await?;
		let game_user = internal_unwrap_owned!(game_user_res.game_users.first());

		// Verify that game user is not deleted
		if game_user.deleted_ts.is_some() {
			let jti = internal_unwrap_owned!(claims.jti);
			op!([ctx] token_revoke {
				jtis: vec![jti],
			})
			.await?;

			panic_with!(TOKEN_REVOKED);
		}

		Ok((
			internal_unwrap!(game_user.user_id).as_uuid(),
			game_user.clone(),
		))
	}

	pub async fn dual_user(
		&self,
		ctx: &OperationContext<()>,
	) -> GlobalResult<(Uuid, Option<game_user::get::response::GameUser>)> {
		if let Ok(user_ent) = self.user(ctx).await {
			Ok((user_ent.user_id, None))
		} else {
			let (user_id, game_user) = self.fetch_game_user(ctx).await?;

			Ok((user_id, Some(game_user)))
		}
	}
}
