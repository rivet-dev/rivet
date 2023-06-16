// This module converts proto data information into smithy models. It's important to separate fetching
// from building models so that we can convert already existing data without having to re-fetch it.
use api_helper::anchor::WatchResponse;
use rivet_cloud_server::models;

pub mod group;

pub fn watch_response(value: WatchResponse) -> models::WatchResponse {
	models::WatchResponse {
		index: value.watch_index().to_owned(),
	}
}
