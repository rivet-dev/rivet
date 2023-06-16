// This module converts proto data information into smithy models. It's important to separate fetching
// from building models so that we can convert already existing data without having to re-fetch it.
use api_helper::anchor::WatchResponse;
use proto::backend;
use rivet_chat_server::models;
use rivet_operation::prelude::*;

pub mod chat;
pub mod identity;
pub mod party;

pub struct GameWithNamespaceIds {
	pub namespace_ids: Vec<Uuid>,
	pub game: backend::game::Game,
}

pub fn game_handle(game: &backend::game::Game) -> GlobalResult<models::GameHandle> {
	Ok(models::GameHandle {
		game_id: internal_unwrap!(game.game_id).as_uuid().to_string(),
		name_id: game.name_id.to_owned(),
		display_name: game.display_name.to_owned(),
		logo_url: util::route::game_logo(
			game.logo_upload_id.as_ref().map(common::Uuid::as_uuid),
			game.logo_file_name.as_ref(),
		),
		banner_url: util::route::game_banner(
			game.banner_upload_id.as_ref().map(common::Uuid::as_uuid),
			game.banner_file_name.as_ref(),
		),
	})
}

pub fn watch_response(value: WatchResponse) -> models::WatchResponse {
	models::WatchResponse {
		index: value.watch_index().to_owned(),
	}
}
