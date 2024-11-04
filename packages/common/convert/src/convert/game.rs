use rivet_api::models;
use rivet_operation::prelude::*;
use types_proto::rivet::backend;

use crate::{convert, fetch, ApiTryInto};

pub fn handle(
	config: &rivet_config::Config,
	game: &backend::game::Game,
) -> GlobalResult<models::GameHandle> {
	Ok(models::GameHandle {
		game_id: unwrap_ref!(game.game_id).as_uuid(),
		name_id: game.name_id.to_owned(),
		display_name: game.display_name.to_owned(),
		logo_url: util::route::game_logo(config, &game),
		banner_url: util::route::game_banner(config, &game),
	})
}

pub fn summary(
	config: &rivet_config::Config,
	game: &backend::game::Game,
	dev_team: &backend::team::Team,
) -> GlobalResult<models::GameSummary> {
	Ok(models::GameSummary {
		game_id: unwrap_ref!(game.game_id).as_uuid(),
		name_id: game.name_id.to_owned(),
		display_name: game.display_name.to_owned(),
		logo_url: util::route::game_logo(config, &game),
		banner_url: util::route::game_banner(config, &game),
		developer: Box::new(convert::group::handle(config, dev_team)?),
		// Deprecated
		total_player_count: 0,
		url: game.url.clone(),
	})
}

pub fn region_summary(
	region: &backend::region::Region,
) -> GlobalResult<models::CloudRegionSummary> {
	Ok(models::CloudRegionSummary {
		region_id: unwrap_ref!(region.region_id).as_uuid(),
		region_name_id: region.name_id.clone(),
		provider: region.provider.clone(),
		universal_region: models::CloudUniversalRegion::Unknown,
		provider_display_name: region.provider_display_name.clone(),
		region_display_name: region.region_display_name.clone(),
	})
}
