use std::sync::{
	Arc,
	atomic::{AtomicUsize, Ordering},
};

use anyhow::{Context, Result};
use async_channel::{Receiver, Sender, TrySendError};
#[cfg(debug_assertions)]
use parking_lot::Mutex;
use rivet_pools::NodeId;
use scc::HashSet;
use sha2::{Digest, Sha256};
use tokio::sync::Notify;
use universaldb::{Database, utils::IsolationLevel::Serializable};

use crate::conveyer::{
	error::SqliteStorageError,
	keys, metrics,
	types::{ColdShardRef, DatabaseBranchId, encode_cold_shard_ref},
};

const DEFAULT_SHARD_CACHE_FILL_QUEUE_CAPACITY: usize = 1024;
const DEFAULT_SHARD_CACHE_FILL_WORKERS: usize = 2;

#[cfg(debug_assertions)]
type AfterNonzeroLoadHook = Arc<dyn Fn() + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) struct ShardCacheFillKey {
	pub(super) branch_id: DatabaseBranchId,
	pub(super) shard_id: u32,
	pub(super) as_of_txid: u64,
}

#[derive(Clone)]
pub(super) struct ShardCacheFillJob {
	key: ShardCacheFillKey,
	reference: ColdShardRef,
	encoded_reference: Vec<u8>,
	object_bytes: Vec<u8>,
}

impl ShardCacheFillJob {
	pub(super) fn new(
		branch_id: DatabaseBranchId,
		reference: ColdShardRef,
		object_bytes: Vec<u8>,
	) -> Result<Self> {
		let key = ShardCacheFillKey {
			branch_id,
			shard_id: reference.shard_id,
			as_of_txid: reference.as_of_txid,
		};
		let encoded_reference = encode_cold_shard_ref(reference.clone())
			.context("encode sqlite cold shard ref for shard cache fill")?;

		Ok(Self {
			key,
			reference,
			encoded_reference,
			object_bytes,
		})
	}

	pub(super) fn key(&self) -> ShardCacheFillKey {
		self.key
	}
}

#[derive(Debug, Clone, Copy)]
pub(in crate::conveyer) struct ShardCacheFillOptions {
	pub(in crate::conveyer) queue_capacity: usize,
	pub(in crate::conveyer) worker_count: usize,
}

impl Default for ShardCacheFillOptions {
	fn default() -> Self {
		Self {
			queue_capacity: DEFAULT_SHARD_CACHE_FILL_QUEUE_CAPACITY,
			worker_count: DEFAULT_SHARD_CACHE_FILL_WORKERS,
		}
	}
}

#[derive(Clone)]
pub(in crate::conveyer) struct ShardCacheFillQueue {
	udb: Arc<Database>,
	sender: Sender<ShardCacheFillJob>,
	receiver: Receiver<ShardCacheFillJob>,
	in_flight: Arc<HashSet<ShardCacheFillKey>>,
	outstanding: Arc<AtomicUsize>,
	idle_notify: Arc<Notify>,
	#[cfg(debug_assertions)]
	after_nonzero_load_hook: Arc<Mutex<Option<AfterNonzeroLoadHook>>>,
	node_id: String,
}

impl ShardCacheFillQueue {
	pub(in crate::conveyer) fn new(
		udb: Arc<Database>,
		node_id: NodeId,
		options: ShardCacheFillOptions,
	) -> Self {
		let capacity = options.queue_capacity.max(1);
		let (sender, receiver) = async_channel::bounded(capacity);
		let queue = Self {
			udb,
			sender,
			receiver,
			in_flight: Arc::new(HashSet::default()),
			outstanding: Arc::new(AtomicUsize::new(0)),
			idle_notify: Arc::new(Notify::new()),
			#[cfg(debug_assertions)]
			after_nonzero_load_hook: Arc::new(Mutex::new(None)),
			node_id: node_id.to_string(),
		};

		queue.spawn_workers(options.worker_count);
		queue
	}

	pub(super) fn enqueue_many(&self, jobs: Vec<ShardCacheFillJob>) {
		for job in jobs {
			self.enqueue(job);
		}
	}

	#[cfg(debug_assertions)]
	pub(in crate::conveyer) async fn fill_once_for_test(
		&self,
		branch_id: DatabaseBranchId,
		reference: ColdShardRef,
		object_bytes: Vec<u8>,
	) -> Result<()> {
		let job = ShardCacheFillJob::new(branch_id, reference, object_bytes)?;
		self.fill_job(job).await
	}

	#[cfg(debug_assertions)]
	pub(in crate::conveyer) async fn wait_idle_for_test(&self) {
		loop {
			let notified = self.idle_notify.notified();
			tokio::pin!(notified);
			notified.as_mut().enable();

			if self.outstanding.load(Ordering::SeqCst) == 0 {
				return;
			}

			if let Some(hook) = self.after_nonzero_load_hook.lock().take() {
				hook();
			}

			notified.as_mut().await;
		}
	}

	#[cfg(debug_assertions)]
	pub(in crate::conveyer) fn outstanding_for_test(&self) -> usize {
		self.outstanding.load(Ordering::SeqCst)
	}

	#[cfg(debug_assertions)]
	pub(in crate::conveyer) fn set_outstanding_for_test(&self, outstanding: usize) {
		self.outstanding.store(outstanding, Ordering::SeqCst);
		if outstanding == 0 {
			self.idle_notify.notify_waiters();
		}
	}

	#[cfg(debug_assertions)]
	pub(in crate::conveyer) fn complete_one_outstanding_for_test(&self) {
		self.outstanding.fetch_sub(1, Ordering::SeqCst);
		self.idle_notify.notify_waiters();
	}

	#[cfg(debug_assertions)]
	pub(in crate::conveyer) fn set_after_nonzero_load_hook_for_test(
		&self,
		hook: AfterNonzeroLoadHook,
	) {
		*self.after_nonzero_load_hook.lock() = Some(hook);
	}

	fn enqueue(&self, job: ShardCacheFillJob) {
		let key = job.key();
		if self.in_flight.insert_sync(key).is_err() {
			metrics::SQLITE_SHARD_CACHE_FILL_TOTAL
				.with_label_values(&[metrics::SHARD_CACHE_FILL_SKIPPED_DUPLICATE])
				.inc();
			return;
		}
		self.outstanding.fetch_add(1, Ordering::SeqCst);

		match self.sender.try_send(job) {
			Ok(()) => {
				metrics::SQLITE_SHARD_CACHE_FILL_TOTAL
					.with_label_values(&[metrics::SHARD_CACHE_FILL_SCHEDULED])
					.inc();
			}
			Err(TrySendError::Full(job)) => {
				self.remove_in_flight(job.key());
				metrics::SQLITE_SHARD_CACHE_FILL_TOTAL
					.with_label_values(&[metrics::SHARD_CACHE_FILL_SKIPPED_QUEUE_FULL])
					.inc();
				metrics::SQLITE_SHARD_CACHE_FILL_SKIPPED_QUEUE_FULL_TOTAL
					.with_label_values(&[self.node_id.as_str()])
					.inc();
			}
			Err(TrySendError::Closed(job)) => {
				self.remove_in_flight(job.key());
				metrics::SQLITE_SHARD_CACHE_FILL_TOTAL
					.with_label_values(&[metrics::SHARD_CACHE_FILL_FAILED])
					.inc();
				tracing::warn!(
					branch_id = ?job.key.branch_id,
					shard_id = job.key.shard_id,
					as_of_txid = job.key.as_of_txid,
					"sqlite shard cache fill queue is closed"
				);
			}
		}
	}

	fn spawn_workers(&self, worker_count: usize) {
		if worker_count == 0 {
			return;
		}
		let Ok(handle) = tokio::runtime::Handle::try_current() else {
			tracing::warn!(
				"sqlite shard cache fill workers could not start without a tokio runtime"
			);
			return;
		};

		for _ in 0..worker_count {
			let queue = self.clone();
			let receiver = self.receiver.clone();
			handle.spawn(async move {
				loop {
					let Ok(job) = receiver.recv().await else {
						break;
					};
					let key = job.key();
					if let Err(error) = queue.fill_job(job).await {
						tracing::warn!(
							?error,
							branch_id = ?key.branch_id,
							shard_id = key.shard_id,
							as_of_txid = key.as_of_txid,
							"sqlite shard cache fill failed"
						);
					}
					queue.remove_in_flight(key);
				}
			});
		}
	}

	fn remove_in_flight(&self, key: ShardCacheFillKey) {
		self.in_flight.remove_sync(&key);
		self.outstanding.fetch_sub(1, Ordering::SeqCst);
		self.idle_notify.notify_waiters();
	}

	async fn fill_job(&self, job: ShardCacheFillJob) -> Result<()> {
		let result = self.fill_job_inner(job).await;
		match &result {
			Ok(ShardCacheFillOutcome::Succeeded { bytes_written }) => {
				metrics::SQLITE_SHARD_CACHE_FILL_TOTAL
					.with_label_values(&[metrics::SHARD_CACHE_FILL_SUCCEEDED])
					.inc();
				if *bytes_written > 0 {
					metrics::SQLITE_SHARD_CACHE_FILL_BYTES_TOTAL.inc_by(*bytes_written);
					metrics::SQLITE_SHARD_CACHE_RESIDENT_BYTES
						.add(i64::try_from(*bytes_written).unwrap_or(i64::MAX));
				}
			}
			Ok(ShardCacheFillOutcome::SkippedNoColdRef) => {
				metrics::SQLITE_SHARD_CACHE_FILL_TOTAL
					.with_label_values(&[metrics::SHARD_CACHE_FILL_SKIPPED_NO_COLD_REF])
					.inc();
			}
			Err(_) => {
				metrics::SQLITE_SHARD_CACHE_FILL_TOTAL
					.with_label_values(&[metrics::SHARD_CACHE_FILL_FAILED])
					.inc();
			}
		}
		result.map(|_| ())
	}

	async fn fill_job_inner(&self, job: ShardCacheFillJob) -> Result<ShardCacheFillOutcome> {
		let expected_len = usize::try_from(job.reference.size_bytes).unwrap_or(usize::MAX);
		if job.object_bytes.len() != expected_len
			|| content_hash(&job.object_bytes) != job.reference.content_hash
		{
			return Err(SqliteStorageError::ShardCacheCorrupt {
				shard_id: job.key.shard_id,
				as_of_txid: job.key.as_of_txid,
			}
			.into());
		}

		let job_for_tx = job.clone();
		self.udb
			.run(move |tx| {
				let job = job_for_tx.clone();
				async move { fill_job_tx(&tx, job).await }
			})
			.await
	}
}

enum ShardCacheFillOutcome {
	Succeeded { bytes_written: u64 },
	SkippedNoColdRef,
}

async fn fill_job_tx(
	tx: &universaldb::Transaction,
	job: ShardCacheFillJob,
) -> Result<ShardCacheFillOutcome> {
	let cold_ref_key = keys::branch_compaction_cold_shard_key(
		job.key.branch_id,
		job.key.shard_id,
		job.key.as_of_txid,
	);
	let Some(live_ref) = tx.informal().get(&cold_ref_key, Serializable).await? else {
		return Ok(ShardCacheFillOutcome::SkippedNoColdRef);
	};
	if live_ref.as_slice() != job.encoded_reference.as_slice() {
		return Ok(ShardCacheFillOutcome::SkippedNoColdRef);
	}

	let shard_key = keys::branch_shard_key(job.key.branch_id, job.key.shard_id, job.key.as_of_txid);
	if let Some(existing) = tx.informal().get(&shard_key, Serializable).await? {
		if existing.as_slice() == job.object_bytes.as_slice() {
			return Ok(ShardCacheFillOutcome::Succeeded { bytes_written: 0 });
		}
		return Err(SqliteStorageError::ShardCacheCorrupt {
			shard_id: job.key.shard_id,
			as_of_txid: job.key.as_of_txid,
		}
		.into());
	}

	tx.informal().set(&shard_key, &job.object_bytes);

	Ok(ShardCacheFillOutcome::Succeeded {
		bytes_written: u64::try_from(job.object_bytes.len()).unwrap_or(u64::MAX),
	})
}

fn content_hash(bytes: &[u8]) -> [u8; 32] {
	let digest = Sha256::digest(bytes);
	let mut hash = [0_u8; 32];
	hash.copy_from_slice(&digest);
	hash
}
