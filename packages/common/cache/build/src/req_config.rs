use std::{fmt::Debug, future::Future, result::Result::Ok};

use anyhow::*;
use serde::{Serialize, de::DeserializeOwned};
use tracing::Instrument;

use super::*;
use crate::{errors::Error, metrics};
use rivet_metrics::KeyValue;

/// Config specifying how cached values will behave.
#[derive(Clone)]
pub struct RequestConfig {
	pub(super) cache: Cache,
	ttl: i64,
}

impl Debug for RequestConfig {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RequestConfig")
			.field("cache", &self.cache)
			.field("ttl", &self.ttl)
			.finish()
	}
}

impl RequestConfig {
	pub(crate) fn new(cache: Cache) -> Self {
		RequestConfig {
			cache,
			ttl: rivet_util::duration::hours(2),
		}
	}

	/// Sets the TTL for the keys in ms.
	///
	/// Defaults to 2 hours.
	pub fn ttl(mut self, ttl: i64) -> Self {
		self.ttl = ttl;
		self
	}
}

// MARK: Fetch
impl RequestConfig {
	/// Attempts to fetch a given key from the database and falls back to the
	/// getter if not found. If both the cache and the getter return `None`,
	/// then this returns `None`.
	///
	/// See `fetch_all` for more details.
	#[tracing::instrument(err, skip(key, getter))]
	pub async fn fetch_one<K, V, Getter, Fut>(
		self,
		base_key: impl ToString + Debug,
		key: K,
		getter: Getter,
	) -> Result<Option<V>>
	where
		K: CacheKey + Send + Sync,
		V: Serialize + DeserializeOwned + Clone + Debug + Send + Sync,
		Getter: Fn(GetterCtx<K, V>, K) -> Fut + Clone,
		Fut: Future<Output = Result<GetterCtx<K, V>>>,
	{
		let values = self
			.fetch_all(base_key, [key], move |cache, keys| {
				let getter = getter.clone();
				async move {
					debug_assert_eq!(1, keys.len());
					if let Some(key) = keys.into_iter().next() {
						getter(cache, key).await
					} else {
						tracing::error!("no keys provided to fetch one");
						Ok(cache)
					}
				}
			})
			.await?;
		Ok(values.into_iter().next().map(|(_, v)| v))
	}

	#[tracing::instrument(err, skip(keys, getter))]
	pub async fn fetch_all<Key, Value, Getter, Fut>(
		self,
		base_key: impl ToString + Debug,
		keys: impl IntoIterator<Item = Key>,
		getter: Getter,
	) -> Result<Vec<(Key, Value)>>
	where
		Key: CacheKey + Send + Sync,
		Value: Serialize + DeserializeOwned + Clone + Debug + Send + Sync,
		Getter: Fn(GetterCtx<Key, Value>, Vec<Key>) -> Fut + Clone,
		Fut: Future<Output = Result<GetterCtx<Key, Value>>>,
	{
		self.fetch_all_convert(
			base_key,
			keys,
			getter,
			|x: &Value| Ok(x.clone()),
			|x: &Value| Ok(x.clone()),
		)
		.await
	}

	#[tracing::instrument(err, skip(keys, getter, encoder, decoder))]
	async fn fetch_all_convert<Key, Value, ValueSerde, Getter, Fut, Encoder, Decoder>(
		self,
		base_key: impl ToString + Debug,
		keys: impl IntoIterator<Item = Key>,
		getter: Getter,
		encoder: Encoder,
		decoder: Decoder,
	) -> Result<Vec<(Key, Value)>>
	where
		Key: CacheKey + Send + Sync,
		Value: Debug + Send + Sync,
		ValueSerde: Serialize + DeserializeOwned + Debug + Send + Sync,
		Getter: Fn(GetterCtx<Key, Value>, Vec<Key>) -> Fut + Clone,
		Fut: Future<Output = Result<GetterCtx<Key, Value>>>,
		Encoder: Fn(&Value) -> Result<ValueSerde> + Clone,
		Decoder: Fn(&ValueSerde) -> Result<Value> + Clone,
	{
		let base_key = base_key.to_string();
		let keys = keys.into_iter().collect::<Vec<Key>>();

		// Ignore empty keys
		if keys.is_empty() {
			return Ok(Vec::new());
		}

		metrics::CACHE_REQUEST_TOTAL.add(1, &[KeyValue::new("key", base_key.clone())]);
		metrics::CACHE_VALUE_TOTAL
			.add(keys.len() as u64, &[KeyValue::new("key", base_key.clone())]);

		// Build context.
		//
		// Drop `keys` bc this is not the same as the keys list in `ctx`, so it should not be used
		// again.
		let mut ctx = GetterCtx::new(base_key.clone(), keys);

		// Build driver-specific cache keys
		let cache_keys = ctx
			.keys()
			.iter()
			.map(|key| self.cache.driver.process_key(&base_key, &key.key))
			.collect::<Vec<_>>();

		// Attempt to fetch value from cache, fall back to getter
		match self.cache.driver.fetch_values(&base_key, &cache_keys).await {
			Ok(cached_values) => {
				debug_assert_eq!(
					cache_keys.len(),
					cached_values.len(),
					"cache returned wrong number of values"
				);

				// Create the getter ctx and resolve the cached values
				for (i, value) in cached_values.into_iter().enumerate() {
					if let Some(value_bytes) = value {
						// Try to decode the value using the driver
						match self.cache.driver.decode_value(&value_bytes) {
							Ok(value_serde) => match decoder(&value_serde) {
								Ok(value) => {
									ctx.resolve_from_cache(i, value);
								}
								Err(err) => {
									tracing::error!(?err, "Failed to decode value");
								}
							},
							Err(err) => {
								tracing::error!(?err, "Failed to decode value");
							}
						}
					}
				}

				// Fetch remaining values and add to the cached list
				if !ctx.all_keys_have_value() {
					// Call the getter
					let remaining_keys = ctx.unresolved_keys();
					let unresolved_len = remaining_keys.len();

					metrics::CACHE_VALUE_MISS_TOTAL.add(
						unresolved_len as u64,
						&[KeyValue::new("key", base_key.clone())],
					);

					ctx = getter(ctx, remaining_keys).await.map_err(Error::Getter)?;

					// Write the values to cache
					let expire_at = rivet_util::timestamp::now() + self.ttl;
					let values_needing_cache_write = ctx.values_needing_cache_write();

					tracing::trace!(
						unresolved_len,
						fetched_len = values_needing_cache_write.len(),
						"writing new values to cache"
					);

					// Convert values to cache bytes
					let keys_values = values_needing_cache_write
						.into_iter()
						.filter_map(|(key, value)| {
							// Process the key with the appropriate driver
							let driver_key = self.cache.driver.process_key(&base_key, &key.key);
							// Try to decode the value using the driver
							match encoder(&value) {
								Ok(value_bytes) => {
									match self.cache.driver.encode_value(&value_bytes) {
										Ok(value_serde) => {
											Some((driver_key, value_serde, expire_at))
										}
										Err(err) => {
											tracing::error!(?err, "Failed to encode value");

											None
										}
									}
								}
								Err(err) => {
									tracing::error!(?err, "Failed to encode value");

									None
								}
							}
						})
						.collect::<Vec<_>>();

					if !keys_values.is_empty() {
						let driver = self.cache.driver.clone();
						let base_key_clone = base_key.clone();

						let spawn_res = tokio::task::Builder::new().name("cache::write").spawn(
							async move {
								if let Err(err) =
									driver.set_values(&base_key_clone, keys_values).await
								{
									tracing::error!(?err, "failed to write to cache");
								}
							}
							.in_current_span(),
						);
						if let Err(err) = spawn_res {
							tracing::error!(?err, "failed to spawn cache::write task");
						}
					}
				}

				metrics::CACHE_VALUE_EMPTY_TOTAL.add(
					ctx.unresolved_keys().len() as u64,
					&[KeyValue::new("key", base_key.clone())],
				);

				Ok(ctx.into_values())
			}
			Err(err) => {
				tracing::error!(
					?err,
					"failed to read batch keys from cache, falling back to getter"
				);

				metrics::CACHE_REQUEST_ERRORS.add(1, &[KeyValue::new("key", base_key.clone())]);

				// Fall back to the getter since we can't fetch the value from
				// the cache
				let keys = ctx.unresolved_keys();
				let ctx = getter(ctx, keys).await.map_err(Error::Getter)?;

				Ok(ctx.into_values())
			}
		}
	}

	#[tracing::instrument(err, skip(keys))]
	pub async fn purge<Key>(
		self,
		base_key: impl AsRef<str> + Debug,
		keys: impl IntoIterator<Item = Key>,
	) -> Result<()>
	where
		Key: CacheKey + Send + Sync,
	{
		// Build keys
		let base_key = base_key.as_ref();
		let cache_keys = keys
			.into_iter()
			.map(|key| self.cache.driver.process_key(base_key, &key))
			.collect::<Vec<_>>();

		if cache_keys.is_empty() {
			return Ok(());
		}

		// Delete keys
		match self.cache.driver.delete_keys(base_key, cache_keys).await {
			Ok(_) => {
				tracing::trace!("successfully deleted keys");
			}
			Err(err) => {
				tracing::error!(?err, "failed to delete from cache, proceeding regardless")
			}
		}

		Ok(())
	}
}

// MARK: JSON fetch
impl RequestConfig {
	#[tracing::instrument(err, skip(key, getter))]
	pub async fn fetch_one_json<Key, Value, Getter, Fut>(
		self,
		base_key: impl ToString + Debug,
		key: Key,
		getter: Getter,
	) -> Result<Option<Value>>
	where
		Key: CacheKey + Send + Sync,
		Value: Serialize + DeserializeOwned + Debug + Send + Sync,
		Getter: Fn(GetterCtx<Key, Value>, Key) -> Fut + Clone,
		Fut: Future<Output = Result<GetterCtx<Key, Value>>>,
	{
		let values = self
			.fetch_all_json_with_keys(base_key, [key], move |cache, keys| {
				let getter = getter.clone();
				async move {
					debug_assert_eq!(1, keys.len());
					if let Some(key) = keys.into_iter().next() {
						getter(cache, key).await
					} else {
						tracing::error!("no keys provided to fetch one");
						Ok(cache)
					}
				}
			})
			.await?;
		Ok(values.into_iter().next().map(|(_, v)| v))
	}

	#[tracing::instrument(err, skip(keys, getter))]
	pub async fn fetch_all_json<Key, Value, Getter, Fut>(
		self,
		base_key: impl ToString + Debug,
		keys: impl IntoIterator<Item = Key>,
		getter: Getter,
	) -> Result<Vec<Value>>
	where
		Key: CacheKey + Send + Sync,
		Value: Serialize + DeserializeOwned + Debug + Send + Sync,
		Getter: Fn(GetterCtx<Key, Value>, Vec<Key>) -> Fut + Clone,
		Fut: Future<Output = Result<GetterCtx<Key, Value>>>,
	{
		self.fetch_all_json_with_keys::<Key, Value, Getter, Fut>(base_key, keys, getter)
			.await
			// TODO: Find a way to not allocate another vec here
			.map(|x| x.into_iter().map(|(_, v)| v).collect::<Vec<_>>())
	}

	#[tracing::instrument(err, skip(keys, getter))]
	pub async fn fetch_all_json_with_keys<Key, Value, Getter, Fut>(
		self,
		base_key: impl ToString + Debug,
		keys: impl IntoIterator<Item = Key>,
		getter: Getter,
	) -> Result<Vec<(Key, Value)>>
	where
		Key: CacheKey + Send + Sync,
		Value: Serialize + DeserializeOwned + Debug + Send + Sync,
		Getter: Fn(GetterCtx<Key, Value>, Vec<Key>) -> Fut + Clone,
		Fut: Future<Output = Result<GetterCtx<Key, Value>>>,
	{
		self.fetch_all_convert(
			base_key,
			keys,
			getter,
			|value: &Value| -> Result<Vec<u8>> {
				serde_json::to_vec(value)
					.map_err(Error::SerdeEncode)
					.map_err(Into::into)
			},
			|value: &Vec<u8>| -> Result<Value> {
				serde_json::from_slice(value.as_slice())
					.map_err(Error::SerdeDecode)
					.map_err(Into::into)
			},
		)
		.await
	}
}
