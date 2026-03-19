use std::{
	fmt::Debug,
	time::{Duration, Instant},
};

use moka::future::{Cache, CacheBuilder};
use tracing::Instrument;

use crate::{RawCacheKey, errors::Error};

/// Type alias for cache values stored as bytes
pub type CacheValue = Vec<u8>;

/// Enum wrapper for different cache driver implementations
#[non_exhaustive]
pub enum Driver {
	InMemory(InMemoryDriver),
}

impl Driver {
	/// Fetch multiple values from cache at once
	#[tracing::instrument(skip_all, fields(driver=%self))]
	pub async fn get<'a>(
		&'a self,
		base_key: &'a str,
		keys: Vec<RawCacheKey>,
	) -> Result<Vec<Option<CacheValue>>, Error> {
		match self {
			Driver::InMemory(d) => d.get(base_key, keys).await,
		}
	}

	/// Set multiple values in cache at once
	#[tracing::instrument(skip_all, fields(driver=%self))]
	pub async fn set<'a>(
		&'a self,
		base_key: &'a str,
		keys_values: Vec<(RawCacheKey, CacheValue, i64)>,
	) -> Result<(), Error> {
		match self {
			Driver::InMemory(d) => d.set(base_key, keys_values).await,
		}
	}

	/// Delete multiple keys from cache
	#[tracing::instrument(skip_all, fields(driver=%self))]
	pub async fn delete<'a>(
		&'a self,
		base_key: &'a str,
		keys: Vec<RawCacheKey>,
	) -> Result<(), Error> {
		match self {
			Driver::InMemory(d) => d.delete(base_key, keys).await,
		}
	}

	/// Process a raw key into a driver-specific format
	///
	/// Different implementations use different key formats:
	/// - In-memory uses simpler keys
	pub fn process_key(&self, base_key: &str, key: &impl crate::CacheKey) -> RawCacheKey {
		match self {
			Driver::InMemory(d) => d.process_key(base_key, key),
		}
	}
}

impl std::fmt::Display for Driver {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Driver::InMemory(_) => write!(f, "in_memory"),
		}
	}
}

/// Entry with custom expiration time
#[derive(Clone, Debug)]
struct ExpiringValue {
	/// The actual cached value
	value: CacheValue,
	/// The expiration time (epoch milliseconds)
	expiry_time: i64,
}

/// Cache expiry implementation for Moka
#[derive(Debug)]
struct ValueExpiry;

impl moka::Expiry<String, ExpiringValue> for ValueExpiry {
	// Define expiration based on creation
	fn expire_after_create(
		&self,
		_key: &String,
		value: &ExpiringValue,
		_current_time: Instant,
	) -> Option<Duration> {
		// Calculate the time remaining until expiration
		let now = rivet_util::timestamp::now();

		if value.expiry_time > now {
			// Convert to Duration
			Some(Duration::from_millis((value.expiry_time - now) as u64))
		} else {
			// Expire immediately if already past expiration
			Some(Duration::from_secs(0))
		}
	}

	// Handle updates - keep using the same expiry logic
	fn expire_after_update(
		&self,
		key: &String,
		value: &ExpiringValue,
		current_time: Instant,
		_last_expire_duration: Option<Duration>,
	) -> Option<Duration> {
		// Just use the same logic as create
		self.expire_after_create(key, value, current_time)
	}

	// Handle reads - keep using the same expiry logic
	fn expire_after_read(
		&self,
		key: &String,
		value: &ExpiringValue,
		current_time: Instant,
		_last_expire_duration: Option<Duration>,
		_last_modified_at: Instant,
	) -> Option<Duration> {
		// Just use the same logic as create
		self.expire_after_create(key, value, current_time)
	}
}

/// In-memory cache driver implementation using the moka crate
pub struct InMemoryDriver {
	cache: Cache<String, ExpiringValue>,
}

impl Debug for InMemoryDriver {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("InMemoryDriver").finish()
	}
}

impl InMemoryDriver {
	pub fn new(max_capacity: u64) -> Self {
		// Create a cache with ValueExpiry implementation for custom expiration times
		let cache = CacheBuilder::new(max_capacity)
			.expire_after(ValueExpiry)
			.build();

		Self { cache }
	}

	pub async fn get<'a>(
		&'a self,
		_base_key: &'a str,
		keys: Vec<RawCacheKey>,
	) -> Result<Vec<Option<CacheValue>>, Error> {
		let mut result = Vec::with_capacity(keys.len());

		// Async block for metrics
		async {
			for key in keys {
				result.push(self.cache.get(&*key).await.map(|x| x.value.clone()));
			}
		}
		.instrument(tracing::info_span!("get"))
		.await;

		tracing::debug!(
			cached_len = result.iter().filter(|x| x.is_some()).count(),
			total_len = result.len(),
			"read from in-memory cache"
		);

		Ok(result)
	}

	pub async fn set<'a>(
		&'a self,
		_base_key: &'a str,
		keys_values: Vec<(RawCacheKey, CacheValue, i64)>,
	) -> Result<(), Error> {
		// Async block for metrics
		async {
			for (key, value, expire_at) in keys_values {
				// Create an entry with the value and expiration time
				let entry = ExpiringValue {
					value,
					expiry_time: expire_at,
				};

				// Store in cache - expiry will be handled by ValueExpiry
				self.cache.insert(key.into(), entry).await;
			}
		}
		.instrument(tracing::info_span!("set"))
		.await;

		tracing::trace!("successfully wrote to in-memory cache with per-key expiry");
		Ok(())
	}

	pub async fn delete<'a>(
		&'a self,
		_base_key: &'a str,
		keys: Vec<RawCacheKey>,
	) -> Result<(), Error> {
		// Async block for metrics
		async {
			for key in keys {
				// Use remove instead of invalidate to ensure it's actually removed
				self.cache.remove(&*key).await;
			}
		}
		.instrument(tracing::info_span!("delete"))
		.await;

		tracing::trace!("successfully deleted keys from in-memory cache");
		Ok(())
	}

	pub fn process_key(&self, base_key: &str, key: &impl crate::CacheKey) -> RawCacheKey {
		RawCacheKey::from(format!("{}:{}", base_key, key.cache_key()))
	}
}
