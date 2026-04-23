//! Commit paths for fast-path and staged writes.

use std::collections::BTreeMap;
use std::sync::atomic::Ordering;
use std::time::Instant;

use anyhow::{Context, Result, bail, ensure};
use scc::hash_map::Entry;
use tracing::Instrument;

use crate::engine::{PendingStage, SqliteEngine};
use crate::error::SqliteStorageError;
use crate::keys::{
	delta_chunk_key, delta_chunk_prefix, delta_prefix, meta_key, pidx_delta_key, pidx_delta_prefix,
	shard_prefix,
};
use crate::ltx::{LtxHeader, decode_ltx_v3, encode_ltx_v3};
use crate::quota::{encode_db_head_with_usage, tracked_storage_entry_size};
use crate::types::{DirtyPage, SQLITE_MAX_DELTA_BYTES, SqliteMeta, SqliteOrigin, decode_db_head};
use crate::udb;

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
pub struct CommitStageBeginRequest {
	pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitStageBeginResult {
	pub txid: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitStageRequest {
	pub generation: u64,
	pub txid: u64,
	pub chunk_idx: u32,
	pub bytes: Vec<u8>,
	pub is_last: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitStageResult {
	pub chunk_idx_committed: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitFinalizeRequest {
	pub generation: u64,
	pub expected_head_txid: u64,
	pub txid: u64,
	pub new_db_size_pages: u32,
	pub now_ms: i64,
	pub origin_override: Option<SqliteOrigin>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitFinalizeResult {
	pub new_head_txid: u64,
	pub meta: SqliteMeta,
	pub delta_bytes: u64,
}

#[derive(Debug, Default)]
struct TruncateCleanup {
	deleted_pidx_rows: Vec<(u32, Vec<u8>, Vec<u8>)>,
	deleted_delta_rows: Vec<(Vec<u8>, Vec<u8>)>,
	deleted_shard_rows: Vec<(Vec<u8>, Vec<u8>)>,
}

impl TruncateCleanup {
	fn tracked_deleted_bytes(&self) -> u64 {
		self.deleted_pidx_rows
			.iter()
			.map(|(_, key, value)| {
				tracked_storage_entry_size(key, value)
					.expect("pidx key should count toward sqlite quota")
			})
			.chain(self.deleted_delta_rows.iter().map(|(key, value)| {
				tracked_storage_entry_size(key, value)
					.expect("delta key should count toward sqlite quota")
			}))
			.chain(self.deleted_shard_rows.iter().map(|(key, value)| {
				tracked_storage_entry_size(key, value)
					.expect("shard key should count toward sqlite quota")
			}))
			.sum()
	}

	fn truncated_pgnos(&self) -> impl Iterator<Item = u32> + '_ {
		self.deleted_pidx_rows.iter().map(|(pgno, _, _)| *pgno)
	}
}

#[cfg(test)]
mod test_hooks {
	use std::sync::Mutex;

	use anyhow::{Result, anyhow};

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
			return Err(anyhow!(
				"InjectedStoreError: fast commit write transaction failed before commit"
			));
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
	#[tracing::instrument(
		level = "debug",
		skip(self, request),
		fields(path = "fast", dirty_pages = tracing::field::Empty)
	)]
	pub async fn commit(&self, actor_id: &str, request: CommitRequest) -> Result<CommitResult> {
		let start = Instant::now();
		let dirty_page_count = request.dirty_pages.len();
		tracing::Span::current().record("dirty_pages", dirty_page_count);
		let mut dirty_pgnos = request
			.dirty_pages
			.iter()
			.map(|page| page.pgno)
			.collect::<Vec<_>>();
		dirty_pgnos.sort_unstable();
		dirty_pgnos.dedup();
		let raw_dirty_bytes = dirty_pages_raw_bytes(&request.dirty_pages)?;
		if raw_dirty_bytes > SQLITE_MAX_DELTA_BYTES {
			return Err(SqliteStorageError::CommitTooLarge {
				actual_size_bytes: raw_dirty_bytes,
				max_size_bytes: SQLITE_MAX_DELTA_BYTES,
			}
			.into());
		}

		let actor_id = actor_id.to_string();
		let actor_id_for_tx = actor_id.clone();
		let subspace = self.subspace.clone();
		let op_count_before = self.op_counter.load(Ordering::Relaxed);
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
		let run_db_op_start = Instant::now();
		let (
			txid,
			head,
			delta_bytes,
			truncated_pgnos,
			meta_read_duration,
			ltx_encode_duration,
			pidx_read_duration,
		) = udb::run_db_op(self.db.as_ref(), self.op_counter.as_ref(), move |tx| {
			let actor_id = actor_id_for_tx.clone();
			let request = request.clone();
			let dirty_pgnos = dirty_pgnos_for_tx.clone();
			let subspace = subspace.clone();
			let cached_existing_pidx = cached_existing_pidx.clone();
			async move {
				let meta_read_start = Instant::now();
				let meta_storage_key = meta_key(&actor_id);
				let meta_bytes = async {
					udb::tx_get_value_serializable(&tx, &subspace, &meta_storage_key).await
				}
				.instrument(tracing::debug_span!("meta_read"))
				.await?
				.ok_or(SqliteStorageError::MetaMissing {
					operation: "commit",
				})?;
				let mut head = decode_db_head(&meta_bytes)?;
				let meta_read_duration = meta_read_start.elapsed();

				if head.generation != request.generation {
					return Err(SqliteStorageError::FenceMismatch {
						reason: format!(
							"commit generation {} did not match current generation {}",
							request.generation, head.generation
						),
					}
					.into());
				}
				if head.head_txid != request.head_txid {
					return Err(SqliteStorageError::FenceMismatch {
						reason: format!(
							"commit head_txid {} did not match current head_txid {}",
							request.head_txid, head.head_txid
						),
					}
					.into());
				}

				let txid = head.next_txid;
				ensure!(
					txid > head.head_txid,
					"next txid {} must advance past head txid {}",
					txid,
					head.head_txid
				);
				let truncate_cleanup = collect_truncate_cleanup(
					&tx,
					&subspace,
					&actor_id,
					head.db_size_pages,
					request.db_size_pages,
					head.shard_size,
				)
				.await?;

				let ltx_encode_start = Instant::now();
				let delta = {
					let _ltx_encode_span = tracing::debug_span!("ltx_encode").entered();
					encode_ltx_v3(
						LtxHeader::delta(txid, request.db_size_pages, request.now_ms),
						&request.dirty_pages,
					)
					.context("encode commit delta")?
				};
				let ltx_encode_duration = ltx_encode_start.elapsed();
				let delta_bytes = delta.len() as u64;

				head.head_txid = txid;
				head.next_txid += 1;
				head.db_size_pages = request.db_size_pages;

				let txid_bytes = txid.to_be_bytes();
				let mut usage_without_meta = head.sqlite_storage_used.saturating_sub(
					tracked_storage_entry_size(&meta_storage_key, &meta_bytes)
						.expect("meta key should count toward sqlite quota"),
				);
				usage_without_meta =
					usage_without_meta.saturating_sub(truncate_cleanup.tracked_deleted_bytes());
				usage_without_meta +=
					tracked_storage_entry_size(&delta_chunk_key(&actor_id, txid, 0), &delta)
						.expect("delta chunk key should count toward sqlite quota");
				let pidx_read_start = Instant::now();
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
				let pidx_read_duration = pidx_read_start.elapsed();
				for pgno in &dirty_pgnos {
					if !existing_pidx.get(pgno).copied().unwrap_or(false) {
						usage_without_meta += tracked_storage_entry_size(
							&pidx_delta_key(&actor_id, *pgno),
							&txid_bytes,
						)
						.expect("pidx key should count toward sqlite quota");
					}
				}

				udb::tx_write_value(&tx, &subspace, &delta_chunk_key(&actor_id, txid, 0), &delta)?;
				for pgno in &dirty_pgnos {
					udb::tx_write_value(
						&tx,
						&subspace,
						&pidx_delta_key(&actor_id, *pgno),
						&txid_bytes,
					)?;
				}
				for (_, key, _) in &truncate_cleanup.deleted_pidx_rows {
					udb::tx_delete_value(&tx, &subspace, key);
				}
				for (key, _) in &truncate_cleanup.deleted_delta_rows {
					udb::tx_delete_value(&tx, &subspace, key);
				}
				for (key, _) in &truncate_cleanup.deleted_shard_rows {
					udb::tx_delete_value(&tx, &subspace, key);
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

				Ok((
					txid,
					updated_head,
					delta_bytes,
					truncate_cleanup.truncated_pgnos().collect::<Vec<_>>(),
					meta_read_duration,
					ltx_encode_duration,
					pidx_read_duration,
				))
			}
		})
		.await
		.map_err(|err| {
			if matches!(
				err.downcast_ref::<SqliteStorageError>(),
				Some(SqliteStorageError::FenceMismatch { .. })
			) {
				self.metrics.inc_fence_mismatch_total();
			}
			err
		})?;
		let run_db_op_duration = run_db_op_start.elapsed();
		let udb_write_duration = run_db_op_duration
			.saturating_sub(meta_read_duration)
			.saturating_sub(ltx_encode_duration)
			.saturating_sub(pidx_read_duration);

		match self.page_indices.entry_async(actor_id.to_string()).await {
			Entry::Occupied(entry) => {
				for pgno in &truncated_pgnos {
					entry.get().remove(*pgno);
				}
				for pgno in dirty_pgnos {
					entry.get().insert(pgno, txid);
				}
			}
			Entry::Vacant(entry) => {
				drop(entry);
			}
		}

		let _ = self.compaction_tx.send(actor_id.to_string());
		self.metrics.set_delta_count_from_head(&head);
		let result = CommitResult {
			txid,
			meta: SqliteMeta::from((head, SQLITE_MAX_DELTA_BYTES)),
			delta_bytes,
		};
		let op_count_after = self.op_counter.load(Ordering::Relaxed);
		let udb_ops = op_count_after.saturating_sub(op_count_before);
		self.metrics
			.observe_commit_phase("fast", "meta_read", meta_read_duration);
		self.metrics
			.observe_commit_phase("fast", "ltx_encode", ltx_encode_duration);
		self.metrics
			.observe_commit_phase("fast", "pidx_read", pidx_read_duration);
		self.metrics
			.observe_commit_phase("fast", "udb_write", udb_write_duration);
		self.metrics
			.observe_commit_payload("fast", dirty_page_count, raw_dirty_bytes, udb_ops);
		self.metrics
			.observe_commit("fast", dirty_page_count, start.elapsed());
		self.metrics.inc_commit_total();

		Ok(result)
	}

	#[tracing::instrument(level = "debug", skip(self, request))]
	pub async fn commit_stage_begin(
		&self,
		actor_id: &str,
		request: CommitStageBeginRequest,
	) -> Result<CommitStageBeginResult> {
		let actor_id = actor_id.to_string();
		let actor_id_for_tx = actor_id.clone();
		let subspace = self.subspace.clone();
		let request = request.clone();
		let txid = udb::run_db_op(self.db.as_ref(), self.op_counter.as_ref(), move |tx| {
			let actor_id = actor_id_for_tx.clone();
			let subspace = subspace.clone();
			let request = request.clone();
			async move {
				let meta_storage_key = meta_key(&actor_id);
				let meta_bytes = udb::tx_get_value_serializable(&tx, &subspace, &meta_storage_key)
					.await?
					.ok_or(SqliteStorageError::MetaMissing {
						operation: "commit_stage_begin",
					})?;
				let mut head = decode_db_head(&meta_bytes)?;
				if head.generation != request.generation {
					return Err(SqliteStorageError::FenceMismatch {
						reason: format!(
							"commit_stage_begin generation {} did not match current generation {}",
							request.generation, head.generation
						),
					}
					.into());
				}

				let txid = head.next_txid;
				ensure!(
					txid > head.head_txid,
					"next txid {} must advance past head txid {}",
					txid,
					head.head_txid
				);
				head.next_txid += 1;
				let usage_without_meta = head.sqlite_storage_used.saturating_sub(
					tracked_storage_entry_size(&meta_storage_key, &meta_bytes)
						.expect("meta key should count toward sqlite quota"),
				);
				let (_, encoded_head) =
					encode_db_head_with_usage(&actor_id, &head, usage_without_meta)?;
				udb::tx_write_value(&tx, &subspace, &meta_storage_key, &encoded_head)?;

				Ok(txid)
			}
		})
		.await
		.map_err(|err| {
			if matches!(
				err.downcast_ref::<SqliteStorageError>(),
				Some(SqliteStorageError::FenceMismatch { .. })
			) {
				self.metrics.inc_fence_mismatch_total();
			}
			err
		})?;
		let _ = self.pending_stages.insert_sync(
			(actor_id, txid),
			std::sync::Arc::new(parking_lot::Mutex::new(PendingStage {
				next_chunk_idx: 0,
				saw_last_chunk: false,
				error_message: None,
			})),
		);

		Ok(CommitStageBeginResult { txid })
	}

	#[tracing::instrument(
		level = "debug",
		skip(self, request),
		fields(txid = request.txid, chunk_idx = request.chunk_idx, chunk_bytes = request.bytes.len())
	)]
	pub async fn commit_stage(
		&self,
		actor_id: &str,
		request: CommitStageRequest,
	) -> Result<CommitStageResult> {
		let decode_start = Instant::now();
		let stage_key = (actor_id.to_string(), request.txid);
		let pending_stage = self
			.pending_stages
			.get_async(&stage_key)
			.await
			.map(|entry| std::sync::Arc::clone(entry.get()))
			.ok_or(SqliteStorageError::StageNotFound {
				stage_id: request.txid,
			})?;
		{
			let stage = pending_stage.lock();
			if let Some(error_message) = stage.error_message.as_ref() {
				return Err(anyhow::anyhow!(error_message.clone()));
			}
			ensure!(
				!stage.saw_last_chunk,
				"commit_stage txid {} received chunk {} after final chunk",
				request.txid,
				request.chunk_idx
			);
			ensure!(
				stage.next_chunk_idx == request.chunk_idx,
				"commit_stage txid {} expected chunk {}, got {}",
				request.txid,
				stage.next_chunk_idx,
				request.chunk_idx
			);
		}
		let decode_duration = decode_start.elapsed();

		let actor_id = actor_id.to_string();
		let actor_id_for_tx = actor_id.clone();
		let subspace = self.subspace.clone();
		let request_for_tx = request.clone();
		let chunk_write_result =
			udb::run_db_op(self.db.as_ref(), self.op_counter.as_ref(), move |tx| {
				let actor_id = actor_id_for_tx.clone();
				let subspace = subspace.clone();
				let request = request_for_tx.clone();
				async move {
					let meta_storage_key = meta_key(&actor_id);
					let meta_bytes =
						udb::tx_get_value_serializable(&tx, &subspace, &meta_storage_key)
							.await?
							.ok_or(SqliteStorageError::MetaMissing {
								operation: "commit_stage",
							})?;
					let head = decode_db_head(&meta_bytes)?;
					if head.generation != request.generation {
						return Err(SqliteStorageError::FenceMismatch {
							reason: format!(
								"commit_stage generation {} did not match current generation {}",
								request.generation, head.generation
							),
						}
						.into());
					}
					if request.txid != head.next_txid.saturating_sub(1) {
						return Err(SqliteStorageError::StageNotFound {
							stage_id: request.txid,
						}
						.into());
					}
					ensure!(
						request.txid > head.head_txid,
						"commit_stage txid {} must be greater than current head txid {}",
						request.txid,
						head.head_txid
					);

					let chunk_key = delta_chunk_key(&actor_id, request.txid, request.chunk_idx);
					let existing_chunk = udb::tx_get_value(&tx, &subspace, &chunk_key).await?;
					let mut usage_without_meta = head.sqlite_storage_used.saturating_sub(
						tracked_storage_entry_size(&meta_storage_key, &meta_bytes)
							.expect("meta key should count toward sqlite quota"),
					);
					if let Some(existing_chunk) = existing_chunk.as_ref() {
						usage_without_meta = usage_without_meta.saturating_sub(
							tracked_storage_entry_size(&chunk_key, existing_chunk)
								.expect("delta chunk key should count toward sqlite quota"),
						);
					}
					usage_without_meta = usage_without_meta.saturating_add(
						tracked_storage_entry_size(&chunk_key, &request.bytes)
							.expect("delta chunk key should count toward sqlite quota"),
					);
					let (updated_head, encoded_head) =
						encode_db_head_with_usage(&actor_id, &head, usage_without_meta)?;
					if updated_head.sqlite_storage_used > updated_head.sqlite_max_storage {
						bail!(
							"SqliteStorageQuotaExceeded: sqlite storage used {} would exceed max {}",
							updated_head.sqlite_storage_used,
							updated_head.sqlite_max_storage
						);
					}
					udb::tx_write_value(&tx, &subspace, &chunk_key, &request.bytes)?;
					udb::tx_write_value(&tx, &subspace, &meta_storage_key, &encoded_head)?;

					Ok(())
				}
			})
			.await;
		let udb_write_duration = decode_start.elapsed().saturating_sub(decode_duration);

		match chunk_write_result {
			Ok(()) => {
				let mut stage = pending_stage.lock();
				stage.next_chunk_idx += 1;
				stage.saw_last_chunk = request.is_last;
			}
			Err(err) => {
				if matches!(
					err.downcast_ref::<SqliteStorageError>(),
					Some(SqliteStorageError::FenceMismatch { .. })
				) {
					self.metrics.inc_fence_mismatch_total();
				}
				pending_stage.lock().error_message = Some(err.to_string());
				return Err(err);
			}
		}

		self.metrics
			.observe_commit_stage_phase("decode", decode_duration);
		self.metrics
			.observe_commit_stage_phase("stage_encode", Default::default());
		self.metrics
			.observe_commit_stage_phase("udb_write", udb_write_duration);

		Ok(CommitStageResult {
			chunk_idx_committed: request.chunk_idx,
		})
	}

	#[tracing::instrument(
		level = "debug",
		skip(self, request),
		fields(path = "slow", txid = request.txid)
	)]
	pub async fn commit_finalize(
		&self,
		actor_id: &str,
		request: CommitFinalizeRequest,
	) -> Result<CommitFinalizeResult> {
		let start = Instant::now();
		let stage_key = (actor_id.to_string(), request.txid);
		let pending_stage = self
			.pending_stages
			.get_async(&stage_key)
			.await
			.map(|entry| std::sync::Arc::clone(entry.get()))
			.ok_or(SqliteStorageError::StageNotFound {
				stage_id: request.txid,
			})?;
		{
			let stage = pending_stage.lock();
			if let Some(error_message) = stage.error_message.as_ref() {
				return Err(anyhow::anyhow!(error_message.clone()));
			}
			if !stage.saw_last_chunk {
				return Err(SqliteStorageError::StageNotFound {
					stage_id: request.txid,
				}
				.into());
			}
		}

		let actor_id = actor_id.to_string();
		let actor_id_for_tx = actor_id.clone();
		let subspace = self.subspace.clone();
		let request_for_tx = request.clone();
		let (
			head,
			staged_pgnos,
			truncated_pgnos,
			meta_read_duration,
			stage_load_duration,
			pidx_read_duration,
			pidx_write_duration,
			meta_write_duration,
		) = udb::run_db_op(self.db.as_ref(), self.op_counter.as_ref(), move |tx| {
			let actor_id = actor_id_for_tx.clone();
			let subspace = subspace.clone();
			let request = request_for_tx.clone();
			async move {
				let meta_storage_key = meta_key(&actor_id);
				let meta_read_start = Instant::now();
				let meta_bytes = udb::tx_get_value_serializable(&tx, &subspace, &meta_storage_key)
					.await?
					.ok_or(SqliteStorageError::MetaMissing {
						operation: "commit_finalize",
					})?;
				let mut head = decode_db_head(&meta_bytes)?;
				let meta_read_duration = meta_read_start.elapsed();
				if head.generation != request.generation {
					return Err(SqliteStorageError::FenceMismatch {
						reason: format!(
							"commit_finalize generation {} did not match current generation {}",
							request.generation, head.generation
						),
					}
					.into());
				}
				if head.head_txid != request.expected_head_txid {
					return Err(SqliteStorageError::FenceMismatch {
						reason: format!(
							"commit_finalize head_txid {} did not match current head_txid {}",
							request.expected_head_txid, head.head_txid
						),
					}
					.into());
				}
				if request.txid != head.next_txid.saturating_sub(1) {
					return Err(SqliteStorageError::StageNotFound {
						stage_id: request.txid,
					}
					.into());
				}

				// Read staged DELTA chunks and decode LTX to recover the page list for
				// this txid. Without writing PIDX entries here, reads after finalize
				// fall through `recover_page_from_delta_history` (full delta scan)
				// until compaction folds the delta.
				let stage_load_start = Instant::now();
				let delta_chunks = udb::tx_scan_prefix_values(
					&tx,
					&subspace,
					&delta_chunk_prefix(&actor_id, request.txid),
				)
				.await?;
				ensure!(
					!delta_chunks.is_empty(),
					"commit_finalize found no staged DELTA chunks for txid {}",
					request.txid,
				);
				let mut delta_blob = Vec::new();
				for (_, chunk) in &delta_chunks {
					delta_blob.extend_from_slice(chunk);
				}
				let decoded = decode_ltx_v3(&delta_blob)
					.context("decode staged delta for commit_finalize")?;
				let staged_pgnos: Vec<u32> =
					decoded.page_index.iter().map(|entry| entry.pgno).collect();
				let stage_load_duration = stage_load_start.elapsed();

				// Check which PIDX entries already exist so we only add quota for new ones.
				let pidx_read_start = Instant::now();
				let mut existing_pidx = BTreeMap::<u32, bool>::new();
				for pgno in &staged_pgnos {
					existing_pidx.insert(
						*pgno,
						udb::tx_get_value(&tx, &subspace, &pidx_delta_key(&actor_id, *pgno))
							.await?
							.is_some(),
					);
				}
				let pidx_read_duration = pidx_read_start.elapsed();
				let truncate_cleanup = collect_truncate_cleanup(
					&tx,
					&subspace,
					&actor_id,
					head.db_size_pages,
					request.new_db_size_pages,
					head.shard_size,
				)
				.await?;

				head.head_txid = request.txid;
				head.db_size_pages = request.new_db_size_pages;
				if let Some(origin_override) = request.origin_override {
					head.origin = origin_override;
				}

				let txid_bytes = request.txid.to_be_bytes();
				let mut usage_without_meta = head.sqlite_storage_used.saturating_sub(
					tracked_storage_entry_size(&meta_storage_key, &meta_bytes)
						.expect("meta key should count toward sqlite quota"),
				);
				usage_without_meta =
					usage_without_meta.saturating_sub(truncate_cleanup.tracked_deleted_bytes());
				for pgno in &staged_pgnos {
					if !existing_pidx.get(pgno).copied().unwrap_or(false) {
						usage_without_meta += tracked_storage_entry_size(
							&pidx_delta_key(&actor_id, *pgno),
							&txid_bytes,
						)
						.expect("pidx key should count toward sqlite quota");
					}
				}

				let pidx_write_start = Instant::now();
				for pgno in &staged_pgnos {
					udb::tx_write_value(
						&tx,
						&subspace,
						&pidx_delta_key(&actor_id, *pgno),
						&txid_bytes,
					)?;
				}
				for (_, key, _) in &truncate_cleanup.deleted_pidx_rows {
					udb::tx_delete_value(&tx, &subspace, key);
				}
				for (key, _) in &truncate_cleanup.deleted_delta_rows {
					udb::tx_delete_value(&tx, &subspace, key);
				}
				for (key, _) in &truncate_cleanup.deleted_shard_rows {
					udb::tx_delete_value(&tx, &subspace, key);
				}
				let pidx_write_duration = pidx_write_start.elapsed();

				let (updated_head, encoded_head) =
					encode_db_head_with_usage(&actor_id, &head, usage_without_meta)?;
				if updated_head.sqlite_storage_used > updated_head.sqlite_max_storage {
					bail!(
						"SqliteStorageQuotaExceeded: sqlite storage used {} would exceed max {}",
						updated_head.sqlite_storage_used,
						updated_head.sqlite_max_storage
					);
				}
				let meta_write_start = Instant::now();
				udb::tx_write_value(&tx, &subspace, &meta_storage_key, &encoded_head)?;
				let meta_write_duration = meta_write_start.elapsed();

				Ok((
					updated_head,
					staged_pgnos,
					truncate_cleanup.truncated_pgnos().collect::<Vec<_>>(),
					meta_read_duration,
					stage_load_duration,
					pidx_read_duration,
					pidx_write_duration,
					meta_write_duration,
				))
			}
		})
		.await
		.map_err(|err| {
			if matches!(
				err.downcast_ref::<SqliteStorageError>(),
				Some(SqliteStorageError::FenceMismatch { .. })
			) {
				self.metrics.inc_fence_mismatch_total();
			}
			err
		})?;

		// Update the in-memory PIDX cache so subsequent reads skip the store scan.
		match self.page_indices.entry_async(actor_id.to_string()).await {
			Entry::Occupied(entry) => {
				for pgno in &truncated_pgnos {
					entry.get().remove(*pgno);
				}
				for pgno in &staged_pgnos {
					entry.get().insert(*pgno, request.txid);
				}
			}
			Entry::Vacant(entry) => {
				drop(entry);
			}
		}

		let _ = self.pending_stages.remove_async(&stage_key).await;
		let _ = self.compaction_tx.send(actor_id.clone());
		self.metrics.set_delta_count_from_head(&head);
		self.metrics
			.observe_commit_finalize_phase("stage_promote", stage_load_duration);
		self.metrics
			.observe_commit_finalize_phase("pidx_write", pidx_write_duration);
		self.metrics
			.observe_commit_finalize_phase("meta_write", meta_write_duration);
		self.metrics
			.observe_commit_phase("slow", "meta_read", meta_read_duration);
		self.metrics
			.observe_commit_phase("slow", "ltx_encode", Default::default());
		self.metrics
			.observe_commit_phase("slow", "pidx_read", pidx_read_duration);
		self.metrics.observe_commit_phase(
			"slow",
			"udb_write",
			pidx_write_duration.saturating_add(meta_write_duration),
		);
		self.metrics
			.observe_commit_payload("slow", staged_pgnos.len(), 0, 1);
		self.metrics
			.observe_commit("slow", staged_pgnos.len(), start.elapsed());
		self.metrics.inc_commit_total();

		Ok(CommitFinalizeResult {
			new_head_txid: request.txid,
			meta: SqliteMeta::from((head, SQLITE_MAX_DELTA_BYTES)),
			delta_bytes: 0,
		})
	}
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

async fn collect_truncate_cleanup(
	tx: &universaldb::Transaction,
	subspace: &universaldb::Subspace,
	actor_id: &str,
	previous_db_size_pages: u32,
	new_db_size_pages: u32,
	shard_size: u32,
) -> Result<TruncateCleanup> {
	if new_db_size_pages >= previous_db_size_pages {
		return Ok(TruncateCleanup::default());
	}

	let pidx_rows = udb::tx_scan_prefix_values(tx, subspace, &pidx_delta_prefix(actor_id)).await?;
	let mut retained_txids = BTreeMap::<u64, usize>::new();
	let mut truncated_txids = BTreeMap::<u64, usize>::new();
	let mut cleanup = TruncateCleanup::default();

	for (key, value) in pidx_rows {
		let pgno = decode_pidx_pgno(actor_id, &key)?;
		let txid = decode_pidx_txid(&value)?;
		if pgno > new_db_size_pages {
			*truncated_txids.entry(txid).or_default() += 1;
			cleanup.deleted_pidx_rows.push((pgno, key, value));
		} else {
			*retained_txids.entry(txid).or_default() += 1;
		}
	}

	if !truncated_txids.is_empty() {
		for (key, value) in
			udb::tx_scan_prefix_values(tx, subspace, &delta_prefix(actor_id)).await?
		{
			let txid = crate::keys::decode_delta_chunk_txid(actor_id, &key)?;
			if truncated_txids.contains_key(&txid) && !retained_txids.contains_key(&txid) {
				cleanup.deleted_delta_rows.push((key, value));
			}
		}
	}

	for (key, value) in udb::tx_scan_prefix_values(tx, subspace, &shard_prefix(actor_id)).await? {
		let shard_id = decode_shard_id(actor_id, &key)?;
		if shard_id.saturating_mul(shard_size) > new_db_size_pages {
			cleanup.deleted_shard_rows.push((key, value));
		}
	}

	Ok(cleanup)
}

fn decode_pidx_pgno(actor_id: &str, key: &[u8]) -> Result<u32> {
	let prefix = pidx_delta_prefix(actor_id);
	ensure!(
		key.starts_with(&prefix),
		"pidx key did not start with expected prefix"
	);

	let suffix = &key[prefix.len()..];
	ensure!(
		suffix.len() == std::mem::size_of::<u32>(),
		"pidx key suffix had {} bytes, expected {}",
		suffix.len(),
		std::mem::size_of::<u32>()
	);

	Ok(u32::from_be_bytes(
		suffix
			.try_into()
			.context("pidx key suffix should decode as u32")?,
	))
}

fn decode_pidx_txid(value: &[u8]) -> Result<u64> {
	ensure!(
		value.len() == std::mem::size_of::<u64>(),
		"pidx value had {} bytes, expected {}",
		value.len(),
		std::mem::size_of::<u64>()
	);

	Ok(u64::from_be_bytes(
		value
			.try_into()
			.context("pidx value should decode as u64")?,
	))
}

fn decode_shard_id(actor_id: &str, key: &[u8]) -> Result<u32> {
	let prefix = shard_prefix(actor_id);
	ensure!(
		key.starts_with(&prefix),
		"shard key did not start with expected prefix"
	);

	let suffix = &key[prefix.len()..];
	ensure!(
		suffix.len() == std::mem::size_of::<u32>(),
		"shard key suffix had {} bytes, expected {}",
		suffix.len(),
		std::mem::size_of::<u32>()
	);

	Ok(u32::from_be_bytes(
		suffix
			.try_into()
			.context("shard key suffix should decode as u32")?,
	))
}

#[cfg(test)]
mod tests {
	use anyhow::Result;
	use rivet_metrics::REGISTRY;
	use rivet_metrics::prometheus::{Encoder, TextEncoder};
	use tokio::sync::mpsc::error::TryRecvError;

	use super::{
		CommitFinalizeRequest, CommitRequest, CommitStageRequest, decode_db_head, test_hooks,
	};
	use crate::engine::SqliteEngine;
	use crate::error::SqliteStorageError;
	use crate::keys::{
		delta_chunk_key, delta_chunk_prefix, meta_key, pidx_delta_key, pidx_delta_prefix, shard_key,
	};
	use crate::ltx::{LtxHeader, encode_ltx_v3};
	use crate::quota::{encode_db_head_with_usage, tracked_storage_entry_size};
	use crate::test_utils::{
		assert_op_count, clear_op_count, read_value, scan_prefix_values, test_db,
	};
	use crate::types::{
		DBHead, DirtyPage, FetchedPage, SQLITE_DEFAULT_MAX_STORAGE_BYTES, SQLITE_PAGE_SIZE,
		SQLITE_SHARD_SIZE, SQLITE_VFS_V2_SCHEMA_VERSION, SqliteOrigin,
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
			origin: SqliteOrigin::Native,
		}
	}

	fn page(fill: u8) -> Vec<u8> {
		vec![fill; SQLITE_PAGE_SIZE as usize]
	}

	fn delta_blob_key(actor_id: &str, txid: u64) -> Vec<u8> {
		delta_chunk_key(actor_id, txid, 0)
	}

	async fn read_delta_blob(
		engine: &SqliteEngine,
		actor_id: &str,
		txid: u64,
	) -> Result<Option<Vec<u8>>> {
		let chunks = scan_prefix_values(engine, delta_chunk_prefix(actor_id, txid)).await?;
		if chunks.is_empty() {
			return Ok(None);
		}

		let mut blob = Vec::new();
		for (_, chunk) in chunks {
			blob.extend_from_slice(&chunk);
		}
		Ok(Some(blob))
	}

	async fn stage_encoded_delta(
		engine: &SqliteEngine,
		actor_id: &str,
		generation: u64,
		expected_head_txid: u64,
		new_db_size_pages: u32,
		now_ms: i64,
		pages: Vec<DirtyPage>,
		max_chunk_bytes: usize,
	) -> Result<u64> {
		let stage_begin = engine
			.commit_stage_begin(actor_id, super::CommitStageBeginRequest { generation })
			.await?;
		let encoded = encode_ltx_v3(
			LtxHeader::delta(stage_begin.txid, new_db_size_pages, now_ms),
			&pages,
		)?;
		for (chunk_idx, chunk) in encoded.chunks(max_chunk_bytes).enumerate() {
			engine
				.commit_stage(
					actor_id,
					CommitStageRequest {
						generation,
						txid: stage_begin.txid,
						chunk_idx: chunk_idx as u32,
						bytes: chunk.to_vec(),
						is_last: chunk_idx + 1 == encoded.chunks(max_chunk_bytes).count(),
					},
				)
				.await?;
		}
		engine
			.commit_finalize(
				actor_id,
				CommitFinalizeRequest {
					generation,
					expected_head_txid,
					txid: stage_begin.txid,
					new_db_size_pages,
					now_ms,
					origin_override: None,
				},
			)
			.await?;
		Ok(stage_begin.txid)
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

	async fn rewrite_meta_with_actual_usage(engine: &SqliteEngine, actor_id: &str) -> Result<()> {
		let meta_key = meta_key(actor_id);
		let meta_bytes = read_value(engine, meta_key.clone())
			.await?
			.expect("meta should exist before rewrite");
		let head = decode_db_head(&meta_bytes)?;
		let usage_without_meta = actual_tracked_usage(engine).await?.saturating_sub(
			tracked_storage_entry_size(&meta_key, &meta_bytes)
				.expect("meta key should count toward sqlite quota"),
		);
		let (_, rewritten_meta) = encode_db_head_with_usage(actor_id, &head, usage_without_meta)?;
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![WriteOp::put(meta_key, rewritten_meta)],
		)
		.await?;
		Ok(())
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

	fn pages_slice(start_pgno: u32, page_count: u32, fill: u8) -> Vec<DirtyPage> {
		(0..page_count)
			.map(|offset| DirtyPage {
				pgno: start_pgno + offset,
				bytes: page(fill),
			})
			.collect()
	}

	fn registry_text() -> String {
		let encoder = TextEncoder::new();
		let metric_families = REGISTRY.gather();
		let mut buffer = Vec::new();
		encoder
			.encode(&metric_families, &mut buffer)
			.expect("encode metrics");
		String::from_utf8(buffer).expect("prometheus output should be utf8")
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

		let stored_delta = read_delta_blob(&engine, TEST_ACTOR, 1)
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
	async fn commit_shrink_reclaims_truncated_rows_and_usage() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		let mut head = seeded_head();
		head.head_txid = 3;
		head.next_txid = 4;
		head.db_size_pages = 130;
		write_seeded_meta(&engine, TEST_ACTOR, head).await?;
		apply_write_ops(
			engine.db.as_ref(),
			&engine.subspace,
			engine.op_counter.as_ref(),
			vec![
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 1),
					encode_ltx_v3(
						LtxHeader::delta(1, 130, 1_000),
						&[DirtyPage {
							pgno: 2,
							bytes: page(0x12),
						}],
					)?,
				),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 2),
					encode_ltx_v3(
						LtxHeader::delta(2, 130, 1_001),
						&[
							DirtyPage {
								pgno: 70,
								bytes: page(0x70),
							},
							DirtyPage {
								pgno: 71,
								bytes: page(0x71),
							},
						],
					)?,
				),
				WriteOp::put(
					delta_blob_key(TEST_ACTOR, 3),
					encode_ltx_v3(
						LtxHeader::delta(3, 130, 1_002),
						&[DirtyPage {
							pgno: 130,
							bytes: page(0x82),
						}],
					)?,
				),
				WriteOp::put(
					shard_key(TEST_ACTOR, 2),
					encode_ltx_v3(
						LtxHeader::delta(3, 130, 1_002),
						&[
							DirtyPage {
								pgno: 129,
								bytes: page(0x91),
							},
							DirtyPage {
								pgno: 130,
								bytes: page(0x92),
							},
						],
					)?,
				),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 2), 1_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 70), 2_u64.to_be_bytes().to_vec()),
				WriteOp::put(pidx_delta_key(TEST_ACTOR, 71), 2_u64.to_be_bytes().to_vec()),
				WriteOp::put(
					pidx_delta_key(TEST_ACTOR, 130),
					3_u64.to_be_bytes().to_vec(),
				),
			],
		)
		.await?;
		rewrite_meta_with_actual_usage(&engine, TEST_ACTOR).await?;
		let before_usage = actual_tracked_usage(&engine).await?;
		let cached_index = engine.get_or_load_pidx(TEST_ACTOR).await?;
		assert_eq!(cached_index.get().get(70), Some(2));
		drop(cached_index);

		let result = engine
			.commit(
				TEST_ACTOR,
				CommitRequest {
					generation: 4,
					head_txid: 3,
					db_size_pages: 2,
					dirty_pages: vec![DirtyPage {
						pgno: 1,
						bytes: page(0x01),
					}],
					now_ms: 2_000,
				},
			)
			.await?;

		assert!(
			read_value(&engine, pidx_delta_key(TEST_ACTOR, 70))
				.await?
				.is_none()
		);
		assert!(
			read_value(&engine, pidx_delta_key(TEST_ACTOR, 71))
				.await?
				.is_none()
		);
		assert!(
			read_value(&engine, pidx_delta_key(TEST_ACTOR, 130))
				.await?
				.is_none()
		);
		assert_eq!(
			read_value(&engine, pidx_delta_key(TEST_ACTOR, 2)).await?,
			Some(1_u64.to_be_bytes().to_vec())
		);
		assert!(
			read_value(&engine, delta_blob_key(TEST_ACTOR, 2))
				.await?
				.is_none()
		);
		assert!(
			read_value(&engine, delta_blob_key(TEST_ACTOR, 3))
				.await?
				.is_none()
		);
		assert!(
			read_value(&engine, shard_key(TEST_ACTOR, 2))
				.await?
				.is_none()
		);

		let after_usage = actual_tracked_usage(&engine).await?;
		let stored_head = decode_db_head(
			&read_value(&engine, meta_key(TEST_ACTOR))
				.await?
				.expect("meta should exist after shrink commit"),
		)?;
		assert!(after_usage < before_usage);
		assert_eq!(result.meta.sqlite_storage_used, after_usage);
		assert_eq!(stored_head.sqlite_storage_used, after_usage);

		let cached_index = engine.get_or_load_pidx(TEST_ACTOR).await?;
		assert_eq!(cached_index.get().get(70), None);
		assert_eq!(cached_index.get().get(130), None);
		assert_eq!(cached_index.get().get(2), Some(1));

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
			read_value(&engine, delta_blob_key(TEST_ACTOR, 1))
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
			read_value(&engine, delta_blob_key(FAIL_ACTOR, 1))
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
		assert!(matches!(
			error.downcast_ref::<SqliteStorageError>(),
			Some(SqliteStorageError::FenceMismatch { .. })
		));
		assert_op_count(&engine, 1);
		assert!(
			read_value(&engine, delta_blob_key(TEST_ACTOR, 1))
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
		assert!(matches!(
			error.downcast_ref::<SqliteStorageError>(),
			Some(SqliteStorageError::FenceMismatch { .. })
		));
		assert_op_count(&engine, 1);
		assert!(
			read_value(&engine, delta_blob_key(TEST_ACTOR, 8))
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

		let txid = stage_encoded_delta(
			&engine,
			TEST_ACTOR,
			4,
			0,
			70,
			1_234,
			vec![
				DirtyPage {
					pgno: 1,
					bytes: page(0x11),
				},
				DirtyPage {
					pgno: 2,
					bytes: page(0x22),
				},
				DirtyPage {
					pgno: 70,
					bytes: page(0x70),
				},
			],
			32,
		)
		.await?;

		assert_eq!(txid, 1);
		assert_eq!(compaction_rx.recv().await, Some(TEST_ACTOR.to_string()));
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
					txid: 999,
					new_db_size_pages: 1,
					now_ms: 777,
					origin_override: None,
				},
			)
			.await
			.expect_err("missing stage should fail");
		assert_eq!(
			error.downcast_ref::<SqliteStorageError>(),
			Some(&SqliteStorageError::StageNotFound { stage_id: 999 })
		);
		assert_op_count(&engine, 0);
		assert!(read_delta_blob(&engine, TEST_ACTOR, 1).await?.is_none());
		assert!(matches!(compaction_rx.try_recv(), Err(TryRecvError::Empty)));

		Ok(())
	}

	#[tokio::test]
	async fn commit_finalize_writes_pidx_entries_for_staged_pages() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		write_seeded_meta(&engine, TEST_ACTOR, seeded_head()).await?;
		clear_op_count(&engine);

		let staged_pgnos = vec![1u32, 17, 4096];
		let pages = staged_pgnos
			.iter()
			.enumerate()
			.map(|(i, pgno)| DirtyPage {
				pgno: *pgno,
				bytes: page(0x30 + i as u8),
			})
			.collect::<Vec<_>>();
		let txid = stage_encoded_delta(&engine, TEST_ACTOR, 4, 0, 4096, 9_000, pages, 128).await?;
		assert_eq!(txid, 1);

		// After finalize, every staged pgno must have a PIDX entry pointing at txid.
		let pidx_rows = scan_prefix_values(&engine, pidx_delta_prefix(TEST_ACTOR)).await?;
		assert_eq!(pidx_rows.len(), staged_pgnos.len());
		let expected_txid_bytes = txid.to_be_bytes();
		for pgno in &staged_pgnos {
			let value = read_value(&engine, pidx_delta_key(TEST_ACTOR, *pgno))
				.await?
				.expect("pidx entry should exist after finalize");
			assert_eq!(
				value.as_slice(),
				&expected_txid_bytes,
				"pidx entry for pgno {} should point at finalize txid",
				pgno
			);
		}

		Ok(())
	}

	#[tokio::test]
	async fn commit_finalize_only_mutates_meta_and_pidx() -> Result<()> {
		// Finalize should not delete or rewrite staged DELTA chunks. The DELTA blob
		// stays in place after finalize and is consumed later by compaction. This
		// keeps finalize mutations proportional to the page count, not the blob size.
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		write_seeded_meta(&engine, TEST_ACTOR, seeded_head()).await?;

		let pages = vec![
			DirtyPage {
				pgno: 1,
				bytes: page(0xAA),
			},
			DirtyPage {
				pgno: 7,
				bytes: page(0xBB),
			},
			DirtyPage {
				pgno: 42,
				bytes: page(0xCC),
			},
		];
		let txid =
			stage_encoded_delta(&engine, TEST_ACTOR, 4, 0, 42, 8_000, pages.clone(), 64).await?;

		// Staged DELTA chunks must survive finalize so compaction can fold them later.
		let delta_chunks =
			scan_prefix_values(&engine, delta_chunk_prefix(TEST_ACTOR, txid)).await?;
		assert!(
			!delta_chunks.is_empty(),
			"finalize must not delete staged DELTA chunks"
		);

		// PIDX rows must exactly cover the staged pgnos.
		let mut pidx_rows = scan_prefix_values(&engine, pidx_delta_prefix(TEST_ACTOR)).await?;
		pidx_rows.sort_by(|a, b| a.0.cmp(&b.0));
		assert_eq!(pidx_rows.len(), pages.len());

		Ok(())
	}

	#[tokio::test]
	async fn commit_finalize_keeps_pidx_entries_that_already_existed() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		write_seeded_meta(&engine, TEST_ACTOR, seeded_head()).await?;

		// Fast-path commit to seed PIDX entries for pgno 1 at txid 1.
		engine.commit(TEST_ACTOR, request(4, 0)).await?;

		// Slow-path commit updates pgno 1 and adds pgno 2 at txid 2.
		clear_op_count(&engine);
		let txid = stage_encoded_delta(
			&engine,
			TEST_ACTOR,
			4,
			1,
			2,
			5_000,
			vec![
				DirtyPage {
					pgno: 1,
					bytes: page(0xAA),
				},
				DirtyPage {
					pgno: 2,
					bytes: page(0xBB),
				},
			],
			64,
		)
		.await?;
		assert_eq!(txid, 2);

		let txid_bytes = txid.to_be_bytes();
		for pgno in [1u32, 2u32] {
			let value = read_value(&engine, pidx_delta_key(TEST_ACTOR, pgno))
				.await?
				.expect("pidx entry should exist after finalize");
			assert_eq!(
				value.as_slice(),
				&txid_bytes,
				"pidx entry for pgno {} should point at latest txid",
				pgno
			);
		}

		Ok(())
	}

	#[tokio::test]
	async fn commit_finalize_accepts_12_mib_staged_delta() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, mut compaction_rx) = SqliteEngine::new(db, subspace);
		write_seeded_meta(&engine, TEST_ACTOR, seeded_head()).await?;
		clear_op_count(&engine);

		let txid = stage_encoded_delta(
			&engine,
			TEST_ACTOR,
			4,
			0,
			3072,
			2_468,
			[
				pages_slice(1, 1024, 0x21),
				pages_slice(1025, 1024, 0x42),
				pages_slice(2049, 1024, 0x63),
			]
			.concat(),
			256 * 1024,
		)
		.await?;

		assert_eq!(txid, 1);
		assert_eq!(compaction_rx.recv().await, Some(TEST_ACTOR.to_string()));
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

	#[tokio::test]
	async fn commit_registers_phase_metrics() -> Result<()> {
		let (db, subspace) = test_db().await?;
		let (engine, _compaction_rx) = SqliteEngine::new(db, subspace);
		write_seeded_meta(&engine, TEST_ACTOR, seeded_head()).await?;

		engine.commit(TEST_ACTOR, request(4, 0)).await?;
		stage_encoded_delta(
			&engine,
			TEST_ACTOR,
			4,
			1,
			1,
			2_000,
			vec![DirtyPage {
				pgno: 1,
				bytes: page(0x11),
			}],
			64,
		)
		.await?;

		let metrics = registry_text();
		assert!(metrics.contains("sqlite_commit_phase_duration_seconds"));
		assert!(metrics.contains("phase=\"meta_read\""));
		assert!(metrics.contains("phase=\"ltx_encode\""));
		assert!(metrics.contains("phase=\"pidx_read\""));
		assert!(metrics.contains("phase=\"udb_write\""));
		assert!(metrics.contains("path=\"fast\""));
		assert!(metrics.contains("sqlite_commit_stage_phase_duration_seconds"));
		assert!(metrics.contains("phase=\"decode\""));
		assert!(metrics.contains("phase=\"stage_encode\""));
		assert!(metrics.contains("phase=\"udb_write\""));
		assert!(metrics.contains("sqlite_commit_finalize_phase_duration_seconds"));
		assert!(metrics.contains("phase=\"stage_promote\""));
		assert!(metrics.contains("phase=\"pidx_write\""));
		assert!(metrics.contains("phase=\"meta_write\""));
		assert!(metrics.contains("path=\"slow\""));
		assert!(metrics.contains("sqlite_commit_dirty_page_count"));
		assert!(metrics.contains("sqlite_commit_dirty_bytes"));
		assert!(metrics.contains("sqlite_udb_ops_per_commit"));

		Ok(())
	}
}
