use api_helper::{
	auth::{ApiAuth, AuthRateLimitCtx},
	util::{as_auth_expired, basic_rate_limit},
};
use proto::claims::Claims;
use rivet_claims::ClaimsDecode;
use rivet_operation::prelude::*;

use crate::{assert, utils};

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

	pub async fn user(&self, ctx: &OperationContext<()>) -> GlobalResult<rivet_claims::ent::User> {
		let claims = self.claims()?;
		let user_ent = claims.as_user()?;

		let user_res = op!([ctx] user_get {
			user_ids: vec![user_ent.user_id.into()],
		})
		.await?;
		let user = unwrap!(user_res.users.first());

		// Verify user is not deleted
		if user.delete_complete_ts.is_some() {
			let jti = unwrap!(claims.jti);
			op!([ctx] token_revoke {
				jtis: vec![jti],
			})
			.await?;

			bail_with!(TOKEN_REVOKED);
		}

		Ok(user_ent)
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

	/// Validates that the agent can write to a given game.
	pub async fn namespace(
		&self,
		ctx: &OperationContext<()>,
		namespace_id: Option<Uuid>,
		allow_users: bool,
	) -> GlobalResult<Uuid> {
		if let Some(namespace_id) = namespace_id {
			self.namespace_from_cloud(ctx, namespace_id).await
		} else if let Some(x) = self.game_ns_dev_option()? {
			Ok(x.namespace_id)
		} else {
			self.namespace_from_user_or_lobby(ctx, allow_users).await
		}
	}

	/// Validates that the agent can write to a given game.
	pub async fn namespace_from_user_or_lobby(
		&self,
		ctx: &OperationContext<()>,
		allow_users: bool,
	) -> GlobalResult<Uuid> {
		let claims = self.claims()?;

		let namespace_id = if let Ok(lobby_ent) = claims.as_matchmaker_lobby() {
			let lobbies_res = op!([ctx] mm_lobby_get {
				lobby_ids: vec![lobby_ent.lobby_id.into()],
				include_stopped: false,
			})
			.await?;

			let lobby = unwrap!(lobbies_res.lobbies.first());

			unwrap!(lobby.namespace_id)
		} else if allow_users {
			let game_user_ent = claims.as_game_user()?;

			let game_users_res = op!([ctx] game_user_get {
				game_user_ids: vec![game_user_ent.game_user_id.into()],
			})
			.await?;
			let game_user = unwrap!(game_users_res.game_users.first());

			// Verify that game user is not deleted
			if game_user.deleted_ts.is_some() {
				let jti = unwrap!(claims.jti);
				op!([ctx] token_revoke {
					jtis: vec![jti],
				})
				.await?;

				bail_with!(TOKEN_REVOKED);
			}

			*unwrap_ref!(game_user.namespace_id)
		} else {
			bail_with!(
				CLAIMS_MISSING_ENTITLEMENT,
				entitlements = if allow_users {
					"User, Lobby"
				} else {
					"Lobby"
				}
			);
		};

		utils::validate_config(ctx, namespace_id).await?;

		Ok(namespace_id.as_uuid())
	}

	/// Validates that the agent can write to a given game.
	pub async fn namespace_from_cloud(
		&self,
		ctx: &OperationContext<()>,
		namespace_id: Uuid,
	) -> GlobalResult<Uuid> {
		let claims = self.claims()?;

		// Pre-fetch entitlements so we don't fetch the namespace if there is no ent
		let (user_ent, cloud_ent) = if let Ok(ent) = claims.as_user() {
			let user_ent = self.user(ctx).await?;

			(Some(user_ent), None)
		} else if let Ok(ent) = claims.as_game_cloud() {
			(None, Some(ent))
		} else {
			bail_with!(
				CLAIMS_MISSING_ENTITLEMENT,
				entitlements = "User, GameCloud"
			);
		};

		let namespaces_res = op!([ctx] game_namespace_get {
			namespace_ids: vec![namespace_id.into()],
		})
		.await?;
		let namespace = unwrap!(namespaces_res.namespaces.first());
		let game_id = unwrap_ref!(namespace.game_id);

		if let Some(user_ent) = user_ent {
			assert::user_registered(ctx, user_ent.user_id).await?;

			// Find the game's development team
			let dev_team_id = {
				let games_res = op!([ctx] game_get {
					game_ids: vec![*game_id],
				})
				.await?;
				let game = unwrap!(games_res.games.first(), "game not found");
				let dev_team_id = unwrap_ref!(game.developer_team_id);

				*dev_team_id
			};

			// Validate can write to the team
			let user_team_list_res = op!([ctx] user_team_list {
				user_ids: vec![user_ent.user_id.into()],
			})
			.await?;
			let user = unwrap!(user_team_list_res.users.first());

			let has_team = user
				.teams
				.iter()
				.any(|t| t.team_id.as_ref() == Some(&dev_team_id));

			ensure_with!(has_team, GROUP_NOT_MEMBER);
		} else if let Some(cloud_ent) = cloud_ent {
			ensure_eq_with!(
				cloud_ent.game_id,
				game_id.as_uuid(),
				API_FORBIDDEN,
				reason = "Game cloud token cannot write to this game",
			);
		}

		utils::validate_config(ctx, namespace_id.into()).await?;

		Ok(namespace_id)
	}
}
