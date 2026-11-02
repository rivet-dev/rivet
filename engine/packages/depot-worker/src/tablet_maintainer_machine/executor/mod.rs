//! Storage and transport operations requested by the tablet maintainer.

pub mod cold_store;

pub(crate) mod repair;

pub(crate) mod repair_claims;

pub(crate) mod split;
