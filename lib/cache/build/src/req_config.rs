use redis::AsyncCommands;
use rivet_pools::prelude::*;
use std::{
	fmt::Debug,
	future::Future,
	time::{Duration, SystemTime},
};
use tracing::Instrument;

use crate::error::{Error, GetterResult};

use super::*;

/// Config specifying how cached values will behave.
#[derive(Clone)]
pub struct RequestConfig {
	pub(super) cache: Cache,
	ttl: i64,
	immutable: bool,
}

impl Debug for RequestConfig {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("RequestConfig")
			.field("cache", &self.cache)
			.field("ttl", &self.ttl)
			.field("immutable", &self.immutable)
			.finish()
	}
}

impl RequestConfig {
	pub(crate) fn new(cache: Cache) -> Self {
		RequestConfig {
			cache,
			ttl: rivet_util::duration::hours(2),
			immutable: false,
		}
	}

	/// Sets the TTL for the keys in ms.
	///
	/// Defaults to 2 days.
	pub fn ttl(mut self, ttl: i64) -> Self {
		self.ttl = ttl;
		self
	}

	/// Deterines if the value for this key can change. If the value is immutable, we apply more
	/// aggressive caching rules to it.
	pub fn immutable(mut self) -> Self {
		self.immutable = true;
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
	) -> Result<Option<V>, Error>
	where
		K: CacheKey + Send + Sync,
		V: redis::ToRedisArgs + redis::FromRedisValue + Clone + Debug + Send + Sync,
		Getter: Fn(GetterCtx<K, V>, K) -> Fut + Clone,
		Fut: Future<Output = GetterResult<GetterCtx<K, V>>>,
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
	) -> Result<Vec<(Key, Value)>, Error>
	where
		Key: CacheKey + Send + Sync,
		Value: redis::ToRedisArgs + redis::FromRedisValue + Clone + Debug + Send + Sync,
		Getter: Fn(GetterCtx<Key, Value>, Vec<Key>) -> Fut + Clone,
		Fut: Future<Output = GetterResult<GetterCtx<Key, Value>>>,
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
	async fn fetch_all_convert<Key, Value, ValueRedis, Getter, Fut, Encoder, Decoder>(
		self,
		base_key: impl ToString + Debug,
		keys: impl IntoIterator<Item = Key>,
		getter: Getter,
		encoder: Encoder,
		decoder: Decoder,
	) -> Result<Vec<(Key, Value)>, Error>
	where
		Key: CacheKey + Send + Sync,
		Value: Debug + Send + Sync,
		ValueRedis: redis::ToRedisArgs + redis::FromRedisValue + Debug + Send + Sync,
		Getter: Fn(GetterCtx<Key, Value>, Vec<Key>) -> Fut + Clone,
		Fut: Future<Output = GetterResult<GetterCtx<Key, Value>>>,
		Encoder: Fn(&Value) -> Result<ValueRedis, Error> + Clone,
		Decoder: Fn(&ValueRedis) -> Result<Value, Error> + Clone,
	{
		let base_key = base_key.to_string();
		let keys = keys.into_iter().collect::<Vec<Key>>();

		// Ignore empty keys
		if keys.is_empty() {
			return Ok(Vec::new());
		}

		let mut conn = self.cache.redis_conn.clone();

		let redis_keys = keys
			.iter()
			.map(|key| self.cache.build_redis_cache_key(&base_key, key))
			.collect::<Vec<_>>();

		// Build Redis command explicitly, since `conn.get` with one value will
		// not return a vector
		let mut mget_cmd = redis::cmd("MGET");
		for key in &redis_keys {
			mget_cmd.arg(key);
		}

		// Attempt to fetch value from cache, fall back to getter
		match mget_cmd
			.query_async::<_, Vec<Option<ValueRedis>>>(&mut conn)
			.await
		{
			Ok(cached_values) => {
				debug_assert_eq!(
					redis_keys.len(),
					cached_values.len(),
					"cache returned wrong number of values"
				);

				tracing::info!(
					cached_len = cached_values.iter().filter(|x| x.is_some()).count(),
					total_len = cached_values.len(),
					"read from cache"
				);

				// Create the getter ctx and resolve the cached values
				let mut ctx = GetterCtx::new(base_key.clone().into(), keys.to_vec());
				for (i, value) in cached_values.into_iter().enumerate() {
					if let Some(value) = value {
						let value = decoder(&value)?;
						ctx.resolve_from_cache(i, value);
					}
				}

				// Fetch remaining values and add to the cached list
				if !ctx.all_keys_have_value() {
					// Call the getter
					let remaining_keys = ctx.unresolved_keys();
					let unresolved_len = remaining_keys.len();
					ctx = getter(ctx, remaining_keys).await.map_err(Error::Getter)?;

					// Write the values to the pipe
					let mut pipe = redis::pipe();
					let expire_at = (SystemTime::now() + Duration::from_millis(self.ttl as u64))
						.duration_since(std::time::UNIX_EPOCH)
						.unwrap_or_default()
						.as_millis() as i64;
					let values_needing_cache_write = ctx.values_needing_cache_write();
					tracing::trace!(
						unresolved_len,
						fetched_len = values_needing_cache_write.len(),
						"writing new values to cache"
					);
					for (key, value) in values_needing_cache_write {
						let redis_svc_key = self.cache.build_redis_cache_key(&base_key, &key.key);
						let value = encoder(value)?;

						// Write the value with the expiration
						pipe.cmd("SET")
							.arg(&redis_svc_key)
							.arg(value)
							.arg("PXAT")
							.arg(expire_at)
							.ignore();
					}
					let spawn_res = tokio::task::Builder::new()
						.name("redis_cache::write")
						.spawn(
							async move {
								match pipe.query_async(&mut conn).await {
									Ok(()) => {
										tracing::trace!("successfully wrote to cache");
									}
									Err(err) => {
										tracing::error!(?err, "failed to write to cache");
									}
								}
							}
							.in_current_span(),
						);
					if let Err(err) = spawn_res {
						tracing::error!(?err, "failed to spawn user_presence_touch task");
					}
				}

				Ok(ctx.into_values())
			}
			Err(err) => {
				tracing::error!(
					?err,
					"failed to read batch keys from cache, falling back to getter"
				);

				// Fall back to the getter since we can't fetch the value from
				// the cache
				let ctx = getter(
					GetterCtx::new(base_key.into(), keys.to_vec()),
					keys.to_vec(),
				)
				.await
				.map_err(Error::Getter)?;
				return Ok(ctx.into_values());
			}
		}
	}

	// /// Attempts to fetch all given keys from the cache. Keys not found in the
	// /// cache will be passed to the getter in order to return the rest of the
	// /// values from the database.
	// ///
	// /// If a value is not found in the cache and database, the value is excluded
	// /// from the return vector.
	// ///
	// /// We use a Redis pipeline with optimistic locking in order to validate
	// /// that the value is not purged while we're writing to it. This will retry
	// /// the optimistic lock 16 times.
	// ///
	// /// `encoder` and `decoder` convert from `Value` to `ValueRedis`.
	// #[tracing::instrument(err, skip(keys, getter, encoder, decoder))]
	// async fn fetch_all_convert<Key, Value, ValueRedis, Getter, Fut, Encoder, Decoder>(
	// 	self,
	// 	base_key: impl ToString + Debug,
	// 	keys: impl IntoIterator<Item = Key>,
	// 	getter: Getter,
	// 	encoder: Encoder,
	// 	decoder: Decoder,
	// ) -> Result<Vec<(Key, Value)>, Error>
	// where
	// 	Key: CacheKey + Send + Sync,
	// 	Value: Debug + Send + Sync,
	// 	ValueRedis: redis::ToRedisArgs + redis::FromRedisValue + Debug + Send + Sync,
	// 	Getter: Fn(GetterCtx<Key, Value>, Vec<Key>) -> Fut + Clone,
	// 	Fut: Future<Output = GetterResult<GetterCtx<Key, Value>>>,
	// 	Encoder: Fn(&Value) -> Result<ValueRedis, Error> + Clone,
	// 	Decoder: Fn(&ValueRedis) -> Result<Value, Error> + Clone,
	// {
	// 	let base_key = base_key.to_string();
	// 	let keys = keys.into_iter().collect::<Vec<Key>>();

	// 	if keys.is_empty() {
	// 		return Ok(Vec::new());
	// 	}

	// 	let mut conn = self.cache.redis_conn.clone();

	// 	// Build the Redis keys
	// 	let redis_svc_keys = keys
	// 		.iter()
	// 		.map(|k| self.cache.build_redis_cache_key(&base_key, k))
	// 		.collect::<Vec<_>>();

	// 	'optimistic_lock: for attempt_idx in 0..16 {
	// 		tracing::debug!(?attempt_idx, "optimistic lock attempt");

	// 		// Watch the keys if the cache value can change
	// 		if !self.immutable {
	// 			let watch_res = redis::cmd("WATCH")
	// 				.arg(&redis_svc_keys)
	// 				.query_async::<_, ()>(&mut conn)
	// 				.await;
	// 			match watch_res {
	// 				Ok(_) => {
	// 					tracing::debug!("successfully watched keys");
	// 				}
	// 				Err(err) => {
	// 					tracing::error!(?err, "failed watch values, falling back to getter");

	// 					// Fall back to the getter since we can't fetch the value from
	// 					// the cache
	// 					let ctx = getter(
	// 						GetterCtx::new(self.clone(), base_key.into(), keys.clone()),
	// 						keys.clone(),
	// 					)
	// 					.await
	// 					.map_err(Error::Getter)?;
	// 					return Ok(ctx.into_values());
	// 				}
	// 			}
	// 		}

	// 		// Create pipe for the optimistic lock
	// 		let mut pipe = redis::pipe();
	// 		pipe.atomic();

	// 		// Attempt to fetch the values
	// 		let fetch_res = self
	// 			.fetch_all_trans::<Key, Value, ValueRedis, Getter, Fut, Encoder, Decoder>(
	// 				&getter,
	// 				&encoder,
	// 				&decoder,
	// 				&base_key,
	// 				&keys,
	// 				redis_svc_keys.as_slice(),
	// 				&mut conn,
	// 				&mut pipe,
	// 			)
	// 			.await;
	// 		match fetch_res {
	// 			Ok(values) => {
	// 				tracing::debug!("successfully fetched cached values");

	// 				// Commit the transaction
	// 				let trans_res = pipe.query_async::<_, Option<()>>(&mut conn).await;
	// 				match trans_res {
	// 					Ok(Some(())) => {
	// 						tracing::debug!("successfully wrote cached values to cache");
	// 					}
	// 					Ok(None) => {
	// 						tracing::info!(?attempt_idx, "optimistic lock failed");
	// 						continue 'optimistic_lock;
	// 					}
	// 					Err(err) => {
	// 						tracing::error!(?err, "failed to execute cache set pipeline");

	// 						// Assume the pipe didn't commit
	// 						if !self.immutable {
	// 							unwatch_gracefully(&mut conn).await;
	// 						}

	// 						// Exit gracefully, since the cache isn't vital to
	// 						// the application's functionality.
	// 					}
	// 				}

	// 				return Ok(values);
	// 			}
	// 			Err(err) => {
	// 				tracing::error!(?err, "failed to execute getter");
	// 				if !self.immutable {
	// 					unwatch_gracefully(&mut conn).await;
	// 				}
	// 				return Err(err);
	// 			}
	// 		}
	// 	}

	// 	Err(Error::OptimisticLockFailedTooManyTimes)
	// }

	// /// Executes the logic inside of the Redis optimistic lock loop.
	// #[tracing::instrument(err, skip(getter, encoder, decoder, keys, redis_svc_keys, conn, pipe))]
	// async fn fetch_all_trans<Key, Value, ValueRedis, Getter, Fut, Encoder, Decoder>(
	// 	&self,
	// 	getter: &Getter,
	// 	encoder: &Encoder,
	// 	decoder: &Decoder,
	// 	base_key: &str,
	// 	keys: &[Key],
	// 	redis_svc_keys: &[String],
	// 	conn: &mut RedisPool,
	// 	pipe: &mut redis::Pipeline,
	// ) -> Result<Vec<(Key, Value)>, Error>
	// where
	// 	Key: CacheKey + Send + Sync,
	// 	Value: Debug + Send + Sync,
	// 	ValueRedis: redis::ToRedisArgs + redis::FromRedisValue + Debug + Send + Sync,
	// 	Getter: Fn(GetterCtx<Key, Value>, Vec<Key>) -> Fut + Clone,
	// 	Fut: Future<Output = GetterResult<GetterCtx<Key, Value>>>,
	// 	Encoder: Fn(&Value) -> Result<ValueRedis, Error> + Clone,
	// 	Decoder: Fn(&ValueRedis) -> Result<Value, Error> + Clone,
	// {
	// 	// Build Redis command explicitly, since `conn.get` with one value will
	// 	// not return a vector
	// 	let mut mget_cmd = redis::cmd("MGET");
	// 	for key in redis_svc_keys {
	// 		mget_cmd.arg(key);
	// 	}

	// 	// Attempt to fetch value from cache, fall back to getter
	// 	// TODO: Make this value time out
	// 	match mget_cmd
	// 		.query_async::<_, Vec<Option<ValueRedis>>>(conn)
	// 		.await
	// 	{
	// 		// Found value in cache
	// 		Ok(cached_values) => {
	// 			debug_assert_eq!(
	// 				redis_svc_keys.len(),
	// 				cached_values.len(),
	// 				"cache returned wrong number of values"
	// 			);

	// 			tracing::info!(
	// 				cached_len = cached_values.iter().filter(|x| x.is_some()).count(),
	// 				total_len = cached_values.len(),
	// 				"read from cache"
	// 			);

	// 			// Create the getter ctx and resolve the cached values
	// 			let mut ctx = GetterCtx::new(self.clone(), base_key.into(), keys.to_vec());
	// 			for (i, value) in cached_values.into_iter().enumerate() {
	// 				if let Some(value) = value {
	// 					let value = decoder(&value)?;
	// 					ctx.resolve_from_cache(i, value);
	// 				}
	// 			}

	// 			// Fetch remaining values and add to the cached list
	// 			if !ctx.all_keys_have_value() {
	// 				// Call the getter
	// 				let remaining_keys = ctx.unresolved_keys();
	// 				let unresolved_len = remaining_keys.len();
	// 				ctx = getter(ctx, remaining_keys).await.map_err(Error::Getter)?;

	// 				// Write the values to the pipe
	// 				let expire_at = (SystemTime::now() + Duration::from_millis(self.ttl as u64))
	// 					.duration_since(std::time::UNIX_EPOCH)
	// 					.unwrap_or_default()
	// 					.as_millis() as i64;
	// 				let values_needing_cache_write = ctx.values_needing_cache_write();
	// 				tracing::info!(
	// 					unresolved_len,
	// 					fetched_len = values_needing_cache_write.len(),
	// 					"writing new values to cache"
	// 				);
	// 				for (key, value) in values_needing_cache_write {
	// 					let redis_svc_key = self.cache.build_redis_svc_key(base_key, &key.key);
	// 					let value = encoder(value)?;

	// 					// Write the value with the expiration
	// 					pipe.cmd("SET")
	// 						.arg(&redis_svc_key)
	// 						.arg(value)
	// 						.arg("PXAT")
	// 						.arg(expire_at)
	// 						.ignore();

	// 					// // Save the topic key with the expiration ts
	// 					// // TODO: These values may be purged occasionally with LRU which will leak memory
	// 					// if let Some(redis_topic_keys) = &key.redis_topic_keys {
	// 					// 	for topic_key in redis_topic_keys {
	// 					// 		pipe.zadd(topic_key, &redis_svc_key, expire_at).ignore();
	// 					// 		pipe.cmd("ZREMRANGEBYSCORE")
	// 					// 			.arg(topic_key)
	// 					// 			.arg("-inf")
	// 					// 			.arg(rivet_util::timestamp::now())
	// 					// 			.ignore();
	// 					// 	}
	// 					// }
	// 				}
	// 			}

	// 			Ok(ctx.into_values())
	// 		}

	// 		// Error fetching from cache
	// 		Err(err) => {
	// 			tracing::error!(
	// 				?err,
	// 				"failed to read batch keys from cache, falling back to getter"
	// 			);

	// 			// Fall back to the getter since we can't fetch the value from
	// 			// the cache
	// 			let ctx = getter(
	// 				GetterCtx::new(self.clone(), base_key.into(), keys.to_vec()),
	// 				keys.to_vec(),
	// 			)
	// 			.await
	// 			.map_err(Error::Getter)?;
	// 			Ok(ctx.into_values())
	// 		}
	// 	}
	// }

	#[tracing::instrument(err, skip(keys))]
	pub async fn purge<Key>(
		self,
		base_key: impl AsRef<str> + Debug,
		keys: impl IntoIterator<Item = Key>,
	) -> Result<(), Error>
	where
		Key: CacheKey + Send + Sync,
	{
		// Build keys
		let base_key = base_key.as_ref();
		let redis_keys = keys
			.into_iter()
			.map(|key| self.cache.build_redis_cache_key(base_key, &key))
			.collect::<Vec<_>>();

		// Delete keys
		let mut conn = self.cache.redis_conn.clone();
		match conn.del::<_, ()>(redis_keys).await {
			Ok(_) => {
				tracing::trace!("successfully wrote");
			}
			Err(err) => {
				tracing::error!(?err, "failed to delete from cache, proceeding regardless")
			}
		}

		Ok(())
	}
}

// MARK: Proto fetch
impl RequestConfig {
	#[tracing::instrument(err, skip(key, getter))]
	pub async fn fetch_one_proto<Key, Value, Getter, Fut>(
		self,
		base_key: impl ToString + Debug,
		key: Key,
		getter: Getter,
	) -> Result<Option<Value>, Error>
	where
		Key: CacheKey + Send + Sync,
		Value: prost::Message + Default + Send + Sync,
		Getter: Fn(GetterCtx<Key, Value>, Key) -> Fut + Clone,
		Fut: Future<Output = GetterResult<GetterCtx<Key, Value>>>,
	{
		let values = self
			.fetch_all_proto_with_keys(base_key, [key], move |cache, keys| {
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
	pub async fn fetch_all_proto<Key, Value, Getter, Fut>(
		self,
		base_key: impl ToString + Debug,
		keys: impl IntoIterator<Item = Key>,
		getter: Getter,
	) -> Result<Vec<Value>, Error>
	where
		Key: CacheKey + Send + Sync,
		Value: prost::Message + Default + Send + Sync,
		Getter: Fn(GetterCtx<Key, Value>, Vec<Key>) -> Fut + Clone,
		Fut: Future<Output = GetterResult<GetterCtx<Key, Value>>>,
	{
		self.fetch_all_proto_with_keys::<Key, Value, Getter, Fut>(base_key, keys, getter)
			.await
			// TODO: Find a way to not allowcate another vec here
			.map(|x| x.into_iter().map(|(_, v)| v).collect::<Vec<_>>())
	}

	#[tracing::instrument(err, skip(keys, getter))]
	pub async fn fetch_all_proto_with_keys<Key, Value, Getter, Fut>(
		self,
		base_key: impl ToString + Debug,
		keys: impl IntoIterator<Item = Key>,
		getter: Getter,
	) -> Result<Vec<(Key, Value)>, Error>
	where
		Key: CacheKey + Send + Sync,
		Value: prost::Message + Default + Send + Sync,
		Getter: Fn(GetterCtx<Key, Value>, Vec<Key>) -> Fut + Clone,
		Fut: Future<Output = GetterResult<GetterCtx<Key, Value>>>,
	{
		self.fetch_all_convert(
			base_key,
			keys,
			getter,
			|value: &Value| -> Result<Vec<u8>, Error> {
				let mut buf = Vec::with_capacity(value.encoded_len());
				value.encode(&mut buf).map_err(Error::ProtoEncode)?;
				Ok(buf)
			},
			|value: &Vec<u8>| -> Result<Value, Error> {
				Value::decode(value.as_slice()).map_err(Error::ProtoDecode)
			},
		)
		.await
	}
}

#[tracing::instrument(skip(conn))]
async fn unwatch_gracefully(conn: &mut RedisPool) {
	tracing::debug!("unwatching");
	match redis::cmd("UNWATCH").query_async::<_, ()>(conn).await {
		Ok(_) => tracing::debug!("unwatched successfully"),
		Err(err) => tracing::error!(?err, "failed to unwatch from cache"),
	}
}
