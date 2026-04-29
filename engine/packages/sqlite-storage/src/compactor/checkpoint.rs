//! Checkpoint creation for PITR-enabled sqlite storage.

use std::{sync::Arc, time::SystemTime};

use anyhow::{Context, Result, bail};
use futures_util::TryStreamExt;
use namespace::types::SqliteNamespaceConfig;
use rivet_pools::NodeId;
use tokio_util::sync::CancellationToken;
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::IsolationLevel::{Serializable, Snapshot},
};

use crate::pump::{
	keys,
	quota,
	types::{
		CheckpointEntry, CheckpointMeta, Checkpoints, decode_checkpoints, decode_db_head,
		encode_checkpoint_meta, encode_checkpoints,
	},
};

use super::metrics;

const CHECKPOINT_COPY_BATCH_ROWS: usize = 75;
const CHECKPOINT_NAMESPACE_UNKNOWN: &str = "unknown";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckpointOutcome {
	Created { bytes: u64, tx_count: u64 },
	SkippedQuota,
}

pub async fn create_checkpoint(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	ckp_txid: u64,
	cancel_token: CancellationToken,
	namespace_config: SqliteNamespaceConfig,
) -> Result<CheckpointOutcome> {
	create_checkpoint_with_node_id(
		udb,
		actor_id,
		ckp_txid,
		cancel_token,
		namespace_config,
		NodeId::new(),
	)
	.await
}

pub(crate) async fn create_checkpoint_with_node_id(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	ckp_txid: u64,
	cancel_token: CancellationToken,
	namespace_config: SqliteNamespaceConfig,
	node_id: NodeId,
) -> Result<CheckpointOutcome> {
	let node_id = node_id.to_string();
	let _timer = metrics::SQLITE_CHECKPOINT_CREATION_DURATION_SECONDS
		.with_label_values(&[node_id.as_str()])
		.start_timer();

	let outcome = create_checkpoint_inner(
		Arc::clone(&udb),
		actor_id.clone(),
		ckp_txid,
		cancel_token,
		namespace_config,
		node_id.as_str(),
	)
	.await;

	if outcome.is_err() {
		if let Err(err) = cleanup_checkpoint_prefix(udb.as_ref(), actor_id.clone(), ckp_txid).await {
			tracing::warn!(
				?err,
				actor_id = %actor_id,
				ckp_txid,
				"failed to clean up partial sqlite checkpoint"
			);
		}
	}

	outcome
}

async fn create_checkpoint_inner(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	ckp_txid: u64,
	cancel_token: CancellationToken,
	namespace_config: SqliteNamespaceConfig,
	node_id: &str,
) -> Result<CheckpointOutcome> {
	ensure_not_cancelled(&cancel_token)?;
	let _concurrency_guard = test_hooks::maybe_enter_checkpoint().await;

	let source = load_checkpoint_source(udb.as_ref(), actor_id.clone(), ckp_txid).await?;
	if source.rows.is_empty() && source.db_size_pages == 0 {
		return Ok(CheckpointOutcome::Created {
			bytes: 0,
			tx_count: 0,
		});
	}

	let now_ms = now_ms()?;
	let mut checkpoint_bytes = source.copy_bytes;
	let meta = CheckpointMeta {
		taken_at_ms: now_ms,
		head_txid: ckp_txid,
		db_size_pages: source.db_size_pages,
		byte_count: source.copy_bytes,
		refcount: 0,
		pinned_reason: None,
	};
	let encoded_meta = encode_checkpoint_meta(meta.clone()).context("encode checkpoint meta")?;
	let checkpoint_meta_key = keys::checkpoint_meta_key(&actor_id, ckp_txid);
	checkpoint_bytes = checkpoint_bytes
		.checked_add(tracked_entry_size_u64(&checkpoint_meta_key, &encoded_meta)?)
		.context("sqlite checkpoint byte count overflowed")?;

	let quota_preflight = udb
		.run({
			let actor_id = actor_id.clone();
			let namespace_config = namespace_config.clone();
			move |tx| {
				let actor_id = actor_id.clone();
				let namespace_config = namespace_config.clone();
				async move {
					let pitr_used = quota::read_pitr(&tx, &actor_id).await?;
					check_checkpoint_quota(pitr_used, checkpoint_bytes, &namespace_config)
				}
			}
		})
		.await?;
	if !quota_preflight {
		metrics::SQLITE_CHECKPOINT_SKIPPED_QUOTA_TOTAL
			.with_label_values(&[CHECKPOINT_NAMESPACE_UNKNOWN])
			.inc();
		return Ok(CheckpointOutcome::SkippedQuota);
	}

	let mut tx_count = 0u64;
	clear_checkpoint_prefix(udb.as_ref(), actor_id.clone(), ckp_txid).await?;
	tx_count += 1;

	for chunk in source.rows.chunks(CHECKPOINT_COPY_BATCH_ROWS) {
		ensure_not_cancelled(&cancel_token)?;
		let rows = chunk.to_vec();
		udb.run(move |tx| {
			let rows = rows.clone();
			async move {
				for row in rows {
					tx.informal().set(&row.dst_key, &row.value);
				}
				Ok(())
			}
		})
		.await?;
		tx_count += 1;
		test_hooks::maybe_pause_after_copy_tx(&actor_id).await;
	}

	ensure_not_cancelled(&cancel_token)?;
	let final_checkpoint_bytes = checkpoint_bytes;
	let source_copy_bytes = source.copy_bytes;
	let final_tx_count = udb
		.run({
			let actor_id = actor_id.clone();
			let namespace_config = namespace_config.clone();
			let encoded_meta = encoded_meta.clone();
			let checkpoint_meta_key = checkpoint_meta_key.clone();
			move |tx| {
				let actor_id = actor_id.clone();
				let namespace_config = namespace_config.clone();
				let encoded_meta = encoded_meta.clone();
				let checkpoint_meta_key = checkpoint_meta_key.clone();
				async move {
					let checkpoints_key = keys::meta_checkpoints_key(&actor_id);
					let existing_checkpoints_bytes =
						tx.informal().get(&checkpoints_key, Serializable).await?;
					let mut checkpoints = existing_checkpoints_bytes
						.as_deref()
						.map(|value| decode_checkpoints(value))
						.transpose()
						.context("decode sqlite checkpoints")?
						.unwrap_or(Checkpoints {
							entries: Vec::new(),
						});
					checkpoints.entries.retain(|entry| entry.ckp_txid != ckp_txid);
					checkpoints.entries.push(CheckpointEntry {
						ckp_txid,
						taken_at_ms: now_ms,
						byte_count: source_copy_bytes,
						refcount: 0,
					});
					checkpoints.entries.sort_by_key(|entry| entry.ckp_txid);
					if namespace_config.default_max_checkpoints > 0 {
						let max = namespace_config.default_max_checkpoints as usize;
						if checkpoints.entries.len() > max {
							let drop_count = checkpoints.entries.len() - max;
							checkpoints.entries.drain(0..drop_count);
						}
					}
					let encoded_checkpoints =
						encode_checkpoints(checkpoints).context("encode sqlite checkpoints")?;

					let old_checkpoints_bytes = existing_checkpoints_bytes
						.as_ref()
						.map(|value| tracked_entry_size_i64(&checkpoints_key, value))
						.transpose()?
						.unwrap_or(0);
					let new_checkpoints_bytes =
						tracked_entry_size_i64(&checkpoints_key, &encoded_checkpoints)?;
					let checkpoint_meta_bytes =
						tracked_entry_size_i64(&checkpoint_meta_key, &encoded_meta)?;
					let pitr_delta = i64::try_from(source_copy_bytes)
						.context("sqlite checkpoint source bytes exceeded i64")?
						.checked_add(checkpoint_meta_bytes)
						.context("sqlite checkpoint pitr delta overflowed")?
						.checked_add(new_checkpoints_bytes - old_checkpoints_bytes)
						.context("sqlite checkpoint pitr delta overflowed")?;

					let pitr_used = quota::read_pitr(&tx, &actor_id).await?;
					if !check_checkpoint_quota_i64(pitr_used, pitr_delta, &namespace_config)? {
						return Ok(false);
					}

					tx.informal().set(&checkpoint_meta_key, &encoded_meta);
					tx.informal().set(&checkpoints_key, &encoded_checkpoints);
					if pitr_delta != 0 {
						quota::atomic_add_pitr(&tx, &actor_id, pitr_delta);
					}

					Ok(true)
				}
			}
		})
		.await?;
	tx_count += 1;

	if !final_tx_count {
		cleanup_checkpoint_prefix(udb.as_ref(), actor_id.clone(), ckp_txid).await?;
		metrics::SQLITE_CHECKPOINT_SKIPPED_QUOTA_TOTAL
			.with_label_values(&[CHECKPOINT_NAMESPACE_UNKNOWN])
			.inc();
		return Ok(CheckpointOutcome::SkippedQuota);
	}

	metrics::SQLITE_CHECKPOINT_CREATION_BYTES
		.with_label_values(&[node_id])
		.observe(final_checkpoint_bytes as f64);
	metrics::SQLITE_COMPACTOR_CHECKPOINT_TX_COUNT
		.with_label_values(&[node_id])
		.observe(tx_count as f64);

	Ok(CheckpointOutcome::Created {
		bytes: final_checkpoint_bytes,
		tx_count,
	})
}

async fn load_checkpoint_source(
	db: &universaldb::Database,
	actor_id: String,
	ckp_txid: u64,
) -> Result<CheckpointSource> {
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			let head_bytes = tx
				.informal()
				.get(&keys::meta_head_key(&actor_id), Snapshot)
				.await?
				.context("sqlite checkpoint requires db head")?;
			let head = decode_db_head(&head_bytes).context("decode checkpoint db head")?;
			let mut rows = Vec::new();
			let mut copy_bytes = 0u64;

			for (src_key, value) in tx_scan_prefix_values(&tx, &keys::shard_prefix(&actor_id)).await?
			{
				let shard_id = decode_shard_id(&actor_id, &src_key)?;
				let dst_key = keys::checkpoint_shard_key(&actor_id, ckp_txid, shard_id);
				copy_bytes = copy_bytes
					.checked_add(tracked_entry_size_u64(&dst_key, &value)?)
					.context("sqlite checkpoint byte count overflowed")?;
				rows.push(CheckpointRow { dst_key, value });
			}

			for (src_key, value) in
				tx_scan_prefix_values(&tx, &keys::pidx_delta_prefix(&actor_id)).await?
			{
				let txid = decode_pidx_txid(&value)?;
				if txid > ckp_txid {
					continue;
				}
				let pgno = decode_pidx_pgno(&actor_id, &src_key)?;
				let dst_key = keys::checkpoint_pidx_delta_key(&actor_id, ckp_txid, pgno);
				copy_bytes = copy_bytes
					.checked_add(tracked_entry_size_u64(&dst_key, &value)?)
					.context("sqlite checkpoint byte count overflowed")?;
				rows.push(CheckpointRow { dst_key, value });
			}

			Ok(CheckpointSource {
				rows,
				copy_bytes,
				db_size_pages: head.db_size_pages,
			})
		}
	})
	.await
}

async fn cleanup_checkpoint_prefix(
	db: &universaldb::Database,
	actor_id: String,
	ckp_txid: u64,
) -> Result<()> {
	clear_checkpoint_prefix(db, actor_id, ckp_txid).await
}

async fn clear_checkpoint_prefix(
	db: &universaldb::Database,
	actor_id: String,
	ckp_txid: u64,
) -> Result<()> {
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		async move {
			let prefix = keys::checkpoint_prefix(&actor_id, ckp_txid);
			let (begin, end) = prefix_range(&prefix);
			tx.informal().clear_range(&begin, &end);
			Ok(())
		}
	})
	.await
}

async fn tx_scan_prefix_values(
	tx: &universaldb::Transaction,
	prefix: &[u8],
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix.to_vec()));
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&prefix_subspace)
		},
		Snapshot,
	);
	let mut rows = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		rows.push((entry.key().to_vec(), entry.value().to_vec()));
	}

	Ok(rows)
}

fn decode_shard_id(actor_id: &str, key: &[u8]) -> Result<u32> {
	let prefix = keys::shard_prefix(actor_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("shard key did not start with expected prefix")?;
	let bytes: [u8; std::mem::size_of::<u32>()] = suffix
		.try_into()
		.map_err(|_| anyhow::anyhow!("shard key suffix had invalid length"))?;

	Ok(u32::from_be_bytes(bytes))
}

fn decode_pidx_pgno(actor_id: &str, key: &[u8]) -> Result<u32> {
	let prefix = keys::pidx_delta_prefix(actor_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("pidx key did not start with expected prefix")?;
	let bytes: [u8; std::mem::size_of::<u32>()] = suffix
		.try_into()
		.map_err(|_| anyhow::anyhow!("pidx key suffix had invalid length"))?;

	Ok(u32::from_be_bytes(bytes))
}

fn decode_pidx_txid(value: &[u8]) -> Result<u64> {
	let bytes: [u8; std::mem::size_of::<u64>()] = value
		.try_into()
		.map_err(|_| anyhow::anyhow!("pidx txid had invalid length"))?;

	Ok(u64::from_be_bytes(bytes))
}

fn check_checkpoint_quota(
	pitr_used: i64,
	checkpoint_bytes: u64,
	namespace_config: &SqliteNamespaceConfig,
) -> Result<bool> {
	let checkpoint_bytes =
		i64::try_from(checkpoint_bytes).context("sqlite checkpoint bytes exceeded i64")?;
	check_checkpoint_quota_i64(pitr_used, checkpoint_bytes, namespace_config)
}

fn check_checkpoint_quota_i64(
	pitr_used: i64,
	pitr_delta: i64,
	namespace_config: &SqliteNamespaceConfig,
) -> Result<bool> {
	let would_be = pitr_used
		.checked_add(pitr_delta)
		.context("sqlite checkpoint quota total overflowed")?;
	let actor_cap = i64::try_from(namespace_config.pitr_max_bytes_per_actor)
		.context("sqlite pitr actor cap exceeded i64")?;
	let namespace_cap = i64::try_from(namespace_config.pitr_namespace_budget_bytes)
		.context("sqlite pitr namespace cap exceeded i64")?;

	Ok(would_be <= actor_cap && would_be <= namespace_cap)
}

fn tracked_entry_size_u64(key: &[u8], value: &[u8]) -> Result<u64> {
	u64::try_from(key.len() + value.len()).context("sqlite tracked entry size exceeded u64")
}

fn tracked_entry_size_i64(key: &[u8], value: &[u8]) -> Result<i64> {
	i64::try_from(key.len() + value.len()).context("sqlite tracked entry size exceeded i64")
}

fn prefix_range(prefix: &[u8]) -> (Vec<u8>, Vec<u8>) {
	universaldb::tuple::Subspace::from_bytes(prefix.to_vec()).range()
}

fn ensure_not_cancelled(cancel_token: &CancellationToken) -> Result<()> {
	if cancel_token.is_cancelled() {
		bail!("sqlite checkpoint creation cancelled");
	}

	Ok(())
}

fn now_ms() -> Result<i64> {
	let elapsed = SystemTime::now()
		.duration_since(std::time::UNIX_EPOCH)
		.context("system clock was before unix epoch")?;
	i64::try_from(elapsed.as_millis()).context("sqlite checkpoint timestamp exceeded i64")
}

#[derive(Debug, Clone)]
struct CheckpointSource {
	rows: Vec<CheckpointRow>,
	copy_bytes: u64,
	db_size_pages: u32,
}

#[derive(Debug, Clone)]
struct CheckpointRow {
	dst_key: Vec<u8>,
	value: Vec<u8>,
}

#[cfg(debug_assertions)]
pub mod test_hooks {
	use std::sync::{
		Arc,
		atomic::{AtomicUsize, Ordering},
	};

	use parking_lot::Mutex;
	use tokio::sync::Notify;

	static CONCURRENCY_HOOK: Mutex<Option<Arc<ConcurrencyHook>>> = Mutex::new(None);
	static PAUSE_AFTER_COPY_TX: Mutex<Option<(String, Arc<Notify>, Arc<Notify>)>> =
		Mutex::new(None);

	pub struct HookGuard {
		clear: fn(),
	}

	pub struct RunningGuard {
		hook: Arc<ConcurrencyHook>,
	}

	pub struct ConcurrencyHook {
		entered: AtomicUsize,
		current: AtomicUsize,
		max_seen: Arc<AtomicUsize>,
		reached: Arc<Notify>,
		release: Arc<Notify>,
	}

	pub fn pause_inside_checkpoint() -> (HookGuard, Arc<AtomicUsize>, Arc<Notify>, Arc<Notify>) {
		let max_seen = Arc::new(AtomicUsize::new(0));
		let reached = Arc::new(Notify::new());
		let release = Arc::new(Notify::new());
		*CONCURRENCY_HOOK.lock() = Some(Arc::new(ConcurrencyHook {
			entered: AtomicUsize::new(0),
			current: AtomicUsize::new(0),
			max_seen: Arc::clone(&max_seen),
			reached: Arc::clone(&reached),
			release: Arc::clone(&release),
		}));

		(HookGuard { clear: clear_concurrency }, max_seen, reached, release)
	}

	pub fn pause_after_copy_tx(actor_id: &str) -> (HookGuard, Arc<Notify>, Arc<Notify>) {
		let reached = Arc::new(Notify::new());
		let release = Arc::new(Notify::new());
		*PAUSE_AFTER_COPY_TX.lock() =
			Some((actor_id.to_string(), Arc::clone(&reached), Arc::clone(&release)));

		(HookGuard { clear: clear_copy_pause }, reached, release)
	}

	pub(super) async fn maybe_enter_checkpoint() -> Option<RunningGuard> {
		let hook = CONCURRENCY_HOOK.lock().as_ref().map(Arc::clone);
		if let Some(hook) = hook {
			let entered = hook.entered.fetch_add(1, Ordering::SeqCst) + 1;
			let current = hook.current.fetch_add(1, Ordering::SeqCst) + 1;
			hook.max_seen.fetch_max(current, Ordering::SeqCst);
			hook.reached.notify_waiters();
			if entered <= 16 {
				hook.release.notified().await;
			}
			Some(RunningGuard { hook })
		} else {
			None
		}
	}

	pub(super) async fn maybe_pause_after_copy_tx(actor_id: &str) {
		let hook = PAUSE_AFTER_COPY_TX
			.lock()
			.as_ref()
			.filter(|(hook_actor_id, _, _)| hook_actor_id == actor_id)
			.map(|(_, reached, release)| (Arc::clone(reached), Arc::clone(release)));

		if let Some((reached, release)) = hook {
			reached.notify_waiters();
			release.notified().await;
		}
	}

	fn clear_concurrency() {
		*CONCURRENCY_HOOK.lock() = None;
	}

	fn clear_copy_pause() {
		*PAUSE_AFTER_COPY_TX.lock() = None;
	}

	impl Drop for HookGuard {
		fn drop(&mut self) {
			(self.clear)();
		}
	}

	impl Drop for RunningGuard {
		fn drop(&mut self) {
			self.hook.current.fetch_sub(1, Ordering::SeqCst);
		}
	}
}

#[cfg(not(debug_assertions))]
mod test_hooks {
	pub(super) async fn maybe_enter_checkpoint() {}

	pub(super) async fn maybe_pause_after_copy_tx(_actor_id: &str) {}
}
