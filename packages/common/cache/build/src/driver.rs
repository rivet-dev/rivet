use std::{
	fmt::Debug,
	time::{Duration, Instant},
};

use moka::future::{Cache, CacheBuilder};
use redis::AsyncCommands;
use rivet_pools::prelude::*;
use tracing::Instrument;

use crate::{error::Error, metrics};

/// Type alias for cache values stored as bytes
pub type CacheValue = Vec<u8>;

/// Enum wrapper for different cache driver implementations
#[derive(Debug, Clone)]
pub enum Driver {
	Redis(RedisDriver),
	InMemory(InMemoryDriver),
}

impl Driver {
	/// Fetch multiple values from cache at once
	#[tracing::instrument(skip_all, fields(driver=%self))]
	pub async fn fetch_values<'a>(
		&'a self,
		base_key: &'a str,
		redis_keys: &[String],
	) -> Result<Vec<Option<CacheValue>>, Error> {
		match self {
			Driver::Redis(d) => d.fetch_values(base_key, redis_keys).await,
			Driver::InMemory(d) => d.fetch_values(base_key, redis_keys).await,
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
			Driver::Redis(d) => d.set_values(base_key, keys_values).await,
			Driver::InMemory(d) => d.set_values(base_key, keys_values).await,
		}
	}

	/// Delete multiple keys from cache
	#[tracing::instrument(skip_all, fields(driver=%self))]
	pub async fn delete_keys<'a>(
		&'a self,
		base_key: &'a str,
		redis_keys: Vec<String>,
	) -> Result<(), Error> {
		match self {
			Driver::Redis(d) => d.delete_keys(base_key, redis_keys).await,
			Driver::InMemory(d) => d.delete_keys(base_key, redis_keys).await,
		}
	}

	/// Process a raw key into a driver-specific format
	///
	/// Different implementations use different key formats:
	/// - Redis uses hash tags for key distribution
	/// - In-memory uses simpler keys
	pub fn process_key(&self, base_key: &str, key: &impl crate::CacheKey) -> String {
		match self {
			Driver::Redis(d) => d.process_key(base_key, key),
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
			Driver::Redis(d) => {
				d.process_rate_limit_key(key, remote_address, bucket, bucket_duration_ms)
			}
			Driver::InMemory(d) => {
				d.process_rate_limit_key(key, remote_address, bucket, bucket_duration_ms)
			}
		}
	}

	/// Get the Redis connection if this is a Redis driver
	pub fn redis_conn(&self) -> Option<RedisPool> {
		match self {
			Driver::Redis(d) => Some(d.redis()),
			Driver::InMemory(_) => None,
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
			Driver::Redis(d) => d.rate_limit_increment(key, ttl_ms).await,
			Driver::InMemory(d) => d.rate_limit_increment(key, ttl_ms).await,
		}
	}

	/// Encode a value into bytes for storage in the cache
	pub fn encode_value<T: redis::ToRedisArgs>(&self, value: &T) -> CacheValue {
		match self {
			Driver::Redis(d) => d.encode_value(value),
			Driver::InMemory(d) => d.encode_value(value),
		}
	}

	/// Decode a value from bytes retrieved from the cache
	pub fn decode_value<T: redis::FromRedisValue>(&self, bytes: &[u8]) -> Result<T, Error> {
		match self {
			Driver::Redis(d) => d.decode_value(bytes),
			Driver::InMemory(d) => d.decode_value(bytes),
		}
	}
}

impl std::fmt::Display for Driver {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Driver::Redis(_) => write!(f, "redis"),
			Driver::InMemory(_) => write!(f, "in_memory"),
		}
	}
}

/// Redis cache driver implementation
#[derive(Clone)]
pub struct RedisDriver {
	service_name: String,
	redis_conn: RedisPool,
}

impl Debug for RedisDriver {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RedisDriver")
			.field("service_name", &self.service_name)
			.finish()
	}
}

impl RedisDriver {
	pub fn new(service_name: String, redis_conn: RedisPool) -> Self {
		Self {
			service_name,
			redis_conn,
		}
	}

	pub fn redis(&self) -> RedisPool {
		self.redis_conn.clone()
	}

	pub fn to_redis_bytes<V: redis::ToRedisArgs>(value: &V) -> CacheValue {
		value
			.to_redis_args()
			.into_iter()
			.next()
			.unwrap_or_default()
			.to_vec()
	}

	pub async fn fetch_values<'a>(
		&'a self,
		base_key: &'a str,
		redis_keys: &[String],
	) -> Result<Vec<Option<CacheValue>>, Error> {
		let redis_keys = redis_keys.to_vec();
		let mut redis_conn = self.redis_conn.clone();

		// Build Redis command explicitly, since `conn.get` with one value will
		// not return a vector
		let mut mget_cmd = redis::cmd("MGET");
		for key in &redis_keys {
			mget_cmd.arg(key);
		}

		match mget_cmd
			.query_async::<_, Vec<Option<CacheValue>>>(&mut redis_conn)
			.instrument(tracing::info_span!("redis_query"))
			.await
		{
			Ok(values) => {
				tracing::debug!(
					cached_len = values.iter().filter(|x| x.is_some()).count(),
					total_len = values.len(),
					"read from cache"
				);
				Ok(values)
			}
			Err(err) => {
				tracing::error!(?err, "failed to read batch keys from cache");
				metrics::CACHE_REQUEST_ERRORS
					.with_label_values(&[base_key])
					.inc();
				Err(Error::ConnectRedis(err))
			}
		}
	}

	pub async fn set_values<'a>(
		&'a self,
		_base_key: &'a str,
		keys_values: Vec<(String, CacheValue, i64)>,
	) -> Result<(), Error> {
		let mut redis_conn = self.redis_conn.clone();

		let mut pipe = redis::pipe();

		for (redis_key, value, expire_at) in keys_values {
			// Write the value with the expiration
			pipe.cmd("SET")
				.arg(&redis_key)
				.arg(value)
				.arg("PXAT")
				.arg(expire_at)
				.ignore();
		}

		match pipe
			.query_async(&mut redis_conn)
			.instrument(tracing::info_span!("redis_query"))
			.await
		{
			Ok(()) => {
				tracing::trace!("successfully wrote to cache");
				Ok(())
			}
			Err(err) => {
				tracing::error!(?err, "failed to write to cache");
				Err(Error::ConnectRedis(err))
			}
		}
	}

	pub async fn delete_keys<'a>(
		&'a self,
		base_key: &'a str,
		redis_keys: Vec<String>,
	) -> Result<(), Error> {
		let mut redis_conn = self.redis_conn.clone();
		let base_key = base_key.to_string();

		metrics::CACHE_PURGE_REQUEST_TOTAL
			.with_label_values(&[&base_key])
			.inc();
		metrics::CACHE_PURGE_VALUE_TOTAL
			.with_label_values(&[&base_key])
			.inc_by(redis_keys.len() as u64);

		match redis_conn
			.del::<_, ()>(redis_keys)
			.instrument(tracing::info_span!("redis_query"))
			.await
		{
			Ok(_) => {
				tracing::trace!("successfully deleted keys");
				Ok(())
			}
			Err(err) => {
				tracing::error!(?err, "failed to delete from cache");
				Err(Error::ConnectRedis(err))
			}
		}
	}

	pub fn process_key(&self, base_key: &str, key: &impl crate::CacheKey) -> String {
		// Redis hash tag for key distribution
		format!("{{key:{}}}:{}", base_key, key.simple_cache_key())
	}

	pub fn process_rate_limit_key(
		&self,
		key: &impl crate::CacheKey,
		remote_address: impl AsRef<str>,
		bucket: i64,
		bucket_duration_ms: i64,
	) -> String {
		// Redis hash tag for key distribution
		format!(
			"{{global}}:cache:rate_limit:{}:{}:{}:{}",
			key.simple_cache_key(),
			remote_address.as_ref(),
			bucket_duration_ms,
			bucket,
		)
	}

	// For compatibility with existing Redis code
	pub fn encode_value<T: redis::ToRedisArgs>(&self, value: &T) -> CacheValue {
		Self::to_redis_bytes(value)
	}

	pub fn decode_value<T: redis::FromRedisValue>(&self, bytes: &[u8]) -> Result<T, Error> {
		redis::from_redis_value(&redis::Value::Data(bytes.to_vec()))
			.map_err(|e| Error::RedisDecode(e))
	}

	/// Increment a rate limit counter using Redis atomic operations
	pub async fn rate_limit_increment<'a>(
		&'a self,
		key: &'a str,
		ttl_ms: i64,
	) -> Result<i64, Error> {
		let mut redis_conn = self.redis_conn.clone();

		// Use Redis INCR and PEXPIRE in an atomic transaction
		let mut pipe = redis::pipe();
		pipe.atomic();
		pipe.incr(key, 1);
		pipe.pexpire(key, ttl_ms as usize).ignore();

		match pipe
			.query_async::<_, (i64,)>(&mut redis_conn)
			.instrument(tracing::info_span!("redis_query"))
			.await
		{
			Ok((incr,)) => Ok(incr),
			Err(err) => {
				tracing::error!(?err, ?key, "failed to increment rate limit key");
				Err(Error::ConnectRedis(err))
			}
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

		metrics::CACHE_PURGE_REQUEST_TOTAL
			.with_label_values(&[&base_key])
			.inc();
		metrics::CACHE_PURGE_VALUE_TOTAL
			.with_label_values(&[&base_key])
			.inc_by(keys.len() as u64);

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
		// For in-memory cache, we use simpler keys without Redis hash tags
		format!("{}:{}", base_key, key.simple_cache_key())
	}

	pub fn process_rate_limit_key(
		&self,
		key: &impl crate::CacheKey,
		remote_address: impl AsRef<str>,
		bucket: i64,
		bucket_duration_ms: i64,
	) -> String {
		// For in-memory cache, we use simpler keys without Redis hash tags
		format!(
			"rate_limit:{}:{}:{}:{}",
			key.simple_cache_key(),
			remote_address.as_ref(),
			bucket_duration_ms,
			bucket,
		)
	}

	// For compatibility with existing Redis code
	pub fn encode_value<T: redis::ToRedisArgs>(&self, value: &T) -> CacheValue {
		// We still use Redis encoding for compatibility
		RedisDriver::to_redis_bytes(value)
	}

	pub fn decode_value<T: redis::FromRedisValue>(&self, bytes: &[u8]) -> Result<T, Error> {
		// We still use Redis decoding for compatibility
		redis::from_redis_value(&redis::Value::Data(bytes.to_vec()))
			.map_err(|e| Error::RedisDecode(e))
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
				match redis::from_redis_value::<i64>(&redis::Value::Data(value.value)) {
					Ok(count) => count,
					Err(_) => 0, // If we can't decode, reset to 0
				}
			}
			None => 0,
		};

		// Increment the value
		let new_value = current_value + 1;
		let encoded = RedisDriver::to_redis_bytes(&new_value);

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
