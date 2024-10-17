use proto::backend;
use rivet_group_server::models;
use rivet_operation::prelude::*;

pub fn handle(
	current_user_id: Uuid,
	user: &backend::user::User,
) -> GlobalResult<models::IdentityHandle> {
	let raw_user_id = unwrap!(user.user_id);
	let user_id = raw_user_id.as_uuid();

	Ok(models::IdentityHandle {
		identity_id: user_id.to_string(),
		display_name: user.display_name.clone(),
		account_number: user.account_number as i32,
		avatar_url: util::route::user_avatar(user),
		is_registered: true, // TODO:
		external: models::IdentityExternalLinks {
			profile: util::route::user_profile(user_id),
			settings: None,
			chat: Default::default(),
		},
		party: None,
		presence: None,
	})
}
