use api_helper::{
	auth::{ApiAuth, AuthRateLimitCtx},
	ctx::Ctx,
	util::{as_auth_expired, basic_rate_limit},
};
use proto::claims::Claims;
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
			.ok_or_else(|| err_code!(API_UNAUTHORIZED, reason = "No bearer token provided."))
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

	pub fn lobby(&self) -> GlobalResult<rivet_claims::ent::MatchmakerLobby> {
		self.claims()?.as_matchmaker_lobby()
	}

	pub fn player(&self) -> GlobalResult<rivet_claims::ent::MatchmakerPlayer> {
		self.claims()?.as_matchmaker_player()
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
}
