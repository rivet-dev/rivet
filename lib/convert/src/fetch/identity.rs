use std::collections::{HashMap, HashSet};

use proto::{
	backend::{self, pkg::*},
	common,
};
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::{convert, fetch};

#[derive(Debug)]
pub struct TeamsCtx {
	pub user_teams: user::team_list::Response,
	pub teams: Vec<backend::team::Team>,
	pub dev_teams: team_dev::get::Response,
}

#[derive(Debug)]
pub struct PresencesCtx {
	pub res: user_presence::get::Response,
	pub games: Vec<backend::game::Game>,
	pub games_with_namespace_ids: Vec<convert::GameWithNamespaceIds>,
}

pub async fn handles(
	ctx: &OperationContext<()>,
	current_user_id: Uuid,
	raw_user_ids: Vec<Uuid>,
) -> GlobalResult<Vec<models::IdentityHandle>> {
	if raw_user_ids.is_empty() {
		return Ok(Vec::new());
	}

	let user_ids = raw_user_ids
		.clone()
		.into_iter()
		.map(Into::into)
		.collect::<Vec<_>>();

	let (users, presences_ctx, mutual_follows) = tokio::try_join!(
		users(ctx, user_ids.clone()),
		presence_data(ctx, current_user_id, user_ids.clone(), false),
		mutual_follows(ctx, current_user_id, raw_user_ids),
	)?;

	let raw_current_user_id = Into::<common::Uuid>::into(current_user_id);

	// Convert all data
	users
		.users
		.iter()
		.map(|user| {
			let is_mutual_following = mutual_follows.follows.iter().any(|follow| {
				follow.is_mutual
					&& follow.follower_user_id.as_ref() == user.user_id.as_ref()
					&& follow.following_user_id.as_ref() == Some(&raw_current_user_id)
			});

			convert::identity::handle(current_user_id, user, &presences_ctx, is_mutual_following)
		})
		.collect::<GlobalResult<Vec<_>>>()
}

pub async fn summaries(
	ctx: &OperationContext<()>,
	current_user_id: Uuid,
	raw_user_ids: Vec<Uuid>,
) -> GlobalResult<Vec<models::IdentitySummary>> {
	if raw_user_ids.is_empty() {
		return Ok(Vec::new());
	}

	let user_ids = raw_user_ids
		.clone()
		.into_iter()
		.map(Into::into)
		.collect::<Vec<_>>();

	let (users, presences_ctx, mutual_follows) = tokio::try_join!(
		users(ctx, user_ids.clone()),
		presence_data(ctx, current_user_id, user_ids.clone(), false),
		mutual_follows(ctx, current_user_id, raw_user_ids),
	)?;

	// Convert all data
	users
		.users
		.iter()
		.map(|user| {
			convert::identity::summary(
				current_user_id,
				user,
				&presences_ctx,
				&mutual_follows.follows,
			)
		})
		.collect::<GlobalResult<Vec<_>>>()
}

pub async fn profiles(
	ctx: &OperationContext<()>,
	current_user_id: Uuid,
	game_user_id: Option<Uuid>,
	raw_user_ids: Vec<Uuid>,
) -> GlobalResult<Vec<models::IdentityProfile>> {
	if raw_user_ids.is_empty() {
		return Ok(Vec::new());
	}

	let is_game_user = game_user_id.is_some();
	let contains_self = raw_user_ids
		.iter()
		.any(|user_id| user_id == &current_user_id);
	let user_ids = raw_user_ids
		.clone()
		.into_iter()
		.map(Into::into)
		.collect::<Vec<_>>();

	let (
		users,
		presences_ctx,
		teams_ctx,
		mutual_follows,
		(follower_counts, following_counts),
		linked_accounts,
		self_is_game_linked,
	) = tokio::try_join!(
		users(ctx, user_ids.clone()),
		presence_data(ctx, current_user_id, user_ids.clone(), true),
		teams(ctx, user_ids.clone()),
		mutual_follows(ctx, current_user_id, raw_user_ids),
		follows(ctx, user_ids.clone()),
		linked_accounts(ctx, user_ids.clone()),
		is_game_linked(ctx, game_user_id, contains_self),
	)?;

	// Convert all data
	users
		.users
		.iter()
		.map(|user| {
			convert::identity::profile(
				current_user_id,
				user,
				convert::identity::ProfileCtx {
					presences_ctx: &presences_ctx,
					teams_ctx: &teams_ctx,
					mutual_follows: &mutual_follows.follows,
					follower_counts: &follower_counts.follows,
					following_counts: &following_counts.follows,
					linked_accounts: &linked_accounts.users,
					self_is_game_linked,
					is_game_user,
				},
			)
		})
		.collect::<GlobalResult<Vec<_>>>()
}

pub async fn users(
	ctx: &OperationContext<()>,
	user_ids: Vec<common::Uuid>,
) -> GlobalResult<user::get::Response> {
	op!([ctx] user_get {
		user_ids: user_ids,
	})
	.await
}

pub async fn presence_data(
	ctx: &OperationContext<()>,
	current_user_id: Uuid,
	user_ids: Vec<common::Uuid>,
	summary_info: bool,
) -> GlobalResult<PresencesCtx> {
	let ((presences_res, game_ids),) = tokio::try_join!(presences_and_game_ids(ctx, user_ids),)?;

	let (games, games_with_namespace_ids) = games(
		ctx,
		game_ids
			.into_iter()
			.map(Into::<common::Uuid>::into)
			.collect::<Vec<_>>(),
		Vec::new(),
	)
	.await?;

	Ok(PresencesCtx {
		res: presences_res,
		games,
		games_with_namespace_ids,
	})
}

async fn presences_and_game_ids(
	ctx: &OperationContext<()>,
	user_ids: Vec<common::Uuid>,
) -> GlobalResult<(user_presence::get::Response, Vec<Uuid>)> {
	let user_presences_res = op!([ctx] user_presence_get {
		user_ids: user_ids,
	})
	.await?;

	// Fetch game ids from game activities
	let game_ids = user_presences_res
		.users
		.iter()
		.filter_map(|user_presence| {
			if let Some(backend::user::Presence {
				game_activity:
					Some(backend::user::presence::GameActivity {
						game_id: Some(game_id),
						..
					}),
				..
			}) = &user_presence.presence
			{
				Some(game_id.as_uuid())
			} else {
				None
			}
		})
		.collect::<HashSet<_>>()
		.into_iter()
		.collect::<Vec<_>>();

	Ok((user_presences_res, game_ids))
}

async fn games(
	ctx: &OperationContext<()>,
	game_ids: Vec<common::Uuid>,
	namespace_ids: Vec<common::Uuid>,
) -> GlobalResult<(Vec<backend::game::Game>, Vec<convert::GameWithNamespaceIds>)> {
	let game_resolve_res = op!([ctx] game_resolve_namespace_id {
		namespace_ids: namespace_ids,
	})
	.await?;

	let games_res = op!([ctx] game_get {
		game_ids: game_resolve_res
			.games
			.iter()
			.map(|game| Ok(unwrap!(game.game_id)))
			.collect::<GlobalResult<Vec<_>>>()?
			.into_iter()
			.chain(game_ids)
			.collect::<Vec<_>>(),
	})
	.await?;

	// Collects games and their namespace ids together
	let games_with_namespace_ids = game_resolve_res
		.games
		.iter()
		.filter_map(|resolved_game| {
			games_res
				.games
				.iter()
				.find(|game| resolved_game.game_id == game.game_id)
				.map(|game| {
					Ok(convert::GameWithNamespaceIds {
						namespace_ids: resolved_game
							.namespace_ids
							.iter()
							.map(|x| x.as_uuid())
							.collect::<Vec<_>>(),
						game: game.clone(),
					})
				})
		})
		.collect::<GlobalResult<Vec<_>>>()?;

	Ok((games_res.games.clone(), games_with_namespace_ids))
}

async fn teams(ctx: &OperationContext<()>, user_ids: Vec<common::Uuid>) -> GlobalResult<TeamsCtx> {
	let user_teams_res = op!([ctx] user_team_list {
		user_ids: user_ids,
	})
	.await?;

	let team_ids = user_teams_res
		.users
		.iter()
		.map(|user| {
			user.teams
				.iter()
				.map(|t| Ok(unwrap!(t.team_id)))
				.collect::<GlobalResult<Vec<_>>>()
		})
		.collect::<GlobalResult<Vec<_>>>()?
		.into_iter()
		.flatten()
		.collect::<Vec<_>>();

	let (teams_res, dev_teams_res) = tokio::try_join!(
		op!([ctx] team_get {
			team_ids: team_ids.clone(),
		}),
		op!([ctx] team_dev_get {
			team_ids: team_ids.clone(),
		}),
	)?;

	// TODO: hide all closed teams
	let teams = teams_res.teams.clone();

	Ok(TeamsCtx {
		user_teams: user_teams_res,
		teams,
		dev_teams: dev_teams_res,
	})
}

async fn follows(
	ctx: &OperationContext<()>,
	user_ids: Vec<common::Uuid>,
) -> GlobalResult<(user_follow::count::Response, user_follow::count::Response)> {
	tokio::try_join!(
		op!([ctx] user_follow_count {
			kind: user_follow::count::request::Kind::Follower as i32,
			user_ids: user_ids.clone(),
		}),
		op!([ctx] user_follow_count {
			kind: user_follow::count::request::Kind::Following as i32,
			user_ids: user_ids,
		}),
	)
}

pub async fn mutual_follows(
	ctx: &OperationContext<()>,
	current_user_id: Uuid,
	raw_user_ids: Vec<Uuid>,
) -> GlobalResult<user_follow::get::Response> {
	// Converts to hashmap to remove duplicate queries
	let queries = raw_user_ids
		.clone()
		.into_iter()
		.flat_map(|user_id| [(current_user_id, user_id), (user_id, current_user_id)])
		.collect::<HashSet<_>>()
		.into_iter()
		.map(|(user_a_id, user_b_id)| user_follow::get::request::Query {
			follower_user_id: Some(user_a_id.into()),
			following_user_id: Some(user_b_id.into()),
		})
		.collect::<Vec<_>>();

	op!([ctx] user_follow_get {
		queries: queries,
	})
	.await
}

async fn linked_accounts(
	ctx: &OperationContext<()>,
	user_ids: Vec<common::Uuid>,
) -> GlobalResult<user_identity::get::Response> {
	op!([ctx] user_identity_get {
		user_ids: user_ids.clone(),
	})
	.await
}

/// Determines if the game user has been linked through the Rivet dashboard.
async fn is_game_linked(
	ctx: &OperationContext<()>,
	game_user_id: Option<Uuid>,
	contains_self: bool,
) -> GlobalResult<bool> {
	if let (Some(game_user_id), true) = (game_user_id, contains_self) {
		let game_user_res = op!([ctx] game_user_get {
			game_user_ids: vec![game_user_id.into()],
		})
		.await?;
		let game_user = unwrap!(game_user_res.game_users.first());

		Ok(game_user.link_id.is_some())
	} else {
		Ok(false)
	}
}
