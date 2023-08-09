use proto::backend;
use rivet_chat_server::models;
use rivet_operation::prelude::*;

pub fn handle_without_presence(
	current_user_id: Uuid,
	user: &backend::user::User,
) -> GlobalResult<models::IdentityHandle> {
	let user_id = internal_unwrap!(user.user_id).as_uuid();
	let is_self = user_id == current_user_id;

	Ok(models::IdentityHandle {
		identity_id: user_id.to_string(),
		display_name: user.display_name.to_owned(),
		account_number: user.account_number as i32,
		avatar_url: util::route::user_avatar(&user)?,
		presence: None,
		is_registered: true, // TODO:
		external: models::IdentityExternalLinks {
			profile: util::route::user_profile(user_id),
			settings: None,
			chat: (!is_self).then(|| util::route::user_chat(user_id)),
		},
		party: None,
	})
}
