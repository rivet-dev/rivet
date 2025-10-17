use serde::{Deserialize, Serialize};

use crate::RawCacheKey;

/// Topic for publishing cache purge messages via UniversalPubSub
pub const CACHE_PURGE_TOPIC: &str = "rivet.cache.purge";

/// Message format for cache purge requests
#[derive(Serialize, Deserialize)]
pub struct CachePurgeMessage {
	pub base_key: String,
	pub keys: Vec<RawCacheKey>,
}
