use std::collections::HashSet;

use api_helper::{
	auth::{ApiAuth, AuthRateLimitCtx},
	util::{as_auth_expired, basic_rate_limit},
};
use proto::claims::Claims;
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

	/// Validates that the agent can read a list of teams.
	pub async fn check_teams_read(
		&self,
		ctx: &OperationContext<()>,
		team_ids: Vec<Uuid>,
	) -> GlobalResult<()> {
		let claims = self.claims()?;

		if let Ok(user_ent) = self.user(ctx).await {
			assert::user_registered(ctx, user_ent.user_id).await?;

			let team_list_res = op!([ctx] user_team_list {
				user_ids: vec![user_ent.user_id.into()],
			})
			.await?;

			let user = internal_unwrap_owned!(team_list_res.users.first());
			let user_team_ids = user
				.teams
				.iter()
				.map(|t| Ok(internal_unwrap!(t.team_id).as_uuid()))
				.collect::<GlobalResult<HashSet<_>>>()?;
			let has_teams = team_ids
				.iter()
				.all(|team_id| user_team_ids.contains(team_id));

			assert_with!(has_teams, GROUP_NOT_MEMBER);

			Ok(())
		} else if claims.as_game_cloud().is_ok() {
			panic_with!(
				API_FORBIDDEN,
				reason = "Game cloud token cannot write to this game",
			);
		} else {
			panic_with!(API_UNAUTHORIZED);
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

	/// Validates that the agent is the owner of a given team.
	pub async fn check_team_owner(
		&self,
		ctx: &OperationContext<()>,
		team_id: Uuid,
	) -> GlobalResult<()> {
		if let Ok(user_ent) = self.user(ctx).await {
			assert::user_registered(ctx, user_ent.user_id).await?;

			let res = op!([ctx] team_get {
				team_ids: vec![team_id.into()],
			})
			.await?;

			// Validate the team exists
			let team = internal_unwrap_owned!(res.teams.first());
			let owner_user_id = internal_unwrap!(team.owner_user_id).as_uuid();

			// Verify user's permissions
			assert_eq_with!(
				user_ent.user_id,
				owner_user_id,
				GROUP_INSUFFICIENT_PERMISSIONS
			);

			Ok(())
		} else {
			panic_with!(API_UNAUTHORIZED);
		}
	}

	/// Validates that the agent can read a list of games.
	pub async fn check_games_read(
		&self,
		ctx: &OperationContext<()>,
		game_ids: Vec<Uuid>,
	) -> GlobalResult<()> {
		let claims = self.claims()?;

		if let Ok(user_ent) = self.user(ctx).await {
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
				internal_assert!(!games_res.games.is_empty(), "games not found");

				games_res
					.games
					.iter()
					.map(|g| Ok(internal_unwrap!(g.developer_team_id).as_uuid()))
					.collect::<GlobalResult<Vec<_>>>()?
			};

			// Validate can read teams
			self.check_teams_read(ctx, dev_team_ids).await
		} else if let Ok(cloud_ent) = claims.as_game_cloud() {
			assert_with!(
				game_ids.iter().any(|id| id == &cloud_ent.game_id),
				API_FORBIDDEN,
				reason = "Game cloud token cannot write to this game",
			);

			Ok(())
		} else {
			panic_with!(API_UNAUTHORIZED);
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

		if let Ok(user_ent) = self.user(ctx).await {
			assert::user_registered(ctx, user_ent.user_id).await?;

			// Find the game's development team
			let dev_team_id = {
				let games_res = op!([ctx] game_get {
						game_ids: vec![game_id.into()],
				})
				.await?;
				let game = internal_unwrap_owned!(games_res.games.first(), "game not found");

				internal_unwrap!(game.developer_team_id).as_uuid()
			};

			// Validate can write to the team
			self.check_team_write(ctx, dev_team_id).await
		} else if let Ok(cloud_ent) = claims.as_game_cloud() {
			assert_eq_with!(
				cloud_ent.game_id,
				game_id,
				API_FORBIDDEN,
				reason = "Game cloud token cannot write to this game",
			);

			Ok(())
		} else {
			panic_with!(API_UNAUTHORIZED);
		}
	}

	/// Validates that the agent can read the given games or is an admin.
	pub async fn check_games_read_or_admin(
		&self,
		ctx: &OperationContext<()>,
		game_ids: Vec<Uuid>,
	) -> GlobalResult<()> {
		match self.check_games_read(ctx, game_ids).await {
			Err(err) if err.is(formatted_error::code::API_FORBIDDEN) => self.admin(ctx).await,
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
			Err(err) if err.is(formatted_error::code::API_FORBIDDEN) => self.admin(ctx).await,
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
			Err(err) if err.is(formatted_error::code::API_FORBIDDEN) => self.admin(ctx).await,
			other => other,
		}
	}

	/// Validates that the given agent is an admin user.
	pub async fn admin(&self, ctx: &OperationContext<()>) -> GlobalResult<()> {
		if let Ok(user_ent) = self.user(ctx).await {
			// Get user
			let user_res = op!([ctx] user_get {
				user_ids: vec![user_ent.user_id.into()]
			})
			.await?;

			let user = internal_unwrap_owned!(user_res.users.first(), "user not found");
			assert_with!(user.is_admin, IDENTITY_NOT_ADMIN);

			Ok(())
		} else {
			panic_with!(API_UNAUTHORIZED);
		}
	}

	/// Validates that the given dev team is active.
	pub async fn check_dev_team_active(
		&self,
		ctx: &OperationContext<()>,
		team_id: Uuid,
	) -> GlobalResult<()> {
		let dev_team_res = op!([ctx] team_dev_get {
			team_ids: vec![team_id.into()],
		})
		.await?;
		let dev_team = unwrap_with_owned!(dev_team_res.teams.first(), GROUP_NOT_DEVELOPER_GROUP);

		assert_with!(dev_team.active, GROUP_INVALID_DEVELOPER_STATUS);

		Ok(())
	}

	pub async fn accessible_games(
		&self,
		ctx: &OperationContext<()>,
	) -> GlobalResult<AccessibleGameIdsResponse> {
		let claims = self.claims()?;

		let (user_id, team_ids, game_ids) = if let Ok(user_ent) = self.user(ctx).await {
			// Fetch teams associated with user
			let teams_res = op!([ctx] user_team_list {
				user_ids: vec![user_ent.user_id.into()],
			})
			.await?;
			let user = internal_unwrap_owned!(teams_res.users.first());
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
			let games_res = op!([ctx] team_dev_game_list {
				team_ids: team_ids_proto,
			})
			.await?;

			// Aggregate game IDs in to a single list
			let mut game_ids = Vec::new();
			for team in &games_res.teams {
				for game_id in &team.game_ids {
					let game_id = game_id.as_uuid();
					game_ids.push(game_id);
				}
			}

			(Some(user_ent.user_id), team_ids, game_ids)
		} else if let Ok(cloud_ent) = claims.as_game_cloud() {
			(None, Vec::new(), vec![cloud_ent.game_id])
		} else {
			panic_with!(API_UNAUTHORIZED);
		};

		Ok(AccessibleGameIdsResponse {
			user_id,
			team_ids,
			game_ids,
		})
	}
}

pub struct AccessibleGameIdsResponse {
	pub user_id: Option<Uuid>,
	pub game_ids: Vec<Uuid>,
	pub team_ids: Vec<Uuid>,
}
