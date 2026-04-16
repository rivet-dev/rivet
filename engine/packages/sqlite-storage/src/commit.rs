//! Commit paths for fast-path and staged writes.

use std::collections::BTreeMap;
use std::time::Instant;

use anyhow::{Context, Result, anyhow, bail, ensure};
use scc::hash_map::Entry;
use serde::{Deserialize, Serialize};

use crate::engine::SqliteEngine;
use crate::keys::{delta_key, meta_key, pidx_delta_key, stage_chunk_prefix, stage_key};
use crate::ltx::{LtxHeader, encode_ltx_v3};
use crate::quota::{encode_db_head_with_usage, tracked_storage_entry_size};
use crate::types::{DBHead, DirtyPage, SQLITE_MAX_DELTA_BYTES, SqliteMeta};
use crate::udb::{self, WriteOp};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRequest {
	pub generation: u64,
	pub head_txid: u64,
	pub db_size_pages: u32,
	pub dirty_pages: Vec<DirtyPage>,
	pub now_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitResult {
	pub txid: u64,
	pub meta: SqliteMeta,
	pub delta_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitStageRequest {
	pub generation: u64,
	pub stage_id: u64,
	pub chunk_idx: u16,
	pub dirty_pages: Vec<DirtyPage>,
	pub is_last: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitStageResult {
	pub chunk_idx_committed: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitFinalizeRequest {
	pub generation: u64,
	pub expected_head_txid: u64,
	pub stage_id: u64,
	pub new_db_size_pages: u32,
	pub now_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitFinalizeResult {
	pub new_head_txid: u64,
	pub meta: SqliteMeta,
	pub delta_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct StagedChunk {
	dirty_pages: Vec<DirtyPage>,
	is_last: bool,
}

#[cfg(test)]
mod test_hooks {
	use std::sync::Mutex;

	use anyhow::{Result, bail};

	static FAIL_NEXT_FAST_COMMIT_WRITE_ACTOR: Mutex<Option<String>> = Mutex::new(None);

	pub(super) struct FastCommitWriteFailureGuard;

	pub(super) fn fail_next_fast_commit_write(actor_id: &str) -> FastCommitWriteFailureGuard {
		*FAIL_NEXT_FAST_COMMIT_WRITE_ACTOR
			.lock()
			.expect("fast commit failpoint mutex should lock") = Some(actor_id.to_string());
		FastCommitWriteFailureGuard
	}

	pub(super) fn maybe_fail_fast_commit_write(actor_id: &str) -> Result<()> {
		let mut fail_actor = FAIL_NEXT_FAST_COMMIT_WRITE_ACTOR
			.lock()
			.expect("fast commit failpoint mutex should lock");
		if fail_actor.as_deref() == Some(actor_id) {
			*fail_actor = None;
			bail!("InjectedStoreError: fast commit write transaction failed before commit");
		}

		Ok(())
	}

	impl Drop for FastCommitWriteFailureGuard {
		fn drop(&mut self) {
			*FAIL_NEXT_FAST_COMMIT_WRITE_ACTOR
				.lock()
				.expect("fast commit failpoint mutex should lock") = None;
		}
	}
}

impl SqliteEngine {
	pub async fn commit(&self, actor_id: &str, request: CommitRequest) -> Result<CommitResult> {
		let start = Instant::now();
		let dirty_page_count = request.dirty_pages.len();
		let mut dirty_pgnos = request
			.dirty_pages
			.iter()
			.map(|page| page.pgno)
			.collect::<Vec<_>>();
		dirty_pgnos.sort_unstable();
		dirty_pgnos.dedup();
		let raw_dirty_bytes = dirty_pages_raw_bytes(&request.dirty_pages)?;
		if raw_dirty_bytes > SQLITE_MAX_DELTA_BYTES {
			bail!(
				"CommitTooLarge: raw dirty pages were {} bytes, limit is {} bytes",
				raw_dirty_bytes,
				SQLITE_MAX_DELTA_BYTES
			);
		}

		let actor_id = actor_id.to_string();
		let actor_id_for_tx = actor_id.clone();
		let subspace = self.subspace.clone();
		let cached_existing_pidx = match self.page_indices.get_async(&actor_id).await {
			Some(index) => Some(
				dirty_pgnos
					.iter()
					.map(|pgno| (*pgno, index.get().get(*pgno).is_some()))
					.collect::<BTreeMap<_, _>>(),
			),
			None => None,
		};
		let request = request.clone();
		let dirty_pgnos_for_tx = dirty_pgnos.clone();
		let (txid, head, delta_bytes) =
			udb::run_db_op(self.db.as_ref(), self.op_counter.as_ref(), move |tx| {
				let actor_id = actor_id_for_tx.clone();
				let request = request.clone();
				let dirty_pgnos = dirty_pgnos_for_tx.clone();
				let subspace = subspace.clone();
				let cached_existing_pidx = cached_existing_pidx.clone();
				async move {
					let meta_storage_key = meta_key(&actor_id);
					let meta_bytes = udb::tx_get_value(&tx, &subspace, &meta_storage_key)
						.await?
						.context("sqlite meta missing for commit")?;
					let mut head = decode_db_head(&meta_bytes)?;

					if head.generation != request.generation {
						bail!(
							"FenceMismatch: commit generation {} did not match current generation {}",
							request.generation,
							head.generation
						);
					}
					if head.head_txid != request.head_txid {
						bail!(
							"FenceMismatch: commit head_txid {} did not match current head_txid {}",
							request.head_txid,
							head.head_txid
						);
					}

					let txid = head.next_txid;
					ensure!(
						txid > head.head_txid,
						"next txid {} must advance past head txid {}",
						txid,
						head.head_txid
					);

					let delta = encode_ltx_v3(
						LtxHeader::delta(txid, request.db_size_pages, request.now_ms),
						&request.dirty_pages,
					)
					.context("encode commit delta")?;
					let delta_bytes = delta.len() as u64;

					head.head_txid = txid;
					head.next_txid += 1;
					head.db_size_pages = request.db_size_pages;

					let txid_bytes = txid.to_be_bytes();
					let mut usage_without_meta = head.sqlite_storage_used.saturating_sub(
						tracked_storage_entry_size(&meta_storage_key, &meta_bytes)
							.expect("meta key should count toward sqlite quota"),
					);
					usage_without_meta +=
						tracked_storage_entry_size(&delta_key(&actor_id, txid), &delta)
							.expect("delta key should count toward sqlite quota");
					let existing_pidx = match cached_existing_pidx {
						Some(ref existing) => existing.clone(),
						None => {
							let mut existing = BTreeMap::new();
							for pgno in &dirty_pgnos {
								existing.insert(
									*pgno,
									udb::tx_get_value(
										&tx,
										&subspace,
										&pidx_delta_key(&actor_id, *pgno),
									)
									.await?
									.is_some(),
								);
							}
							existing
						}
					};
					for pgno in &dirty_pgnos {
						if !existing_pidx.get(pgno).copied().unwrap_or(false) {
							usage_without_meta += tracked_storage_entry_size(
								&pidx_delta_key(&actor_id, *pgno),
								&txid_bytes,
							)
							.expect("pidx key should count toward sqlite quota");
						}
					}

					udb::tx_write_value(&tx, &subspace, &delta_key(&actor_id, txid), &delta)?;
					for pgno in &dirty_pgnos {
						udb::tx_write_value(
							&tx,
							&subspace,
							&pidx_delta_key(&actor_id, *pgno),
							&txid_bytes,
						)?;
					}

					let (updated_head, encoded_head) =
						encode_db_head_with_usage(&actor_id, &head, usage_without_meta)?;
					if updated_head.sqlite_storage_used > updated_head.sqlite_max_storage {
						bail!(
							"SqliteStorageQuotaExceeded: sqlite storage used {} would exceed max {}",
							updated_head.sqlite_storage_used,
							updated_head.sqlite_max_storage
						);
					}
					udb::tx_write_value(&tx, &subspace, &meta_storage_key, &encoded_head)?;
					#[cfg(test)]
					test_hooks::maybe_fail_fast_commit_write(&actor_id)?;

					Ok((txid, updated_head, delta_bytes))
				}
			})
			.await
			.map_err(|err| {
				if err.to_string().contains("FenceMismatch") {
					self.metrics.inc_fence_mismatch_total();
				}
				err
			})?;

		match self.page_indices.entry_async(actor_id.to_string()).await {
			Entry::Occupied(entry) => {
				for pgno in dirty_pgnos {
					entry.get().insert(pgno, txid);
				}
			}
			Entry::Vacant(entry) => {
				drop(entry);
			}
		}

		let _ = self.compaction_tx.send(actor_id.to_string());
		self.metrics
			.observe_commit("fast", dirty_page_count, start.elapsed());
		self.metrics.inc_commit_total();
		self.metrics.set_delta_count_from_head(&head);

		Ok(CommitResult {
			txid,
			meta: SqliteMeta::from((head, SQLITE_MAX_DELTA_BYTES)),
			delta_bytes,
		})
	}

	pub async fn commit_stage(
		&self,
		actor_id: &str,
		request: CommitStageRequest,
	) -> Result<CommitStageResult> {
		let meta_bytes = udb::get_value(
			self.db.as_ref(),
			&self.subspace,
			self.op_counter.as_ref(),
			meta_key(actor_id),
		)
		.await?
		.context("sqlite meta missing for staged commit")?;
		let head = decode_db_head(&meta_bytes)?;

		if head.generation != request.generation {
			self.metrics.inc_fence_mismatch_total();
			bail!(
				"FenceMismatch: commit_stage generation {} did not match current generation {}",
				request.generation,
				head.generation
			);
		}

		let staged_chunk = serde_bare::to_vec(&StagedChunk {
			dirty_pages: request.dirty_pages,
			is_last: request.is_last,
		})
		.context("serialize staged chunk")?;

		udb::apply_write_ops(
			self.db.as_ref(),
			&self.subspace,
			self.op_counter.as_ref(),
			vec![WriteOp::put(
				stage_key(actor_id, request.stage_id, request.chunk_idx),
				staged_chunk,
			)],
		)
		.await?;

		Ok(CommitStageResult {
			chunk_idx_committed: request.chunk_idx,
		})
	}

	pub async fn commit_finalize(
		&self,
		actor_id: &str,
		request: CommitFinalizeRequest,
	) -> Result<CommitFinalizeResult> {
		let start = Instant::now();
		let meta_bytes = udb::get_value(
			self.db.as_ref(),
			&self.subspace,
			self.op_counter.as_ref(),
			meta_key(actor_id),
		)
		.await?
		.context("sqlite meta missing for commit finalize")?;
		let mut head = decode_db_head(&meta_bytes)?;

		if head.generation != request.generation {
			self.metrics.inc_fence_mismatch_total();
			bail!(
				"FenceMismatch: commit_finalize generation {} did not match current generation {}",
				request.generation,
				head.generation
			);
		}
		if head.head_txid != request.expected_head_txid {
			self.metrics.inc_fence_mismatch_total();
			bail!(
				"FenceMismatch: commit_finalize head_txid {} did not match current head_txid {}",
				request.expected_head_txid,
				head.head_txid
			);
		}

		let staged_entries = udb::scan_prefix_values(
			self.db.as_ref(),
			&self.subspace,
			self.op_counter.as_ref(),
			stage_chunk_prefix(actor_id, request.stage_id),
		)
		.await?;
		if staged_entries.is_empty() {
			bail!("StageNotFound: stage {} missing", request.stage_id);
		}

		let staged_pages = decode_staged_pages(actor_id, request.stage_id, staged_entries)?;
		let txid = head.next_txid;
		ensure!(
			txid > head.head_txid,
			"next txid {} must advance past head txid {}",
			txid,
			head.head_txid
		);

		let delta = encode_ltx_v3(
			LtxHeader::delta(txid, request.new_db_size_pages, request.now_ms),
			&staged_pages.dirty_pages,
		)
		.context("encode finalized staged delta")?;
		let delta_bytes = delta.len() as u64;

		head.head_txid = txid;
		head.next_txid += 1;
		head.db_size_pages = request.new_db_size_pages;

		let mut dirty_pgnos = staged_pages
			.dirty_pages
			.iter()
			.map(|page| page.pgno)
			.collect::<Vec<_>>();
		dirty_pgnos.sort_unstable();
		dirty_pgnos.dedup();
		let dirty_page_count = dirty_pgnos.len();

		let txid_bytes = txid.to_be_bytes();
		let mut usage_without_meta = head.sqlite_storage_used.saturating_sub(
			tracked_storage_entry_size(&meta_key(actor_id), &meta_bytes)
				.expect("meta key should count toward sqlite quota"),
		);
		usage_without_meta += tracked_storage_entry_size(&delta_key(actor_id, txid), &delta)
			.expect("delta key should count toward sqlite quota");
		let existing_pidx = existing_pidx_entries(self, actor_id, &dirty_pgnos).await?;
		for pgno in &dirty_pgnos {
			if !existing_pidx.get(pgno).copied().unwrap_or(false) {
				usage_without_meta +=
					tracked_storage_entry_size(&pidx_delta_key(actor_id, *pgno), &txid_bytes)
						.expect("pidx key should count toward sqlite quota");
			}
		}

		let mut mutations = Vec::with_capacity(2 + dirty_pgnos.len());
		mutations.push(WriteOp::put(delta_key(actor_id, txid), delta));
		for pgno in &dirty_pgnos {
			mutations.push(WriteOp::put(
				pidx_delta_key(actor_id, *pgno),
				txid_bytes.to_vec(),
			));
		}
		for key in staged_pages.stage_keys {
			mutations.push(WriteOp::delete(key));
		}
		let (updated_head, encoded_head) =
			encode_db_head_with_usage(actor_id, &head, usage_without_meta)?;
		if updated_head.sqlite_storage_used > updated_head.sqlite_max_storage {
			bail!(
				"SqliteStorageQuotaExceeded: sqlite storage used {} would exceed max {}",
				updated_head.sqlite_storage_used,
				updated_head.sqlite_max_storage
			);
		}
		head = updated_head;
		mutations.push(WriteOp::put(meta_key(actor_id), encoded_head));

		// Best-effort defense against concurrent writers. The real protection comes from
		// pegboard-envoy serializing actor lifecycle, but we re-read META here to detect
		// races that slip past the outer layer.
		let recheck_meta = udb::get_value(
			self.db.as_ref(),
			&self.subspace,
			self.op_counter.as_ref(),
			meta_key(actor_id),
		)
		.await?;
		if recheck_meta.as_deref() != Some(meta_bytes.as_slice()) {
			tracing::error!(
				?actor_id,
				"meta changed during commit finalize, concurrent writer detected"
			);
			return Err(anyhow!("concurrent takeover detected, disconnecting actor"));
		}

		udb::apply_write_ops(
			self.db.as_ref(),
			&self.subspace,
			self.op_counter.as_ref(),
			mutations,
		)
		.await?;

		match self.page_indices.entry_async(actor_id.to_string()).await {
			Entry::Occupied(entry) => {
				for pgno in dirty_pgnos {
					entry.get().insert(pgno, txid);
				}
			}
			Entry::Vacant(entry) => {
				drop(entry);
			}
		}

		let _ = self.compaction_tx.send(actor_id.to_string());
		self.metrics
			.observe_commit("slow", dirty_page_count, start.elapsed());
		self.metrics.inc_commit_total();
		self.metrics.set_delta_count_from_head(&head);

		Ok(CommitFinalizeResult {
			new_head_txid: txid,
			meta: SqliteMeta::from((head, SQLITE_MAX_DELTA_BYTES)),
			delta_bytes,
		})
	}
}

fn decode_db_head(bytes: &[u8]) -> Result<DBHead> {
	serde_bare::from_slice(bytes).context("decode sqlite db head")
}

fn dirty_pages_raw_bytes(dirty_pages: &[DirtyPage]) -> Result<u64> {
	dirty_pages.iter().try_fold(0u64, |total, page| {
		let page_bytes =
			u64::try_from(page.bytes.len()).context("dirty page length exceeded u64")?;
		total
			.checked_add(page_bytes)
			.context("dirty page bytes exceeded u64")
	})
}

async fn existing_pidx_entries(
	engine: &SqliteEngine,
	actor_id: &str,
	dirty_pgnos: &[u32],
) -> Result<BTreeMap<u32, bool>> {
	let actor_id = actor_id.to_string();
	if let Some(index) = engine.page_indices.get_async(&actor_id).await {
		let existing = dirty_pgnos
			.iter()
			.map(|pgno| (*pgno, index.get().get(*pgno).is_some()))
			.collect::<BTreeMap<_, _>>();
		return Ok(existing);
	}

	let keys = dirty_pgnos
		.iter()
		.map(|pgno| pidx_delta_key(&actor_id, *pgno))
		.collect::<Vec<_>>();
	let values = udb::batch_get_values(
		engine.db.as_ref(),
		&engine.subspace,
		engine.op_counter.as_ref(),
		keys,
	)
	.await?;

	Ok(dirty_pgnos
		.iter()
		.copied()
		.zip(values.into_iter().map(|value| value.is_some()))
		.collect())
}

struct DecodedStagedPages {
	dirty_pages: Vec<DirtyPage>,
	stage_keys: Vec<Vec<u8>>,
}

fn decode_staged_pages(
	actor_id: &str,
	stage_id: u64,
	staged_entries: Vec<(Vec<u8>, Vec<u8>)>,
) -> Result<DecodedStagedPages> {
	let mut chunks = staged_entries
		.into_iter()
		.map(|(key, value)| {
			let chunk_idx = decode_stage_chunk_idx(actor_id, stage_id, &key)?;
			let chunk: StagedChunk =
				serde_bare::from_slice(&value).context("decode staged commit chunk")?;
			Ok((chunk_idx, key, chunk))
		})
		.collect::<Result<Vec<_>>>()?;
	chunks.sort_by_key(|(chunk_idx, _, _)| *chunk_idx);

	let mut expected_chunk_idx = 0u16;
	let mut saw_last_chunk = false;
	let mut pages_by_pgno = std::collections::BTreeMap::new();
	let mut stage_keys = Vec::with_capacity(chunks.len());
	for (chunk_idx, key, chunk) in chunks {
		ensure!(
			chunk_idx == expected_chunk_idx,
			"stage {} missing chunk {}, found chunk {} instead",
			stage_id,
			expected_chunk_idx,
			chunk_idx
		);
		ensure!(
			!saw_last_chunk,
			"stage {} had chunks after the last chunk marker",
			stage_id
		);

		stage_keys.push(key);
		for dirty_page in chunk.dirty_pages {
			pages_by_pgno.insert(dirty_page.pgno, dirty_page.bytes);
		}

		saw_last_chunk = chunk.is_last;
		expected_chunk_idx = expected_chunk_idx
			.checked_add(1)
			.context("stage chunk index overflow")?;
	}

	ensure!(
		saw_last_chunk,
		"stage {} did not include a last chunk marker",
		stage_id
	);

	Ok(DecodedStagedPages {
		dirty_pages: pages_by_pgno
			.into_iter()
			.map(|(pgno, bytes)| DirtyPage { pgno, bytes })
			.collect(),
		stage_keys,
	})
}

fn decode_stage_chunk_idx(actor_id: &str, stage_id: u64, key: &[u8]) -> Result<u16> {
	let prefix = stage_chunk_prefix(actor_id, stage_id);
	ensure!(
		key.starts_with(&prefix),
		"stage key {:?} did not match stage {}",
		key,
		stage_id
	);
	ensure!(
		key.len() == prefix.len() + std::mem::size_of::<u16>(),
		"stage key for stage {} had invalid length {}",
		stage_id,
		key.len()
	);

	Ok(u16::from_be_bytes(
		key[prefix.len()..]
			.try_into()
			.expect("stage chunk suffix should be two bytes"),
	))
}

#[cfg(test)]
mod tests {
	use anyhow::Result;
	use tokio::sync::mpsc::error::TryRecvError;

	use super::{
		CommitFinalizeRequest, CommitRequest, CommitStageRequest, decode_db_head, test_hooks,
	};
	use crate::engine::SqliteEngine;
	use crate::keys::{delta_key, meta_key, stage_chunk_prefix};
	use crate::quota::{encode_db_head_with_usage, tracked_storage_entry_size};
	use crate::test_utils::{
		assert_op_count, clear_op_count, read_value, scan_prefix_values, test_db,
	};
	use crate::types::{
		DBHead, DirtyPage, FetchedPage, SQLITE_DEFAULT_MAX_STORAGE_BYTES, SQLITE_PAGE_SIZE,
		SQLITE_SHARD_SIZE, SQLITE_VFS_V2_SCHEMA_VERSION,
	};
	use crate::udb::{WriteOp, apply_write_ops};

	const TEST_ACTOR: &str = "test-actor";

	fn seeded_head() -> DBHead {
		DBHead {
			schema_version: SQLITE_VFS_V2_SCHEMA_VERSION,
			generation: 4,
			head_txid: 0,
			next_txid: 1,
			materialized_txid: 0,
			db_size_pages: 0,
			page_size: SQLITE_PAGE_SIZE,
			shard_size: SQLITE_SHARD_SIZE,
			creation_ts_ms: 123,
			sqlite_storage_used: 0,
			sqlite_max_storage: SQLITE_DEFAULT_MAX_STORAGE_BYTES,
		}
	}

	fn page(fill: u8) -> Vec<u8> {
		vec![fill; SQLITE_PAGE_SIZE as usize]
	}

	async fn write_seeded_meta(
		engine: &SqliteEngine,
		actor_id: &str,
		head: DBHead,
	) -> Result<DBHead> {
		let (head, meta_bytes) = encode_db_head_with_usage(actor_id, &head, 0)?;
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![WriteOp::put(meta_key(actor_id), meta_bytes)],
		)
		.await?;
		Ok(head)
	}

	async fn actual_tracked_usage(engine: &SqliteEngine) -> Result<u64> {
		Ok(scan_prefix_values(engine, vec![0x02])
			.await?
			.into_iter()
			.filter_map(|(key, value)| tracked_storage_entry_size(&key, &value))
			.sum())
	}

	fn request(generation: u64, head_txid: u64) -> CommitRequest {
		CommitRequest {
			generation,
			head_txid,
			db_size_pages: 1,
			dirty_pages: vec![DirtyPage {
				pgno: 1,
				bytes: page(0x55),
			}],
			now_ms: 999,
		}
	}

	fn stage_request(
		generation: u64,
		stage_id: u64,
		chunk_idx: u16,
		pages: &[(u32, u8)],
		is_last: bool,
	) -> CommitStageRequest {
		CommitStageRequest {
			generation,
			stage_id,
			chunk_idx,
			dirty_pages: pages
				.iter()
				.map(|(pgno, fill)| DirtyPage {
					pgno: *pgno,
					bytes: page(*fill),
				})
				.collect(),
			is_last,
		}
	}

	fn bulk_request(
		generation: u64,
		head_txid: u64,
		start_pgno: u32,
		page_count: u32,
		fill: u8,
	) -> CommitRequest {
		CommitRequest {
			generation,
			head_txid,
			db_size_pages: start_pgno + page_count - 1,
			dirty_pages: (0..page_count)
				.map(|offset| DirtyPage {
					pgno: start_pgno + offset,
					bytes: page(fill),
				})
				.collect(),
			now_ms: 9_999,
		}
	}

	fn bulk_stage_request(
		generation: u64,
		stage_id: u64,
		chunk_idx: u16,
		start_pgno: u32,
		page_count: u32,
		fill: u8,
		is_last: bool,
	) -> CommitStageRequest {
		CommitStageRequest {
			generation,
			stage_id,
			chunk_idx,
			dirty_pages: (0..page_count)
				.map(|offset| DirtyPage {
					pgno: start_pgno + offset,
					bytes: page(fill),
				})
				.collect(),
			is_last,
		}
	}

	#[tokio::test]
	async fn commit_writes_delta_updates_meta_and_cached_pidx() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, mut compaction_rx) = SqliteEngine::new(db, subspace);
		write_seeded_meta(&engine, TEST_ACTOR, seeded_head()).await?;
		let _ = engine.get_or_load_pidx(TEST_ACTOR).await?;
		clear_op_count(&engine);

		let result = engine.commit(TEST_ACTOR, request(4, 0)).await?;
		assert_eq!(result.txid, 1);
		assert_eq!(compaction_rx.recv().await, Some(TEST_ACTOR.to_string()));
		assert_op_count(&engine, 1);

		let stored_delta = read_value(&engine, delta_key(TEST_ACTOR, 1))
			.await?
			.expect("delta should be stored");
		assert_eq!(stored_delta.len() as u64, result.delta_bytes);
		let stored_head = decode_db_head(
			&read_value(&engine, meta_key(TEST_ACTOR))
				.await?
				.expect("meta should exist after commit"),
		)?;
		assert_eq!(stored_head.head_txid, 1);
		assert_eq!(stored_head.next_txid, 2);
		assert_eq!(stored_head.db_size_pages, 1);

		clear_op_count(&engine);
		let pages = engine.get_pages(TEST_ACTOR, 4, vec![1]).await?;
		assert_eq!(
			pages,
			vec![FetchedPage {
				pgno: 1,
				bytes: Some(page(0x55)),
			}]
		);
		assert_op_count(&engine, 1);

		Ok(())
	}

	#[tokio::test]
	async fn commit_and_read_back() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		write_seeded_meta(&engine, TEST_ACTOR, seeded_head()).await?;

		let result = engine.commit(TEST_ACTOR, request(4, 0)).await?;
		assert_eq!(result.txid, 1);
		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![1]).await?,
			vec![FetchedPage {
				pgno: 1,
				bytes: Some(page(0x55)),
			}]
		);

		Ok(())
	}

	#[tokio::test]
	async fn commit_multiple_pages() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		write_seeded_meta(&engine, TEST_ACTOR, seeded_head()).await?;

		engine
			.commit(TEST_ACTOR, bulk_request(4, 0, 1, 100, 0x77))
			.await?;

		let requested_pages = (1..=100).collect::<Vec<_>>();
		let fetched_pages = engine.get_pages(TEST_ACTOR, 4, requested_pages).await?;
		assert_eq!(fetched_pages.len(), 100);
		assert!(
			fetched_pages
				.iter()
				.all(|fetched_page| fetched_page.bytes == Some(page(0x77)))
		);

		Ok(())
	}

	#[tokio::test]
	async fn commit_overwrites_previous() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		write_seeded_meta(&engine, TEST_ACTOR, seeded_head()).await?;

		engine.commit(TEST_ACTOR, request(4, 0)).await?;
		engine
			.commit(
				TEST_ACTOR,
				CommitRequest {
					generation: 4,
					head_txid: 1,
					db_size_pages: 1,
					dirty_pages: vec![DirtyPage {
						pgno: 1,
						bytes: page(0xaa),
					}],
					now_ms: 1_111,
				},
			)
			.await?;

		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![1]).await?,
			vec![FetchedPage {
				pgno: 1,
				bytes: Some(page(0xaa)),
			}]
		);

		Ok(())
	}

	#[tokio::test]
	async fn read_nonexistent_page_returns_none() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		write_seeded_meta(&engine, TEST_ACTOR, seeded_head()).await?;

		engine.commit(TEST_ACTOR, request(4, 0)).await?;

		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![2]).await?,
			vec![FetchedPage {
				pgno: 2,
				bytes: None,
			}]
		);

		Ok(())
	}

	#[tokio::test]
	async fn multiple_actors_isolated() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		write_seeded_meta(&engine, "actor-a", seeded_head()).await?;
		write_seeded_meta(&engine, "actor-b", seeded_head()).await?;

		engine
			.commit(
				"actor-a",
				CommitRequest {
					generation: 4,
					head_txid: 0,
					db_size_pages: 1,
					dirty_pages: vec![DirtyPage {
						pgno: 1,
						bytes: page(0x1a),
					}],
					now_ms: 1_000,
				},
			)
			.await?;
		engine
			.commit(
				"actor-b",
				CommitRequest {
					generation: 4,
					head_txid: 0,
					db_size_pages: 1,
					dirty_pages: vec![DirtyPage {
						pgno: 1,
						bytes: page(0x2b),
					}],
					now_ms: 2_000,
				},
			)
			.await?;

		assert_eq!(
			engine.get_pages("actor-a", 4, vec![1]).await?,
			vec![FetchedPage {
				pgno: 1,
				bytes: Some(page(0x1a)),
			}]
		);
		assert_eq!(
			engine.get_pages("actor-b", 4, vec![1]).await?,
			vec![FetchedPage {
				pgno: 1,
				bytes: Some(page(0x2b)),
			}]
		);

		Ok(())
	}

	#[tokio::test]
	async fn commit_updates_db_size_pages() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		write_seeded_meta(&engine, TEST_ACTOR, seeded_head()).await?;

		engine
			.commit(
				TEST_ACTOR,
				CommitRequest {
					generation: 4,
					head_txid: 0,
					db_size_pages: 100,
					dirty_pages: vec![DirtyPage {
						pgno: 100,
						bytes: page(0x64),
					}],
					now_ms: 3_333,
				},
			)
			.await?;

		let stored_head = decode_db_head(
			&read_value(&engine, meta_key(TEST_ACTOR))
				.await?
				.expect("meta should exist after commit"),
		)?;
		assert_eq!(stored_head.db_size_pages, 100);
		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![100]).await?,
			vec![FetchedPage {
				pgno: 100,
				bytes: Some(page(0x64)),
			}]
		);

		Ok(())
	}

	#[tokio::test]
	async fn commit_tracks_sqlite_usage_without_counting_unrelated_keys() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		write_seeded_meta(&engine, TEST_ACTOR, seeded_head()).await?;
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![WriteOp::put(b"/kv/untracked".to_vec(), b"ignored".to_vec())],
		)
		.await?;
		let result = engine.commit(TEST_ACTOR, request(4, 0)).await?;
		let stored_head = decode_db_head(
			&read_value(&engine, meta_key(TEST_ACTOR))
				.await?
				.expect("meta should exist after commit"),
		)?;

		assert_eq!(
			stored_head.sqlite_storage_used,
			result.meta.sqlite_storage_used
		);
		assert_eq!(
			stored_head.sqlite_storage_used,
			actual_tracked_usage(&engine).await?
		);
		assert_eq!(
			stored_head.sqlite_max_storage,
			SQLITE_DEFAULT_MAX_STORAGE_BYTES
		);

		Ok(())
	}

	#[tokio::test]
	async fn commit_succeeds_within_quota_even_with_large_untracked_kv() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		let mut head = seeded_head();
		head.sqlite_max_storage = 5_000;
		write_seeded_meta(&engine, TEST_ACTOR, head).await?;
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![WriteOp::put(
				b"/kv/untracked-large".to_vec(),
				vec![0x99; 16 * 1024],
			)],
		)
		.await?;

		let result = engine.commit(TEST_ACTOR, request(4, 0)).await?;

		assert!(result.meta.sqlite_storage_used <= 5_000);
		assert_eq!(
			result.meta.sqlite_storage_used,
			actual_tracked_usage(&engine).await?
		);

		Ok(())
	}

	#[tokio::test]
	async fn commit_rejects_when_sqlite_quota_would_be_exceeded() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		let mut head = seeded_head();
		head.sqlite_max_storage = 256;
		write_seeded_meta(&engine, TEST_ACTOR, head).await?;
		clear_op_count(&engine);
		let error = engine
			.commit(TEST_ACTOR, request(4, 0))
			.await
			.expect_err("commit should fail once sqlite quota is exceeded");
		let error_text = format!("{error:#}");

		assert!(
			error_text.contains("SqliteStorageQuotaExceeded"),
			"{error_text}"
		);
		assert!(
			read_value(&engine, delta_key(TEST_ACTOR, 1))
				.await?
				.is_none()
		);

		Ok(())
	}

	#[tokio::test]
	async fn commit_rolls_back_cleanly_when_write_transaction_errors() -> Result<()> {
		const FAIL_ACTOR: &str = "test-actor-fast-commit-failure";

		let (db, subspace) = test_db().await?;
		let (engine, mut compaction_rx) = SqliteEngine::new(db, subspace);
		let initial_head = write_seeded_meta(&engine, FAIL_ACTOR, seeded_head()).await?;
		let initial_usage = actual_tracked_usage(&engine).await?;
		let _guard = test_hooks::fail_next_fast_commit_write(FAIL_ACTOR);

		let error = engine
			.commit(FAIL_ACTOR, request(4, 0))
			.await
			.expect_err("injected fast-commit write failure should bubble up");
		let error_text = format!("{error:#}");

		assert!(error_text.contains("InjectedStoreError"), "{error_text}");
		assert!(
			read_value(&engine, delta_key(FAIL_ACTOR, 1))
				.await?
				.is_none()
		);
		assert_eq!(
			decode_db_head(
				&read_value(&engine, meta_key(FAIL_ACTOR))
					.await?
					.expect("meta should still exist after rollback"),
			)?,
			initial_head
		);
		assert_eq!(actual_tracked_usage(&engine).await?, initial_usage);
		assert!(matches!(compaction_rx.try_recv(), Err(TryRecvError::Empty)));

		Ok(())
	}

	#[tokio::test]
	async fn commit_rejects_stale_generation() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, mut compaction_rx) = SqliteEngine::new(db, subspace);
		write_seeded_meta(&engine, TEST_ACTOR, seeded_head()).await?;
		clear_op_count(&engine);
		let error = engine
			.commit(TEST_ACTOR, request(99, 0))
			.await
			.expect_err("stale generation should fail");
		let error_text = format!("{error:#}");

		assert!(error_text.contains("FenceMismatch"), "{error_text}");
		assert_op_count(&engine, 1);
		assert!(
			read_value(&engine, delta_key(TEST_ACTOR, 1))
				.await?
				.is_none()
		);
		assert!(matches!(compaction_rx.try_recv(), Err(TryRecvError::Empty)));

		Ok(())
	}

	#[tokio::test]
	async fn commit_4_mib_raw_stays_on_fast_path_in_one_store_transaction() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, mut compaction_rx) = SqliteEngine::new(db, subspace);
		write_seeded_meta(&engine, TEST_ACTOR, seeded_head()).await?;
		clear_op_count(&engine);

		let result = engine
			.commit(TEST_ACTOR, bulk_request(4, 0, 1, 1024, 0x44))
			.await?;

		assert_eq!(result.txid, 1);
		assert_eq!(compaction_rx.recv().await, Some(TEST_ACTOR.to_string()));
		assert_op_count(&engine, 1);

		Ok(())
	}

	#[tokio::test]
	async fn commit_rejects_stale_head_txid() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, mut compaction_rx) = SqliteEngine::new(db, subspace);
		let mut head = seeded_head();
		head.head_txid = 7;
		head.next_txid = 8;
		write_seeded_meta(&engine, TEST_ACTOR, head).await?;
		clear_op_count(&engine);
		let error = engine
			.commit(TEST_ACTOR, request(4, 6))
			.await
			.expect_err("stale head txid should fail");
		let error_text = format!("{error:#}");

		assert!(error_text.contains("FenceMismatch"), "{error_text}");
		assert_op_count(&engine, 1);
		assert!(
			read_value(&engine, delta_key(TEST_ACTOR, 8))
				.await?
				.is_none()
		);
		assert!(matches!(compaction_rx.try_recv(), Err(TryRecvError::Empty)));

		Ok(())
	}

	#[tokio::test]
	async fn commit_stage_and_finalize_promotes_staged_delta() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, mut compaction_rx) = SqliteEngine::new(db, subspace);
		write_seeded_meta(&engine, TEST_ACTOR, seeded_head()).await?;
		clear_op_count(&engine);

		engine
			.commit_stage(TEST_ACTOR, stage_request(4, 77, 0, &[(1, 0x11)], false))
			.await?;
		engine
			.commit_stage(TEST_ACTOR, stage_request(4, 77, 1, &[(2, 0x22)], false))
			.await?;
		engine
			.commit_stage(TEST_ACTOR, stage_request(4, 77, 2, &[(70, 0x70)], true))
			.await?;

		let result = engine
			.commit_finalize(
				TEST_ACTOR,
				CommitFinalizeRequest {
					generation: 4,
					expected_head_txid: 0,
					stage_id: 77,
					new_db_size_pages: 70,
					now_ms: 1_234,
				},
			)
			.await?;

		assert_eq!(result.new_head_txid, 1);
		assert_eq!(compaction_rx.recv().await, Some(TEST_ACTOR.to_string()));
		assert!(
			scan_prefix_values(&engine, stage_chunk_prefix(TEST_ACTOR, 77))
				.await?
				.is_empty()
		);
		let stored_head = decode_db_head(
			&read_value(&engine, meta_key(TEST_ACTOR))
				.await?
				.expect("meta should exist after commit finalize"),
		)?;
		assert_eq!(stored_head.head_txid, 1);
		assert_eq!(stored_head.next_txid, 2);
		assert_eq!(stored_head.db_size_pages, 70);

		clear_op_count(&engine);
		let pages = engine.get_pages(TEST_ACTOR, 4, vec![1, 2, 70]).await?;
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
					pgno: 70,
					bytes: Some(page(0x70)),
				},
			]
		);

		Ok(())
	}

	#[tokio::test]
	async fn commit_finalize_rejects_missing_stage() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, mut compaction_rx) = SqliteEngine::new(db, subspace);
		write_seeded_meta(&engine, TEST_ACTOR, seeded_head()).await?;
		clear_op_count(&engine);
		let error = engine
			.commit_finalize(
				TEST_ACTOR,
				CommitFinalizeRequest {
					generation: 4,
					expected_head_txid: 0,
					stage_id: 999,
					new_db_size_pages: 1,
					now_ms: 777,
				},
			)
			.await
			.expect_err("missing stage should fail");

		assert!(error.to_string().contains("StageNotFound"));
		assert_op_count(&engine, 2);
		assert!(
			read_value(&engine, delta_key(TEST_ACTOR, 1))
				.await?
				.is_none()
		);
		assert!(matches!(compaction_rx.try_recv(), Err(TryRecvError::Empty)));

		Ok(())
	}

	#[tokio::test]
	async fn commit_finalize_accepts_12_mib_staged_delta() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, mut compaction_rx) = SqliteEngine::new(db, subspace);
		write_seeded_meta(&engine, TEST_ACTOR, seeded_head()).await?;
		clear_op_count(&engine);

		engine
			.commit_stage(
				TEST_ACTOR,
				bulk_stage_request(4, 88, 0, 1, 1024, 0x21, false),
			)
			.await?;
		engine
			.commit_stage(
				TEST_ACTOR,
				bulk_stage_request(4, 88, 1, 1025, 1024, 0x42, false),
			)
			.await?;
		engine
			.commit_stage(
				TEST_ACTOR,
				bulk_stage_request(4, 88, 2, 2049, 1024, 0x63, true),
			)
			.await?;

		let result = engine
			.commit_finalize(
				TEST_ACTOR,
				CommitFinalizeRequest {
					generation: 4,
					expected_head_txid: 0,
					stage_id: 88,
					new_db_size_pages: 3072,
					now_ms: 2_468,
				},
			)
			.await?;

		assert_eq!(result.new_head_txid, 1);
		assert_eq!(compaction_rx.recv().await, Some(TEST_ACTOR.to_string()));
		assert!(
			scan_prefix_values(&engine, stage_chunk_prefix(TEST_ACTOR, 88))
				.await?
				.is_empty()
		);
		assert_eq!(
			engine.get_pages(TEST_ACTOR, 4, vec![1, 1025, 3072]).await?,
			vec![
				FetchedPage {
					pgno: 1,
					bytes: Some(page(0x21)),
				},
				FetchedPage {
					pgno: 1025,
					bytes: Some(page(0x42)),
				},
				FetchedPage {
					pgno: 3072,
					bytes: Some(page(0x63)),
				},
			]
		);

		Ok(())
	}
}
