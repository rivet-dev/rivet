// This module converts proto data information into smithy models. It's important to separate fetching
// from building models so that we can convert already existing data without having to re-fetch it.
use rivet_operation::prelude::*;
use types_proto::rivet::backend;

pub mod game;
pub mod group;
pub mod identity;

#[derive(Debug)]
pub struct GameWithNamespaceIds {
	pub namespace_ids: Vec<Uuid>,
	pub game: backend::game::Game,
}
