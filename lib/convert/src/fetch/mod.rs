// This module fetches information used to convert proto data into smithy models. It's important to separate
// fetching from building models so that we can convert already existing data without having to re-fetch it.

pub mod chat;
pub mod game;
pub mod group;
pub mod identity;
pub mod party;
