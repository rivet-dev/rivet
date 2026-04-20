use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

#[cfg(test)]
use std::sync::Mutex;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::{Result, anyhow};
use rivet_envoy_client::handle::EnvoyHandle;

use crate::types::ListOpts;

#[derive(Clone)]
pub struct Kv {
	backend: KvBackend,
	actor_id: String,
}

#[derive(Clone)]
enum KvBackend {
	Unconfigured,
	Envoy(EnvoyHandle),
	#[cfg_attr(not(test), allow(dead_code))]
	InMemory(Arc<InMemoryKv>),
}

struct InMemoryKv {
	store: RwLock<BTreeMap<Vec<u8>, Vec<u8>>>,
	#[cfg(test)]
	stats: InMemoryKvStats,
}

#[cfg(test)]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct KvApplyBatchSnapshot {
	pub puts: Vec<(Vec<u8>, Vec<u8>)>,
	pub deletes: Vec<Vec<u8>>,
}

#[cfg(test)]
#[derive(Default)]
struct InMemoryKvStats {
	apply_batch_calls: AtomicUsize,
	batch_delete_calls: AtomicUsize,
	last_apply_batch: Mutex<Option<KvApplyBatchSnapshot>>,
}

impl Kv {
	/// `actor_id` stays on `Kv` because envoy-client KV calls require it on every request.
	pub fn new(handle: EnvoyHandle, actor_id: impl Into<String>) -> Self {
		Self {
			backend: KvBackend::Envoy(handle),
			actor_id: actor_id.into(),
		}
	}

	pub fn new_in_memory() -> Self {
		Self {
			backend: KvBackend::InMemory(Arc::new(InMemoryKv {
				store: RwLock::new(BTreeMap::new()),
				#[cfg(test)]
				stats: InMemoryKvStats::default(),
			})),
			actor_id: String::new(),
		}
	}

	pub async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
		let mut values = self.batch_get(&[key]).await?;
		Ok(values.pop().flatten())
	}

	pub async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
		self.batch_put(&[(key, value)]).await
	}

	pub async fn delete(&self, key: &[u8]) -> Result<()> {
		self.batch_delete(&[key]).await
	}

	pub async fn delete_range(&self, start: &[u8], end: &[u8]) -> Result<()> {
		match &self.backend {
			KvBackend::Envoy(handle) => {
				handle
					.kv_delete_range(
						self.actor_id.clone(),
						start.to_vec(),
						end.to_vec(),
					)
					.await
			}
			KvBackend::InMemory(entries) => {
				let keys: Vec<Vec<u8>> = entries
					.store
					.read()
					.expect("in-memory kv lock poisoned")
					.range(start.to_vec()..end.to_vec())
					.map(|(key, _)| key.clone())
					.collect();

				let mut entries = entries.store.write().expect("in-memory kv lock poisoned");
				for key in keys {
					entries.remove(&key);
				}

				Ok(())
			}
			KvBackend::Unconfigured => Err(anyhow!("kv handle is not configured")),
		}
	}

	pub async fn list_prefix(&self, prefix: &[u8], opts: ListOpts) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
		match &self.backend {
			KvBackend::Envoy(handle) => {
				handle
					.kv_list_prefix(
						self.actor_id.clone(),
						prefix.to_vec(),
						Some(opts.reverse),
						opts.limit.map(u64::from),
					)
					.await
			}
			KvBackend::InMemory(entries) => {
				let mut listed: Vec<_> = entries
					.store
					.read()
					.expect("in-memory kv lock poisoned")
					.iter()
					.filter(|(key, _)| key.starts_with(prefix))
					.map(|(key, value)| (key.clone(), value.clone()))
					.collect();
				apply_list_opts(&mut listed, opts);
				Ok(listed)
			}
			KvBackend::Unconfigured => Err(anyhow!("kv handle is not configured")),
		}
	}

	pub async fn list_range(
		&self,
		start: &[u8],
		end: &[u8],
		opts: ListOpts,
	) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
		match &self.backend {
			KvBackend::Envoy(handle) => {
				handle
					.kv_list_range(
						self.actor_id.clone(),
						start.to_vec(),
						end.to_vec(),
						true,
						Some(opts.reverse),
						opts.limit.map(u64::from),
					)
					.await
			}
			KvBackend::InMemory(entries) => {
				let mut listed: Vec<_> = entries
					.store
					.read()
					.expect("in-memory kv lock poisoned")
					.range(start.to_vec()..end.to_vec())
					.map(|(key, value)| (key.clone(), value.clone()))
					.collect();
				apply_list_opts(&mut listed, opts);
				Ok(listed)
			}
			KvBackend::Unconfigured => Err(anyhow!("kv handle is not configured")),
		}
	}

	pub async fn batch_get(&self, keys: &[&[u8]]) -> Result<Vec<Option<Vec<u8>>>> {
		match &self.backend {
			KvBackend::Envoy(handle) => {
				handle
					.kv_get(
						self.actor_id.clone(),
						keys.iter().map(|key| key.to_vec()).collect(),
					)
					.await
			}
			KvBackend::InMemory(entries) => {
				let entries = entries.store.read().expect("in-memory kv lock poisoned");
				Ok(keys
					.iter()
					.map(|key| entries.get(*key).cloned())
					.collect())
			}
			KvBackend::Unconfigured => Err(anyhow!("kv handle is not configured")),
		}
	}

	pub async fn batch_put(&self, entries: &[(&[u8], &[u8])]) -> Result<()> {
		match &self.backend {
			KvBackend::Envoy(handle) => {
				handle
					.kv_put(
						self.actor_id.clone(),
						entries
							.iter()
							.map(|(key, value)| (key.to_vec(), value.to_vec()))
							.collect(),
					)
					.await
			}
			KvBackend::InMemory(store) => {
				let mut store = store.store.write().expect("in-memory kv lock poisoned");
				for (key, value) in entries {
					store.insert(key.to_vec(), value.to_vec());
				}
				Ok(())
			}
			KvBackend::Unconfigured => Err(anyhow!("kv handle is not configured")),
		}
	}

	pub async fn apply_batch(
		&self,
		puts: &[(Vec<u8>, Vec<u8>)],
		deletes: &[Vec<u8>],
	) -> Result<()> {
		match &self.backend {
			KvBackend::Envoy(_) => {
				if !puts.is_empty() {
					let put_refs: Vec<(&[u8], &[u8])> = puts
						.iter()
						.map(|(key, value)| (key.as_slice(), value.as_slice()))
						.collect();
					self.batch_put(&put_refs).await?;
				}

				if !deletes.is_empty() {
					let delete_refs: Vec<&[u8]> =
						deletes.iter().map(Vec::as_slice).collect();
					self.batch_delete(&delete_refs).await?;
				}

				Ok(())
			}
			KvBackend::InMemory(store) => {
				#[cfg(test)]
				{
					store
						.stats
						.apply_batch_calls
						.fetch_add(1, Ordering::SeqCst);
					*store
						.stats
						.last_apply_batch
						.lock()
						.expect("in-memory kv stats lock poisoned") = Some(
						KvApplyBatchSnapshot {
							puts: puts.to_vec(),
							deletes: deletes.to_vec(),
						},
					);
				}
				let mut store = store.store.write().expect("in-memory kv lock poisoned");
				for key in deletes {
					store.remove(key);
				}
				for (key, value) in puts {
					store.insert(key.clone(), value.clone());
				}
				Ok(())
			}
			KvBackend::Unconfigured => Err(anyhow!("kv handle is not configured")),
		}
	}

	pub async fn batch_delete(&self, keys: &[&[u8]]) -> Result<()> {
		match &self.backend {
			KvBackend::Envoy(handle) => {
				handle
					.kv_delete(
						self.actor_id.clone(),
						keys.iter().map(|key| key.to_vec()).collect(),
					)
					.await
			}
			KvBackend::InMemory(entries) => {
				#[cfg(test)]
				entries
					.stats
					.batch_delete_calls
					.fetch_add(1, Ordering::SeqCst);
				let mut entries = entries.store.write().expect("in-memory kv lock poisoned");
				for key in keys {
					entries.remove(*key);
				}
				Ok(())
			}
			KvBackend::Unconfigured => Err(anyhow!("kv handle is not configured")),
		}
	}
}

impl std::fmt::Debug for Kv {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Kv")
			.field("configured", &!matches!(self.backend, KvBackend::Unconfigured))
			.field(
				"in_memory",
				&matches!(self.backend, KvBackend::InMemory(_)),
			)
			.field("actor_id", &self.actor_id)
			.finish()
	}
}

impl Default for Kv {
	fn default() -> Self {
		Self {
			backend: KvBackend::Unconfigured,
			actor_id: String::new(),
		}
	}
}

#[cfg(test)]
impl Kv {
	pub(crate) fn test_apply_batch_call_count(&self) -> usize {
		match &self.backend {
			KvBackend::InMemory(store) => {
				store.stats.apply_batch_calls.load(Ordering::SeqCst)
			}
			_ => 0,
		}
	}

	pub(crate) fn test_batch_delete_call_count(&self) -> usize {
		match &self.backend {
			KvBackend::InMemory(store) => {
				store.stats.batch_delete_calls.load(Ordering::SeqCst)
			}
			_ => 0,
		}
	}

	pub(crate) fn test_last_apply_batch(&self) -> Option<KvApplyBatchSnapshot> {
		match &self.backend {
			KvBackend::InMemory(store) => store
				.stats
				.last_apply_batch
				.lock()
				.expect("in-memory kv stats lock poisoned")
				.clone(),
			_ => None,
		}
	}
}

fn apply_list_opts(entries: &mut Vec<(Vec<u8>, Vec<u8>)>, opts: ListOpts) {
	if opts.reverse {
		entries.reverse();
	}
	if let Some(limit) = opts.limit {
		entries.truncate(limit as usize);
	}
}

#[cfg(test)]
#[path = "../tests/modules/kv.rs"]
pub(crate) mod tests;
