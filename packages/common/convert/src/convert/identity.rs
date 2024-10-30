use rivet_api::models;
use rivet_operation::prelude::*;
use types_proto::rivet::{
	backend::{self, pkg::*},
	common,
};

use crate::{convert, fetch, ApiTryInto};

pub fn handle(
	config: &rivet_config::Config,
	_current_user_id: Uuid,
	user: &backend::user::User,
) -> GlobalResult<models::IdentityHandle> {
	let user_id = unwrap_ref!(user.user_id).as_uuid();

	Ok(models::IdentityHandle {
		identity_id: user_id,
		display_name: user.display_name.clone(),
		account_number: user.account_number as i32,
		avatar_url: util::route::user_avatar(config, &user),
		is_registered: true, // TODO:
		external: Box::new(models::IdentityExternalLinks {
			profile: util::route::user_profile(config, user_id),
			settings: None,
		}),
	})
}

pub fn summary(
	config: &rivet_config::Config,
	current_user_id: Uuid,
	user: &backend::user::User,
) -> GlobalResult<models::IdentitySummary> {
	let user_id_proto = unwrap!(user.user_id);
	let user_id = user_id_proto.as_uuid();

	Ok(models::IdentitySummary {
		identity_id: user_id,
		display_name: user.display_name.clone(),
		account_number: user.account_number as i32,
		avatar_url: util::route::user_avatar(config, &user),
		is_registered: true, // TODO:
		external: Box::new(models::IdentityExternalLinks {
			profile: util::route::user_profile(config, user_id),
			settings: None,
		}),
		following: false,
		is_following_me: false,
		is_mutual_following: false,
	})
}

#[derive(Debug)]
pub struct ProfileCtx<'a> {
	pub teams_ctx: &'a fetch::identity::TeamsCtx,
	pub linked_accounts: &'a [user_identity::get::response::User],
	pub self_is_game_linked: bool,
}

pub fn profile(
	config: &rivet_config::Config,
	current_user_id: Uuid,
	user: &backend::user::User,
	pctx: ProfileCtx,
) -> GlobalResult<models::IdentityProfile> {
	let user_id_proto = unwrap!(user.user_id);
	let user_id = user_id_proto.as_uuid();

	let is_self = user_id == current_user_id;

	let identities = unwrap!(pctx
		.linked_accounts
		.iter()
		.find(|identity| identity.user_id == user.user_id));
	let identities = &identities.identities;
	// If the user has at least one identity they are considered registered
	let is_registered = !identities.is_empty();

	// Get user's groups
	let user_groups = {
		let user = unwrap!(pctx
			.teams_ctx
			.user_teams
			.users
			.iter()
			.find(|u| u.user_id == user.user_id));
		let team_ids = user
			.teams
			.iter()
			.map(|t| Ok(unwrap!(t.team_id)))
			.collect::<GlobalResult<Vec<_>>>()?;

		pctx.teams_ctx
			.teams
			.iter()
			.filter(|team| {
				team_ids
					.iter()
					.any(|team_id| Some(team_id) == team.team_id.as_ref())
			})
			.map(|team| {
				Ok(models::IdentityGroup {
					group: Box::new(convert::group::handle(config, team)?),
				})
			})
			.collect::<GlobalResult<Vec<_>>>()?
	};

	Ok(models::IdentityProfile {
		identity_id: user_id,
		display_name: user.display_name.to_owned(),
		account_number: user.account_number as i32,
		avatar_url: util::route::user_avatar(config, &user),
		is_registered,
		external: Box::new(models::IdentityExternalLinks {
			profile: util::route::user_profile(config, user_id),
			settings: None,
		}),
		dev_state: None,
		is_admin: is_self && user.is_admin,
		is_game_linked: None,

		follower_count: 0,
		following_count: 0,
		following: false,
		is_following_me: false,
		is_mutual_following: false,
		awaiting_deletion: is_self.then(|| user.delete_request_ts.is_some()),

		join_ts: util::timestamp::to_string(user.join_ts)?,
		bio: user.bio.clone(),
		linked_accounts: if is_self {
			identities
				.iter()
				.cloned()
				.map(ApiTryInto::api_try_into)
				.collect::<GlobalResult<Vec<_>>>()?
		} else {
			Vec::new()
		},

		groups: user_groups,
		games: Vec::new(), // TODO:
	})
}
