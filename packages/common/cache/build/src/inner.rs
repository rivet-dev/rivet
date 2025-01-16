use std::{fmt::Debug, sync::Arc};

use rivet_pools::prelude::*;

use super::*;

pub type Cache = Arc<CacheInner>;

/// Utility type used to hold information relating to caching.
pub struct CacheInner {
	service_name: String,
	pub(crate) redis_conn: RedisPool,
}

impl Debug for CacheInner {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("CacheInner")
			.field("service_name", &self.service_name)
			.finish()
	}
}

impl CacheInner {
	#[tracing::instrument(skip(pools))]
	pub fn from_env(pools: rivet_pools::Pools) -> Result<Cache, Error> {
		let service_name = rivet_env::service_name();
		let redis_cache = pools.redis_cache().map_err(Error::Pools)?;

		Ok(Self::new(
			service_name.to_string(),
			redis_cache,
		))
	}

	#[tracing::instrument(skip(redis_conn))]
	pub fn new(service_name: String, redis_conn: RedisPool) -> Cache {
		Arc::new(CacheInner {
			service_name,
			redis_conn,
		})
	}

	pub fn redis(&self) -> RedisPool {
		self.redis_conn.clone()
	}

	pub(crate) fn build_redis_cache_key<K>(&self, base_key: &str, key: &K) -> String
	where
		K: CacheKey,
	{
		format!("{{key:{}}}:{}", base_key, key.simple_cache_key())
	}

	// pub(crate) fn build_redis_topic_key(&self, base_key: &str, key: &impl CacheKey) -> String {
	// 	format!("{{topic:{}}}:{}:keys", base_key, key.simple_cache_key())
	// }

	pub(crate) fn build_redis_rate_limit_key(
		&self,
		key: &impl CacheKey,
		remote_address: impl AsRef<str>,
		bucket: i64,
		bucket_duration_ms: i64,
	) -> String {
		format!(
			"{{global}}:cache:rate_limit:{}:{}:{}:{}",
			key.simple_cache_key(),
			remote_address.as_ref(),
			bucket_duration_ms,
			bucket,
		)
	}
}

impl CacheInner {
	/// Returns a new request config builder.
	pub fn request(self: Arc<Self>) -> RequestConfig {
		RequestConfig::new(self.clone())
	}
}
