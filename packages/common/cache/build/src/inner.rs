use std::{fmt::Debug, sync::Arc};

use rivet_pools::prelude::*;

use super::*;
use crate::driver::{Driver, InMemoryDriver, RedisDriver};

pub type Cache = Arc<CacheInner>;

/// Utility type used to hold information relating to caching.
pub struct CacheInner {
	service_name: String,
	pub(crate) driver: Driver,
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

		match config.server().map_err(Error::Config)?.rivet.cache.driver {
			rivet_config::config::CacheDriver::Redis => {
				let redis_cache = pools.redis_cache().map_err(Error::Pools)?;

				Ok(Self::new_redis(service_name.to_string(), redis_cache))
			}
			rivet_config::config::CacheDriver::InMemory => {
				Ok(Self::new_in_memory(service_name.to_string(), 1000))
			}
		}
	}

	#[tracing::instrument(skip(redis_conn))]
	pub fn new_redis(service_name: String, redis_conn: RedisPool) -> Cache {
		let driver = Driver::Redis(RedisDriver::new(service_name.clone(), redis_conn));
		Arc::new(CacheInner {
			service_name,
			driver,
		})
	}

	#[tracing::instrument]
	pub fn new_in_memory(service_name: String, max_capacity: u64) -> Cache {
		let driver = Driver::InMemory(InMemoryDriver::new(service_name.clone(), max_capacity));
		Arc::new(CacheInner {
			service_name,
			driver,
		})
	}
}

impl CacheInner {
	/// Returns a new request config builder.
	pub fn request(self: Arc<Self>) -> RequestConfig {
		RequestConfig::new(self.clone())
	}
}
