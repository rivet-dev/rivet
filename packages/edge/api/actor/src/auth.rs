use api_helper::{
	auth::{ApiAuth, AuthRateLimitCtx},
	util::{as_auth_expired, basic_rate_limit},
};
use proto::claims::Claims;
use rivet_claims::ClaimsDecode;
use rivet_operation::prelude::*;

use crate::route::GlobalQuery;

pub struct Auth {
	claims: Option<Claims>,
}

pub struct CheckOpts<'a> {
	pub query: &'a GlobalQuery,
	pub allow_service_token: bool,
	pub opt_auth: bool,
}

pub struct CheckOutput {
	pub game_id: Uuid,
	pub env_id: Uuid,
}

#[async_trait]
impl ApiAuth for Auth {
	async fn new(
		config: rivet_config::Config,
		api_token: Option<String>,
		rate_limit_ctx: AuthRateLimitCtx<'_>,
	) -> GlobalResult<Auth> {
		Self::rate_limit(&config, rate_limit_ctx).await?;

		Ok(Auth {
			claims: if let Some(api_token) = api_token {
				Some(as_auth_expired(rivet_claims::decode(
					&config.server()?.jwt.public,
					&api_token,
				)?)?)
			} else {
				None
			},
		})
	}

	async fn rate_limit(
		config: &rivet_config::Config,
		rate_limit_ctx: AuthRateLimitCtx<'_>,
	) -> GlobalResult<()> {
		basic_rate_limit(config, rate_limit_ctx).await
	}
}

impl Auth {
	pub fn claims(&self) -> GlobalResult<&Claims> {
		self.claims
			.as_ref()
			.ok_or_else(|| err_code!(API_UNAUTHORIZED, reason = "No bearer token provided."))
	}

	pub fn env_service(&self) -> GlobalResult<rivet_claims::ent::EnvService> {
		self.claims()?.as_env_service()
	}

	/// Check if the provided token is authenticated for the provided project & env.
	///
	/// If in development mode:
	/// - The project & env will fallback to "default" if not provided
	/// - The auth token will always pass
	pub async fn check(
		&self,
		ctx: &OperationContext<()>,
		opts: CheckOpts<'_>,
	) -> GlobalResult<CheckOutput> {
		let is_development = ctx.config().server()?.rivet.auth.access_kind
			== rivet_config::config::rivet::AccessKind::Development;

		let (project_query, environment_query) = opts.query.project_and_env()?;

		// Lookup project name ID
		let project = if is_development {
			project_query.unwrap_or(util::dev_defaults::PROJECT_SLUG)
		} else {
			unwrap_with!(project_query, PROJECT_NOT_FOUND)
		};
		let game_res = op!([ctx] game_resolve_name_id {
			name_ids: vec![project.to_string()],
		})
		.await?;
		let game = unwrap_with!(game_res.games.first(), PROJECT_NOT_FOUND);
		let game_id = unwrap!(game.game_id).as_uuid();

		// Lookup environment name ID
		let environment = if is_development {
			environment_query.unwrap_or(util::dev_defaults::ENVIRONMENT_SLUG)
		} else {
			unwrap_with!(environment_query, ENVIRONMENT_NOT_FOUND)
		};
		let env_res = op!([ctx] game_namespace_resolve_name_id {
			game_id: game.game_id,
			name_ids: vec![environment.to_string()],
		})
		.await?;
		let env = unwrap_with!(env_res.namespaces.first(), ENVIRONMENT_NOT_FOUND);
		let env_id = unwrap!(env.namespace_id).as_uuid();

		// Get the game this env belongs to
		let ns_res = op!([ctx] game_namespace_get {
			namespace_ids: vec![env_id.into()],
		})
		.await?;
		let env = unwrap_with!(ns_res.namespaces.first(), ENVIRONMENT_NOT_FOUND);

		// Ensure belongs to game
		ensure_with!(
			unwrap!(env.game_id).as_uuid() == game_id,
			ENVIRONMENT_NOT_FOUND
		);

		// Build output
		let output = CheckOutput { game_id, env_id };

		// Skip auth if in development mode
		if self.claims.is_none() && is_development {
			return Ok(output);
		}

		// Skip auth if not needed
		if self.claims.is_none() && opts.opt_auth {
			return Ok(output);
		}

		// Validate claims
		let claims = self.claims()?;

		// Validate token
		if let Ok(cloud_ent) = claims.as_game_cloud() {
			ensure_with!(
				cloud_ent.game_id == game_id,
				API_FORBIDDEN,
				reason = "Cloud token cannot write to this game",
			);
			Ok(output)
		} else if let Ok(service_ent) = claims.as_env_service() {
			ensure_with!(
				opts.allow_service_token,
				API_FORBIDDEN,
				reason = "Cannot use service token for this endpoint."
			);
			ensure_with!(
				service_ent.env_id == env_id,
				API_FORBIDDEN,
				reason = "Service token cannot write to this game",
			);
			Ok(output)
		} else if let Ok(user_ent) = claims.as_user() {
			// Get the user
			let (user_res, game_res, team_list_res) = tokio::try_join!(
				op!([ctx] user_get {
					user_ids: vec![user_ent.user_id.into()],
				}),
				op!([ctx] game_get {
					game_ids: vec![game_id.into()],
				}),
				op!([ctx] user_team_list {
					user_ids: vec![user_ent.user_id.into()],
				}),
			)?;
			let Some(user) = user_res.users.first() else {
				bail_with!(TOKEN_REVOKED)
			};
			let game = unwrap_with!(game_res.games.first(), PROJECT_NOT_FOUND);
			let user_teams = unwrap!(team_list_res.users.first());
			let dev_team_id = unwrap_ref!(game.developer_team_id).as_uuid();

			// Allow admin
			if user.is_admin {
				return Ok(output);
			}

			// Verify user is not deleted
			ensure_with!(user.delete_complete_ts.is_none(), TOKEN_REVOKED);

			// Validate user is member of team
			let is_part_of_team = user_teams
				.teams
				.iter()
				.filter_map(|x| x.team_id)
				.any(|x| x.as_uuid() == dev_team_id);
			ensure_with!(is_part_of_team, GROUP_NOT_MEMBER);

			// Get team
			let team_res = op!([ctx] team_get {
				team_ids: vec![dev_team_id.into()],
			})
			.await?;
			let dev_team = unwrap!(team_res.teams.first());

			// Check team active
			ensure_with!(
				dev_team.deactivate_reasons.is_empty(),
				GROUP_DEACTIVATED,
				reasons = util_team::format_deactivate_reasons(&dev_team.deactivate_reasons)?,
			);

			Ok(output)
		} else {
			bail_with!(
				CLAIMS_MISSING_ENTITLEMENT,
				entitlements = "User, GameCloud, EnvService"
			);
		}
	}
}
