use api_helper::{
	auth::{ApiAuth, AuthRateLimitCtx},
	ctx::Ctx,
	util::{as_auth_expired, basic_rate_limit},
};
use proto::{backend::pkg::*, claims::Claims};
use rivet_claims::ClaimsDecode;
use rivet_operation::prelude::*;

pub struct ResolvedGameUser {
	pub game_user_id: Uuid,
	pub user_id: Uuid,
	pub namespace_id: Uuid,
	pub game_id: Uuid,
}

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

	/// Authenticates with either the public namespace token or the origin header (if allowed).
	pub async fn game_ns(
		&self,
		ctx: &Ctx<Auth>,
	) -> GlobalResult<rivet_claims::ent::GameNamespacePublic> {
		// Attempt to parse existing claim if exists
		if let Some(game_ns) = self
			.claims
			.as_ref()
			.and_then(|claims| claims.as_game_namespace_public_option().transpose())
			.transpose()?
		{
			return Ok(game_ns);
		} else {
			tracing::info!("no ns claims");
		}

		// Attempt to authenticate by the header
		tracing::info!(origin = ?ctx.origin(), "origin");

		if let Some(origin) = ctx.origin() {
			let resolve_res = op!([ctx] game_namespace_resolve_url {
				url: origin.to_string(),
			})
			.await?;
			tracing::info!(res = ?resolve_res, "resolution");

			if let Some(resolution) = &resolve_res.resolution {
				let namespace_id = internal_unwrap!(resolution.namespace_id).as_uuid();

				// Validate that this namespace can be authenticated by domain
				let ns_res = op!([ctx] cdn_namespace_get {
					namespace_ids: vec![namespace_id.into()],
				})
				.await?;

				let cdn_ns = internal_unwrap_owned!(ns_res.namespaces.first());
				let cdn_ns_config = internal_unwrap!(cdn_ns.config);

				if cdn_ns_config.enable_domain_public_auth {
					return Ok(rivet_claims::ent::GameNamespacePublic { namespace_id });
				}
			}
		}

		// Return default error
		panic_with!(
			CLAIMS_MISSING_ENTITLEMENT,
			entitlement = "GameNamespacePublic"
		)
	}

	pub fn game_ns_dev_option(
		&self,
	) -> GlobalResult<Option<rivet_claims::ent::GameNamespaceDevelopment>> {
		if let Some(claims) = &self.claims {
			Ok(claims.as_game_namespace_development_option()?)
		} else {
			Ok(None)
		}
	}

	pub async fn user(&self, ctx: &OperationContext<()>) -> GlobalResult<rivet_claims::ent::User> {
		let claims = self.claims()?;
		let user_ent = claims.as_user()?;

		let user_res = op!([ctx] user_get {
			user_ids: vec![user_ent.user_id.into()],
		})
		.await?;
		let user = internal_unwrap_owned!(user_res.users.first());

		// Verify user is not deleted
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

	pub fn game_user_link_ent(
		&self,
		token: String,
	) -> GlobalResult<(rivet_claims::ent::GameUserLink, Uuid)> {
		// Decode & validate claims
		let claims = rivet_claims::decode(&token)
			.map_err(|_| err_code!(API_FORBIDDEN, reason = "Claims error"))??;
		let game_user_link_ent = claims
			.as_game_user_link()
			.map_err(|_| err_code!(API_FORBIDDEN, reason = "Decode error"))?;
		let token_jti = internal_unwrap_owned!(claims.jti).as_uuid();

		Ok((game_user_link_ent, token_jti))
	}

	pub async fn fetch_game_user(
		&self,
		ctx: &OperationContext<()>,
	) -> GlobalResult<ResolvedGameUser> {
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

		let namespace_id = internal_unwrap!(game_user.namespace_id);

		let namespace_res = op!([ctx] game_namespace_get {
			namespace_ids: vec![*namespace_id]
		})
		.await?;
		let namespace = internal_unwrap_owned!(namespace_res.namespaces.first());

		Ok(ResolvedGameUser {
			game_user_id: game_user_ent.game_user_id,
			user_id: internal_unwrap!(game_user.user_id).as_uuid(),
			namespace_id: namespace_id.as_uuid(),
			game_id: internal_unwrap!(namespace.game_id).as_uuid(),
		})
	}

	pub async fn dual_user(
		&self,
		ctx: &OperationContext<()>,
	) -> GlobalResult<(Uuid, Option<game_user::get::response::GameUser>)> {
		if let Ok(user_ent) = self.user(ctx).await {
			Ok((user_ent.user_id, None))
		} else {
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
				Some(game_user.clone()),
			))
		}
	}

	/// Validates that the given agent is an admin user
	pub async fn admin(&self, ctx: &OperationContext<()>) -> GlobalResult<()> {
		let user_ent = self.user(ctx).await?;

		// Get user
		let user_res = op!([ctx] user_get {
			user_ids: vec![user_ent.user_id.into()]
		})
		.await?;

		let user = internal_unwrap_owned!(user_res.users.first(), "user not found");

		assert_with!(user.is_admin, IDENTITY_NOT_ADMIN);

		Ok(())
	}
}
