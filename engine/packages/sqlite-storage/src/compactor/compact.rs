//! Per-actor compaction pass for the stateless sqlite-storage layout.

use std::{collections::BTreeMap, sync::Arc};

use anyhow::{Context, Result, bail};
use futures_util::TryStreamExt;
use rivet_pools::NodeId;
use tokio_util::sync::CancellationToken;
use universaldb::{
	RangeOption,
	options::StreamingMode,
	utils::{
		IsolationLevel::{Serializable, Snapshot},
	},
};

use crate::pump::{
	branch,
	keys::{self, SHARD_SIZE},
	ltx::decode_ltx_v3,
	quota,
	types::{ActorBranchId, MetaCompact, decode_db_head, decode_meta_compact, encode_meta_compact},
	udb,
};

use super::{fold_branch_shard, fold_shard, metrics};

const PIDX_TXID_BYTES: usize = std::mem::size_of::<u64>();

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompactionOutcome {
	pub pages_folded: u64,
	pub deltas_freed: u64,
	pub compare_and_clear_noops: u64,
	pub bytes_freed: i64,
	pub materialized_txid: u64,
}

pub async fn compact_default_batch(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	batch_size_deltas: u32,
	cancel_token: CancellationToken,
) -> Result<CompactionOutcome> {
	compact_default_batch_with_node_id(
		udb,
		actor_id,
		batch_size_deltas,
		cancel_token,
		NodeId::new(),
	)
	.await
}

pub(crate) async fn compact_default_batch_with_node_id(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	batch_size_deltas: u32,
	cancel_token: CancellationToken,
	node_id: NodeId,
) -> Result<CompactionOutcome> {
	let node_id = node_id.to_string();
	let labels = &[node_id.as_str()];
	let _timer = metrics::SQLITE_COMPACTOR_PASS_DURATION
		.with_label_values(labels)
		.start_timer();

	ensure_not_cancelled(&cancel_token)?;
	let plan = plan_batch(udb.as_ref(), actor_id.clone(), batch_size_deltas).await?;
	metrics::SQLITE_COMPACTOR_LAG
		.with_label_values(labels)
		.observe(plan.selected_delta_txids.len() as f64);
	if plan.selected_delta_txids.is_empty() {
		return Ok(CompactionOutcome::default());
	}

	test_hooks::maybe_pause_after_plan(&actor_id).await;
	ensure_not_cancelled(&cancel_token)?;
	let write_result = write_batch(udb.as_ref(), actor_id.clone(), plan).await?;

	ensure_not_cancelled(&cancel_token)?;
	let compare_and_clear_noops =
		count_compare_and_clear_noops(udb.as_ref(), actor_id.clone(), write_result.attempted_pidx_deletes)
			.await?;

	metrics::SQLITE_COMPACTOR_PAGES_FOLDED_TOTAL
		.with_label_values(labels)
		.inc_by(write_result.pages_folded);
	metrics::SQLITE_COMPACTOR_DELTAS_FREED_TOTAL
		.with_label_values(labels)
		.inc_by(write_result.deltas_freed);
	metrics::SQLITE_COMPACTOR_COMPARE_AND_CLEAR_NOOP_TOTAL
		.with_label_values(labels)
		.inc_by(compare_and_clear_noops);

	Ok(CompactionOutcome {
		pages_folded: write_result.pages_folded,
		deltas_freed: write_result.deltas_freed,
		compare_and_clear_noops,
		bytes_freed: write_result.bytes_freed,
		materialized_txid: write_result.materialized_txid,
	})
}

async fn plan_batch(
	db: &universaldb::Database,
	actor_id: String,
	batch_size_deltas: u32,
) -> Result<CompactionPlan> {
	db.run(move |tx| {
		let actor_id = actor_id.clone();

		async move {
			let scope = resolve_storage_scope(&tx, &actor_id, Snapshot).await?;
			let Some(head_bytes) = tx_get_value(&tx, &scope.meta_head_key(&actor_id), Snapshot).await?
			else {
				return Ok(CompactionPlan::default());
			};
			let head = decode_db_head(&head_bytes).context("decode sqlite db head for compaction")?;
			let compact = tx_get_value(&tx, &scope.meta_compact_key(&actor_id), Snapshot)
				.await?
				.as_deref()
				.map(decode_meta_compact)
				.transpose()
				.context("decode sqlite compact meta")?
				.unwrap_or(MetaCompact {
					materialized_txid: 0,
				});
			if head.head_txid <= compact.materialized_txid || batch_size_deltas == 0 {
				return Ok(CompactionPlan::default());
			}

			let pidx_rows = load_pidx_rows(&tx, &scope, &actor_id).await?;
			let delta_entries = load_delta_entries(&tx, &scope, &actor_id).await?;
			let selected_delta_txids = delta_entries
				.keys()
				.copied()
				.filter(|txid| {
					*txid > compact.materialized_txid && *txid <= head.head_txid
				})
				.take(batch_size_deltas as usize)
				.collect::<Vec<_>>();

			let mut selected_deltas = BTreeMap::new();
			for txid in &selected_delta_txids {
				let entry = delta_entries
					.get(txid)
					.with_context(|| format!("missing selected delta {txid}"))?;
				let decoded = decode_ltx_v3(&entry.blob)
					.with_context(|| format!("decode delta {txid} for compaction"))?;
				selected_deltas.insert(*txid, decoded);
			}

			let mut pages_by_shard = BTreeMap::<u32, Vec<FoldPage>>::new();
			for row in pidx_rows {
				if row.pgno > head.db_size_pages || !selected_deltas.contains_key(&row.txid) {
					continue;
				}

				let bytes = selected_deltas
					.get(&row.txid)
					.and_then(|decoded| decoded.get_page(row.pgno))
					.with_context(|| {
						format!("PIDX row for page {} pointed at delta {} without the page", row.pgno, row.txid)
					})?
					.to_vec();
				pages_by_shard
					.entry(row.pgno / SHARD_SIZE)
					.or_default()
					.push(FoldPage {
						pgno: row.pgno,
						expected_txid: row.txid,
						bytes,
					});
			}

			let selected_delta_entries = selected_delta_txids
				.iter()
				.map(|txid| {
					let entry = delta_entries
						.get(txid)
						.with_context(|| format!("missing selected delta entry {txid}"))?;
					Ok((*txid, entry.clone()))
				})
				.collect::<Result<BTreeMap<_, _>>>()?;
			let materialized_txid = selected_delta_txids.iter().copied().max().unwrap_or(0);

			Ok(CompactionPlan {
				scope,
				selected_delta_txids,
				selected_delta_entries,
				pages_by_shard,
				materialized_txid,
			})
		}
	})
	.await
}

async fn write_batch(
	db: &universaldb::Database,
	actor_id: String,
	plan: CompactionPlan,
) -> Result<WriteResult> {
	db.run(move |tx| {
		let actor_id = actor_id.clone();
		let plan = plan.clone();

		async move {
			let Some(head_bytes) =
				tx_get_value(&tx, &plan.scope.meta_head_key(&actor_id), Serializable).await?
			else {
				return Ok(WriteResult::default());
			};
			let head = decode_db_head(&head_bytes).context("decode sqlite db head for compaction write")?;

			test_hooks::maybe_pause_after_write_head_read(&actor_id).await;

			let mut attempted_pidx_deletes = Vec::new();
			let mut pages_folded = 0u64;
			let mut bytes_freed = plan
				.selected_delta_entries
				.values()
				.map(|entry| entry.tracked_size)
				.sum::<i64>();

			for (shard_id, fold_pages) in &plan.pages_by_shard {
				let page_updates = fold_pages
					.iter()
					.filter(|page| page.pgno <= head.db_size_pages)
					.map(|page| (page.pgno, page.bytes.clone()))
					.collect::<Vec<_>>();
				if page_updates.is_empty() {
					continue;
				}

				let bytes_added = match plan.scope {
					StorageScope::Branch(branch_id) => {
						fold_branch_shard(&tx, branch_id, *shard_id, plan.materialized_txid, page_updates)
							.await?
					}
					StorageScope::Legacy => {
						fold_shard(&tx, &actor_id, *shard_id, plan.materialized_txid, page_updates)
							.await?
					}
				};
				bytes_freed -= bytes_added;
				for page in fold_pages.iter().filter(|page| page.pgno <= head.db_size_pages) {
					let key = plan.scope.pidx_key(&actor_id, page.pgno);
					let expected_value = page.expected_txid.to_be_bytes();
					udb::compare_and_clear(&tx, &key, &expected_value);
					bytes_freed += tracked_entry_size(&key, &expected_value)?;
					attempted_pidx_deletes.push(PidxDelete {
						key,
						expected_value: expected_value.to_vec(),
					});
					pages_folded += 1;
				}
			}

			for txid in &plan.selected_delta_txids {
				let prefix = plan.scope.delta_chunk_prefix(&actor_id, *txid);
				let (begin, end) = prefix_range(&prefix);
				tx.informal().clear_range(&begin, &end);
			}

			let compact = encode_meta_compact(MetaCompact {
				materialized_txid: plan.materialized_txid,
			})
			.context("encode compact meta")?;
			tx.informal()
				.set(&plan.scope.meta_compact_key(&actor_id), &compact);
			let manifest_branch_id = match plan.scope {
				StorageScope::Branch(branch_id) => branch_id,
				StorageScope::Legacy => head.branch_id,
			};
			tx.informal().set(
				&keys::branch_manifest_last_hot_pass_txid_key(manifest_branch_id),
				&plan.materialized_txid.to_be_bytes(),
			);
			if bytes_freed != 0 {
				match plan.scope {
					StorageScope::Branch(branch_id) => {
						quota::atomic_add_branch(&tx, branch_id, -bytes_freed);
					}
					StorageScope::Legacy => {
						quota::atomic_add(&tx, &actor_id, -bytes_freed);
					}
				}
			}

			Ok(WriteResult {
				pages_folded,
				deltas_freed: plan.selected_delta_txids.len() as u64,
				bytes_freed,
				materialized_txid: plan.materialized_txid,
				attempted_pidx_deletes,
			})
		}
	})
	.await
}

#[cfg(debug_assertions)]
pub async fn validate_quota(
	udb: Arc<universaldb::Database>,
	actor_id: String,
) -> Result<()> {
	validate_quota_with_node_id(udb, actor_id, NodeId::new()).await
}

#[cfg(debug_assertions)]
pub(crate) async fn validate_quota_with_node_id(
	udb: Arc<universaldb::Database>,
	actor_id: String,
	node_id: NodeId,
) -> Result<()> {
	let (manual_total, counter_value) = udb
		.run({
			let actor_id = actor_id.clone();
			move |tx| {
				let actor_id = actor_id.clone();

				async move {
					let manual_total =
						if let Some(branch_id) =
							branch::resolve_actor_branch(&tx, &actor_id, Snapshot).await?
						{
							scan_tracked_prefix_bytes(&tx, &keys::branch_pidx_prefix(branch_id)).await?
								+ scan_tracked_prefix_bytes(&tx, &keys::branch_delta_prefix(branch_id)).await?
								+ scan_tracked_prefix_bytes(&tx, &keys::branch_shard_prefix(branch_id)).await?
						} else {
							scan_tracked_prefix_bytes(&tx, &keys::pidx_delta_prefix(&actor_id)).await?
								+ scan_tracked_prefix_bytes(&tx, &keys::delta_prefix(&actor_id)).await?
								+ scan_tracked_prefix_bytes(&tx, &keys::shard_prefix(&actor_id)).await?
						};
					let counter_value = quota::read(&tx, &actor_id).await?;

					Ok((manual_total, counter_value))
				}
			}
		})
		.await?;

	if manual_total != counter_value {
		let node_id = node_id.to_string();
		metrics::SQLITE_QUOTA_VALIDATE_MISMATCH_TOTAL
			.with_label_values(&[node_id.as_str()])
			.inc();
		tracing::error!(
			actor_id = %actor_id,
			manual_total,
			counter_value,
			"sqlite quota validation mismatch"
		);

		#[cfg(test)]
		panic!(
			"sqlite quota validation mismatch for actor {actor_id}: manual_total={manual_total}, counter_value={counter_value}"
		);

		#[cfg(not(test))]
		bail!("sqlite quota validation mismatch for actor {actor_id}");
	}

	Ok(())
}

async fn count_compare_and_clear_noops(
	db: &universaldb::Database,
	actor_id: String,
	attempted_pidx_deletes: Vec<PidxDelete>,
) -> Result<u64> {
	db.run(move |tx| {
		let attempted_pidx_deletes = attempted_pidx_deletes.clone();
		let actor_id = actor_id.clone();

		async move {
			let mut noops = 0u64;
			for delete in attempted_pidx_deletes {
				if let Some(value) = tx_get_value(&tx, &delete.key, Snapshot).await? {
					if value != delete.expected_value {
						noops += 1;
					} else {
						bail!("PIDX compare-and-clear left expected value for actor {actor_id}");
					}
				}
			}
			Ok(noops)
		}
	})
	.await
}

async fn load_pidx_rows(
	tx: &universaldb::Transaction,
	scope: &StorageScope,
	actor_id: &str,
) -> Result<Vec<PidxRow>> {
	tx_scan_prefix_values(tx, &scope.pidx_prefix(actor_id))
		.await?
		.into_iter()
		.map(|(key, value)| {
			Ok(PidxRow {
				pgno: scope.decode_pidx_pgno(actor_id, &key)?,
				txid: decode_pidx_txid(&value)?,
			})
		})
		.collect()
}

async fn load_delta_entries(
	tx: &universaldb::Transaction,
	scope: &StorageScope,
	actor_id: &str,
) -> Result<BTreeMap<u64, DeltaEntry>> {
	let mut chunks_by_txid = BTreeMap::<u64, Vec<DeltaChunk>>::new();
	for (key, value) in tx_scan_prefix_values(tx, &scope.delta_prefix(actor_id)).await? {
		let txid = scope.decode_delta_chunk_txid(actor_id, &key)?;
		let chunk_idx = scope.decode_delta_chunk_idx(actor_id, txid, &key)?;
		chunks_by_txid.entry(txid).or_default().push(DeltaChunk {
			key,
			chunk_idx,
			value,
		});
	}

	let mut entries = BTreeMap::new();
	for (txid, mut chunks) in chunks_by_txid {
		chunks.sort_by_key(|chunk| chunk.chunk_idx);
		let mut blob = Vec::new();
		let mut tracked_size = 0i64;
		for chunk in chunks {
			tracked_size += tracked_entry_size(&chunk.key, &chunk.value)?;
			blob.extend_from_slice(&chunk.value);
		}
		entries.insert(txid, DeltaEntry { blob, tracked_size });
	}

	Ok(entries)
}

async fn tx_get_value(
	tx: &universaldb::Transaction,
	key: &[u8],
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<Option<Vec<u8>>> {
	Ok(tx
		.informal()
		.get(key, isolation_level)
		.await?
		.map(Vec::<u8>::from))
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

#[cfg(debug_assertions)]
async fn scan_tracked_prefix_bytes(
	tx: &universaldb::Transaction,
	prefix: &[u8],
) -> Result<i64> {
	tx_scan_prefix_values(tx, prefix)
		.await?
		.iter()
		.map(|(key, value)| tracked_entry_size(key, value))
		.sum()
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

fn decode_branch_pidx_pgno(branch_id: ActorBranchId, key: &[u8]) -> Result<u32> {
	let prefix = keys::branch_pidx_prefix(branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("branch pidx key did not start with expected prefix")?;
	let bytes: [u8; std::mem::size_of::<u32>()] = suffix
		.try_into()
		.map_err(|_| anyhow::anyhow!("branch pidx key suffix had invalid length"))?;

	Ok(u32::from_be_bytes(bytes))
}

fn decode_pidx_txid(value: &[u8]) -> Result<u64> {
	let bytes: [u8; PIDX_TXID_BYTES] = value
		.try_into()
		.map_err(|_| anyhow::anyhow!("pidx txid had invalid length"))?;

	Ok(u64::from_be_bytes(bytes))
}

fn tracked_entry_size(key: &[u8], value: &[u8]) -> Result<i64> {
	i64::try_from(key.len() + value.len()).context("sqlite tracked entry size exceeded i64")
}

fn prefix_range(prefix: &[u8]) -> (Vec<u8>, Vec<u8>) {
	universaldb::tuple::Subspace::from_bytes(prefix.to_vec()).range()
}

fn ensure_not_cancelled(cancel_token: &CancellationToken) -> Result<()> {
	if cancel_token.is_cancelled() {
		bail!("sqlite compaction cancelled");
	}

	Ok(())
}

#[derive(Debug, Clone, Default)]
struct CompactionPlan {
	scope: StorageScope,
	selected_delta_txids: Vec<u64>,
	selected_delta_entries: BTreeMap<u64, DeltaEntry>,
	pages_by_shard: BTreeMap<u32, Vec<FoldPage>>,
	materialized_txid: u64,
}

#[derive(Debug, Clone, Copy, Default)]
enum StorageScope {
	Branch(ActorBranchId),
	#[default]
	Legacy,
}

impl StorageScope {
	fn meta_head_key(self, actor_id: &str) -> Vec<u8> {
		match self {
			Self::Branch(branch_id) => keys::branch_meta_head_key(branch_id),
			Self::Legacy => keys::meta_head_key(actor_id),
		}
	}

	fn meta_compact_key(self, actor_id: &str) -> Vec<u8> {
		match self {
			Self::Branch(branch_id) => keys::branch_meta_compact_key(branch_id),
			Self::Legacy => keys::meta_compact_key(actor_id),
		}
	}

	fn pidx_key(self, actor_id: &str, pgno: u32) -> Vec<u8> {
		match self {
			Self::Branch(branch_id) => keys::branch_pidx_key(branch_id, pgno),
			Self::Legacy => keys::pidx_delta_key(actor_id, pgno),
		}
	}

	fn pidx_prefix(self, actor_id: &str) -> Vec<u8> {
		match self {
			Self::Branch(branch_id) => keys::branch_pidx_prefix(branch_id),
			Self::Legacy => keys::pidx_delta_prefix(actor_id),
		}
	}

	fn delta_prefix(self, actor_id: &str) -> Vec<u8> {
		match self {
			Self::Branch(branch_id) => keys::branch_delta_prefix(branch_id),
			Self::Legacy => keys::delta_prefix(actor_id),
		}
	}

	fn delta_chunk_prefix(self, actor_id: &str, txid: u64) -> Vec<u8> {
		match self {
			Self::Branch(branch_id) => keys::branch_delta_chunk_prefix(branch_id, txid),
			Self::Legacy => keys::delta_chunk_prefix(actor_id, txid),
		}
	}

	fn decode_pidx_pgno(self, actor_id: &str, key: &[u8]) -> Result<u32> {
		match self {
			Self::Branch(branch_id) => decode_branch_pidx_pgno(branch_id, key),
			Self::Legacy => decode_pidx_pgno(actor_id, key),
		}
	}

	fn decode_delta_chunk_txid(self, actor_id: &str, key: &[u8]) -> Result<u64> {
		match self {
			Self::Branch(branch_id) => keys::decode_branch_delta_chunk_txid(branch_id, key),
			Self::Legacy => keys::decode_delta_chunk_txid(actor_id, key),
		}
	}

	fn decode_delta_chunk_idx(self, actor_id: &str, txid: u64, key: &[u8]) -> Result<u32> {
		match self {
			Self::Branch(branch_id) => keys::decode_branch_delta_chunk_idx(branch_id, txid, key),
			Self::Legacy => keys::decode_delta_chunk_idx(actor_id, txid, key),
		}
	}
}

async fn resolve_storage_scope(
	tx: &universaldb::Transaction,
	actor_id: &str,
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<StorageScope> {
	Ok(
		match branch::resolve_actor_branch(tx, actor_id, isolation_level).await? {
			Some(branch_id) => StorageScope::Branch(branch_id),
			None => StorageScope::Legacy,
		},
	)
}

#[derive(Debug, Clone)]
struct DeltaEntry {
	blob: Vec<u8>,
	tracked_size: i64,
}

#[derive(Debug, Clone)]
struct DeltaChunk {
	key: Vec<u8>,
	chunk_idx: u32,
	value: Vec<u8>,
}

#[derive(Debug, Clone)]
struct PidxRow {
	pgno: u32,
	txid: u64,
}

#[derive(Debug, Clone)]
struct FoldPage {
	pgno: u32,
	expected_txid: u64,
	bytes: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
struct WriteResult {
	pages_folded: u64,
	deltas_freed: u64,
	bytes_freed: i64,
	materialized_txid: u64,
	attempted_pidx_deletes: Vec<PidxDelete>,
}

#[derive(Debug, Clone)]
struct PidxDelete {
	key: Vec<u8>,
	expected_value: Vec<u8>,
}

#[cfg(debug_assertions)]
pub mod test_hooks {
	use std::sync::Arc;

	use parking_lot::Mutex;
	use tokio::sync::Notify;

	static PAUSE_AFTER_PLAN: Mutex<Option<(String, Arc<Notify>, Arc<Notify>)>> = Mutex::new(None);
	static PAUSE_AFTER_WRITE_HEAD_READ: Mutex<Option<(String, Arc<Notify>, Arc<Notify>)>> =
		Mutex::new(None);

	pub struct PauseGuard {
		slot: &'static Mutex<Option<(String, Arc<Notify>, Arc<Notify>)>>,
	}

	pub fn pause_after_plan(actor_id: &str) -> (PauseGuard, Arc<Notify>, Arc<Notify>) {
		pause(&PAUSE_AFTER_PLAN, actor_id)
	}

	pub fn pause_after_write_head_read(actor_id: &str) -> (PauseGuard, Arc<Notify>, Arc<Notify>) {
		pause(&PAUSE_AFTER_WRITE_HEAD_READ, actor_id)
	}

	pub(super) async fn maybe_pause_after_plan(actor_id: &str) {
		maybe_pause(&PAUSE_AFTER_PLAN, actor_id).await;
	}

	pub(super) async fn maybe_pause_after_write_head_read(actor_id: &str) {
		maybe_pause(&PAUSE_AFTER_WRITE_HEAD_READ, actor_id).await;
	}

	fn pause(
		slot: &'static Mutex<Option<(String, Arc<Notify>, Arc<Notify>)>>,
		actor_id: &str,
	) -> (PauseGuard, Arc<Notify>, Arc<Notify>) {
		let reached = Arc::new(Notify::new());
		let release = Arc::new(Notify::new());
		*slot.lock() = Some((actor_id.to_string(), Arc::clone(&reached), Arc::clone(&release)));

		(PauseGuard { slot }, reached, release)
	}

	async fn maybe_pause(
		slot: &'static Mutex<Option<(String, Arc<Notify>, Arc<Notify>)>>,
		actor_id: &str,
	) {
		let hook = slot
			.lock()
			.as_ref()
			.filter(|(hook_actor_id, _, _)| hook_actor_id == actor_id)
			.map(|(_, reached, release)| (Arc::clone(reached), Arc::clone(release)));

		if let Some((reached, release)) = hook {
			reached.notify_waiters();
			release.notified().await;
		}
	}

	impl Drop for PauseGuard {
		fn drop(&mut self) {
			*self.slot.lock() = None;
		}
	}
}

#[cfg(not(debug_assertions))]
mod test_hooks {
	pub(super) async fn maybe_pause_after_plan(_actor_id: &str) {}

	pub(super) async fn maybe_pause_after_write_head_read(_actor_id: &str) {}
}
