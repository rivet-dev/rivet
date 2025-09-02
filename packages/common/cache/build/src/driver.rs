use std::{
	fmt::Debug,
	time::{Duration, Instant},
};

use moka::future::{Cache, CacheBuilder};
use serde::{Serialize, de::DeserializeOwned};
use tracing::Instrument;

use rivet_metrics::KeyValue;

use crate::{errors::Error, metrics};

/// Type alias for cache values stored as bytes
pub type CacheValue = Vec<u8>;

/// Enum wrapper for different cache driver implementations
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum Driver {
	InMemory(InMemoryDriver),
}

impl Driver {
	/// Fetch multiple values from cache at once
	#[tracing::instrument(skip_all, fields(driver=%self))]
	pub async fn fetch_values<'a>(
		&'a self,
		base_key: &'a str,
		keys: &[String],
	) -> Result<Vec<Option<CacheValue>>, Error> {
		match self {
			Driver::InMemory(d) => d.fetch_values(base_key, keys).await,
		}
	}

	/// Set multiple values in cache at once
	#[tracing::instrument(skip_all, fields(driver=%self))]
	pub async fn set_values<'a>(
		&'a self,
		base_key: &'a str,
		keys_values: Vec<(String, CacheValue, i64)>,
	) -> Result<(), Error> {
		match self {
			Driver::InMemory(d) => d.set_values(base_key, keys_values).await,
		}
	}

	/// Delete multiple keys from cache
	#[tracing::instrument(skip_all, fields(driver=%self))]
	pub async fn delete_keys<'a>(
		&'a self,
		base_key: &'a str,
		keys: Vec<String>,
	) -> Result<(), Error> {
		match self {
			Driver::InMemory(d) => d.delete_keys(base_key, keys).await,
		}
	}

	/// Process a raw key into a driver-specific format
	///
	/// Different implementations use different key formats:
	/// - In-memory uses simpler keys
	pub fn process_key(&self, base_key: &str, key: &impl crate::CacheKey) -> String {
		match self {
			Driver::InMemory(d) => d.process_key(base_key, key),
		}
	}

	/// Process a rate limit key into a driver-specific format
	pub fn process_rate_limit_key(
		&self,
		key: &impl crate::CacheKey,
		remote_address: impl AsRef<str>,
		bucket: i64,
		bucket_duration_ms: i64,
	) -> String {
		match self {
			Driver::InMemory(d) => {
				d.process_rate_limit_key(key, remote_address, bucket, bucket_duration_ms)
			}
		}
	}

	/// Increment a rate limit counter and return the new count
	#[tracing::instrument(skip_all, fields(driver=%self))]
	pub async fn rate_limit_increment<'a>(
		&'a self,
		key: &'a str,
		ttl_ms: i64,
	) -> Result<i64, Error> {
		match self {
			Driver::InMemory(d) => d.rate_limit_increment(key, ttl_ms).await,
		}
	}

	pub fn encode_value<T: Serialize>(&self, value: &T) -> Result<CacheValue, Error> {
		serde_json::to_vec(value).map_err(Error::SerdeEncode)
	}

	pub fn decode_value<T: DeserializeOwned>(&self, bytes: &[u8]) -> Result<T, Error> {
		serde_json::from_slice(bytes).map_err(Error::SerdeDecode)
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
#[derive(Clone, Debug)]
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
#[derive(Clone)]
pub struct InMemoryDriver {
	service_name: String,
	cache: Cache<String, ExpiringValue>,
	/// In-memory rate limiting store - maps keys to hit counts with expiration
	rate_limits: Cache<String, ExpiringValue>,
}

impl Debug for InMemoryDriver {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("InMemoryDriver")
			.field("service_name", &self.service_name)
			.finish()
	}
}

impl InMemoryDriver {
	pub fn new(service_name: String, max_capacity: u64) -> Self {
		// Create a cache with ValueExpiry implementation for custom expiration times
		let cache = CacheBuilder::new(max_capacity)
			.expire_after(ValueExpiry)
			.build();

		// Create a separate cache for rate limiting with the same expiry mechanism
		let rate_limits = CacheBuilder::new(max_capacity)
			.expire_after(ValueExpiry)
			.build();

		Self {
			service_name,
			cache,
			rate_limits,
		}
	}

	pub async fn fetch_values<'a>(
		&'a self,
		_base_key: &'a str,
		keys: &[String],
	) -> Result<Vec<Option<CacheValue>>, Error> {
		let keys = keys.to_vec();
		let cache = self.cache.clone();

		let mut result = Vec::with_capacity(keys.len());

		// Async block for metrics
		async {
			for key in keys {
				result.push(cache.get(&key).await.map(|x| x.value.clone()));
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

	pub async fn set_values<'a>(
		&'a self,
		_base_key: &'a str,
		keys_values: Vec<(String, CacheValue, i64)>,
	) -> Result<(), Error> {
		let cache = self.cache.clone();

		// Async block for metrics
		async {
			for (key, value, expire_at) in keys_values {
				// Create an entry with the value and expiration time
				let entry = ExpiringValue {
					value,
					expiry_time: expire_at,
				};

				// Store in cache - expiry will be handled by ValueExpiry
				cache.insert(key, entry).await;
			}
		}
		.instrument(tracing::info_span!("set"))
		.await;

		tracing::trace!("successfully wrote to in-memory cache with per-key expiry");
		Ok(())
	}

	pub async fn delete_keys<'a>(
		&'a self,
		base_key: &'a str,
		keys: Vec<String>,
	) -> Result<(), Error> {
		let cache = self.cache.clone();
		let base_key = base_key.to_string();

		metrics::CACHE_PURGE_REQUEST_TOTAL.add(1, &[KeyValue::new("key", base_key.clone())]);
		metrics::CACHE_PURGE_VALUE_TOTAL
			.add(keys.len() as u64, &[KeyValue::new("key", base_key.clone())]);

		// Async block for metrics
		async {
			for key in keys {
				// Use remove instead of invalidate to ensure it's actually removed
				cache.remove(&key).await;
			}
		}
		.instrument(tracing::info_span!("remove"))
		.await;

		tracing::trace!("successfully deleted keys from in-memory cache");
		Ok(())
	}

	pub fn process_key(&self, base_key: &str, key: &impl crate::CacheKey) -> String {
		format!("{}:{}", base_key, key.cache_key())
	}

	pub fn process_rate_limit_key(
		&self,
		key: &impl crate::CacheKey,
		remote_address: impl AsRef<str>,
		bucket: i64,
		bucket_duration_ms: i64,
	) -> String {
		format!(
			"rate_limit:{}:{}:{}:{}",
			key.cache_key(),
			remote_address.as_ref(),
			bucket_duration_ms,
			bucket,
		)
	}

	/// Increment a rate limit counter for in-memory storage
	pub async fn rate_limit_increment<'a>(
		&'a self,
		key: &'a str,
		ttl_ms: i64,
	) -> Result<i64, Error> {
		let rate_limits = self.rate_limits.clone();

		// Get current value or default to 0
		let current_value = match rate_limits.get(key).await {
			Some(value) => {
				// Try to decode the value as an integer
				match serde_json::from_slice::<i64>(&value.value).map_err(Error::SerdeDecode) {
					Ok(count) => count,
					Err(_) => 0, // If we can't decode, reset to 0
				}
			}
			None => 0,
		};

		// Increment the value
		let new_value = current_value + 1;
		let encoded = serde_json::to_vec(&new_value).map_err(Error::SerdeEncode)?;

		// Store with expiration
		let entry = ExpiringValue {
			value: encoded,
			expiry_time: rivet_util::timestamp::now() + ttl_ms,
		};

		// Update the rate limit cache
		rate_limits.insert(key.to_string(), entry).await;

		Ok(new_value)
	}
}
