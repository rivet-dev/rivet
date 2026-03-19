use std::{collections::HashMap, fmt::Debug};

use super::*;

/// Entry for a single value that is going to be read/written to the cache.
#[derive(Debug)]
pub(super) struct GetterCtxEntry<V> {
	/// The value that was read from the cache or getter.
	value: Option<V>,

	/// If this value was read from the cache. If false and a value is present,
	/// then this value was read from the getter and will be written to the
	/// cache.
	from_cache: bool,
}

/// Context passed to the getter function. This is used to resolve and configure
/// values inside the getter.
pub struct GetterCtx<K, V>
where
	K: CacheKey,
{
	/// The entries to get/populate from the cache.
	entries: HashMap<K, GetterCtxEntry<V>>,
}

impl<K, V> GetterCtx<K, V>
where
	K: CacheKey,
{
	pub(super) fn new(keys: Vec<K>) -> Self {
		GetterCtx {
			entries: keys
				.into_iter()
				.map(|k| {
					(
						k,
						GetterCtxEntry {
							value: None,
							from_cache: false,
						},
					)
				})
				.collect(),
		}
	}

	pub(super) fn merge(&mut self, other: GetterCtx<K, V>) {
		self.entries.extend(other.entries);
	}

	pub(super) fn into_values(self) -> Vec<(K, V)> {
		self.entries
			.into_iter()
			.filter_map(|(k, x)| x.value.map(|v| (k, v)))
			.collect()
	}

	/// All entries.
	pub(super) fn entries(&self) -> impl Iterator<Item = (&K, &GetterCtxEntry<V>)> {
		self.entries.iter()
	}

	/// If all entries have an associated value.
	pub(super) fn all_entries_have_value(&self) -> bool {
		self.entries.iter().all(|(_, x)| x.value.is_some())
	}

	/// Keys that do not have a value yet.
	pub(super) fn unresolved_keys(&self) -> Vec<K> {
		self.entries
			.iter()
			.filter(|(_, x)| x.value.is_none())
			.map(|(k, _)| k.clone())
			.collect()
	}

	/// Entries that have been resolved in a getter and need to be written to the
	/// cache.
	pub(super) fn entries_needing_cache_write(&self) -> Vec<(&K, &V)> {
		self.entries
			.iter()
			.filter(|(_, x)| !x.from_cache)
			.filter_map(|(k, x)| x.value.as_ref().map(|v| (k, v)))
			.collect()
	}
}

impl<K, V> GetterCtx<K, V>
where
	K: CacheKey,
	V: Debug,
{
	/// Sets an entry with the value provided from the cache.
	pub(super) fn resolve_from_cache(&mut self, key: &K, value: V) {
		if let Some(entry) = self.entries.get_mut(key) {
			entry.value = Some(value);
			entry.from_cache = true;
		} else {
			tracing::warn!(?key, ?value, "resolving nonexistent cache entry");
		}
	}

	/// Sets a value with the value provided from the getter function.
	pub fn resolve(&mut self, key: &K, value: V) {
		if let Some(entry) = self.entries.get_mut(key) {
			if entry.value.is_some() {
				tracing::warn!(?entry, "cache entry already has value");
			} else {
				entry.value = Some(value);
			}
		} else {
			tracing::warn!(?key, "resolved value for nonexistent cache entry");
		}
	}
}
