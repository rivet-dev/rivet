use std::fmt::Debug;

use super::*;

/// Entry for a single value that is going to be read/written to the cache.
#[derive(Debug)]
pub(super) struct GetterCtxKey<K, V> {
	/// `CacheKey` that will be used to build Redis keys.
	pub(super) key: K,

	/// The value that was read from the cache or getter.
	value: Option<V>,

	/// If this value was read from the cacahe. If false and a value is present,
	/// then this value was read from the getter and will be written to the
	/// cache.
	from_cache: bool,
	// /// List of Redis keys for topics associated with this key.
	// ///
	// /// Topics are handles used to purge multiple other cached values at once.
	// ///
	// /// The topics almost always match the primary keys of the given table.
	// ///
	// /// For example: if you cache both the display name of a game and the
	// /// description in two separate keys, you can purge the game's topic to
	// /// remove both of those cached values.
	// pub(super) redis_topic_keys: Option<Vec<String>>,
}

/// Context passed to the getter function. This is used to resolve and configure
/// values inside the getter.
pub struct GetterCtx<K, V>
where
	K: CacheKey,
{
	/// The name of the service-specific key to write this cached value to. For
	/// example, a team get service would use the "team_profile" key to store
	/// the profile a "team_members" to store a cache of members.
	///
	/// This is local to the service & source hash that caches this value.
	#[allow(unused)]
	base_key: String,

	/// The keys to get/populate from the cache.
	keys: Vec<GetterCtxKey<K, V>>,
}

impl<K, V> GetterCtx<K, V>
where
	K: CacheKey,
{
	pub(super) fn new(base_key: String, keys: Vec<K>) -> Self {
		GetterCtx {
			base_key,
			keys: {
				// Create deduplicated ctx keys
				let mut ctx_keys = Vec::<GetterCtxKey<K, V>>::new();
				for key in keys {
					if !ctx_keys.iter().any(|x| x.key == key) {
						ctx_keys.push(GetterCtxKey {
							key,
							value: None,
							from_cache: false,
							// redis_topic_keys: None,
						});
					}
				}
				ctx_keys
			},
		}
	}

	pub(super) fn into_values(self) -> Vec<(K, V)> {
		self.keys
			.into_iter()
			.filter_map(|k| {
				if let Some(v) = k.value {
					Some((k.key, v))
				} else {
					None
				}
			})
			.collect()
	}

	/// If all keys have an associated value.
	pub(super) fn all_keys_have_value(&self) -> bool {
		self.keys.iter().all(|x| x.value.is_some())
	}

	/// Keys that do not have a value yet.
	pub(super) fn unresolved_keys(&self) -> Vec<K> {
		self.keys
			.iter()
			.filter(|x| x.value.is_none())
			.map(|x| x.key.clone())
			.collect()
	}

	/// Keys that have been resolved in a getter and need to be written to the
	/// cache.
	pub(super) fn values_needing_cache_write(&self) -> Vec<(&GetterCtxKey<K, V>, &V)> {
		self.keys
			.iter()
			.filter(|x| !x.from_cache)
			.filter_map(|k| k.value.as_ref().map(|v| (k, v)))
			.collect()
	}
}

impl<K, V> GetterCtx<K, V>
where
	K: CacheKey,
	V: Debug,
{
	/// Sets a value with the value provided from the cache.
	pub(super) fn resolve_from_cache(&mut self, idx: usize, value: V) {
		if let Some(key) = self.keys.iter_mut().nth(idx) {
			key.value = Some(value);
			key.from_cache = true;
		} else {
			tracing::warn!(?idx, ?value, "resolving cache key index out of range");
		}
	}

	/// Calls the callback with a mutable reference to a given key. Validates
	/// that the key does not already have a value.
	fn get_key_for_resolve(&mut self, key: &K, cb: impl FnOnce(&mut GetterCtxKey<K, V>)) {
		if let Some(key) = self.keys.iter_mut().find(|x| x.key == *key) {
			if key.value.is_some() {
				tracing::warn!(?key, "cache key already has value");
			} else {
				cb(key);
			}
		} else {
			tracing::warn!(?key, "resolved value for nonexistent cache key");
		}
	}

	/// Sets a value with the value provided from the getter function.
	pub fn resolve(&mut self, key: &K, value: V) {
		self.get_key_for_resolve(&key, |key| key.value = Some(value));
	}

	// pub fn resolve_with_topic<T>(&mut self, key: &K, value: V, (topic_base_key, topic): (&str, &T))
	// where
	// 	T: CacheKey,
	// {
	// 	let redis_key = self
	// 		.config
	// 		.cache
	// 		.build_redis_topic_key(topic_base_key, topic);
	// 	self.get_key_for_resolve(&key, |key| {
	// 		key.value = Some(value);
	// 		key.redis_topic_keys = Some(vec![redis_key]);
	// 	});
	// }

	// TODO: Add multiple topics with multiple T types using dyn
}
