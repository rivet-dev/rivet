use std::{fmt::Debug, sync::Arc};

use super::*;
use crate::driver::{Driver, InMemoryDriver};

pub type Cache = Arc<CacheInner>;

/// Utility type used to hold information relating to caching.
pub struct CacheInner {
	service_name: String,
	pub(crate) driver: Driver,
	pub(crate) ups: Option<universalpubsub::PubSub>,
}

impl Debug for CacheInner {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CacheInner")
			.field("service_name", &self.service_name)
			.finish()
	}
}

impl CacheInner {
	#[tracing::instrument(skip_all)]
	pub fn from_env(
		config: &rivet_config::Config,
		pools: rivet_pools::Pools,
	) -> Result<Cache, Error> {
		let service_name = rivet_env::service_name();
		let ups = pools.ups().ok();

		match &config.cache().driver {
			rivet_config::config::CacheDriver::Redis => todo!(),
			rivet_config::config::CacheDriver::InMemory => {
				Ok(Self::new_in_memory(service_name.to_string(), 1000, ups))
			}
		}
	}

	#[tracing::instrument(skip(ups))]
	pub fn new_in_memory(
		service_name: String,
		max_capacity: u64,
		ups: Option<universalpubsub::PubSub>,
	) -> Cache {
		let driver = Driver::InMemory(InMemoryDriver::new(service_name.clone(), max_capacity));
		Arc::new(CacheInner {
			service_name,
			driver,
			ups,
		})
	}
}

impl CacheInner {
	/// Returns a new request config builder.
	pub fn request(self: Arc<Self>) -> RequestConfig {
		RequestConfig::new(self.clone())
	}
}
