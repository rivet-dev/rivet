#[cfg(test)]
use std::collections::BTreeMap;
#[cfg(test)]
use std::sync::Arc;

use anyhow::Result;
#[cfg(test)]
use parking_lot::{Mutex, RwLock};
use rivet_envoy_client::handle::EnvoyHandle;

use crate::types::ListOpts;

/// Narrow access to the actor's pre-SQLite KV namespace.
///
/// Production code may use this only for the one-time SQLite importer and the
/// temporary inspector-token mirror. It intentionally has no default or
/// unconfigured state. The in-memory backend exists only in unit tests.
#[derive(Clone)]
pub(crate) struct LegacyActorKv {
	backend: LegacyActorKvBackend,
}

#[derive(Clone)]
enum LegacyActorKvBackend {
	Envoy {
		handle: EnvoyHandle,
		actor_id: String,
	},
	#[cfg(test)]
	InMemory(Arc<TestLegacyActorKv>),
}

#[cfg(test)]
struct TestLegacyActorKv {
	store: RwLock<BTreeMap<Vec<u8>, Vec<u8>>>,
	delete_range_after_write_lock: Mutex<Option<Arc<dyn Fn() + Send + Sync + 'static>>>,
	list_limit_cap: Mutex<Option<u32>>,
	range_start_inclusive: Mutex<bool>,
}

impl LegacyActorKv {
	pub(crate) fn new(handle: EnvoyHandle, actor_id: impl Into<String>) -> Self {
		Self {
			backend: LegacyActorKvBackend::Envoy {
				handle,
				actor_id: actor_id.into(),
			},
		}
	}

	#[cfg(test)]
	pub(crate) fn new_in_memory() -> Self {
		Self {
			backend: LegacyActorKvBackend::InMemory(Arc::new(TestLegacyActorKv {
				store: RwLock::new(BTreeMap::new()),
				delete_range_after_write_lock: Mutex::new(None),
				list_limit_cap: Mutex::new(None),
				range_start_inclusive: Mutex::new(true),
			})),
		}
	}

	#[cfg(test)]
	pub(crate) async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
		Ok(self.batch_get(&[key]).await?.pop().flatten())
	}

	pub(crate) async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
		self.batch_put(&[(key, value)]).await
	}

	pub(crate) async fn batch_get(&self, keys: &[&[u8]]) -> Result<Vec<Option<Vec<u8>>>> {
		match &self.backend {
			LegacyActorKvBackend::Envoy { handle, actor_id } => {
				handle
					.kv_get(
						actor_id.clone(),
						keys.iter().map(|key| key.to_vec()).collect(),
					)
					.await
			}
			#[cfg(test)]
			LegacyActorKvBackend::InMemory(store) => {
				let store = store.store.read();
				Ok(keys.iter().map(|key| store.get(*key).cloned()).collect())
			}
		}
	}

	pub(crate) async fn batch_put(&self, entries: &[(&[u8], &[u8])]) -> Result<()> {
		match &self.backend {
			LegacyActorKvBackend::Envoy { handle, actor_id } => {
				handle
					.kv_put(
						actor_id.clone(),
						entries
							.iter()
							.map(|(key, value)| (key.to_vec(), value.to_vec()))
							.collect(),
					)
					.await
			}
			#[cfg(test)]
			LegacyActorKvBackend::InMemory(store) => {
				let mut store = store.store.write();
				for (key, value) in entries {
					store.insert(key.to_vec(), value.to_vec());
				}
				Ok(())
			}
		}
	}

	pub(crate) async fn list_prefix(
		&self,
		prefix: &[u8],
		opts: ListOpts,
	) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
		match &self.backend {
			LegacyActorKvBackend::Envoy { handle, actor_id } => {
				handle
					.kv_list_prefix(
						actor_id.clone(),
						prefix.to_vec(),
						Some(opts.reverse),
						opts.limit.map(u64::from),
					)
					.await
			}
			#[cfg(test)]
			LegacyActorKvBackend::InMemory(store) => {
				let mut entries: Vec<_> = store
					.store
					.read()
					.iter()
					.filter(|(key, _)| key.starts_with(prefix))
					.map(|(key, value)| (key.clone(), value.clone()))
					.collect();
				apply_list_opts(&mut entries, store.capped_opts(opts));
				Ok(entries)
			}
		}
	}

	pub(crate) async fn list_range(
		&self,
		start: &[u8],
		end: &[u8],
		opts: ListOpts,
	) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
		match &self.backend {
			LegacyActorKvBackend::Envoy { handle, actor_id } => {
				handle
					.kv_list_range(
						actor_id.clone(),
						start.to_vec(),
						end.to_vec(),
						true,
						Some(opts.reverse),
						opts.limit.map(u64::from),
					)
					.await
			}
			#[cfg(test)]
			LegacyActorKvBackend::InMemory(store) => {
				let entries_guard = store.store.read();
				let mut entries: Vec<_> = entries_guard
					.range(start.to_vec()..end.to_vec())
					.filter(|(key, _)| {
						*store.range_start_inclusive.lock() || key.as_slice() > start
					})
					.map(|(key, value)| (key.clone(), value.clone()))
					.collect();
				apply_list_opts(&mut entries, store.capped_opts(opts));
				Ok(entries)
			}
		}
	}

	#[cfg(test)]
	pub(crate) async fn batch_delete(&self, keys: &[&[u8]]) -> Result<()> {
		match &self.backend {
			LegacyActorKvBackend::Envoy { handle, actor_id } => {
				handle
					.kv_delete(
						actor_id.clone(),
						keys.iter().map(|key| key.to_vec()).collect(),
					)
					.await
			}
			LegacyActorKvBackend::InMemory(store) => {
				let mut store = store.store.write();
				for key in keys {
					store.remove(*key);
				}
				Ok(())
			}
		}
	}

	#[cfg(test)]
	pub(crate) async fn delete_range(&self, start: &[u8], end: &[u8]) -> Result<()> {
		match &self.backend {
			LegacyActorKvBackend::Envoy { handle, actor_id } => {
				handle
					.kv_delete_range(actor_id.clone(), start.to_vec(), end.to_vec())
					.await
			}
			LegacyActorKvBackend::InMemory(store) => {
				let mut entries = store.store.write();
				if let Some(hook) = store.delete_range_after_write_lock.lock().clone() {
					hook();
				}
				entries.retain(|key, _| key.as_slice() < start || key.as_slice() >= end);
				Ok(())
			}
		}
	}

	#[cfg(test)]
	pub(crate) fn test_identity(&self) -> usize {
		match &self.backend {
			LegacyActorKvBackend::Envoy { handle, .. } => handle.get_envoy_key().as_ptr() as usize,
			LegacyActorKvBackend::InMemory(store) => Arc::as_ptr(store) as usize,
		}
	}

	#[cfg(test)]
	pub(crate) fn test_set_delete_range_after_write_lock_hook(
		&self,
		hook: impl Fn() + Send + Sync + 'static,
	) {
		if let LegacyActorKvBackend::InMemory(store) = &self.backend {
			*store.delete_range_after_write_lock.lock() = Some(Arc::new(hook));
		}
	}

	#[cfg(test)]
	pub(crate) fn test_set_list_limit_cap(&self, cap: u32) {
		if let LegacyActorKvBackend::InMemory(store) = &self.backend {
			*store.list_limit_cap.lock() = Some(cap);
		}
	}

	#[cfg(test)]
	pub(crate) fn test_set_range_start_inclusive(&self, inclusive: bool) {
		if let LegacyActorKvBackend::InMemory(store) = &self.backend {
			*store.range_start_inclusive.lock() = inclusive;
		}
	}
}

#[cfg(test)]
impl TestLegacyActorKv {
	fn capped_opts(&self, opts: ListOpts) -> ListOpts {
		let Some(cap) = *self.list_limit_cap.lock() else {
			return opts;
		};
		ListOpts {
			reverse: opts.reverse,
			limit: Some(opts.limit.map_or(cap, |limit| limit.min(cap))),
		}
	}
}

#[cfg(test)]
fn apply_list_opts(entries: &mut Vec<(Vec<u8>, Vec<u8>)>, opts: ListOpts) {
	if opts.reverse {
		entries.reverse();
	}
	if let Some(limit) = opts.limit {
		entries.truncate(limit as usize);
	}
}

#[cfg(test)]
pub(crate) type Kv = LegacyActorKv;

#[cfg(test)]
#[path = "../../tests/kv.rs"]
pub(crate) mod tests;
