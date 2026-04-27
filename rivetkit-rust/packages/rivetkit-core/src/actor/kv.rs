use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};

use anyhow::Result;
#[cfg(test)]
use parking_lot::Mutex;
use parking_lot::RwLock;
use rivet_envoy_client::handle::EnvoyHandle;

use crate::error::ActorRuntime;
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
	InMemory(Arc<InMemoryKv>),
}

struct InMemoryKv {
	// Forced-sync: the in-memory backend never holds this guard across `.await`,
	// and test hook setters are synchronous.
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
	batch_get_calls: AtomicUsize,
	batch_delete_calls: AtomicUsize,
	// Forced-sync: test instrumentation is synchronous and never awaited under lock.
	last_apply_batch: Mutex<Option<KvApplyBatchSnapshot>>,
	apply_batch_before_write_lock: Mutex<Option<Arc<dyn Fn() + Send + Sync + 'static>>>,
	delete_range_after_write_lock: Mutex<Option<Arc<dyn Fn() + Send + Sync + 'static>>>,
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
		let started_at = Instant::now();
		let result = match &self.backend {
			KvBackend::Envoy(handle) => {
				handle
					.kv_delete_range(self.actor_id.clone(), start.to_vec(), end.to_vec())
					.await
			}
			KvBackend::InMemory(store) => {
				let start = start.to_vec();
				let end = end.to_vec();
				let mut entries = store.store.write();

				#[cfg(test)]
				{
					let hook = store.stats.delete_range_after_write_lock.lock().clone();
					if let Some(hook) = hook {
						hook();
					}
				}

				entries.retain(|key, _| key < &start || key >= &end);
				Ok(())
			}
			KvBackend::Unconfigured => Err(kv_not_configured_error()),
		};
		self.log_call("delete_range", None, None, started_at, &result);
		result
	}

	pub async fn list_prefix(
		&self,
		prefix: &[u8],
		opts: ListOpts,
	) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
		let started_at = Instant::now();
		let result = match &self.backend {
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
					.iter()
					.filter(|(key, _)| key.starts_with(prefix))
					.map(|(key, value)| (key.clone(), value.clone()))
					.collect();
				apply_list_opts(&mut listed, opts);
				Ok(listed)
			}
			KvBackend::Unconfigured => Err(kv_not_configured_error()),
		};
		let result_count = result.as_ref().ok().map(Vec::len);
		self.log_call("list_prefix", None, result_count, started_at, &result);
		result
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
					.range(start.to_vec()..end.to_vec())
					.map(|(key, value)| (key.clone(), value.clone()))
					.collect();
				apply_list_opts(&mut listed, opts);
				Ok(listed)
			}
			KvBackend::Unconfigured => Err(kv_not_configured_error()),
		}
	}

	pub async fn batch_get(&self, keys: &[&[u8]]) -> Result<Vec<Option<Vec<u8>>>> {
		let started_at = Instant::now();
		let result = match &self.backend {
			KvBackend::Envoy(handle) => {
				handle
					.kv_get(
						self.actor_id.clone(),
						keys.iter().map(|key| key.to_vec()).collect(),
					)
					.await
			}
			KvBackend::InMemory(entries) => {
				#[cfg(test)]
				entries.stats.batch_get_calls.fetch_add(1, Ordering::SeqCst);
				let entries = entries.store.read();
				Ok(keys.iter().map(|key| entries.get(*key).cloned()).collect())
			}
			KvBackend::Unconfigured => Err(kv_not_configured_error()),
		};
		self.log_call("batch_get", Some(keys.len()), None, started_at, &result);
		result
	}

	pub async fn batch_put(&self, entries: &[(&[u8], &[u8])]) -> Result<()> {
		let started_at = Instant::now();
		let result = match &self.backend {
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
				let mut store = store.store.write();
				for (key, value) in entries {
					store.insert(key.to_vec(), value.to_vec());
				}
				Ok(())
			}
			KvBackend::Unconfigured => Err(kv_not_configured_error()),
		};
		self.log_call("batch_put", Some(entries.len()), None, started_at, &result);
		result
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
					let delete_refs: Vec<&[u8]> = deletes.iter().map(Vec::as_slice).collect();
					self.batch_delete(&delete_refs).await?;
				}

				Ok(())
			}
			KvBackend::InMemory(store) => {
				#[cfg(test)]
				{
					store.stats.apply_batch_calls.fetch_add(1, Ordering::SeqCst);
					*store.stats.last_apply_batch.lock() = Some(KvApplyBatchSnapshot {
						puts: puts.to_vec(),
						deletes: deletes.to_vec(),
					});
					let hook = store.stats.apply_batch_before_write_lock.lock().clone();
					if let Some(hook) = hook {
						hook();
					}
				}
				let mut store = store.store.write();
				for key in deletes {
					store.remove(key);
				}
				for (key, value) in puts {
					store.insert(key.clone(), value.clone());
				}
				Ok(())
			}
			KvBackend::Unconfigured => Err(kv_not_configured_error()),
		}
	}

	pub async fn batch_delete(&self, keys: &[&[u8]]) -> Result<()> {
		let started_at = Instant::now();
		let result = match &self.backend {
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
				let mut entries = entries.store.write();
				for key in keys {
					entries.remove(*key);
				}
				Ok(())
			}
			KvBackend::Unconfigured => Err(kv_not_configured_error()),
		};
		self.log_call("delete", Some(keys.len()), None, started_at, &result);
		result
	}

	fn backend_label(&self) -> &'static str {
		match &self.backend {
			KvBackend::Unconfigured => "unconfigured",
			KvBackend::Envoy(_) => "envoy",
			KvBackend::InMemory(_) => "in_memory",
		}
	}

	fn log_call<T>(
		&self,
		operation: &'static str,
		key_count: Option<usize>,
		result_count: Option<usize>,
		started_at: Instant,
		result: &Result<T>,
	) {
		let elapsed_us = duration_micros(started_at.elapsed());
		match result {
			Ok(_) => {
				tracing::debug!(
					actor_id = %self.actor_id,
					backend = self.backend_label(),
					operation,
					key_count = ?key_count,
					result_count = ?result_count,
					elapsed_us,
					outcome = "ok",
					"kv call completed"
				);
			}
			Err(error) => {
				tracing::debug!(
					actor_id = %self.actor_id,
					backend = self.backend_label(),
					operation,
					key_count = ?key_count,
					result_count = ?result_count,
					elapsed_us,
					outcome = "error",
					error = %error,
					"kv call completed"
				);
			}
		}
	}
}

fn kv_not_configured_error() -> anyhow::Error {
	ActorRuntime::NotConfigured {
		component: "kv handle".to_owned(),
	}
	.build()
}

fn duration_micros(duration: Duration) -> u64 {
	duration.as_micros().try_into().unwrap_or(u64::MAX)
}

impl std::fmt::Debug for Kv {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Kv")
			.field(
				"configured",
				&!matches!(self.backend, KvBackend::Unconfigured),
			)
			.field("in_memory", &matches!(self.backend, KvBackend::InMemory(_)))
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
			KvBackend::InMemory(store) => store.stats.apply_batch_calls.load(Ordering::SeqCst),
			_ => 0,
		}
	}

	pub(crate) fn test_batch_delete_call_count(&self) -> usize {
		match &self.backend {
			KvBackend::InMemory(store) => store.stats.batch_delete_calls.load(Ordering::SeqCst),
			_ => 0,
		}
	}

	pub(crate) fn test_batch_get_call_count(&self) -> usize {
		match &self.backend {
			KvBackend::InMemory(store) => store.stats.batch_get_calls.load(Ordering::SeqCst),
			_ => 0,
		}
	}

	pub(crate) fn test_last_apply_batch(&self) -> Option<KvApplyBatchSnapshot> {
		match &self.backend {
			KvBackend::InMemory(store) => store.stats.last_apply_batch.lock().clone(),
			_ => None,
		}
	}

	pub(crate) fn test_set_delete_range_after_write_lock_hook(
		&self,
		hook: impl Fn() + Send + Sync + 'static,
	) {
		if let KvBackend::InMemory(store) = &self.backend {
			*store.stats.delete_range_after_write_lock.lock() = Some(Arc::new(hook));
		}
	}

	pub(crate) fn test_set_apply_batch_before_write_lock_hook(
		&self,
		hook: impl Fn() + Send + Sync + 'static,
	) {
		if let KvBackend::InMemory(store) = &self.backend {
			*store.stats.apply_batch_before_write_lock.lock() = Some(Arc::new(hook));
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

// Test shim keeps moved tests in crate-root tests/ with private-module access.
#[cfg(test)]
#[path = "../../tests/kv.rs"]
pub(crate) mod tests;
