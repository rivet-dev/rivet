use types_proto::rivet::backend;
use uuid::Uuid;

// TODO: Remove unwraps of server config

pub fn user_settings(config: &rivet_config::Config) -> String {
	format!(
		"{}/settings",
		config.server().unwrap().rivet.hub.public_origin
	)
}

pub fn user_profile(config: &rivet_config::Config, user_id: Uuid) -> String {
	format!(
		"{}/identities/{}",
		config.server().unwrap().rivet.hub.public_origin,
		user_id
	)
}

pub fn team_profile(config: &rivet_config::Config, team_id: Uuid) -> String {
	format!(
		"{}/groups/{}",
		config.server().unwrap().rivet.hub.public_origin,
		team_id
	)
}

pub fn user_avatar(config: &rivet_config::Config, user: &backend::user::User) -> String {
	if let (Some(upload_id), Some(file_name)) =
		(user.profile_upload_id, user.profile_file_name.as_ref())
	{
		format!(
			"{}/media/user-avatar/{}/{}",
			config.server().unwrap().rivet.api.public_origin,
			upload_id,
			file_name
		)
	} else {
		format!("https://assets2.rivet.gg/avatars/{}.png", user.avatar_id)
	}
}

pub fn custom_avatar(
	config: &rivet_config::Config,
	upload_id: Uuid,
	file_name: &str,
	_provider: i32,
) -> String {
	format!(
		"{}/media/user-avatar/{}/{}",
		config.server().unwrap().rivet.api.public_origin,
		upload_id,
		file_name
	)
}

pub fn team_avatar(config: &rivet_config::Config, team: &backend::team::Team) -> Option<String> {
	if let (Some(upload_id), Some(file_name)) =
		(team.profile_upload_id, team.profile_file_name.as_ref())
	{
		Some(format!(
			"{}/media/team-avatar/{}/{}",
			config.server().unwrap().rivet.api.public_origin,
			upload_id,
			file_name
		))
	} else {
		None
	}
}

pub fn game_logo(config: &rivet_config::Config, game: &backend::game::Game) -> Option<String> {
	if let (Some(upload_id), Some(file_name)) = (game.logo_upload_id, game.logo_file_name.as_ref())
	{
		Some(format!(
			"{}/media/game-logo/{}/{}",
			config.server().unwrap().rivet.api.public_origin,
			upload_id,
			file_name
		))
	} else {
		None
	}
}

pub fn game_banner(config: &rivet_config::Config, game: &backend::game::Game) -> Option<String> {
	if let (Some(upload_id), Some(file_name)) =
		(game.banner_upload_id, game.banner_file_name.as_ref())
	{
		Some(format!(
			"{}/media/game-banner/{}/{}",
			config.server().unwrap().rivet.api.public_origin,
			upload_id,
			file_name
		))
	} else {
		None
	}
}

pub fn identity_game_link(config: &rivet_config::Config, link_token: &str) -> String {
	format!(
		"{}/link/{}",
		config.server().unwrap().rivet.hub.public_origin,
		link_token
	)
}

pub fn cloud_device_link(config: &rivet_config::Config, link_token: &str) -> String {
	format!(
		"{}/devices/link/{}",
		config.server().unwrap().rivet.hub.public_origin,
		link_token
	)
}

pub fn access_token_link(config: &rivet_config::Config, access_token_token: &str) -> String {
	format!(
		"{}/access-token/{}",
		config.server().unwrap().rivet.hub.public_origin,
		access_token_token
	)
}

pub fn backend_endpoint(config: &rivet_config::Config, backend_slug: &str) -> String {
	let backend_domain = &config
		.server()
		.unwrap()
		.rivet
		.backend
		.as_ref()
		.expect("backend not enabled")
		.base_domain;

	format!("https://{}.{}", backend_slug, backend_domain)
}
