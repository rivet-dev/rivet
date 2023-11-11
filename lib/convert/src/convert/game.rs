use rivet_api::models;
use rivet_operation::prelude::*;
use types::rivet::backend;

use crate::convert;

pub fn handle(game: &backend::game::Game) -> GlobalResult<models::GameHandle> {
	Ok(models::GameHandle {
		game_id: unwrap_ref!(game.game_id).as_uuid(),
		name_id: game.name_id.to_owned(),
		display_name: game.display_name.to_owned(),
		logo_url: util::route::game_logo(&game),
		banner_url: util::route::game_banner(&game),
	})
}

pub fn summary(
	game: &backend::game::Game,
	cdn_config: &backend::cdn::NamespaceConfig,
	dev_team: &backend::team::Team,
) -> GlobalResult<models::GameSummary> {
	let game_url = cdn_config
		.domains
		.first()
		.map(|d| d.domain.clone())
		.unwrap_or_else(|| game.url.clone());

	Ok(models::GameSummary {
		game_id: unwrap_ref!(game.game_id).as_uuid(),
		name_id: game.name_id.to_owned(),
		display_name: game.display_name.to_owned(),
		logo_url: util::route::game_logo(&game),
		banner_url: util::route::game_banner(&game),
		url: game_url,
		developer: Box::new(convert::group::handle(dev_team, true)?),
	})
}
