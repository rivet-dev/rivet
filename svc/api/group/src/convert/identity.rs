use proto::backend;
use rivet_group_server::models;
use rivet_operation::prelude::*;

use crate::{convert, fetch};

pub fn handle(
	current_user_id: &Uuid,
	user: &backend::user::User,
	presences_ctx: &fetch::identity::PresencesCtx,
	is_mutual_following: bool,
) -> GlobalResult<models::IdentityHandle> {
	let raw_user_id = internal_unwrap_owned!(user.user_id);
	let user_id = raw_user_id.as_uuid();

	let is_self = &user_id == current_user_id;

	let user_presence = internal_unwrap_owned!(presences_ctx
		.res
		.users
		.iter()
		.find(|presence| presence.user_id == user.user_id));
	let user_presence = internal_unwrap!(user_presence.presence);
	let status = internal_unwrap_owned!(backend::user::Status::from_i32(user_presence.status));
	let party = if status != backend::user::Status::Offline {
		if let Some(party_id) = presences_ctx.parties.member_to_party.get(&user_id) {
			if let Some(party) = presences_ctx.parties.parties.get(party_id) {
				Some(convert::party::handle(
					party,
					&presences_ctx.games_with_namespace_ids,
				)?)
			} else {
				None
			}
		} else {
			None
		}
	} else {
		None
	};

	Ok(models::IdentityHandle {
		identity_id: user_id.to_string(),
		display_name: user.display_name.clone(),
		account_number: user.account_number as i32,
		avatar_url: util::route::user_avatar(
			&user.avatar_id,
			user.profile_upload_id.as_ref().map(common::Uuid::as_uuid),
			user.profile_file_name.as_ref(),
		),
		presence: Some(presence(
			user_presence,
			&presences_ctx.games,
			is_self || is_mutual_following,
		)?),
		party,
		is_registered: true, // TODO:
		external: models::IdentityExternalLinks {
			profile: util::route::user_profile(&user_id),
			settings: None,
			chat: (!is_self).then(|| util::route::user_chat(&user_id)),
		},
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

		Some(models::IdentityGameActivity {
			game: convert::game_handle(game)?,
			message: game_activity.message.to_owned(),
			public_metadata: game_activity
				.public_metadata
				.as_ref()
				.map(|metadata| serde_json::from_str::<serde_json::Value>(metadata.as_str()))
				.transpose()?,
			mutual_metadata: if is_mutual_following {
				game_activity
					.friend_metadata
					.as_ref()
					.map(|metadata| serde_json::from_str::<serde_json::Value>(metadata.as_str()))
					.transpose()?
			} else {
				None
			},
		})
	} else {
		None
	};

	Ok(models::IdentityPresence {
		update_ts: util::timestamp::to_chrono(presence.update_ts)?,
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
