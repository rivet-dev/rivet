// This module converts proto data information into smithy models. It's important to separate fetching
// from building models so that we can convert already existing data without having to re-fetch it.
use api_helper::anchor::WatchResponse;
use proto::backend;
use rivet_group_server::models;
use rivet_operation::prelude::*;

pub mod group;
pub mod identity;

// TODO: Move all below functions to own files

pub struct GameWithNamespaceIds {
	pub namespace_ids: Vec<Uuid>,
	pub game: backend::game::Game,
}

pub fn game_handle(game: &backend::game::Game) -> GlobalResult<models::GameHandle> {
	Ok(models::GameHandle {
		game_id: unwrap_ref!(game.game_id).as_uuid().to_string(),
		name_id: game.name_id.to_owned(),
		display_name: game.display_name.to_owned(),
		logo_url: util::route::game_logo(game),
		banner_url: util::route::game_banner(game),
	})
}

pub fn watch_response(value: WatchResponse) -> models::WatchResponse {
	models::WatchResponse {
		index: value.watch_index().to_owned(),
	}
}
