//! Single-shot commit path for the stateless sqlite-storage pump.

use std::{collections::BTreeSet, time::Duration};

use anyhow::{Context, Result};
use futures_util::TryStreamExt;
use universaldb::{
	options::MutationType,
	RangeOption,
	options::StreamingMode,
	utils::IsolationLevel::{Serializable, Snapshot},
};

use crate::pump::{
	ActorDb,
	actor_db::{BranchAncestry, load_branch_ancestry},
	branch,
	keys::{self, SHARD_SIZE},
	ltx::{LtxHeader, encode_ltx_v3},
	metrics,
	quota,
	types::{
		ActorBranchId, ActorBranchRecord, ActorPointer, BranchState, CommitRow, DBHead, DirtyPage,
		NamespaceBranchId, NamespaceId, decode_db_head, decode_meta_compact,
		encode_actor_branch_record, encode_actor_pointer, encode_commit_row, encode_db_head,
	},
	udb,
};

const DELTA_CHUNK_BYTES: usize = 10_000;
const INCOMPLETE_COMMIT_VERSIONSTAMP: [u8; 16] = [
	0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0, 0, 0,
];

impl ActorDb {
	pub async fn commit(
		&self,
		dirty_pages: Vec<DirtyPage>,
		db_size_pages: u32,
		now_ms: i64,
	) -> Result<()> {
		let node_id = self.node_id.to_string();
		let labels = &[node_id.as_str()];
		let _timer = metrics::SQLITE_PUMP_COMMIT_DURATION
			.with_label_values(labels)
			.start_timer();
		metrics::SQLITE_PUMP_COMMIT_DIRTY_PAGE_COUNT
			.with_label_values(labels)
			.observe(dirty_pages.len() as f64);

		let cached_storage_used = *self.storage_used.lock();
		let cached_branch_id = *self.branch_id.lock();
		let cached_ancestry = self.ancestors.lock().clone();
		let cache_was_warm = !self.cache.lock().range(0, u32::MAX).is_empty();
		let actor_id = self.actor_id.clone();
		let namespace_id = self.sqlite_namespace_id();
		let dirty_pages_for_tx = dirty_pages.clone();

		let result = self
			.udb
			.run(move |tx| {
				let actor_id = actor_id.clone();
				let namespace_id = namespace_id;
				let dirty_pages = dirty_pages_for_tx.clone();
				let cached_ancestry = cached_ancestry.clone();

				async move {
					let branch_resolution =
						resolve_or_allocate_branch(&tx, namespace_id, &actor_id).await?;
					let branch_id = branch_resolution.branch_id;
					let branch_ancestry = if branch_resolution.actor_initialized {
						BranchAncestry::root(branch_id)
					} else if let Some(cached_ancestry) =
						cached_ancestry.filter(|ancestry| ancestry.root_branch_id == branch_id)
					{
						cached_ancestry
					} else {
						load_branch_ancestry(&tx, branch_id).await?
					};
					let head_key = keys::branch_meta_head_key(branch_id);
					let head_at_fork_key = keys::branch_meta_head_at_fork_key(branch_id);
					let branch_cache_matches = cached_branch_id == Some(branch_id);
					let (head_bytes, head_at_fork_bytes, storage_used) =
						if let (true, Some(storage_used)) = (branch_cache_matches, cached_storage_used) {
							(
								tx_get_value(&tx, &head_key, Serializable).await?,
								tx_get_value(&tx, &head_at_fork_key, Serializable).await?,
								storage_used,
							)
						} else {
							let quota_fut = quota::read_branch(&tx, branch_id);
							let head_fut = tx_get_value(&tx, &head_key, Serializable);
							let head_at_fork_fut = tx_get_value(&tx, &head_at_fork_key, Serializable);
							let (head_bytes, head_at_fork_bytes, storage_used) =
								tokio::try_join!(head_fut, head_at_fork_fut, quota_fut)?;
							(head_bytes, head_at_fork_bytes, storage_used)
						};

					let previous_head_bytes = head_bytes.as_ref().or(head_at_fork_bytes.as_ref());
					let previous_head = previous_head_bytes
						.map(|bytes| decode_db_head(bytes.as_slice()))
						.transpose()
						.context("decode current sqlite db head")?;
					let materialized_txid =
						tx_get_value(&tx, &keys::branch_meta_compact_key(branch_id), Snapshot)
							.await?
							.as_deref()
							.map(decode_meta_compact)
							.transpose()
							.context("decode sqlite compact meta for trigger")?
							.map_or(0, |compact| compact.materialized_txid);
					let previous_db_size_pages =
						previous_head.as_ref().map_or(db_size_pages, |head| head.db_size_pages);
					let txid = match previous_head.as_ref() {
						Some(head) => head
							.head_txid
							.checked_add(1)
							.context("sqlite head txid overflowed")?,
						None => 1,
					};

					let truncate_cleanup =
						collect_truncate_cleanup(&tx, branch_id, previous_db_size_pages, db_size_pages)
							.await?;

					let encoded_delta = encode_ltx_v3(
						LtxHeader::delta(txid, db_size_pages, now_ms),
						&dirty_pages,
					)
					.context("encode commit delta")?;
					let delta_chunks = encoded_delta
						.chunks(DELTA_CHUNK_BYTES)
						.enumerate()
						.map(|(chunk_idx, chunk)| {
							let chunk_idx = u32::try_from(chunk_idx)
								.context("delta chunk index exceeded u32")?;
							Ok((
								keys::branch_delta_chunk_key(branch_id, txid, chunk_idx),
								chunk.to_vec(),
							))
						})
						.collect::<Result<Vec<_>>>()?;

					let new_head = DBHead {
						head_txid: txid,
						db_size_pages,
						post_apply_checksum: previous_head
							.as_ref()
							.map_or(0, |head| head.post_apply_checksum),
						branch_id,
						#[cfg(debug_assertions)]
						generation: previous_head.as_ref().map_or(0, |head| head.generation),
					};
					let encoded_head =
						encode_db_head(new_head.clone()).context("encode new sqlite db head")?;
					let txid_bytes = txid.to_be_bytes();
					let commit_row = CommitRow {
						wall_clock_ms: now_ms,
						versionstamp: INCOMPLETE_COMMIT_VERSIONSTAMP,
						db_size_pages,
						post_apply_checksum: new_head.post_apply_checksum,
					};
					let encoded_commit_row =
						encode_commit_row(commit_row).context("encode sqlite commit row")?;
					let versionstamped_commit_row = udb::append_versionstamp_offset(
						encoded_commit_row.clone(),
						&INCOMPLETE_COMMIT_VERSIONSTAMP,
					)
					.context("prepare versionstamped sqlite commit row")?;
					let commit_key = keys::branch_commit_key(branch_id, txid);
					let vtx_storage_key =
						keys::branch_vtx_key(branch_id, INCOMPLETE_COMMIT_VERSIONSTAMP);
					let versionstamped_vtx_key = udb::append_versionstamp_offset(
						vtx_storage_key.clone(),
						&INCOMPLETE_COMMIT_VERSIONSTAMP,
					)
					.context("prepare versionstamped sqlite vtx key")?;
					let dirty_pgnos = dirty_pages
						.iter()
						.map(|page| page.pgno)
						.collect::<BTreeSet<_>>();

					let added_bytes = tracked_entry_size(&head_key, &encoded_head)?
						+ tracked_entry_size(&commit_key, &encoded_commit_row)?
						+ tracked_entry_size(&vtx_storage_key, &txid_bytes)?
						+ delta_chunks
							.iter()
							.map(|(key, value)| tracked_entry_size(key, value))
							.sum::<Result<i64>>()?
						+ dirty_pgnos
							.iter()
							.map(|pgno| {
								tracked_entry_size(&keys::branch_pidx_key(branch_id, *pgno), &txid_bytes)
							})
							.sum::<Result<i64>>()?;
					let removed_bytes = head_bytes
						.as_ref()
						.map_or(Ok(0), |bytes| tracked_entry_size(&head_key, bytes))?
						+ truncate_cleanup.deleted_bytes;
					let quota_delta = added_bytes
						.checked_sub(removed_bytes)
						.context("sqlite commit quota delta overflowed i64")?;
					let would_be = storage_used
						.checked_add(quota_delta)
						.context("sqlite commit quota check overflowed i64")?;

					quota::cap_check(would_be)?;

					for (key, value) in &delta_chunks {
						tx.informal().set(key, value);
					}
					for pgno in &dirty_pgnos {
						tx.informal()
							.set(&keys::branch_pidx_key(branch_id, *pgno), &txid_bytes);
					}
					for key in &truncate_cleanup.pidx_keys {
						tx.informal().clear(key);
					}
					for key in &truncate_cleanup.shard_keys {
						tx.informal().clear(key);
					}
					tx.informal().set(&head_key, &encoded_head);
					if head_at_fork_bytes.is_some() {
						tx.informal().clear(&head_at_fork_key);
					}
					if branch_resolution.namespace_initialized {
						branch::write_root_namespace_metadata(
							&tx,
							namespace_id,
							branch_resolution.namespace_branch_id,
							now_ms,
							&INCOMPLETE_COMMIT_VERSIONSTAMP,
						)?;
					}
					if branch_resolution.actor_initialized {
						write_root_branch_metadata(
							&tx,
							branch_id,
							branch_resolution.namespace_branch_id,
							&actor_id,
							now_ms,
							&INCOMPLETE_COMMIT_VERSIONSTAMP,
						)?;
					}
					tx.informal().atomic_op(
						&commit_key,
						&versionstamped_commit_row,
						MutationType::SetVersionstampedValue,
					);
					tx.informal().atomic_op(
						&versionstamped_vtx_key,
						&txid_bytes,
						MutationType::SetVersionstampedKey,
					);
					if quota_delta != 0 {
						quota::atomic_add_branch(&tx, branch_id, quota_delta);
					}

					Ok(CommitTxResult {
						branch_id,
						branch_ancestry,
						txid,
						materialized_txid,
						dirty_pgnos,
						truncated_pgnos: truncate_cleanup.truncated_pgnos,
						added_bytes,
						storage_used: would_be,
					})
				}
			})
			.await?;

		*self.storage_used.lock() = Some(result.storage_used);
		*self.branch_id.lock() = Some(result.branch_id);
		*self.ancestors.lock() = Some(result.branch_ancestry.clone());
		*self.commit_bytes_since_rollup.lock() += u64::try_from(result.added_bytes)
			.context("commit added bytes should be non-negative")?;

		let branch_changed = cached_branch_id.is_some_and(|branch_id| branch_id != result.branch_id);
		if branch_changed {
			self.cache.lock().clear();
		}
		if cache_was_warm || branch_changed {
			let cache = self.cache.lock();
			for pgno in result.truncated_pgnos {
				cache.remove(pgno);
			}
			for pgno in result.dirty_pgnos {
				cache.insert(pgno, result.txid);
			}
		}

		self.publish_compact_trigger_if_needed(result.txid, result.materialized_txid);

		Ok(())
	}

	fn publish_compact_trigger_if_needed(&self, head_txid: u64, materialized_txid: u64) {
		let Some(delta_count) = head_txid.checked_sub(materialized_txid) else {
			return;
		};
		if delta_count < quota::COMPACTION_DELTA_THRESHOLD {
			return;
		}

		let now = tokio::time::Instant::now();
		let should_publish = {
			let mut last_trigger_at = self.last_trigger_at.lock();
			let should_publish = last_trigger_at.is_none_or(|last| {
				now.duration_since(last) >= Duration::from_millis(quota::TRIGGER_THROTTLE_MS)
					|| now.duration_since(last)
						> Duration::from_millis(quota::TRIGGER_MAX_SILENCE_MS)
			});
			if should_publish {
				*last_trigger_at = Some(now);
			}
			should_publish
		};

		if should_publish {
			let (commit_bytes_since_rollup, read_bytes_since_rollup) =
				self.take_metering_snapshot();
			crate::compactor::publish_compact_payload_with_node_id(
				&self.ups,
				crate::compactor::SqliteCompactPayload {
					actor_id: self.actor_id.clone(),
					namespace_id: Some(self.namespace_id),
					actor_name: None,
					commit_bytes_since_rollup,
					read_bytes_since_rollup,
				},
				self.node_id,
			);
		}
	}
}

struct CommitTxResult {
	branch_id: ActorBranchId,
	branch_ancestry: BranchAncestry,
	txid: u64,
	materialized_txid: u64,
	dirty_pgnos: BTreeSet<u32>,
	truncated_pgnos: Vec<u32>,
	added_bytes: i64,
	storage_used: i64,
}

#[derive(Default)]
struct TruncateCleanup {
	pidx_keys: Vec<Vec<u8>>,
	shard_keys: Vec<Vec<u8>>,
	truncated_pgnos: Vec<u32>,
	deleted_bytes: i64,
}

async fn collect_truncate_cleanup(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
	previous_db_size_pages: u32,
	new_db_size_pages: u32,
) -> Result<TruncateCleanup> {
	if new_db_size_pages >= previous_db_size_pages {
		return Ok(TruncateCleanup::default());
	}

	let mut cleanup = TruncateCleanup::default();
	for (key, value) in tx_scan_prefix_values(tx, &keys::branch_pidx_prefix(branch_id)).await? {
		let pgno = decode_branch_pidx_pgno(branch_id, &key)?;
		if pgno > new_db_size_pages {
			cleanup.deleted_bytes += tracked_entry_size(&key, &value)?;
			cleanup.truncated_pgnos.push(pgno);
			cleanup.pidx_keys.push(key);
		}
	}

	for (key, value) in tx_scan_prefix_values(tx, &keys::branch_shard_prefix(branch_id)).await? {
		let shard_id = decode_branch_shard_id(branch_id, &key)?;
		if shard_id.saturating_mul(SHARD_SIZE) > new_db_size_pages {
			cleanup.deleted_bytes += tracked_entry_size(&key, &value)?;
			cleanup.shard_keys.push(key);
		}
	}

	Ok(cleanup)
}

struct BranchResolution {
	branch_id: ActorBranchId,
	namespace_branch_id: NamespaceBranchId,
	namespace_initialized: bool,
	actor_initialized: bool,
}

async fn resolve_or_allocate_branch(
	tx: &universaldb::Transaction,
	namespace_id: NamespaceId,
	actor_id: &str,
) -> Result<BranchResolution> {
	let namespace = branch::resolve_or_allocate_root_namespace_branch(tx, namespace_id).await?;

	if let Some(branch_id) =
		branch::resolve_actor_branch_in_namespace(tx, namespace.branch_id, actor_id, Serializable)
			.await?
	{
		return Ok(BranchResolution {
			branch_id,
			namespace_branch_id: namespace.branch_id,
			namespace_initialized: namespace.initialized,
			actor_initialized: false,
		});
	}

	Ok(BranchResolution {
		branch_id: ActorBranchId::new_v4(),
		namespace_branch_id: namespace.branch_id,
		namespace_initialized: namespace.initialized,
		actor_initialized: true,
	})
}

fn write_root_branch_metadata(
	tx: &universaldb::Transaction,
	branch_id: ActorBranchId,
	namespace_branch: NamespaceBranchId,
	actor_id: &str,
	now_ms: i64,
	root_versionstamp: &[u8; 16],
) -> Result<()> {
	let record = ActorBranchRecord {
		branch_id,
		namespace_branch,
		parent: None,
		parent_versionstamp: None,
		root_versionstamp: *root_versionstamp,
		fork_depth: 0,
		created_at_ms: now_ms,
		created_from_bookmark: None,
		state: BranchState::Live,
	};
	let encoded_record =
		encode_actor_branch_record(record).context("encode sqlite root actor branch record")?;
	let versionstamped_record = udb::append_versionstamp_offset(encoded_record, root_versionstamp)
		.context("prepare versionstamped sqlite root actor branch record")?;
	tx.informal().atomic_op(
		&keys::branches_list_key(branch_id),
		&versionstamped_record,
		MutationType::SetVersionstampedValue,
	);
	tx.informal().atomic_op(
		&keys::branches_refcount_key(branch_id),
		&1_i64.to_le_bytes(),
		MutationType::Add,
	);

	let pointer = ActorPointer {
		current_branch: branch_id,
		last_swapped_at_ms: now_ms,
	};
	let encoded_pointer = encode_actor_pointer(pointer).context("encode sqlite actor pointer")?;
	tx.informal().set(
		&keys::actor_pointer_cur_key(namespace_branch, actor_id),
		&encoded_pointer,
	);

	Ok(())
}

fn tracked_entry_size(key: &[u8], value: &[u8]) -> Result<i64> {
	i64::try_from(key.len() + value.len()).context("sqlite tracked entry size exceeded i64")
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
		universaldb::RangeOption {
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

fn decode_branch_pidx_pgno(branch_id: ActorBranchId, key: &[u8]) -> Result<u32> {
	let prefix = keys::branch_pidx_prefix(branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("pidx key did not start with expected prefix")?;
	let bytes: [u8; std::mem::size_of::<u32>()] = suffix
		.try_into()
		.map_err(|_| anyhow::anyhow!("pidx key suffix had invalid length"))?;

	Ok(u32::from_be_bytes(bytes))
}

fn decode_branch_shard_id(branch_id: ActorBranchId, key: &[u8]) -> Result<u32> {
	let prefix = keys::branch_shard_prefix(branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("shard key did not start with expected prefix")?;
	let bytes: [u8; std::mem::size_of::<u32>()] = suffix
		.get(..std::mem::size_of::<u32>())
		.context("shard key suffix had invalid length")?
		.try_into()
		.map_err(|_| anyhow::anyhow!("shard key suffix had invalid length"))?;
	if suffix.len() != std::mem::size_of::<u32>()
		&& (suffix.len() != std::mem::size_of::<u32>() + 1 + std::mem::size_of::<u64>()
			|| suffix[std::mem::size_of::<u32>()] != b'/')
	{
		anyhow::bail!("shard key suffix had invalid length");
	}

	Ok(u32::from_be_bytes(bytes))
}
