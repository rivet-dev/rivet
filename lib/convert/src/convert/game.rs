use rivet_api::models;
use rivet_operation::prelude::*;
use types::rivet::backend;

use crate::{convert, fetch, ApiInto, ApiTryInto};

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
	state: &fetch::game::GameState,
	dev_team: &backend::team::Team,
) -> GlobalResult<models::GameSummary> {
	let game_url = state
		.prod_config
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
		developer: Box::new(convert::group::handle(dev_team)?),
		total_player_count: ApiTryInto::api_try_into(state.total_player_count)?,
	})
}

pub fn region_summary(
	region: &backend::region::Region,
) -> GlobalResult<models::CloudRegionSummary> {
	Ok(models::CloudRegionSummary {
		region_id: unwrap_ref!(region.region_id).as_uuid(),
		region_name_id: region.name_id.clone(),
		provider: region.provider.clone(),
		universal_region: unwrap!(backend::region::UniversalRegion::from_i32(
			region.universal_region
		))
		.api_into(),
		provider_display_name: region.provider_display_name.clone(),
		region_display_name: region.region_display_name.clone(),
	})
}
