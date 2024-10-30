use proto::{
	backend::{self, pkg::*},
	common,
};
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::convert;

#[derive(Debug)]
pub struct TeamsCtx {
	pub user_teams: user::team_list::Response,
	pub teams: Vec<backend::team::Team>,
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

	let users = users(ctx, user_ids.clone()).await?;

	// Convert all data
	users
		.users
		.iter()
		.map(|user| convert::identity::handle(ctx.config(), current_user_id, user))
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

	let users = users(ctx, user_ids.clone()).await?;

	// Convert all data
	users
		.users
		.iter()
		.map(|user| convert::identity::summary(ctx.config(), current_user_id, user))
		.collect::<GlobalResult<Vec<_>>>()
}

pub async fn profiles(
	ctx: &OperationContext<()>,
	current_user_id: Uuid,
	raw_user_ids: Vec<Uuid>,
) -> GlobalResult<Vec<models::IdentityProfile>> {
	if raw_user_ids.is_empty() {
		return Ok(Vec::new());
	}

	let user_ids = raw_user_ids
		.clone()
		.into_iter()
		.map(Into::into)
		.collect::<Vec<_>>();

	let (users, teams_ctx, linked_accounts) = tokio::try_join!(
		users(ctx, user_ids.clone()),
		teams(ctx, user_ids.clone()),
		linked_accounts(ctx, user_ids.clone()),
	)?;

	// Convert all data
	users
		.users
		.iter()
		.map(|user| {
			convert::identity::profile(
				ctx.config(),
				current_user_id,
				user,
				convert::identity::ProfileCtx {
					teams_ctx: &teams_ctx,
					linked_accounts: &linked_accounts.users,
					self_is_game_linked: false,
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

	let teams_res = op!([ctx] team_get {
		team_ids: team_ids.clone(),
	})
	.await?;

	// TODO: hide all closed teams
	let teams = teams_res.teams.clone();

	Ok(TeamsCtx {
		user_teams: user_teams_res,
		teams,
	})
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
