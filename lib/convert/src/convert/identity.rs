use rivet_api::models;
use rivet_operation::prelude::*;
use types::rivet::{
	backend::{self, pkg::*},
	common,
};

use crate::{convert, fetch, ApiTryInto};

pub fn handle(
	current_user_id: Uuid,
	user: &backend::user::User,
	presences_ctx: &fetch::identity::PresencesCtx,
	is_mutual_following: bool,
) -> GlobalResult<models::IdentityHandle> {
	let user_id = internal_unwrap!(user.user_id).as_uuid();
	let is_self = user_id == current_user_id;

	let user_presence = internal_unwrap_owned!(presences_ctx
		.res
		.users
		.iter()
		.find(|presence| presence.user_id == user.user_id));
	let user_presence = internal_unwrap!(user_presence.presence);
	let status = internal_unwrap_owned!(backend::user::Status::from_i32(user_presence.status));

	Ok(models::IdentityHandle {
		identity_id: user_id,
		display_name: user.display_name.clone(),
		account_number: user.account_number as i32,
		avatar_url: util::route::user_avatar(
			&user.avatar_id,
			user.profile_upload_id.map(|x| x.as_uuid()),
			user.profile_file_name.as_ref(),
		),
		presence: Some(Box::new(presence(
			user_presence,
			&presences_ctx.games,
			is_self || is_mutual_following,
		)?)),
		is_registered: true, // TODO:
		external: Box::new(models::IdentityExternalLinks {
			profile: util::route::user_profile(user_id),
			settings: None,
			chat: (!is_self).then(|| util::route::user_chat(user_id)),
		}),
	})
}

pub fn handle_without_presence(
	current_user_id: Uuid,
	user: &backend::user::User,
) -> GlobalResult<models::IdentityHandle> {
	let user_id = internal_unwrap!(user.user_id).as_uuid();
	let is_self = user_id == current_user_id;

	Ok(models::IdentityHandle {
		identity_id: user_id,
		display_name: user.display_name.to_owned(),
		account_number: user.account_number as i32,
		avatar_url: util::route::user_avatar(
			&user.avatar_id,
			user.profile_upload_id.map(|x| x.as_uuid()),
			user.profile_file_name.as_ref(),
		),
		presence: None,
		is_registered: true, // TODO:
		external: Box::new(models::IdentityExternalLinks {
			profile: util::route::user_profile(user_id),
			settings: None,
			chat: (!is_self).then(|| util::route::user_chat(user_id)),
		}),
	})
}

pub fn summary(
	current_user_id: Uuid,
	user: &backend::user::User,
	presences_ctx: &fetch::identity::PresencesCtx,
	mutual_follows: &[user_follow::get::response::Follow],
) -> GlobalResult<models::IdentitySummary> {
	let user_id_proto = internal_unwrap_owned!(user.user_id);
	let user_id = user_id_proto.as_uuid();
	let is_self = user_id == current_user_id;

	let user_presence = internal_unwrap_owned!(presences_ctx
		.res
		.users
		.iter()
		.find(|presence| presence.user_id == user.user_id));
	let user_presence = internal_unwrap!(user_presence.presence);

	let current_user_id = Into::<common::Uuid>::into(current_user_id);
	let following = mutual_follows.iter().any(|follow| {
		follow.follower_user_id.as_ref() == Some(&current_user_id)
			&& follow.following_user_id.as_ref() == Some(&user_id_proto)
	});
	let is_following_me = mutual_follows.iter().any(|follow| {
		follow.follower_user_id.as_ref() == Some(&user_id_proto)
			&& follow.following_user_id.as_ref() == Some(&current_user_id)
	});
	let is_mutual_following = following && is_following_me;

	Ok(models::IdentitySummary {
		identity_id: user_id,
		display_name: user.display_name.clone(),
		account_number: user.account_number as i32,
		avatar_url: util::route::user_avatar(
			&user.avatar_id,
			user.profile_upload_id.map(|x| x.as_uuid()),
			user.profile_file_name.as_ref(),
		),
		presence: Some(Box::new(presence(
			user_presence,
			&presences_ctx.games,
			is_self || is_mutual_following,
		)?)),
		is_registered: true, // TODO:
		external: Box::new(models::IdentityExternalLinks {
			profile: util::route::user_profile(user_id),
			settings: None,
			chat: (!is_self).then(|| util::route::user_chat(user_id)),
		}),
		following,
		is_following_me,
		is_mutual_following,
	})
}

#[derive(Debug)]
pub struct ProfileCtx<'a> {
	pub presences_ctx: &'a fetch::identity::PresencesCtx,
	pub teams_ctx: &'a fetch::identity::TeamsCtx,
	pub mutual_follows: &'a [user_follow::get::response::Follow],
	pub follower_counts: &'a [user_follow::count::response::Follows],
	pub following_counts: &'a [user_follow::count::response::Follows],
	pub linked_accounts: &'a [user_identity::get::response::User],
	pub self_is_game_linked: bool,
	pub is_game_user: bool,
}

pub fn profile(
	current_user_id: Uuid,
	user: &backend::user::User,
	pctx: ProfileCtx,
) -> GlobalResult<models::IdentityProfile> {
	let user_id_proto = internal_unwrap_owned!(user.user_id);
	let user_id = user_id_proto.as_uuid();

	let is_self = user_id == current_user_id;

	let identities = internal_unwrap_owned!(pctx
		.linked_accounts
		.iter()
		.find(|identity| identity.user_id == user.user_id));
	let identities = &identities.identities;
	// If the user has at least one identity they are considered registered
	let is_registered = !identities.is_empty();

	// Get user's groups
	let user_groups = {
		let user = internal_unwrap_owned!(pctx
			.teams_ctx
			.user_teams
			.users
			.iter()
			.find(|u| u.user_id == user.user_id));
		let team_ids = user
			.teams
			.iter()
			.map(|t| Ok(internal_unwrap_owned!(t.team_id)))
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
				let is_developer = pctx
					.teams_ctx
					.dev_teams
					.teams
					.iter()
					.any(|dev_team| team.team_id == dev_team.team_id);
				Ok(models::IdentityGroup {
					group: Box::new(convert::group::handle(team, is_developer)?),
				})
			})
			.collect::<GlobalResult<Vec<_>>>()?
	};

	let user_presence = internal_unwrap_owned!(pctx
		.presences_ctx
		.res
		.users
		.iter()
		.find(|presence| presence.user_id == user.user_id));
	let user_presence = internal_unwrap!(user_presence.presence);
	let status = internal_unwrap_owned!(backend::user::Status::from_i32(user_presence.status));

	let current_user_id = Into::<common::Uuid>::into(current_user_id);
	let following = pctx.mutual_follows.iter().any(|follow| {
		follow.follower_user_id.as_ref() == Some(&current_user_id)
			&& follow.following_user_id.as_ref() == Some(&user_id_proto)
	});
	let is_following_me = pctx.mutual_follows.iter().any(|follow| {
		follow.follower_user_id.as_ref() == Some(&user_id_proto)
			&& follow.following_user_id.as_ref() == Some(&current_user_id)
	});
	let is_mutual_following = following && is_following_me;

	let follower_count = pctx
		.follower_counts
		.iter()
		.find(|f| f.user_id == user.user_id)
		.map(|f| f.count)
		.unwrap_or_default();
	let following_count = pctx
		.following_counts
		.iter()
		.find(|f| f.user_id == user.user_id)
		.map(|f| f.count)
		.unwrap_or_default();

	Ok(models::IdentityProfile {
		identity_id: user_id,
		display_name: user.display_name.to_owned(),
		account_number: user.account_number as i32,
		avatar_url: util::route::user_avatar(
			&user.avatar_id,
			user.profile_upload_id.map(|x| x.as_uuid()),
			user.profile_file_name.as_ref(),
		),
		presence: Some(Box::new(presence(
			user_presence,
			&pctx.presences_ctx.games,
			is_self || is_mutual_following,
		)?)),
		is_registered,
		external: Box::new(models::IdentityExternalLinks {
			profile: util::route::user_profile(user_id),
			settings: (is_self && pctx.is_game_user).then(util::route::user_settings),
			chat: (!is_self).then(|| util::route::user_chat(user_id)),
		}),
		dev_state: None,
		is_admin: is_self && user.is_admin,
		is_game_linked: (is_self && pctx.is_game_user).then_some(pctx.self_is_game_linked),

		follower_count,
		following_count,
		following,
		is_following_me,
		is_mutual_following,
		awaiting_deletion: is_self.then(|| user.delete_request_ts.is_some()),

		join_ts: util::timestamp::to_string(user.join_ts)?,
		bio: user.bio.clone(),
		linked_accounts: if is_self && !pctx.is_game_user {
			identities
				.iter()
				.cloned()
				.map(ApiTryInto::try_into)
				.collect::<GlobalResult<Vec<_>>>()?
		} else {
			Vec::new()
		},

		groups: user_groups,
		games: Vec::new(), // TODO:
	})
}

pub fn presence(
	presence: &backend::user::Presence,
	games: &[backend::game::Game],
	is_mutual_following: bool,
) -> GlobalResult<models::IdentityPresence> {
	let status = internal_unwrap_owned!(backend::user::Status::from_i32(presence.status));

	let game_activity = if let Some(game_activity) = &presence.game_activity {
		let game_id = internal_unwrap!(game_activity.game_id);

		let game = internal_unwrap_owned!(games
			.iter()
			.find(|game| game.game_id.as_ref() == Some(game_id)));

		Some(Box::new(models::IdentityGameActivity {
			game: Box::new(convert::game::handle(game)?),
			message: game_activity.message.to_owned(),
			public_metadata: game_activity
				.public_metadata
				.as_ref()
				.and_then(|s| serde_json::from_str(s).ok()),
			mutual_metadata: if is_mutual_following {
				game_activity
					.friend_metadata
					.as_ref()
					.and_then(|s| serde_json::from_str(s).ok())
			} else {
				None
			},
		}))
	} else {
		None
	};

	Ok(models::IdentityPresence {
		update_ts: util::timestamp::to_string(presence.update_ts)?,
		status: match status {
			backend::user::Status::Offline => models::IdentityStatus::Offline,
			backend::user::Status::Away => models::IdentityStatus::Away,
			backend::user::Status::Online => models::IdentityStatus::Online,
		},
		game_activity: match status {
			backend::user::Status::Offline => None,
			_ => game_activity,
		},
	})
}
