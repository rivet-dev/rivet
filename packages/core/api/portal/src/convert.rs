/// Use in conjunction with the prefetch module to fetch and send large amounts of data efficiently.
use api_helper::anchor::WatchResponse;
use proto::backend;
use rivet_operation::prelude::*;
use rivet_portal_server::models;

pub fn game_summary(
	config: &rivet_config::Config,
	game: backend::game::Game,
	team: &backend::team::Team,
) -> GlobalResult<models::GameSummary> {
	Ok(models::GameSummary {
		game_id: unwrap_ref!(game.game_id).as_uuid().to_string(),
		name_id: game.name_id.clone(),
		display_name: game.display_name.clone(),
		logo_url: util::route::game_logo(config, &game),
		banner_url: util::route::game_banner(config, &game),

		url: game.url,
		developer: group_handle(config, team, true)?,
		tags: game.tags,
	})
}

pub fn group_handle(
	config: &rivet_config::Config,
	team: &backend::team::Team,
	is_developer: bool,
) -> GlobalResult<models::GroupHandle> {
	let team_id = unwrap_ref!(team.team_id).as_uuid();

	Ok(models::GroupHandle {
		group_id: team_id.to_string(),
		display_name: team.display_name.to_owned(),
		avatar_url: util::route::team_avatar(config, team),
		external: models::GroupExternalLinks {
			profile: util::route::team_profile(config, team_id),
			chat: Default::default(),
		},
		is_developer: is_developer.then_some(true),
	})
}

pub fn watch_response(value: WatchResponse) -> models::WatchResponse {
	models::WatchResponse {
		index: value.watch_index().to_owned(),
	}
}
