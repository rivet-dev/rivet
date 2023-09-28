use types::rivet::backend;
use uuid::Uuid;

use crate::env::{domain_main, origin_hub};

pub fn user_settings() -> String {
	format!("{}/settings", origin_hub())
}

pub fn user_profile(user_id: Uuid) -> String {
	format!("{}/identities/{}", origin_hub(), user_id)
}

pub fn user_chat(user_id: Uuid) -> String {
	format!("{}/identities/{}/chat", origin_hub(), user_id)
}

pub fn team_profile(team_id: Uuid) -> String {
	format!("{}/groups/{}", origin_hub(), team_id)
}

pub fn team_chat(team_id: Uuid) -> String {
	format!("{}/groups/{}/chat", origin_hub(), team_id)
}

pub fn thread(thread_id: Uuid) -> String {
	format!("{}/threads/{}", origin_hub(), thread_id)
}

pub fn game_profile(game_name_id: &str) -> String {
	format!("{}/games/{}", origin_hub(), game_name_id)
}

pub fn user_avatar(user: &backend::user::User) -> String {
	if let (Some(upload_id), Some(file_name), Some(provider)) = (
		user.profile_upload_id,
		user.profile_file_name.as_ref(),
		user.profile_provider,
	) {
		format!(
			"https://{}/media/{}/user-avatar/{}/{}",
			domain_main(),
			provider_str(provider),
			upload_id,
			file_name
		)
	} else {
		format!("https://assets2.rivet.gg/avatars/{}.png", user.avatar_id)
	}
}

pub fn custom_avatar(upload_id: Uuid, file_name: &str, provider: i32) -> String {
	format!(
		"https://{}/media/{}/user-avatar/{}/{}",
		domain_main(),
		provider_str(provider),
		upload_id,
		file_name
	)
}

pub fn team_avatar(team: &backend::team::Team) -> Option<String> {
	if let (Some(upload_id), Some(file_name), Some(provider)) = (
		team.profile_upload_id,
		team.profile_file_name.as_ref(),
		team.profile_provider,
	) {
		Some(format!(
			"https://{}/media/{}/team-avatar/{}/{}",
			domain_main(),
			provider_str(provider),
			upload_id,
			file_name
		))
	} else {
		None
	}
}

pub fn game_logo(game: &backend::game::Game) -> Option<String> {
	if let (Some(upload_id), Some(file_name), Some(provider)) = (
		game.logo_upload_id,
		game.logo_file_name.as_ref(),
		game.logo_provider,
	) {
		Some(format!(
			"https://{}/media/{}/game-logo/{}/{}",
			domain_main(),
			provider_str(provider),
			upload_id,
			file_name
		))
	} else {
		None
	}
}

pub fn game_banner(game: &backend::game::Game) -> Option<String> {
	if let (Some(upload_id), Some(file_name), Some(provider)) = (
		game.banner_upload_id,
		game.banner_file_name.as_ref(),
		game.banner_provider,
	) {
		Some(format!(
			"https://{}/media/{}/game-banner/{}/{}",
			domain_main(),
			provider_str(provider),
			upload_id,
			file_name
		))
	} else {
		None
	}
}

pub fn identity_game_link(link_token: &str) -> String {
	format!("{}/link/{}", origin_hub(), link_token)
}

pub fn cloud_device_link(link_token: &str) -> String {
	format!("{}/devices/link/{}", origin_hub(), link_token)
}

pub fn team_billing(team_id: Uuid) -> String {
	format!("{}/groups/{}/billing", origin_hub(), team_id)
}

fn provider_str(provider: i32) -> &'static str {
	// Default gracefully
	match backend::upload::Provider::from_i32(provider).unwrap_or_default() {
		backend::upload::Provider::Minio => "minio",
		backend::upload::Provider::Backblaze => "backblaze",
		backend::upload::Provider::Aws => "aws",
	}
}
