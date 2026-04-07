use std::{
	fmt::Debug,
	sync::{Arc, OnceLock},
};

use tokio::sync::broadcast;

use super::*;
use crate::driver::{Driver, InMemoryDriver};

static IN_FLIGHT: OnceLock<scc::HashMap<RawCacheKey, broadcast::Sender<()>>> = OnceLock::new();

pub type Cache = Arc<CacheInner>;

/// Utility type used to hold information relating to caching.
pub struct CacheInner {
	pub(crate) driver: Option<Driver>,
	pub(crate) ups: Option<universalpubsub::PubSub>,
}

impl Debug for CacheInner {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CacheInner").finish()
	}
}

impl CacheInner {
	#[tracing::instrument(skip_all)]
	pub fn from_env(
		config: &rivet_config::Config,
		pools: rivet_pools::Pools,
	) -> Result<Cache, Error> {
		let ups = pools.ups().ok();

		match &config.cache().driver {
			Some(rivet_config::config::CacheDriver::InMemory) => {
				Ok(Self::new_in_memory(10000, ups))
			}
			None => Ok(Self::new_disabled()),
		}
	}

	#[tracing::instrument(skip(ups))]
	pub fn new_in_memory(max_capacity: u64, ups: Option<universalpubsub::PubSub>) -> Cache {
		let driver = Driver::InMemory(InMemoryDriver::new(max_capacity));

		Arc::new(CacheInner {
			driver: Some(driver),
			ups,
		})
	}

	pub fn new_disabled() -> Cache {
		Arc::new(CacheInner {
			driver: None,
			ups: None,
		})
	}

	pub(crate) fn in_flight(&self) -> &scc::HashMap<RawCacheKey, broadcast::Sender<()>> {
		IN_FLIGHT.get_or_init(scc::HashMap::new)
	}
}

impl CacheInner {
	/// Returns a new request config builder.
	pub fn request(self: Arc<Self>) -> RequestConfig {
		RequestConfig::new(self.clone())
	}
}
