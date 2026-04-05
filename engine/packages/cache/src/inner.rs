use std::{fmt::Debug, sync::Arc};

use super::*;
use crate::driver::{Driver, InMemoryDriver};

pub type Cache = Arc<CacheInner>;

/// Utility type used to hold information relating to caching.
pub struct CacheInner {
	pub(crate) driver: Driver,
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
			rivet_config::config::CacheDriver::InMemory => Ok(Self::new_in_memory(10000, ups)),
		}
	}

	#[tracing::instrument(skip(ups))]
	pub fn new_in_memory(max_capacity: u64, ups: Option<universalpubsub::PubSub>) -> Cache {
		let driver = Driver::InMemory(InMemoryDriver::new(max_capacity));
		Arc::new(CacheInner { driver, ups })
	}
}

impl CacheInner {
	/// Returns a new request config builder.
	pub fn request(self: Arc<Self>) -> RequestConfig {
		RequestConfig::new(self.clone())
	}
}
