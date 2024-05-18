use std::collections::HashSet;

use api_helper::{
	auth::{ApiAuth, AuthRateLimitCtx},
	util::{as_auth_expired, basic_rate_limit},
};
use proto::{backend, claims::Claims};
use rivet_claims::ClaimsDecode;
use rivet_operation::prelude::*;

use crate::assert;

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

	pub async fn user(
		&self,
		ctx: &OperationContext<()>,
	) -> GlobalResult<(backend::user::User, rivet_claims::ent::User)> {
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

		Ok((user.clone(), user_ent))
	}

	/// Validates that the agent can read a list of teams.
	pub async fn check_teams_read(
		&self,
		ctx: &OperationContext<()>,
		team_ids: Vec<Uuid>,
	) -> GlobalResult<()> {
		let claims = self.claims()?;

		if claims.as_user().is_ok() {
			let (user, user_ent) = self.user(ctx).await?;
			assert::user_registered(ctx, user_ent.user_id).await?;

			let team_list_res = op!([ctx] user_team_list {
				user_ids: vec![user_ent.user_id.into()],
			})
			.await?;

			let user_teams = unwrap!(team_list_res.users.first());
			let user_team_ids = user_teams
				.teams
				.iter()
				.map(|t| Ok(unwrap_ref!(t.team_id).as_uuid()))
				.collect::<GlobalResult<HashSet<_>>>()?;
			let has_teams = team_ids
				.iter()
				.all(|team_id| user_team_ids.contains(team_id));

			ensure_with!(has_teams || user.is_admin, GROUP_NOT_MEMBER);

			Ok(())
		} else if claims.as_game_cloud().is_ok() {
			bail_with!(
				API_FORBIDDEN,
				reason = "Game cloud token cannot write to this game",
			);
		} else {
			bail_with!(CLAIMS_MISSING_ENTITLEMENT, entitlements = "User, GameCloud");
		}
	}

	/// Validates that the agent can read a given team.
	pub async fn check_team_read(
		&self,
		ctx: &OperationContext<()>,
		team_id: Uuid,
	) -> GlobalResult<()> {
		self.check_teams_read(ctx, vec![team_id]).await
	}

	/// Validates that the agent can write to a given team.
	pub async fn check_team_write(
		&self,
		ctx: &OperationContext<()>,
		team_id: Uuid,
	) -> GlobalResult<()> {
		tokio::try_join!(
			self.check_team_read(ctx, team_id),
			self.check_dev_team_active(ctx, team_id)
		)?;

		Ok(())
	}

	/// Validates that the agent can read a list of games.
	pub async fn check_games_read(
		&self,
		ctx: &OperationContext<()>,
		game_ids: Vec<Uuid>,
	) -> GlobalResult<()> {
		let claims = self.claims()?;

		if claims.as_user().is_ok() {
			let (_user, user_ent) = self.user(ctx).await?;

			assert::user_registered(ctx, user_ent.user_id).await?;

			// Find the game's development teams
			let dev_team_ids = {
				let games_res = op!([ctx] game_get {
					game_ids: game_ids
						.into_iter()
						.map(Into::into)
						.collect::<Vec<_>>(),
				})
				.await?;
				ensure!(!games_res.games.is_empty(), "games not found");

				games_res
					.games
					.iter()
					.map(|g| Ok(unwrap_ref!(g.developer_team_id).as_uuid()))
					.collect::<GlobalResult<Vec<_>>>()?
			};

			// Validate can read teams
			self.check_teams_read(ctx, dev_team_ids).await
		} else if let Ok(cloud_ent) = claims.as_game_cloud() {
			ensure_with!(
				game_ids.iter().any(|id| id == &cloud_ent.game_id),
				API_FORBIDDEN,
				reason = "Game cloud token cannot write to this game",
			);

			Ok(())
		} else {
			bail_with!(CLAIMS_MISSING_ENTITLEMENT, entitlements = "User, GameCloud");
		}
	}

	/// Validates that the agent can read a given game.
	pub async fn check_game_read(
		&self,
		ctx: &OperationContext<()>,
		game_id: Uuid,
	) -> GlobalResult<()> {
		self.check_games_read(ctx, vec![game_id]).await
	}

	/// Validates that the agent can write to a given game.
	pub async fn check_game_write(
		&self,
		ctx: &OperationContext<()>,
		game_id: Uuid,
	) -> GlobalResult<()> {
		let claims = self.claims()?;

		if claims.as_user().is_ok() {
			let (_user, user_ent) = self.user(ctx).await?;

			assert::user_registered(ctx, user_ent.user_id).await?;

			// Find the game's development team
			let dev_team_id = {
				let games_res = op!([ctx] game_get {
						game_ids: vec![game_id.into()],
				})
				.await?;
				let game = unwrap!(games_res.games.first(), "game not found");

				unwrap_ref!(game.developer_team_id).as_uuid()
			};

			// Validate can write to the team
			self.check_team_write(ctx, dev_team_id).await
		} else if let Ok(cloud_ent) = claims.as_game_cloud() {
			ensure_eq_with!(
				cloud_ent.game_id,
				game_id,
				API_FORBIDDEN,
				reason = "Game cloud token cannot write to this game",
			);

			Ok(())
		} else {
			bail_with!(CLAIMS_MISSING_ENTITLEMENT, entitlements = "User, GameCloud");
		}
	}

	/// Validates that the given dev team is active.
	pub async fn check_dev_team_active(
		&self,
		ctx: &OperationContext<()>,
		team_id: Uuid,
	) -> GlobalResult<()> {
		let team_res = op!([ctx] team_get {
			team_ids: vec![team_id.into()],
		})
		.await?;
		let team = unwrap!(team_res.teams.first());

		ensure_with!(
			team.deactivate_reasons.is_empty(),
			GROUP_DEACTIVATED,
			reasons = util_team::format_deactivate_reasons(&team.deactivate_reasons)?,
		);

		Ok(())
	}

	pub async fn accessible_games(
		&self,
		ctx: &OperationContext<()>,
	) -> GlobalResult<AccessibleGameIdsResponse> {
		let claims = self.claims()?;

		let (user_id, team_ids, game_ids) = if claims.as_user().is_ok() {
			let (_, user_ent) = self.user(ctx).await?;

			// Fetch teams associated with user
			let teams_res = op!([ctx] user_team_list {
				user_ids: vec![user_ent.user_id.into()],
			})
			.await?;
			let user = unwrap!(teams_res.users.first());
			let team_ids_proto = user
				.teams
				.iter()
				.filter_map(|t| t.team_id)
				.collect::<Vec<common::Uuid>>();
			let team_ids = team_ids_proto
				.iter()
				.map(common::Uuid::as_uuid)
				.collect::<Vec<_>>();

			// Fetch games associated with teams
			let games_res = op!([ctx] game_list_for_team {
				team_ids: team_ids_proto,
			})
			.await?;

			let game_ids = games_res
				.teams
				.iter()
				.flat_map(|team| &team.game_ids)
				.map(|id| id.as_uuid())
				.collect::<Vec<_>>();

			(Some(user_ent.user_id), team_ids, game_ids)
		} else if let Ok(cloud_ent) = claims.as_game_cloud() {
			(None, Vec::new(), vec![cloud_ent.game_id])
		} else {
			bail_with!(CLAIMS_MISSING_ENTITLEMENT, entitlements = "User, GameCloud");
		};

		Ok(AccessibleGameIdsResponse {
			user_id,
			team_ids,
			game_ids,
		})
	}
}

// Admin
impl Auth {
	/// Validates that the agent can read the given games or is an admin.
	pub async fn check_games_read_or_admin(
		&self,
		ctx: &OperationContext<()>,
		game_ids: Vec<Uuid>,
	) -> GlobalResult<()> {
		match self.check_games_read(ctx, game_ids).await {
			Err(err) if err.is(formatted_error::code::API_FORBIDDEN) => self.or_admin(ctx).await,
			other => other,
		}
	}

	/// Validates that the agent can read the given team or is an admin.
	pub async fn check_team_read_or_admin(
		&self,
		ctx: &OperationContext<()>,
		team_id: Uuid,
	) -> GlobalResult<()> {
		match self.check_team_read(ctx, team_id).await {
			Err(err) if err.is(formatted_error::code::API_FORBIDDEN) => self.or_admin(ctx).await,
			other => other,
		}
	}

	/// Validates that the agent can write the given team or is an admin.
	pub async fn check_team_write_or_admin(
		&self,
		ctx: &OperationContext<()>,
		team_id: Uuid,
	) -> GlobalResult<()> {
		match self.check_team_write(ctx, team_id).await {
			Err(err) if err.is(formatted_error::code::API_FORBIDDEN) => self.or_admin(ctx).await,
			other => other,
		}
	}

	/// Validates that the agent can read the given game or is an admin.
	pub async fn check_game_read_or_admin(
		&self,
		ctx: &OperationContext<()>,
		game_id: Uuid,
	) -> GlobalResult<()> {
		match self.check_game_read(ctx, game_id).await {
			Err(err) if err.is(formatted_error::code::API_FORBIDDEN) => self.or_admin(ctx).await,
			other => other,
		}
	}

	/// Validates that the agent can write the given game or is an admin.
	pub async fn check_game_write_or_admin(
		&self,
		ctx: &OperationContext<()>,
		game_id: Uuid,
	) -> GlobalResult<()> {
		match self.check_game_write(ctx, game_id).await {
			Err(err) if err.is(formatted_error::code::API_FORBIDDEN) => self.or_admin(ctx).await,
			other => other,
		}
	}

	/// Validates that the given agent is an admin user.
	pub async fn admin(&self, ctx: &OperationContext<()>) -> GlobalResult<()> {
		let claims = self.claims()?;

		if claims.as_user().is_ok() {
			let (user, _) = self.user(ctx).await?;

			ensure_with!(user.is_admin, IDENTITY_NOT_ADMIN);

			Ok(())
		} else {
			bail_with!(CLAIMS_MISSING_ENTITLEMENT, entitlements = "User");
		}
	}

	// Helper function
	async fn or_admin(&self, ctx: &OperationContext<()>) -> GlobalResult<()> {
		match self.admin(ctx).await {
			Err(err)
				if err.is(formatted_error::code::API_FORBIDDEN)
					|| err.is(formatted_error::code::IDENTITY_NOT_ADMIN) =>
			{
				bail_with!(CLAIMS_MISSING_ENTITLEMENT, entitlements = "User, GameCloud");
			}
			other => other,
		}
	}
}

pub struct AccessibleGameIdsResponse {
	pub user_id: Option<Uuid>,
	pub game_ids: Vec<Uuid>,
	pub team_ids: Vec<Uuid>,
}
