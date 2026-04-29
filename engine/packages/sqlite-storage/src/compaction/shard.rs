//! Shard compaction pass that folds live DELTA pages into immutable SHARD blobs.

use std::collections::{BTreeMap, BTreeSet};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, ensure};
use scc::hash_map::Entry;

use crate::engine::SqliteEngine;
use crate::keys::{
	decode_delta_chunk_txid, delta_chunk_prefix, delta_prefix, meta_key, pidx_delta_prefix,
	shard_key,
};
use crate::ltx::{LtxHeader, decode_ltx_v3, encode_ltx_v3};
use crate::quota::{encode_db_head_with_usage, tracked_storage_entry_size};
use crate::types::{DBHead, DirtyPage, SQLITE_PAGE_SIZE, decode_db_head};
use crate::udb::{self, WriteOp};

const PIDX_PGNO_BYTES: usize = std::mem::size_of::<u32>();
const PIDX_TXID_BYTES: usize = std::mem::size_of::<u64>();

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PidxRow {
	pub key: Vec<u8>,
	pub pgno: u32,
	pub txid: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DeltaEntry {
	pub key_prefix: Vec<u8>,
	pub chunk_keys: Vec<Vec<u8>>,
	pub blob: Vec<u8>,
	pub tracked_size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(super) struct ShardCompactionOutcome {
	pub consumed_pidx_pgnos: BTreeSet<u32>,
	pub deleted_delta_txids: BTreeSet<u64>,
}

#[cfg(test)]
mod test_hooks {
	use std::sync::{Arc, Mutex};

	use tokio::sync::Notify;

	static PAUSE_BEFORE_COMMIT: Mutex<Option<(String, Arc<Notify>, Arc<Notify>)>> =
		Mutex::new(None);

	pub(super) struct PauseBeforeCommitGuard;

	pub(super) fn pause_before_commit(
		actor_id: &str,
	) -> (PauseBeforeCommitGuard, Arc<Notify>, Arc<Notify>) {
		let reached = Arc::new(Notify::new());
		let release = Arc::new(Notify::new());
		*PAUSE_BEFORE_COMMIT
			.lock()
			.expect("compaction pause hook mutex should lock") = Some((
			actor_id.to_string(),
			Arc::clone(&reached),
			Arc::clone(&release),
		));

		(PauseBeforeCommitGuard, reached, release)
	}

	pub(super) async fn maybe_pause_before_commit(actor_id: &str) {
		let hook = PAUSE_BEFORE_COMMIT
			.lock()
			.expect("compaction pause hook mutex should lock")
			.as_ref()
			.filter(|(hook_actor_id, _, _)| hook_actor_id == actor_id)
			.map(|(_, reached, release)| (Arc::clone(reached), Arc::clone(release)));

		if let Some((reached, release)) = hook {
			reached.notify_waiters();
			release.notified().await;
		}
	}

	impl Drop for PauseBeforeCommitGuard {
		fn drop(&mut self) {
			*PAUSE_BEFORE_COMMIT
				.lock()
				.expect("compaction pause hook mutex should lock") = None;
		}
	}
}

impl SqliteEngine {
	pub async fn compact_shard(&self, actor_id: &str, shard_id: u32) -> Result<bool> {
		let actor_lock = self.actor_op_lock(actor_id).await;
		let _actor_guard = actor_lock.lock().await;
		let meta_bytes = udb::get_value(
			&self.db,
			&self.subspace,
			self.op_counter.as_ref(),
			meta_key(actor_id),
		)
		.await?
		.context("sqlite meta missing for shard compaction")?;
		let head = decode_db_head(&meta_bytes)?;
		let all_pidx_rows = load_pidx_rows(self, actor_id).await?;
		let delta_entries = load_delta_entries(self, actor_id).await?;

		Ok(self
			.compact_shard_preloaded(actor_id, shard_id, &head, &all_pidx_rows, &delta_entries)
			.await?
			.is_some())
	}

	pub(super) async fn compact_shard_preloaded(
		&self,
		actor_id: &str,
		shard_id: u32,
		head: &DBHead,
		all_pidx_rows: &[PidxRow],
		delta_entries: &BTreeMap<u64, DeltaEntry>,
	) -> Result<Option<ShardCompactionOutcome>> {
		let initial_generation = head.generation;
		let initial_head_txid = head.head_txid;

		let shard_start_pgno = shard_id * head.shard_size;
		let shard_end_pgno = shard_start_pgno + head.shard_size.saturating_sub(1);

		let shard_rows = all_pidx_rows
			.iter()
			.filter(|row| {
				row.pgno >= shard_start_pgno
					&& row.pgno <= shard_end_pgno
					&& row.pgno <= head.db_size_pages
			})
			.cloned()
			.collect::<Vec<_>>();
		if shard_rows.is_empty() {
			return Ok(None);
		}

		let _shard_txids = shard_rows
			.iter()
			.map(|row| row.txid)
			.collect::<BTreeSet<_>>();
		let shard_blob_key = shard_key(actor_id, shard_id);
		let shard_blob = udb::get_value(
			&self.db,
			&self.subspace,
			self.op_counter.as_ref(),
			shard_blob_key.clone(),
		)
		.await?;
		let mut blobs = BTreeMap::new();
		blobs.insert(shard_blob_key.clone(), shard_blob);
		for entry in delta_entries.values() {
			blobs.insert(entry.key_prefix.clone(), Some(entry.blob.clone()));
		}
		let delta_keys = delta_entries
			.iter()
			.map(|(txid, entry)| (*txid, entry.key_prefix.clone()))
			.collect::<BTreeMap<_, _>>();
		let merged_pages = merge_shard_pages(
			head,
			shard_start_pgno,
			shard_end_pgno,
			&shard_blob_key,
			&blobs,
			&shard_rows,
			&delta_keys,
		)?;
		ensure!(
			!merged_pages.is_empty(),
			"shard {} compaction produced no pages",
			shard_id
		);

		let mut total_refs_by_txid = BTreeMap::<u64, usize>::new();
		for row in all_pidx_rows {
			*total_refs_by_txid.entry(row.txid).or_default() += 1;
		}
		let mut consumed_refs_by_txid = BTreeMap::<u64, usize>::new();
		for row in &shard_rows {
			*consumed_refs_by_txid.entry(row.txid).or_default() += 1;
		}

		let deleted_delta_txids = delta_keys
			.keys()
			.filter(|txid| {
				let total = total_refs_by_txid.get(txid).copied().unwrap_or(0);
				let consumed = consumed_refs_by_txid.get(txid).copied().unwrap_or(0);
				total <= consumed
			})
			.copied()
			.collect::<BTreeSet<_>>();
		let now_ms = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
			.unwrap_or_default();
		let compaction_lags = deleted_delta_txids
			.iter()
			.filter_map(|txid| delta_entries.get(txid))
			.filter_map(|entry| decode_ltx_v3(&entry.blob).ok())
			.filter_map(|decoded| {
				let lag_ms = now_ms.checked_sub(decoded.header.timestamp_ms)?;
				Some(lag_ms as f64 / 1000.0)
			})
			.collect::<Vec<_>>();
		let remaining_delta_txids = delta_entries.keys().copied().collect::<Vec<_>>();

		let shard_commit_txid = shard_rows
			.iter()
			.map(|row| row.txid)
			.max()
			.expect("non-empty shard rows should have a max txid");
		let shard_blob = encode_ltx_v3(
			LtxHeader::delta(shard_commit_txid, head.db_size_pages, head.creation_ts_ms),
			&merged_pages,
		)
		.context("encode compacted shard blob")?;
		let existing_shard_size = blobs
			.get(&shard_blob_key)
			.and_then(|existing_shard| existing_shard.as_ref())
			.map(|existing_shard| {
				tracked_storage_entry_size(&shard_blob_key, existing_shard)
					.expect("shard key should count toward sqlite quota")
			})
			.unwrap_or(0);
		let compacted_pidx_size = shard_rows
			.iter()
			.map(|row| {
				tracked_storage_entry_size(&row.key, &row.txid.to_be_bytes())
					.expect("pidx key should count toward sqlite quota")
			})
			.sum::<u64>();
		let deleted_delta_size = deleted_delta_txids
			.iter()
			.filter_map(|txid| delta_entries.get(txid))
			.map(|entry| entry.tracked_size)
			.sum::<u64>();
		let new_shard_size = tracked_storage_entry_size(&shard_blob_key, &shard_blob)
			.expect("shard key should count toward sqlite quota");

		let mut mutations = Vec::with_capacity(1 + shard_rows.len() + deleted_delta_txids.len());
		mutations.push(WriteOp::put(shard_blob_key.clone(), shard_blob));
		for row in &shard_rows {
			mutations.push(WriteOp::delete(row.key.clone()));
		}
		for txid in &deleted_delta_txids {
			if let Some(entry) = delta_entries.get(txid) {
				for chunk_key in &entry.chunk_keys {
					mutations.push(WriteOp::delete(chunk_key.clone()));
				}
			}
		}
		#[cfg(test)]
		test_hooks::maybe_pause_before_commit(actor_id).await;

		let actor_id_for_tx = actor_id.to_string();
		let meta_key_for_tx = meta_key(actor_id);
		let deleted_delta_txids_for_tx = deleted_delta_txids.clone();
		let updated_head = udb::run_db_op(&self.db, self.op_counter.as_ref(), move |tx| {
			let actor_id = actor_id_for_tx.clone();
			let subspace = self.subspace.clone();
			let meta_key = meta_key_for_tx.clone();
			let mutations = mutations.clone();
			let deleted_delta_txids = deleted_delta_txids_for_tx.clone();
			let remaining_delta_txids = remaining_delta_txids.clone();
			async move {
				let current_meta = udb::tx_get_value_serializable(&tx, &subspace, &meta_key)
					.await?
					.context("sqlite meta missing for shard compaction write")?;
				let current_head = decode_db_head(&current_meta)?;
				if current_head.generation != initial_generation
					|| current_head.head_txid != initial_head_txid
				{
					tracing::debug!(
						%actor_id,
						initial_generation,
						initial_head_txid,
						current_generation = current_head.generation,
						current_head_txid = current_head.head_txid,
						"sqlite compaction skipped after concurrent meta change"
					);
					return Ok(None);
				}

				let current_meta_size = tracked_storage_entry_size(&meta_key, &current_meta)
					.expect("meta key should count toward sqlite quota");
				let usage_without_meta = current_head
					.sqlite_storage_used
					.saturating_sub(current_meta_size)
					.saturating_sub(existing_shard_size)
					.saturating_sub(compacted_pidx_size)
					.saturating_sub(deleted_delta_size)
					.saturating_add(new_shard_size);
				let updated_head = DBHead {
					materialized_txid: compute_materialized_txid(
						&current_head,
						remaining_delta_txids.iter().copied(),
						&deleted_delta_txids,
					),
					..current_head
				};
				let (updated_head, encoded_head) =
					encode_db_head_with_usage(&actor_id, &updated_head, usage_without_meta)?;
				let mut mutations = mutations.clone();
				mutations.push(WriteOp::put(meta_key.clone(), encoded_head));

				for op in &mutations {
					match op {
						WriteOp::Put(key, value) => {
							udb::tx_write_value(&tx, &subspace, key, value)?
						}
						WriteOp::Delete(key) => udb::tx_delete_value(&tx, &subspace, key),
					}
				}
				#[cfg(test)]
				crate::udb::test_hooks::maybe_fail_apply_write_ops(&mutations)?;

				Ok(Some(updated_head))
			}
		})
		.await?;
		let Some(updated_head) = updated_head else {
			return Ok(None);
		};

		self.metrics.add_compaction_pages_folded(shard_rows.len());
		self.metrics
			.add_compaction_deltas_deleted(deleted_delta_txids.len());
		self.metrics.set_delta_count_from_head(&updated_head);
		for lag_seconds in compaction_lags {
			self.metrics.observe_compaction_lag_seconds(lag_seconds);
		}

		let consumed_pidx_pgnos: BTreeSet<u32> = shard_rows.iter().map(|row| row.pgno).collect();
		match self.page_indices.entry_async(actor_id.to_string()).await {
			Entry::Occupied(entry) => {
				for pgno in &consumed_pidx_pgnos {
					entry.get().remove(*pgno);
				}
			}
			Entry::Vacant(entry) => {
				drop(entry);
			}
		}

		Ok(Some(ShardCompactionOutcome {
			consumed_pidx_pgnos,
			deleted_delta_txids,
		}))
	}
}

pub(super) async fn load_pidx_rows(engine: &SqliteEngine, actor_id: &str) -> Result<Vec<PidxRow>> {
	udb::scan_prefix_values(
		&engine.db,
		&engine.subspace,
		engine.op_counter.as_ref(),
		pidx_delta_prefix(actor_id),
	)
	.await?
	.into_iter()
	.map(|(key, value)| {
		let pgno = decode_pidx_pgno(actor_id, &key)?;
		let txid = decode_pidx_txid(&value)?;
		Ok(PidxRow { key, pgno, txid })
	})
	.collect()
}

pub(super) async fn load_delta_entries(
	engine: &SqliteEngine,
	actor_id: &str,
) -> Result<BTreeMap<u64, DeltaEntry>> {
	udb::scan_prefix_values(
		&engine.db,
		&engine.subspace,
		engine.op_counter.as_ref(),
		delta_prefix(actor_id),
	)
	.await?
	.into_iter()
	.try_fold(
		BTreeMap::<u64, DeltaEntry>::new(),
		|mut entries, (key, value)| {
			let txid = decode_delta_chunk_txid(actor_id, &key)?;
			let entry = entries.entry(txid).or_insert_with(|| DeltaEntry {
				key_prefix: delta_chunk_prefix(actor_id, txid),
				chunk_keys: Vec::new(),
				blob: Vec::new(),
				tracked_size: 0,
			});
			entry.tracked_size += tracked_storage_entry_size(&key, &value)
				.expect("delta chunk key should count toward sqlite quota");
			entry.chunk_keys.push(key);
			entry.blob.extend_from_slice(&value);

			Ok::<BTreeMap<u64, DeltaEntry>, anyhow::Error>(entries)
		},
	)
}

fn merge_shard_pages(
	head: &DBHead,
	shard_start_pgno: u32,
	shard_end_pgno: u32,
	shard_blob_key: &[u8],
	blobs: &BTreeMap<Vec<u8>, Option<Vec<u8>>>,
	shard_rows: &[PidxRow],
	delta_keys: &BTreeMap<u64, Vec<u8>>,
) -> Result<Vec<DirtyPage>> {
	let mut merged_pages = BTreeMap::<u32, (u64, Vec<u8>)>::new();

	if let Some(shard_blob) = blobs.get(shard_blob_key).cloned().flatten() {
		let decoded = decode_ltx_v3(&shard_blob).context("decode existing shard blob")?;
		for page in decoded.pages {
			if page.pgno >= shard_start_pgno
				&& page.pgno <= shard_end_pgno
				&& page.pgno <= head.db_size_pages
			{
				merged_pages.insert(page.pgno, (head.materialized_txid, page.bytes));
			}
		}
	}

	let shard_txids = shard_rows
		.iter()
		.map(|row| row.txid)
		.collect::<BTreeSet<_>>();
	for txid in shard_txids {
		let delta_key = delta_keys
			.get(&txid)
			.with_context(|| format!("missing delta key for txid {txid}"))?;
		let delta_blob = blobs
			.get(delta_key)
			.cloned()
			.flatten()
			.with_context(|| format!("missing delta blob for txid {txid}"))?;
		let decoded =
			decode_ltx_v3(&delta_blob).with_context(|| format!("decode delta blob {txid}"))?;
		for page in decoded.pages {
			ensure!(
				page.bytes.len() == SQLITE_PAGE_SIZE as usize,
				"page {} had {} bytes, expected {}",
				page.pgno,
				page.bytes.len(),
				SQLITE_PAGE_SIZE
			);
			if page.pgno >= shard_start_pgno
				&& page.pgno <= shard_end_pgno
				&& page.pgno <= head.db_size_pages
			{
				merged_pages.insert(page.pgno, (txid, page.bytes));
			}
		}
	}

	Ok(merged_pages
		.into_iter()
		.map(|(pgno, (_, bytes))| DirtyPage { pgno, bytes })
		.collect())
}

fn compute_materialized_txid(
	head: &DBHead,
	remaining_delta_txids: impl IntoIterator<Item = u64>,
	deleted_delta_txids: &BTreeSet<u64>,
) -> u64 {
	let next_live_txid = remaining_delta_txids
		.into_iter()
		.filter(|txid| *txid > head.materialized_txid && !deleted_delta_txids.contains(txid))
		.min();

	match next_live_txid {
		Some(txid) => txid.saturating_sub(1).max(head.materialized_txid),
		None => head.head_txid,
	}
}

fn decode_pidx_pgno(actor_id: &str, key: &[u8]) -> Result<u32> {
	let prefix = pidx_delta_prefix(actor_id);
	ensure!(
		key.starts_with(&prefix),
		"pidx key did not start with expected prefix"
	);

	let suffix = &key[prefix.len()..];
	ensure!(
		suffix.len() == PIDX_PGNO_BYTES,
		"pidx key suffix had {} bytes, expected {}",
		suffix.len(),
		PIDX_PGNO_BYTES
	);

	Ok(u32::from_be_bytes(
		suffix
			.try_into()
			.context("pidx key suffix should decode as u32")?,
	))
}

fn decode_pidx_txid(value: &[u8]) -> Result<u64> {
	ensure!(
		value.len() == PIDX_TXID_BYTES,
		"pidx value had {} bytes, expected {}",
		value.len(),
		PIDX_TXID_BYTES
	);

	Ok(u64::from_be_bytes(
		value
			.try_into()
			.context("pidx value should decode as u64")?,
	))
}

#[cfg(test)]
mod tests {
	use anyhow::Result;

	use super::decode_db_head;
	use crate::commit::CommitRequest;
	use crate::engine::SqliteEngine;
	use crate::keys::{delta_chunk_key, meta_key, pidx_delta_key, pidx_delta_prefix, shard_key};
	use crate::ltx::{LtxHeader, decode_ltx_v3, encode_ltx_v3};
	use crate::open::OpenConfig;
	use crate::quota::{encode_db_head_with_usage, tracked_storage_entry_size};
	use crate::test_utils::{read_value, scan_prefix_values, test_db};
	use crate::types::{
		DBHead, DirtyPage, FetchedPage, SQLITE_DEFAULT_MAX_STORAGE_BYTES, SQLITE_PAGE_SIZE,
		SQLITE_SHARD_SIZE, SQLITE_VFS_V2_SCHEMA_VERSION, SqliteOrigin, encode_db_head,
	};
	use crate::udb::{WriteOp, apply_write_ops, test_hooks};

	const TEST_ACTOR: &str = "test-actor";

	fn seeded_head() -> DBHead {
		DBHead {
			schema_version: SQLITE_VFS_V2_SCHEMA_VERSION,
			generation: 4,
			head_txid: 5,
			next_txid: 6,
			materialized_txid: 0,
			db_size_pages: 129,
			page_size: SQLITE_PAGE_SIZE,
			shard_size: SQLITE_SHARD_SIZE,
			creation_ts_ms: 123,
			sqlite_storage_used: 0,
			sqlite_max_storage: SQLITE_DEFAULT_MAX_STORAGE_BYTES,
			origin: SqliteOrigin::CreatedOnV2,
		}
	}

	fn page(fill: u8) -> Vec<u8> {
		vec![fill; SQLITE_PAGE_SIZE as usize]
	}

	fn noisy_page(seed: u32) -> Vec<u8> {
		let mut state = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
		(0..SQLITE_PAGE_SIZE)
			.map(|_| {
				state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
				(state >> 24) as u8
			})
			.collect()
	}

	fn delta_blob_key(actor_id: &str, txid: u64) -> Vec<u8> {
		delta_chunk_key(actor_id, txid, 0)
	}

	fn commit_request(generation: u64, head_txid: u64, pages: &[(u32, u8)]) -> CommitRequest {
		CommitRequest {
			generation,
			head_txid,
			db_size_pages: pages.iter().map(|(pgno, _)| *pgno).max().unwrap_or(0),
			dirty_pages: pages
				.iter()
				.map(|(pgno, fill)| DirtyPage {
					pgno: *pgno,
					bytes: page(*fill),
				})
				.collect(),
			now_ms: 1_234,
		}
	}

	async fn actual_tracked_usage(engine: &SqliteEngine) -> Result<u64> {
		Ok(scan_prefix_values(engine, vec![0x02])
			.await?
			.into_iter()
			.filter_map(|(key, value)| tracked_storage_entry_size(&key, &value))
			.sum())
	}

	async fn rewrite_meta_with_actual_usage(engine: &SqliteEngine) -> Result<DBHead> {
		let head = decode_db_head(
			&read_value(engine, meta_key(TEST_ACTOR))
				.await?
				.expect("meta should exist before rewrite"),
		)?;
		let usage_without_meta = actual_tracked_usage(engine).await?.saturating_sub(
			tracked_storage_entry_size(
				&meta_key(TEST_ACTOR),
				&read_value(engine, meta_key(TEST_ACTOR))
					.await?
					.expect("meta should exist before rewrite"),
			)
			.expect("meta key should count toward sqlite quota"),
		);
		let (head, meta_bytes) = encode_db_head_with_usage(TEST_ACTOR, &head, usage_without_meta)?;
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![WriteOp::put(meta_key(TEST_ACTOR), meta_bytes)],
		)
		.await?;
		Ok(head)
	}

	fn encoded_blob(txid: u64, commit: u32, pages: &[(u32, u8)]) -> Vec<u8> {
		let pages = pages
			.iter()
			.map(|(pgno, fill)| DirtyPage {
				pgno: *pgno,
				bytes: page(*fill),
			})
			.collect::<Vec<_>>();
		encode_ltx_v3(LtxHeader::delta(txid, commit, 999), &pages).expect("encode test blob")
	}

	fn encoded_noisy_blob(txid: u64, commit: u32, pgnos: impl IntoIterator<Item = u32>) -> Vec<u8> {
		let pages = pgnos
			.into_iter()
			.map(|pgno| DirtyPage {
				pgno,
				bytes: noisy_page(pgno),
			})
			.collect::<Vec<_>>();
		encode_ltx_v3(LtxHeader::delta(txid, commit, 999), &pages).expect("encode test blob")
	}

	#[tokio::test]
	async fn compact_worker_folds_five_deltas_into_one_shard() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.db_size_pages = 5;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 1),
					encoded_blob(1, 5, &[(1, 0x11)]),
				),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 2),
					encoded_blob(2, 5, &[(2, 0x22)]),
				),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 3),
					encoded_blob(3, 5, &[(3, 0x33)]),
				),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 4),
					encoded_blob(4, 5, &[(4, 0x44)]),
				),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 5),
					encoded_blob(5, 5, &[(5, 0x55)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 1_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 2_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 3), 3_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 4), 4_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 5), 5_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;
		let _ = engine.get_or_load_pidx(TEST_ACTOR).await?;

		assert_eq!(engine.compact_worker(TEST_ACTOR, 8).await?, 1);
		assert!(
			read_value(&engine, delta_blob_key(TEST_ACTOR, 1))
				.await?
				.is_none()
		);
		assert!(
			read_value(&engine, delta_blob_key(TEST_ACTOR, 5))
				.await?
				.is_none()
		);
		assert!(
			scan_prefix_values(&engine, pidx_delta_prefix(TEST_ACTOR))
				.await?
				.is_empty()
		);

		let stored_head = decode_db_head(
			&read_value(&engine, meta_key(TEST_ACTOR))
				.await?
				.expect("meta should exist after compaction"),
		)?;
		assert_eq!(stored_head.materialized_txid, 5);
		let pages = engine.get_pages(TEST_ACTOR, 4, vec![1, 2, 3, 4, 5]).await?;
		assert_eq!(
			pages,
			vec![
				FetchedPage {
					pgno: 1,
					bytes: Some(page(0x11)),
				},
				FetchedPage {
					pgno: 2,
					bytes: Some(page(0x22)),
				},
				FetchedPage {
					pgno: 3,
					bytes: Some(page(0x33)),
				},
				FetchedPage {
					pgno: 4,
					bytes: Some(page(0x44)),
				},
				FetchedPage {
					pgno: 5,
					bytes: Some(page(0x55)),
				},
			]
		);

		Ok(())
	}

	#[tokio::test]
	async fn compact_worker_prefers_latest_delta_over_old_shard_pages() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 2;
		head.next_txid = 3;
		head.db_size_pages = 2;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					shard_key(TEST_ACTOR, 0),
					encoded_blob(0.max(1), 2, &[(1, 0x10), (2, 0x20)]),
				),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 1),
					encoded_blob(1, 2, &[(1, 0x11)]),
				),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 2),
					encoded_blob(2, 2, &[(1, 0x22), (2, 0x33)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 2_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 2_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;
		assert_eq!(engine.compact_worker(TEST_ACTOR, 8).await?, 1);
		assert!(
			read_value(&engine, delta_blob_key(TEST_ACTOR, 1))
				.await?
				.is_none()
		);
		assert!(
			read_value(&engine, delta_blob_key(TEST_ACTOR, 2))
				.await?
				.is_none()
		);

		let pages = engine.get_pages(TEST_ACTOR, 4, vec![1, 2]).await?;
		assert_eq!(
			pages,
			vec![
				FetchedPage {
					pgno: 1,
					bytes: Some(page(0x22)),
				},
				FetchedPage {
					pgno: 2,
					bytes: Some(page(0x33)),
				},
			]
		);

		Ok(())
	}

	#[tokio::test]
	async fn compact_shard_reads_existing_chunked_shard_blob() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 2;
		head.next_txid = 3;
		head.db_size_pages = SQLITE_SHARD_SIZE;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		let pgnos = (1..=SQLITE_SHARD_SIZE).collect::<Vec<_>>();
		let existing_shard = encoded_noisy_blob(1, SQLITE_SHARD_SIZE, pgnos.iter().copied());
		assert!(existing_shard.len() > crate::udb::VALUE_CHUNK_SIZE * 10);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(shard_key(TEST_ACTOR, 0), existing_shard),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 2),
					encoded_blob(2, SQLITE_SHARD_SIZE, &[(1, 0x22)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 2_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;

		assert!(engine.compact_shard(TEST_ACTOR, 0).await?);

		let shard_blob = read_value(&engine, shard_key(TEST_ACTOR, 0))
			.await?
			.expect("shard should exist after compaction");
		let decoded = decode_ltx_v3(&shard_blob)?;
		assert_eq!(decoded.get_page(1), Some(page(0x22).as_slice()));
		assert_eq!(decoded.get_page(2), Some(noisy_page(2).as_slice()));
		assert_eq!(
			decoded.get_page(SQLITE_SHARD_SIZE - 1),
			Some(noisy_page(SQLITE_SHARD_SIZE - 1).as_slice())
		);

		Ok(())
	}

	#[tokio::test]
	async fn compact_shard_keeps_quota_usage_in_sync() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.db_size_pages = 2;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 4),
					encoded_blob(4, 2, &[(1, 0x10)]),
				),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 5),
					encoded_blob(5, 2, &[(2, 0x20)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 4_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 5_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;
		rewrite_meta_with_actual_usage(&engine).await?;
		let before_usage = actual_tracked_usage(&engine).await?;

		assert!(engine.compact_shard(TEST_ACTOR, 0).await?);

		let after_usage = actual_tracked_usage(&engine).await?;
		let stored_head = decode_db_head(
			&read_value(&engine, meta_key(TEST_ACTOR))
				.await?
				.expect("meta should exist after compaction"),
		)?;

		assert_eq!(stored_head.sqlite_storage_used, after_usage);
		assert!(after_usage <= before_usage);

		Ok(())
	}

	#[tokio::test]
	async fn compact_shard_discards_pages_above_eof() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 2;
		head.next_txid = 3;
		head.db_size_pages = 1;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					shard_key(TEST_ACTOR, 0),
					encoded_blob(1, 2, &[(1, 0x10), (2, 0x20)]),
				),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 2),
					encoded_blob(2, 2, &[(1, 0x11), (2, 0x22)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 2_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;
		rewrite_meta_with_actual_usage(&engine).await?;

		assert!(engine.compact_shard(TEST_ACTOR, 0).await?);

		let shard_blob = read_value(&engine, shard_key(TEST_ACTOR, 0))
			.await?
			.expect("shard should exist after compaction");
		let decoded = decode_ltx_v3(&shard_blob)?;
		assert_eq!(decoded.pages.len(), 1);
		assert_eq!(decoded.pages[0].pgno, 1);
		assert_eq!(decoded.pages[0].bytes, page(0x11));
		assert!(
			read_value(&engine, delta_blob_key(TEST_ACTOR, 2))
				.await?
				.is_none()
		);

		Ok(())
	}

	#[tokio::test]
	async fn compact_shard_retries_cleanly_after_store_error() -> Result<()> {
		const FAIL_ACTOR: &str = "test-actor-compaction-failure";

		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.db_size_pages = 2;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(FAIL_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					delta_blob_key(FAIL_ACTOR, 4),
					encoded_blob(4, 2, &[(1, 0x10)]),
				),
				WriteOp::put(
					delta_blob_key(FAIL_ACTOR, 5),
					encoded_blob(5, 2, &[(2, 0x20)]),
				),
				WriteOp::put(pidx_delta_key(FAIL_ACTOR, 1), 4_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(FAIL_ACTOR, 2), 5_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		engine.open(FAIL_ACTOR, OpenConfig::new(0)).await?;
		let head = decode_db_head(
			&read_value(&engine, meta_key(FAIL_ACTOR))
				.await?
				.expect("meta should exist before quota rewrite"),
		)?;
		let usage_without_meta = actual_tracked_usage(&engine).await?.saturating_sub(
			tracked_storage_entry_size(
				&meta_key(FAIL_ACTOR),
				&read_value(&engine, meta_key(FAIL_ACTOR))
					.await?
					.expect("meta should exist before quota rewrite"),
			)
			.expect("meta key should count toward sqlite quota"),
		);
		let (_, meta_bytes) = encode_db_head_with_usage(FAIL_ACTOR, &head, usage_without_meta)?;
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![WriteOp::put(meta_key(FAIL_ACTOR), meta_bytes)],
		)
		.await?;
		let before_usage = actual_tracked_usage(&engine).await?;
		let _guard = test_hooks::fail_next_apply_write_ops_matching(meta_key(FAIL_ACTOR));

		let error = engine
			.compact_shard(FAIL_ACTOR, 0)
			.await
			.expect_err("injected compaction store error should fail the pass");
		let error_text = format!("{error:#}");

		assert!(error_text.contains("InjectedStoreError"), "{error_text}");
		assert_eq!(actual_tracked_usage(&engine).await?, before_usage);
		assert!(
			read_value(&engine, delta_blob_key(FAIL_ACTOR, 4))
				.await?
				.is_some()
		);
		assert!(
			read_value(&engine, delta_blob_key(FAIL_ACTOR, 5))
				.await?
				.is_some()
		);
		assert_eq!(
			scan_prefix_values(&engine, pidx_delta_prefix(FAIL_ACTOR))
				.await?
				.len(),
			2
		);

		assert!(engine.compact_shard(FAIL_ACTOR, 0).await?);
		assert_eq!(
			engine.get_pages(FAIL_ACTOR, 4, vec![1, 2]).await?,
			vec![
				FetchedPage {
					pgno: 1,
					bytes: Some(page(0x10)),
				},
				FetchedPage {
					pgno: 2,
					bytes: Some(page(0x20)),
				},
			]
		);

		Ok(())
	}

	#[tokio::test]
	async fn compact_shard_skips_stale_meta_without_rewinding_head() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 1;
		head.next_txid = 2;
		head.db_size_pages = 1;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		let engine = std::sync::Arc::new(engine);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 1),
					encoded_blob(1, 1, &[(1, 0x10)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 1_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;
		let (_guard, reached, release) = super::test_hooks::pause_before_commit(TEST_ACTOR);
		let compact_engine = std::sync::Arc::clone(&engine);
		let compact_task =
			tokio::spawn(async move { compact_engine.compact_shard(TEST_ACTOR, 0).await });

		reached.notified().await;

		let mut updated_head = decode_db_head(
			&read_value(engine.as_ref(), meta_key(TEST_ACTOR))
				.await?
				.expect("meta should exist before stale compaction check"),
		)?;
		updated_head.head_txid = 2;
		updated_head.next_txid = 3;
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![WriteOp::put(
				meta_key(TEST_ACTOR),
				encode_db_head(&updated_head)?,
			)],
		)
		.await?;
		release.notify_waiters();

		assert!(!compact_task.await??);
		assert_eq!(
			decode_db_head(
				&read_value(engine.as_ref(), meta_key(TEST_ACTOR))
					.await?
					.expect("meta should remain after skipped compaction"),
			)?
			.head_txid,
			2
		);
		assert!(
			read_value(engine.as_ref(), shard_key(TEST_ACTOR, 0))
				.await?
				.is_none()
		);
		assert!(
			read_value(engine.as_ref(), delta_blob_key(TEST_ACTOR, 1))
				.await?
				.is_some()
		);
		assert_eq!(
			read_value(engine.as_ref(), pidx_delta_key(TEST_ACTOR, 1)).await?,
			Some(1_u64.to_be_bytes().to_vec())
		);

		Ok(())
	}

	#[tokio::test]
	async fn compact_shard_serializes_with_concurrent_commit() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 1;
		head.next_txid = 2;
		head.db_size_pages = 1;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		let engine = std::sync::Arc::new(engine);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 1),
					encoded_blob(1, 1, &[(1, 0x10)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 1_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;

		let (guard, reached, release) = super::test_hooks::pause_before_commit(TEST_ACTOR);
		let compact_engine = std::sync::Arc::clone(&engine);
		let compact_task =
			tokio::spawn(async move { compact_engine.compact_shard(TEST_ACTOR, 0).await });

		reached.notified().await;

		let generation = head.generation;
		let commit_engine = std::sync::Arc::clone(&engine);
		let commit_task = tokio::spawn(async move {
			commit_engine
				.commit(TEST_ACTOR, commit_request(generation, 1, &[(2, 0x22)]))
				.await
		});
		release.notify_waiters();

		assert!(compact_task.await??);
		let commit = commit_task.await??;
		assert_eq!(commit.txid, 2);
		let stored_head = decode_db_head(
			&read_value(engine.as_ref(), meta_key(TEST_ACTOR))
				.await?
				.expect("meta should exist after serialized commit"),
		)?;
		assert_eq!(stored_head.head_txid, 2);
		assert_eq!(stored_head.next_txid, 3);
		assert_eq!(stored_head.materialized_txid, 1);
		assert_eq!(
			engine
				.get_pages(TEST_ACTOR, head.generation, vec![1, 2])
				.await?,
			vec![
				FetchedPage {
					pgno: 1,
					bytes: Some(page(0x10)),
				},
				FetchedPage {
					pgno: 2,
					bytes: Some(page(0x22)),
				},
			]
		);

		drop(guard);
		assert!(engine.compact_shard(TEST_ACTOR, 0).await?);
		let stored_head = decode_db_head(
			&read_value(engine.as_ref(), meta_key(TEST_ACTOR))
				.await?
				.expect("meta should exist after retry"),
		)?;
		assert_eq!(stored_head.head_txid, 2);
		assert_eq!(stored_head.materialized_txid, 2);

		Ok(())
	}

	#[tokio::test]
	async fn open_during_inflight_compaction_keeps_generation() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 1;
		head.next_txid = 2;
		head.db_size_pages = 1;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		let engine = std::sync::Arc::new(engine);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 1),
					encoded_blob(1, 1, &[(1, 0x10)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 1_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;

		let (_guard, reached, release) = super::test_hooks::pause_before_commit(TEST_ACTOR);
		let compact_engine = std::sync::Arc::clone(&engine);
		let compact_task =
			tokio::spawn(async move { compact_engine.compact_shard(TEST_ACTOR, 0).await });

		reached.notified().await;

		let open = engine.open(TEST_ACTOR, OpenConfig::new(2_345)).await?;
		release.notify_waiters();

		assert_eq!(open.generation, head.generation);
		// Compaction is no longer fenced by `open()` — it proceeds and folds
		// the delta into a shard. The generation field stays stable across
		// the open + concurrent compaction, which is what this test guards.
		assert!(compact_task.await??);
		let stored_head = decode_db_head(
			&read_value(engine.as_ref(), meta_key(TEST_ACTOR))
				.await?
				.expect("meta should exist after open"),
		)?;
		assert_eq!(stored_head.generation, head.generation);
		assert_eq!(stored_head.head_txid, 1);
		assert_eq!(stored_head.materialized_txid, 1);
		assert!(
			read_value(engine.as_ref(), delta_blob_key(TEST_ACTOR, 1))
				.await?
				.is_none(),
			"compaction should have folded the delta into a shard",
		);
		assert!(
			read_value(engine.as_ref(), shard_key(TEST_ACTOR, 0))
				.await?
				.is_some(),
			"compaction should have written the shard",
		);

		Ok(())
	}

	#[tokio::test]
	async fn compact_worker_handles_multi_shard_delta_across_three_passes() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.head_txid = 1;
		head.next_txid = 2;
		head.db_size_pages = 129;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 1),
					encoded_blob(1, 129, &[(1, 0x11), (65, 0x65), (129, 0x81)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 1_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 65), 1_u64.to_be_bytes().to_vec()),
				WriteOp::put(
					pidx_delta_key(TEST_ACTOR, 129),
					1_u64.to_be_bytes().to_vec(),
				),
			],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;

		assert!(engine.compact_shard(TEST_ACTOR, 0).await?);
		assert!(
			read_value(&engine, delta_blob_key(TEST_ACTOR, 1))
				.await?
				.is_some()
		);

		assert!(engine.compact_shard(TEST_ACTOR, 1).await?);
		assert!(
			read_value(&engine, delta_blob_key(TEST_ACTOR, 1))
				.await?
				.is_some()
		);

		assert!(engine.compact_shard(TEST_ACTOR, 2).await?);
		assert!(
			read_value(&engine, delta_blob_key(TEST_ACTOR, 1))
				.await?
				.is_none()
		);
		assert!(
			scan_prefix_values(&engine, pidx_delta_prefix(TEST_ACTOR))
				.await?
				.is_empty()
		);
		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![1, 65, 129]).await?,
			vec![
				FetchedPage {
					pgno: 1,
					bytes: Some(page(0x11)),
				},
				FetchedPage {
					pgno: 65,
					bytes: Some(page(0x65)),
				},
				FetchedPage {
					pgno: 129,
					bytes: Some(page(0x81)),
				},
			]
		);

		Ok(())
	}

	#[tokio::test]
	async fn compact_worker_is_idempotent() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let mut head = seeded_head();
		head.db_size_pages = 2;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		apply_write_ops(
			&engine.db,
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(meta_key(TEST_ACTOR), encode_db_head(&head)?),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 4),
					encoded_blob(4, 2, &[(1, 0x10)]),
				),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 5),
					encoded_blob(5, 2, &[(2, 0x20)]),
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 1), 4_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 5_u64.to_be_bytes().to_vec()),
			],
		)
		.await?;
		engine.open(TEST_ACTOR, OpenConfig::new(0)).await?;

		assert_eq!(engine.compact_worker(TEST_ACTOR, 8).await?, 1);
		assert_eq!(engine.compact_worker(TEST_ACTOR, 8).await?, 0);
		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![1, 2]).await?,
			vec![
				FetchedPage {
					pgno: 1,
					bytes: Some(page(0x10)),
				},
				FetchedPage {
					pgno: 2,
					bytes: Some(page(0x20)),
				},
			]
		);

		Ok(())
	}
}
