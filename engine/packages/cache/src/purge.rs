use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use universalpubsub::Subject;

use crate::RawCacheKey;

/// Topic for publishing cache purge messages via UniversalPubSub
pub const CACHE_PURGE_TOPIC: &str = "rivet.cache.purge";

pub struct CachePurgeSubject;

impl std::fmt::Display for CachePurgeSubject {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		CACHE_PURGE_TOPIC.fmt(f)
	}
}

impl Subject for CachePurgeSubject {
	fn root<'a>() -> Option<Cow<'a, str>> {
		Some(Cow::Borrowed(CACHE_PURGE_TOPIC))
	}

	fn as_str(&self) -> Option<&str> {
		Some(CACHE_PURGE_TOPIC)
	}
}

/// Message format for cache purge requests
#[derive(Serialize, Deserialize)]
pub struct CachePurgeMessage {
	pub base_key: String,
	pub keys: Vec<RawCacheKey>,
}
