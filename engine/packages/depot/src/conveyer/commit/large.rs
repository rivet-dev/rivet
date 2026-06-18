use std::{
	collections::{BTreeMap, BTreeSet},
	sync::Arc,
};

use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use universaldb::{
	options::MutationType,
	utils::IsolationLevel::{Serializable, Snapshot},
};
use uuid::Uuid;

use crate::{
	burst_mode,
	conveyer::{
		Db, branch,
		constants::DELTA_OBJECT_CHUNK_BYTES,
		db::{BranchAncestry, CacheSnapshot, load_branch_ancestry, touch_access_if_bucket_advanced},
		error::SqliteStorageError,
		keys,
		ltx::{LtxEncoder, LtxHeader, LtxPageIndexEntry},
		page_index::DeltaPageIndex,
		quota,
		types::{
			BranchState, CommitOptions, CommitResult, CommitRow, CommitStageBeginResult,
			CommitStageComplete, CommitStageFinalized, CommitStageMeta, CommitStageState, DBHead,
			DatabaseBranchId, DeltaManifest, DeltaObjectMeta, DeltaObjectState,
			DeltaPageIndexEntry, DirtyPage, DirtyPageBatch, decode_commit_stage_complete,
			decode_commit_stage_finalized, decode_commit_stage_meta, decode_compaction_root,
			decode_database_branch_record, decode_db_head, decode_delta_object_meta,
			decode_dirty_page_batch, encode_commit_row, encode_commit_stage_complete,
			encode_commit_stage_finalized, encode_commit_stage_meta, encode_db_head,
			encode_delta_manifest, encode_delta_object_meta, encode_delta_page_index_entry,
			encode_dirty_page_batch,
		},
		udb,
	},
	workflows::compaction::DeltasAvailable,
};

use super::{
	branch_init::{BranchResolution, resolve_or_allocate_branch, write_root_branch_metadata},
	dirty::admit_deltas_available,
	helpers::{tracked_entry_size, tx_get_value, tx_scan_prefix_values},
	test_hooks,
	truncate::{collect_truncate_cleanup, fence_truncate_cleanup_row},
};

pub(super) const MAX_SQLITE_COMMIT_DIRTY_PAGES: usize = 8_192;
pub(super) const MAX_SQLITE_COMMIT_DIRTY_BYTES: usize = 32 * 1024 * 1024;
pub(super) const SLOW_COMMIT_DIRTY_BYTES_THRESHOLD: usize = 8 * 1024 * 1024;

const DELTA_OBJECT_WRITE_BATCH_BYTES: usize = 8 * 1024 * 1024;
const STAGE_TTL_MS: i64 = 24 * 60 * 60 * 1_000;
const MAX_STAGE_PAGES_PER_BATCH: usize = 16;
const MAX_STAGE_BATCH_BYTES: usize = 96 * 1024;
const STAGE_RESERVATION_HEADROOM_BYTES: i64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CommitStageGcOutcome {
	pub stages_deleted: usize,
	pub finalized_tombstones_deleted: usize,
	pub stale_lookups_deleted: usize,
	pub quota_released_bytes: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OrphanDeltaObjectGcOutcome {
	pub stages_deleted: usize,
	pub orphan_objects_deleted: usize,
	pub quota_released_bytes: i64,
}

impl Db {
	pub(super) async fn commit_slow_with_options(
		&self,
		dirty_pages: Vec<DirtyPage>,
		db_size_pages: u32,
		now_ms: i64,
		options: CommitOptions,
	) -> Result<CommitResult> {
		let cached_storage_used = *self.storage_used.read().await;
		let cached_snapshot = self.cache_snapshot.read().await.clone();
		let cached_branch_id = cached_snapshot.as_ref().map(|snapshot| snapshot.branch_id);
		let cached_ancestry = cached_snapshot
			.as_ref()
			.map(|snapshot| snapshot.ancestors.clone());
		let cached_access_bucket = cached_snapshot
			.as_ref()
			.and_then(|snapshot| snapshot.last_access_bucket);
		let last_deltas_available_at_ms = *self.last_deltas_available_at_ms.read().await;
		let cache_was_warm = cached_snapshot
			.as_ref()
			.is_some_and(|snapshot| !snapshot.pidx.range(0, u32::MAX).is_empty());

		let preflight = self
			.prepare_slow_commit(
				cached_branch_id,
				cached_ancestry,
				options.expected_head_txid,
			)
			.await?;
		let staged_txid = preflight
			.observed_head_txid
			.checked_add(1)
			.context("sqlite head txid overflowed")?;
		let object = build_delta_object(staged_txid, db_size_pages, now_ms, &dirty_pages)?;

		tracing::debug!(
			database_id = %self.database_id,
			branch_id = ?preflight.branch_id,
			stage_id = %object.stage_id,
			object_id = %object.object_id,
			dirty_page_count = dirty_pages.len(),
			staged_bytes = object.encoded_len,
			staged_txid,
			"sqlite large commit writing staged object"
		);
		self.write_delta_object(preflight.branch_id, &object).await?;

		let result = self
			.finalize_slow_commit(
				preflight.clone(),
				object,
				dirty_pages,
				db_size_pages,
				now_ms,
				options,
				cached_storage_used,
				cached_access_bucket,
				last_deltas_available_at_ms,
				None,
			)
			.await
			.map_err(map_udb_commit_error)?;

		*self.storage_used.write().await = Some(result.storage_used);
		self.commit_bytes_since_rollup.fetch_add(
			u64::try_from(result.added_bytes)
				.context("commit added bytes should be non-negative")?,
			std::sync::atomic::Ordering::Relaxed,
		);

		let mut cache_snapshot = self.cache_snapshot.write().await;
		let current_branch_id = cache_snapshot.as_ref().map(|snapshot| snapshot.branch_id);
		let publish_branch_changed =
			current_branch_id.is_some_and(|branch_id| branch_id != result.branch_id);
		let pidx = if publish_branch_changed {
			Arc::new(DeltaPageIndex::new())
		} else {
			cache_snapshot
				.as_ref()
				.map(|snapshot| Arc::clone(&snapshot.pidx))
				.unwrap_or_else(|| Arc::new(DeltaPageIndex::new()))
		};
		let pidx_was_warm = !pidx.range(0, u32::MAX).is_empty();
		if cache_was_warm || pidx_was_warm || publish_branch_changed {
			for pgno in result.truncated_pgnos {
				pidx.remove(pgno);
			}
			for pgno in result.dirty_pgnos {
				pidx.insert(pgno, result.txid);
			}
		}
		let last_access_bucket = result.access_bucket.or_else(|| {
			cache_snapshot
				.as_ref()
				.filter(|snapshot| snapshot.branch_id == result.branch_id)
				.and_then(|snapshot| snapshot.last_access_bucket)
		});
		*cache_snapshot = Some(CacheSnapshot {
			branch_id: result.branch_id,
			ancestors: result.branch_ancestry.clone(),
			last_access_bucket,
			pidx,
		});

		self.publish_deltas_available_if_needed(result.deltas_available, result.branch_id)
			.await?;

		Ok(CommitResult {
			head_txid: result.txid,
			db_size_pages,
		})
	}

	pub async fn commit_stage_begin(
		&self,
		dirty_pgnos: Vec<u32>,
		db_size_pages: u32,
		now_ms: i64,
		options: CommitOptions,
	) -> Result<CommitStageBeginResult> {
		validate_stage_dirty_pgnos(&dirty_pgnos)?;
		let preflight = self
			.prepare_slow_commit(None, None, options.expected_head_txid)
			.await?;
		let staged_txid = preflight
			.observed_head_txid
			.checked_add(1)
			.context("sqlite staged txid overflowed")?;
		let stage_id = Uuid::new_v4();
		let object_id = Uuid::new_v4();
		let dirty_page_count =
			u32::try_from(dirty_pgnos.len()).context("sqlite dirty page count exceeded u32")?;
		let reserved_storage_bytes =
			estimate_stage_reservation(preflight.branch_id, object_id, staged_txid, &dirty_pgnos)?;
		let meta = CommitStageMeta {
			stage_id,
			object_id,
			observed_head_txid: preflight.observed_head_txid,
			staged_txid,
			caller_expected_head_txid: options.expected_head_txid,
			db_size_pages,
			now_ms,
			dirty_page_count,
			dirty_pgnos_hash: hash_dirty_pgnos(&dirty_pgnos),
			reserved_storage_bytes,
			state: CommitStageState::Uploading,
			created_at_ms: now_ms,
			expires_after_ms: now_ms.saturating_add(STAGE_TTL_MS),
		};
		let encoded_meta = encode_commit_stage_meta(meta)?;
		let stage_key = keys::branch_commit_stage_meta_key(preflight.branch_id, stage_id);
		let lookup_key = keys::commit_stage_lookup_key(stage_id);
		let branch_id_bytes = preflight.branch_id.as_uuid().as_bytes().to_vec();
		let branch_id = preflight.branch_id;

		self.udb
			.run(move |tx| {
				let stage_key = stage_key.clone();
				let lookup_key = lookup_key.clone();
				let encoded_meta = encoded_meta.clone();
				let branch_id_bytes = branch_id_bytes.clone();
				async move {
					if tx_get_value(&tx, &lookup_key, Serializable).await?.is_some() {
						return Err(SqliteStorageError::SqliteCommitStageInvalid {
							reason: format!("stage id collision for {stage_id}"),
						}
						.into());
					}
					let committed = quota::read_branch(&tx, branch_id).await?;
					let staged = quota::read_branch_staged(&tx, branch_id).await?;
					let would_be = committed
						.checked_add(staged)
						.and_then(|value| value.checked_add(reserved_storage_bytes))
						.context("sqlite staged quota reservation overflowed")?;
					quota::cap_check(would_be)?;
					tx.informal().set(&stage_key, &encoded_meta);
					tx.informal().set(&lookup_key, &branch_id_bytes);
					quota::atomic_add_branch_staged(&tx, branch_id, reserved_storage_bytes);
					Ok(())
				}
			})
			.await?;

		tracing::debug!(
			database_id = %self.database_id,
			?branch_id,
			%stage_id,
			%object_id,
			dirty_page_count,
			reserved_storage_bytes,
			staged_txid,
			"sqlite commit stage begun"
		);

		Ok(CommitStageBeginResult {
			stage_id,
			max_pages_per_batch: MAX_STAGE_PAGES_PER_BATCH as u32,
			max_batch_bytes: MAX_STAGE_BATCH_BYTES as u32,
			observed_head_txid: preflight.observed_head_txid,
			staged_txid,
		})
	}

	pub async fn commit_stage_pages(
		&self,
		stage_id: Uuid,
		batch_idx: u32,
		pages: Vec<DirtyPage>,
	) -> Result<()> {
		let stage = self.load_commit_stage(stage_id).await?;
		if !matches!(stage.meta.state, CommitStageState::Uploading) {
			return Err(SqliteStorageError::SqliteCommitStageInvalid {
				reason: format!("stage {stage_id} is not accepting page batches"),
			}
			.into());
		}
		validate_stage_page_batch(&pages)?;
		let batch = DirtyPageBatch {
			batch_idx,
			batch_hash: hash_dirty_pages(&pages),
			pages,
		};
		let encoded_batch = encode_dirty_page_batch(batch)?;
		ensure!(
			encoded_batch.len() <= MAX_STAGE_BATCH_BYTES,
			"sqlite commit stage batch encoded to {} bytes, limit is {} bytes",
			encoded_batch.len(),
			MAX_STAGE_BATCH_BYTES
		);

		let batch_key = keys::branch_commit_stage_pages_key(stage.branch_id, stage_id, batch_idx);
		let meta_key = keys::branch_commit_stage_meta_key(stage.branch_id, stage_id);
		self.udb
			.run(move |tx| {
				let batch_key = batch_key.clone();
				let meta_key = meta_key.clone();
				let encoded_batch = encoded_batch.clone();
				async move {
					let meta_bytes = tx_get_value(&tx, &meta_key, Serializable)
						.await?
						.ok_or(SqliteStorageError::SqliteCommitStageNotFound { stage_id })?;
					let meta = decode_commit_stage_meta(&meta_bytes)?;
					if !matches!(meta.state, CommitStageState::Uploading) {
						return Err(SqliteStorageError::SqliteCommitStageInvalid {
							reason: format!("stage {stage_id} is not accepting page batches"),
						}
						.into());
					}
					if let Some(existing) = tx_get_value(&tx, &batch_key, Serializable).await? {
						if existing == encoded_batch {
							return Ok(());
						}
						return Err(SqliteStorageError::SqliteCommitStageInvalid {
							reason: format!("stage {stage_id} batch {batch_idx} changed"),
						}
						.into());
					}
					tx.informal().set(&batch_key, &encoded_batch);
					Ok(())
				}
			})
			.await
	}

	pub async fn commit_stage_complete(&self, stage_id: Uuid, page_batch_count: u32) -> Result<()> {
		let stage = self.load_commit_stage(stage_id).await?;
		match stage.meta.state {
			CommitStageState::ObjectWritten | CommitStageState::Finalized { .. } => return Ok(()),
			CommitStageState::Uploading | CommitStageState::Complete => {}
			CommitStageState::Finalizing | CommitStageState::Aborted => {
				return Err(SqliteStorageError::SqliteCommitStageInvalid {
					reason: format!("stage {stage_id} cannot be completed from its current state"),
				}
				.into());
			}
		}

		let dirty_pages = self
			.load_stage_pages(stage.branch_id, &stage.meta, page_batch_count)
			.await?;
		let object = build_delta_object_with_ids(
			stage.meta.staged_txid,
			stage.meta.db_size_pages,
			stage.meta.now_ms,
			stage.meta.stage_id,
			stage.meta.object_id,
			&dirty_pages,
		)?;
		self.write_delta_object(stage.branch_id, &object).await?;

		let mut updated_meta = stage.meta.clone();
		updated_meta.state = CommitStageState::ObjectWritten;
		let encoded_meta = encode_commit_stage_meta(updated_meta)?;
		let complete = CommitStageComplete {
			page_batch_count,
			dirty_page_count: stage.meta.dirty_page_count,
			dirty_pages_hash: stage.meta.dirty_pgnos_hash,
			completed_at_ms: stage.meta.now_ms,
		};
		let encoded_complete = encode_commit_stage_complete(complete)?;
		let meta_key = keys::branch_commit_stage_meta_key(stage.branch_id, stage_id);
		let complete_key = keys::branch_commit_stage_complete_key(stage.branch_id, stage_id);
		self.udb
			.run(move |tx| {
				let meta_key = meta_key.clone();
				let complete_key = complete_key.clone();
				let encoded_meta = encoded_meta.clone();
				let encoded_complete = encoded_complete.clone();
				async move {
					let current_meta = tx_get_value(&tx, &meta_key, Serializable)
						.await?
						.ok_or(SqliteStorageError::SqliteCommitStageNotFound { stage_id })?;
					let current_meta = decode_commit_stage_meta(&current_meta)?;
					match current_meta.state {
						CommitStageState::ObjectWritten | CommitStageState::Finalized { .. } => {
							return Ok(());
						}
						CommitStageState::Uploading | CommitStageState::Complete => {}
						CommitStageState::Finalizing | CommitStageState::Aborted => {
							return Err(SqliteStorageError::SqliteCommitStageInvalid {
								reason: format!(
									"stage {stage_id} cannot be completed from its current state"
								),
							}
							.into());
						}
					}
					tx.informal().set(&complete_key, &encoded_complete);
					tx.informal().set(&meta_key, &encoded_meta);
					Ok(())
				}
			})
			.await?;

		tracing::debug!(
			database_id = %self.database_id,
			branch_id = ?stage.branch_id,
			%stage_id,
			object_id = %stage.meta.object_id,
			dirty_page_count = stage.meta.dirty_page_count,
			staged_txid = stage.meta.staged_txid,
			"sqlite commit stage object written"
		);

		Ok(())
	}

	pub async fn commit_stage_finalize(&self, stage_id: Uuid) -> Result<CommitResult> {
		let stage = self.load_commit_stage(stage_id).await?;
		if let Some(finalized) = self.read_commit_stage_finalized(&stage).await? {
			tracing::debug!(
				database_id = %self.database_id,
				branch_id = ?stage.branch_id,
				%stage_id,
				txid = finalized.txid,
				"sqlite commit stage finalize retry returned existing txid"
			);
			return Ok(CommitResult {
				head_txid: finalized.txid,
				db_size_pages: stage.meta.db_size_pages,
			});
		}
		if let CommitStageState::Finalized { txid } = stage.meta.state {
			return Ok(CommitResult {
				head_txid: txid,
				db_size_pages: stage.meta.db_size_pages,
			});
		}
		if !matches!(stage.meta.state, CommitStageState::ObjectWritten) {
			return Err(SqliteStorageError::SqliteCommitStageInvalid {
				reason: format!("stage {stage_id} has not finished object staging"),
			}
			.into());
		}
		let complete = self.load_commit_stage_complete(&stage).await?;
		let dirty_pages = self
			.load_stage_pages(stage.branch_id, &stage.meta, complete.page_batch_count)
			.await?;
		let object = build_delta_object_with_ids(
			stage.meta.staged_txid,
			stage.meta.db_size_pages,
			stage.meta.now_ms,
			stage.meta.stage_id,
			stage.meta.object_id,
			&dirty_pages,
		)?;
		let preflight = self.stage_preflight(stage.branch_id, &stage.meta).await?;
		let cached_storage_used = *self.storage_used.read().await;
		let cached_snapshot = self.cache_snapshot.read().await.clone();
		let cached_access_bucket = cached_snapshot
			.as_ref()
			.and_then(|snapshot| snapshot.last_access_bucket);
		let last_deltas_available_at_ms = *self.last_deltas_available_at_ms.read().await;
		let cache_was_warm = cached_snapshot
			.as_ref()
			.is_some_and(|snapshot| !snapshot.pidx.range(0, u32::MAX).is_empty());
		let result = self
			.finalize_slow_commit(
				preflight,
				object,
				dirty_pages,
				stage.meta.db_size_pages,
				stage.meta.now_ms,
				CommitOptions {
					expected_head_txid: stage.meta.caller_expected_head_txid,
				},
				cached_storage_used,
				cached_access_bucket,
				last_deltas_available_at_ms,
				Some(StageFinalizeContext {
					stage_id,
					stage_meta: stage.meta.clone(),
					reserved_storage_bytes: stage.meta.reserved_storage_bytes,
				}),
			)
			.await
			.map_err(map_udb_commit_error)?;

		self.apply_slow_commit_result_to_cache(cache_was_warm, result)
			.await?;
		self.cleanup_stage_upload_rows(stage.branch_id, stage_id)
			.await?;

		Ok(CommitResult {
			head_txid: stage.meta.staged_txid,
			db_size_pages: stage.meta.db_size_pages,
		})
	}

	pub async fn commit_stage_abort(&self, stage_id: Uuid) -> Result<()> {
		let stage = match self.load_commit_stage(stage_id).await {
			Ok(stage) => stage,
			Err(err)
				if err
					.chain()
					.any(|source| source.downcast_ref::<SqliteStorageError>().is_some_and(
						|storage_err| {
							matches!(
								storage_err,
								SqliteStorageError::SqliteCommitStageNotFound { .. }
							)
						},
					)) =>
			{
				return Ok(());
			}
			Err(err) => return Err(err),
		};
		if matches!(stage.meta.state, CommitStageState::Finalized { .. }) {
			return Ok(());
		}
		self.clear_commit_stage(stage.branch_id, &stage.meta, true)
			.await
	}

	pub async fn gc_expired_commit_stages(&self, now_ms: i64) -> Result<CommitStageGcOutcome> {
		let lookup_prefix = keys::commit_stage_lookup_prefix();
		let rows = self
			.udb
			.run(move |tx| {
				let lookup_prefix = lookup_prefix.clone();
				async move { tx_scan_prefix_values(&tx, &lookup_prefix).await }
			})
			.await?;
		let mut outcome = CommitStageGcOutcome::default();

		for (lookup_key, branch_bytes) in rows {
			let stage_id = keys::decode_commit_stage_lookup_id(&lookup_key)
				.context("decode sqlite commit stage lookup id during GC")?;
			let branch_uuid = Uuid::from_slice(&branch_bytes)
				.context("sqlite commit stage lookup had invalid branch id during GC")?;
			let branch_id = DatabaseBranchId::from_uuid(branch_uuid);
			let meta_key = keys::branch_commit_stage_meta_key(branch_id, stage_id);
			let meta_bytes = self
				.udb
				.run(move |tx| {
					let meta_key = meta_key.clone();
					async move { tx_get_value(&tx, &meta_key, Serializable).await }
				})
				.await?;
			let Some(meta_bytes) = meta_bytes else {
				self.clear_commit_stage_lookup(stage_id).await?;
				outcome.stale_lookups_deleted += 1;
				continue;
			};
			let meta = decode_commit_stage_meta(&meta_bytes)
				.context("decode sqlite commit stage meta during GC")?;
			if meta.expires_after_ms > now_ms {
				continue;
			}

			match meta.state {
				CommitStageState::Finalized { .. } => {
					self.clear_finalized_stage_rows(branch_id, stage_id).await?;
					outcome.finalized_tombstones_deleted += 1;
				}
				CommitStageState::Uploading
				| CommitStageState::Complete
				| CommitStageState::ObjectWritten
				| CommitStageState::Finalizing
				| CommitStageState::Aborted => {
					let released = meta.reserved_storage_bytes;
					self.clear_commit_stage(branch_id, &meta, true).await?;
					outcome.stages_deleted += 1;
					outcome.quota_released_bytes += released;
				}
			}
		}

		Ok(outcome)
	}

	pub async fn gc_expired_orphan_delta_objects(
		&self,
		branch_id: DatabaseBranchId,
		now_ms: i64,
	) -> Result<OrphanDeltaObjectGcOutcome> {
		let object_root_prefix = keys::branch_delta_object_root_prefix(branch_id);
		let rows = self
			.udb
			.run(move |tx| {
				let object_root_prefix = object_root_prefix.clone();
				async move { tx_scan_prefix_values(&tx, &object_root_prefix).await }
			})
			.await?;
		let mut outcome = OrphanDeltaObjectGcOutcome::default();

		for (key, value) in rows {
			let Ok(object_id) = keys::decode_branch_delta_object_meta_object_id(branch_id, &key) else {
				continue;
			};
			let meta =
				decode_delta_object_meta(&value).context("decode sqlite delta object meta during GC")?;
			if meta.expires_after_ms > now_ms {
				continue;
			}
			match meta.state {
				DeltaObjectState::Committed { .. } => continue,
				DeltaObjectState::StageOwned => {}
			}

			if self
				.delta_object_ref_exists(branch_id, object_id)
				.await
				.context("check sqlite delta object ref during orphan GC")?
			{
				continue;
			}

			let stage_meta_key = keys::branch_commit_stage_meta_key(branch_id, meta.stage_id);
			let stage_meta = self
				.udb
				.run(move |tx| {
					let stage_meta_key = stage_meta_key.clone();
					async move { tx_get_value(&tx, &stage_meta_key, Serializable).await }
				})
				.await?;
			if let Some(stage_meta_bytes) = stage_meta {
				let stage_meta = decode_commit_stage_meta(&stage_meta_bytes)
					.context("decode sqlite commit stage meta during orphan GC")?;
				if stage_meta.expires_after_ms > now_ms {
					continue;
				}
				if matches!(stage_meta.state, CommitStageState::Finalized { .. }) {
					continue;
				}
				let released = stage_meta.reserved_storage_bytes;
				self.clear_commit_stage(branch_id, &stage_meta, true).await?;
				outcome.stages_deleted += 1;
				outcome.quota_released_bytes += released;
				continue;
			}

			if self
				.clear_delta_object_if_unreferenced(branch_id, object_id, Some(meta.stage_id))
				.await?
			{
				outcome.orphan_objects_deleted += 1;
			}
		}

		Ok(outcome)
	}

	async fn load_commit_stage(&self, stage_id: Uuid) -> Result<LoadedCommitStage> {
		let lookup_key = keys::commit_stage_lookup_key(stage_id);
		let udb = self.udb.clone();
		udb.run(move |tx| {
			let lookup_key = lookup_key.clone();
			async move {
				let branch_bytes = tx_get_value(&tx, &lookup_key, Serializable)
					.await?
					.ok_or(SqliteStorageError::SqliteCommitStageNotFound { stage_id })?;
				let branch_uuid = Uuid::from_slice(&branch_bytes)
					.context("sqlite commit stage lookup had invalid branch id")?;
				let branch_id = DatabaseBranchId::from_uuid(branch_uuid);
				let meta_key = keys::branch_commit_stage_meta_key(branch_id, stage_id);
				let meta = tx_get_value(&tx, &meta_key, Serializable)
					.await?
					.ok_or(SqliteStorageError::SqliteCommitStageNotFound { stage_id })?;
				Ok(LoadedCommitStage {
					branch_id,
					meta: decode_commit_stage_meta(&meta)?,
				})
			}
		})
		.await
	}

	async fn load_commit_stage_complete(
		&self,
		stage: &LoadedCommitStage,
	) -> Result<CommitStageComplete> {
		let key = keys::branch_commit_stage_complete_key(stage.branch_id, stage.meta.stage_id);
		self.udb
			.run(move |tx| {
				let key = key.clone();
				async move {
					let value = tx_get_value(&tx, &key, Serializable).await?.ok_or_else(|| {
						SqliteStorageError::SqliteCommitStageInvalid {
							reason: "stage complete marker is missing".to_string(),
						}
					})?;
					decode_commit_stage_complete(&value)
				}
			})
			.await
	}

	async fn read_commit_stage_finalized(
		&self,
		stage: &LoadedCommitStage,
	) -> Result<Option<CommitStageFinalized>> {
		let key = keys::branch_commit_stage_finalized_key(stage.branch_id, stage.meta.stage_id);
		self.udb
			.run(move |tx| {
				let key = key.clone();
				async move {
					tx_get_value(&tx, &key, Serializable)
						.await?
						.as_deref()
						.map(decode_commit_stage_finalized)
						.transpose()
				}
			})
			.await
	}

	async fn load_stage_pages(
		&self,
		branch_id: DatabaseBranchId,
		meta: &CommitStageMeta,
		page_batch_count: u32,
	) -> Result<Vec<DirtyPage>> {
		ensure!(page_batch_count > 0, "sqlite commit stage has no page batches");
		let mut pages_by_pgno = BTreeMap::new();
		for batch_idx in 0..page_batch_count {
			let key = keys::branch_commit_stage_pages_key(branch_id, meta.stage_id, batch_idx);
			let batch = self
				.udb
				.run(move |tx| {
					let key = key.clone();
					async move {
						let value = tx_get_value(&tx, &key, Serializable).await?.ok_or_else(|| {
							SqliteStorageError::SqliteCommitStageInvalid {
								reason: format!("stage page batch {batch_idx} is missing"),
							}
						})?;
						decode_dirty_page_batch(&value)
					}
				})
				.await?;
			ensure!(
				batch.batch_idx == batch_idx,
				"sqlite commit stage batch index mismatch"
			);
			ensure!(
				batch.batch_hash == hash_dirty_pages(&batch.pages),
				"sqlite commit stage batch hash mismatch"
			);
			for page in batch.pages {
				validate_stage_dirty_page(&page)?;
				if pages_by_pgno.insert(page.pgno, page).is_some() {
					return Err(SqliteStorageError::SqliteCommitStageInvalid {
						reason: "stage uploaded a duplicate dirty page".to_string(),
					}
					.into());
				}
			}
		}

		let dirty_pgnos = pages_by_pgno.keys().copied().collect::<Vec<_>>();
		if dirty_pgnos.len() != meta.dirty_page_count as usize {
			return Err(SqliteStorageError::SqliteCommitStageInvalid {
				reason: format!(
					"stage uploaded {} pages, expected {}",
					dirty_pgnos.len(),
					meta.dirty_page_count
				),
			}
			.into());
		}
		if hash_dirty_pgnos(&dirty_pgnos) != meta.dirty_pgnos_hash {
			return Err(SqliteStorageError::SqliteCommitStageInvalid {
				reason: "stage uploaded page numbers do not match begin request".to_string(),
			}
			.into());
		}

		Ok(pages_by_pgno.into_values().collect())
	}

	async fn stage_preflight(
		&self,
		branch_id: DatabaseBranchId,
		meta: &CommitStageMeta,
	) -> Result<SlowCommitPreflight> {
		let database_id = self.database_id.clone();
		let bucket_id = self.sqlite_bucket_id();
		let observed_head_txid = meta.observed_head_txid;
		self.udb
			.run(move |tx| {
				let database_id = database_id.clone();
				async move {
					let bucket = branch::resolve_or_allocate_root_bucket_branch(&tx, bucket_id).await?;
					let current = branch::resolve_database_branch_in_bucket(
						&tx,
						bucket.branch_id,
						&database_id,
						Serializable,
					)
					.await?;
					let database_initialized = current.is_none();
					if let Some(current_branch_id) = current {
						if current_branch_id != branch_id {
							let actual_head_txid = read_branch_head_txid(&tx, current_branch_id)
								.await
								.unwrap_or(0);
							return Err(SqliteStorageError::HeadFenceMismatch {
								expected_head_txid: observed_head_txid,
								actual_head_txid,
							}
							.into());
						}
					}
					let branch_ancestry = if database_initialized {
						BranchAncestry::root(branch_id)
					} else {
						load_branch_ancestry(&tx, branch_id).await?
					};
					Ok(SlowCommitPreflight {
						branch_id,
						bucket_branch_id: bucket.branch_id,
						bucket_initialized: bucket.initialized,
						database_initialized,
						branch_ancestry,
						observed_head_txid,
					})
				}
			})
			.await
	}

	async fn apply_slow_commit_result_to_cache(
		&self,
		cache_was_warm: bool,
		result: SlowCommitTxResult,
	) -> Result<()> {
		*self.storage_used.write().await = Some(result.storage_used);
		self.commit_bytes_since_rollup.fetch_add(
			u64::try_from(result.added_bytes)
				.context("commit added bytes should be non-negative")?,
			std::sync::atomic::Ordering::Relaxed,
		);

		let mut cache_snapshot = self.cache_snapshot.write().await;
		let current_branch_id = cache_snapshot.as_ref().map(|snapshot| snapshot.branch_id);
		let publish_branch_changed =
			current_branch_id.is_some_and(|branch_id| branch_id != result.branch_id);
		let pidx = if publish_branch_changed {
			Arc::new(DeltaPageIndex::new())
		} else {
			cache_snapshot
				.as_ref()
				.map(|snapshot| Arc::clone(&snapshot.pidx))
				.unwrap_or_else(|| Arc::new(DeltaPageIndex::new()))
		};
		let pidx_was_warm = !pidx.range(0, u32::MAX).is_empty();
		if cache_was_warm || pidx_was_warm || publish_branch_changed {
			for pgno in result.truncated_pgnos {
				pidx.remove(pgno);
			}
			for pgno in result.dirty_pgnos {
				pidx.insert(pgno, result.txid);
			}
		}
		let last_access_bucket = result.access_bucket.or_else(|| {
			cache_snapshot
				.as_ref()
				.filter(|snapshot| snapshot.branch_id == result.branch_id)
				.and_then(|snapshot| snapshot.last_access_bucket)
		});
		*cache_snapshot = Some(CacheSnapshot {
			branch_id: result.branch_id,
			ancestors: result.branch_ancestry.clone(),
			last_access_bucket,
			pidx,
		});
		drop(cache_snapshot);

		self.publish_deltas_available_if_needed(result.deltas_available, result.branch_id)
			.await
	}

	async fn cleanup_stage_upload_rows(
		&self,
		branch_id: DatabaseBranchId,
		stage_id: Uuid,
	) -> Result<()> {
		let pages_prefix = {
			let mut prefix = keys::branch_commit_stage_prefix(branch_id, stage_id);
			prefix.extend_from_slice(b"/pages/");
			prefix
		};
		let complete_key = keys::branch_commit_stage_complete_key(branch_id, stage_id);
		self.udb
			.run(move |tx| {
				let pages_prefix = pages_prefix.clone();
				let complete_key = complete_key.clone();
				async move {
					for (key, _) in tx_scan_prefix_values(&tx, &pages_prefix).await? {
						tx.informal().clear(&key);
					}
					tx.informal().clear(&complete_key);
					Ok(())
				}
			})
			.await
	}

	async fn clear_commit_stage(
		&self,
		branch_id: DatabaseBranchId,
		meta: &CommitStageMeta,
		release_quota: bool,
	) -> Result<()> {
		let stage_prefix = keys::branch_commit_stage_prefix(branch_id, meta.stage_id);
		let object_prefix = keys::branch_delta_object_prefix(branch_id, meta.object_id);
		let lookup_key = keys::commit_stage_lookup_key(meta.stage_id);
		let reserved_storage_bytes = meta.reserved_storage_bytes;
		self.udb
			.run(move |tx| {
				let stage_prefix = stage_prefix.clone();
				let object_prefix = object_prefix.clone();
				let lookup_key = lookup_key.clone();
				async move {
					for (key, _) in tx_scan_prefix_values(&tx, &stage_prefix).await? {
						tx.informal().clear(&key);
					}
					for (key, _) in tx_scan_prefix_values(&tx, &object_prefix).await? {
						tx.informal().clear(&key);
					}
					tx.informal().clear(&lookup_key);
					if release_quota && reserved_storage_bytes != 0 {
						quota::atomic_add_branch_staged(&tx, branch_id, -reserved_storage_bytes);
					}
					Ok(())
				}
			})
			.await
	}

	async fn clear_commit_stage_lookup(&self, stage_id: Uuid) -> Result<()> {
		let lookup_key = keys::commit_stage_lookup_key(stage_id);
		self.udb
			.run(move |tx| {
				let lookup_key = lookup_key.clone();
				async move {
					tx.informal().clear(&lookup_key);
					Ok(())
				}
			})
			.await
	}

	async fn clear_finalized_stage_rows(
		&self,
		branch_id: DatabaseBranchId,
		stage_id: Uuid,
	) -> Result<()> {
		let stage_prefix = keys::branch_commit_stage_prefix(branch_id, stage_id);
		let lookup_key = keys::commit_stage_lookup_key(stage_id);
		self.udb
			.run(move |tx| {
				let stage_prefix = stage_prefix.clone();
				let lookup_key = lookup_key.clone();
				async move {
					for (key, _) in tx_scan_prefix_values(&tx, &stage_prefix).await? {
						tx.informal().clear(&key);
					}
					tx.informal().clear(&lookup_key);
					Ok(())
				}
			})
			.await
	}

	async fn delta_object_ref_exists(
		&self,
		branch_id: DatabaseBranchId,
		object_id: Uuid,
	) -> Result<bool> {
		let object_ref_key = keys::branch_delta_object_ref_key(branch_id, object_id);
		self.udb
			.run(move |tx| {
				let object_ref_key = object_ref_key.clone();
				async move { Ok(tx_get_value(&tx, &object_ref_key, Serializable).await?.is_some()) }
			})
			.await
	}

	async fn clear_delta_object_if_unreferenced(
		&self,
		branch_id: DatabaseBranchId,
		object_id: Uuid,
		stage_id: Option<Uuid>,
	) -> Result<bool> {
		let object_ref_key = keys::branch_delta_object_ref_key(branch_id, object_id);
		let object_prefix = keys::branch_delta_object_prefix(branch_id, object_id);
		let lookup_key = stage_id.map(keys::commit_stage_lookup_key);
		self.udb
			.run(move |tx| {
				let object_ref_key = object_ref_key.clone();
				let object_prefix = object_prefix.clone();
				let lookup_key = lookup_key.clone();
				async move {
					if tx_get_value(&tx, &object_ref_key, Serializable)
						.await?
						.is_some()
					{
						return Ok(false);
					}
					for (key, _) in tx_scan_prefix_values(&tx, &object_prefix).await? {
						tx.informal().clear(&key);
					}
					if let Some(lookup_key) = lookup_key {
						tx.informal().clear(&lookup_key);
					}
					Ok(true)
				}
			})
			.await
	}

	async fn prepare_slow_commit(
		&self,
		cached_branch_id: Option<DatabaseBranchId>,
		cached_ancestry: Option<BranchAncestry>,
		expected_head_txid: Option<u64>,
	) -> Result<SlowCommitPreflight> {
		let database_id = self.database_id.clone();
		let bucket_id = self.sqlite_bucket_id();
		#[cfg(feature = "test-faults")]
		let fault_controller = self.fault_controller.clone();

		self.udb
			.run(move |tx| {
				let database_id = database_id.clone();
				let cached_ancestry = cached_ancestry.clone();
				#[cfg(feature = "test-faults")]
				let fault_controller = fault_controller.clone();

				async move {
					let branch_resolution =
						resolve_or_allocate_branch(&tx, bucket_id, &database_id).await?;
					let branch_id = branch_resolution.branch_id;
					#[cfg(feature = "test-faults")]
					super::apply::maybe_fire_commit_fault(
						&fault_controller,
						&database_id,
						crate::fault::CommitFaultPoint::AfterBranchResolution,
						Some(branch_id),
					)
					.await?;
					if !branch_resolution.database_initialized {
						let branch_record =
							tx_get_value(&tx, &keys::branches_list_key(branch_id), Serializable)
								.await?
								.as_deref()
								.map(decode_database_branch_record)
								.transpose()
								.context("decode sqlite database branch record for slow commit")?;
						if !branch_record
							.as_ref()
							.is_some_and(|record| record.state == BranchState::Live)
						{
							return Err(SqliteStorageError::BranchNotWritable.into());
						}
					}
					let branch_ancestry = if branch_resolution.database_initialized {
						BranchAncestry::root(branch_id)
					} else if let Some(cached_ancestry) =
						cached_ancestry.filter(|ancestry| {
							cached_branch_id == Some(branch_id) && ancestry.root_branch_id == branch_id
						}) {
						cached_ancestry
					} else {
						load_branch_ancestry(&tx, branch_id).await?
					};
					let head_bytes =
						tx_get_value(&tx, &keys::branch_meta_head_key(branch_id), Serializable)
							.await?;
					let head_at_fork_bytes = tx_get_value(
						&tx,
						&keys::branch_meta_head_at_fork_key(branch_id),
						Serializable,
					)
					.await?;
					let previous_head_bytes = head_bytes.as_ref().or(head_at_fork_bytes.as_ref());
					let previous_head = previous_head_bytes
						.map(|bytes| decode_db_head(bytes.as_slice()))
						.transpose()
						.context("decode current sqlite db head for slow commit")?;
					let actual_head_txid = previous_head.as_ref().map_or(0, |head| head.head_txid);
					if let Some(expected_head_txid) = expected_head_txid {
						if expected_head_txid != actual_head_txid {
							tracing::error!(
								%database_id,
								?branch_id,
								expected_head_txid,
								actual_head_txid,
								"sqlite head fence mismatch; this indicates multiple actor instances are writing the same sqlite database in parallel, which is incorrect actor lifecycle behavior"
							);
							return Err(SqliteStorageError::HeadFenceMismatch {
								expected_head_txid,
								actual_head_txid,
							}
							.into());
						}
					}

					Ok(SlowCommitPreflight {
						branch_id,
						bucket_branch_id: branch_resolution.bucket_branch_id,
						bucket_initialized: branch_resolution.bucket_initialized,
						database_initialized: branch_resolution.database_initialized,
						branch_ancestry,
						observed_head_txid: actual_head_txid,
					})
				}
			})
			.await
	}

	async fn write_delta_object(
		&self,
		branch_id: DatabaseBranchId,
		object: &LargeDeltaObject,
	) -> Result<()> {
		let mut batch = Vec::<(Vec<u8>, Vec<u8>)>::new();
		let mut batch_bytes = 0usize;
		for (chunk_idx, chunk) in &object.chunks {
			let key = keys::branch_delta_object_chunk_key(branch_id, object.object_id, *chunk_idx);
			let next_bytes = key.len() + chunk.len();
			if !batch.is_empty() && batch_bytes + next_bytes > DELTA_OBJECT_WRITE_BATCH_BYTES {
				write_object_batch(&self.udb, std::mem::take(&mut batch)).await?;
				batch_bytes = 0;
			}
			batch_bytes += next_bytes;
			batch.push((key, chunk.clone()));
		}
		if !batch.is_empty() {
			write_object_batch(&self.udb, batch).await?;
		}

		let object_meta_key = keys::branch_delta_object_meta_key(branch_id, object.object_id);
		let object_meta = object.encoded_stage_meta.clone();
		self.udb
			.run(move |tx| {
				let object_meta_key = object_meta_key.clone();
				let object_meta = object_meta.clone();
				async move {
					tx.informal().set(&object_meta_key, &object_meta);
					Ok(())
				}
			})
			.await
	}

	#[allow(clippy::too_many_arguments)]
	async fn finalize_slow_commit(
		&self,
		preflight: SlowCommitPreflight,
		object: LargeDeltaObject,
		dirty_pages: Vec<DirtyPage>,
		db_size_pages: u32,
		now_ms: i64,
		options: CommitOptions,
		_cached_storage_used: Option<i64>,
		cached_access_bucket: Option<i64>,
		last_deltas_available_at_ms: Option<i64>,
		stage_context: Option<StageFinalizeContext>,
	) -> Result<SlowCommitTxResult> {
		let database_id = self.database_id.clone();
		let bucket_id = self.sqlite_bucket_id();
		let expected_head_txid = options.expected_head_txid;
		#[cfg(feature = "test-faults")]
		let fault_controller = self.fault_controller.clone();

		self.udb
			.run(move |tx| {
				let database_id = database_id.clone();
				let dirty_pages = dirty_pages.clone();
				let preflight = preflight.clone();
				let object = object.clone();
				let stage_context = stage_context.clone();
				#[cfg(feature = "test-faults")]
				let fault_controller = fault_controller.clone();

				async move {
					let branch_resolution =
						resolve_slow_publish_branch(&tx, bucket_id, &database_id, &preflight)
							.await?;
					let branch_id = branch_resolution.branch_id;
					#[cfg(feature = "test-faults")]
					super::apply::maybe_fire_commit_fault(
						&fault_controller,
						&database_id,
						crate::fault::CommitFaultPoint::AfterBranchResolution,
						Some(branch_id),
					)
					.await?;
					if !branch_resolution.database_initialized {
						let branch_record =
							tx_get_value(&tx, &keys::branches_list_key(branch_id), Serializable)
								.await?
								.as_deref()
								.map(decode_database_branch_record)
								.transpose()
								.context("decode sqlite database branch record for slow finalize")?;
						if !branch_record
							.as_ref()
							.is_some_and(|record| record.state == BranchState::Live)
						{
							return Err(SqliteStorageError::BranchNotWritable.into());
						}
					}

					let head_key = keys::branch_meta_head_key(branch_id);
					let head_at_fork_key = keys::branch_meta_head_at_fork_key(branch_id);
					let quota_fut = quota::read_branch(&tx, branch_id);
					let staged_quota_fut = quota::read_branch_staged(&tx, branch_id);
					let head_fut = tx_get_value(&tx, &head_key, Serializable);
					let head_at_fork_fut = tx_get_value(&tx, &head_at_fork_key, Serializable);
					let (head_bytes, head_at_fork_bytes, storage_used, staged_storage_used) =
						tokio::try_join!(head_fut, head_at_fork_fut, quota_fut, staged_quota_fut)?;

					let previous_head_bytes = head_bytes.as_ref().or(head_at_fork_bytes.as_ref());
					let previous_head = previous_head_bytes
						.map(|bytes| decode_db_head(bytes.as_slice()))
						.transpose()
						.context("decode current sqlite db head for slow finalize")?;
					let actual_head_txid = previous_head.as_ref().map_or(0, |head| head.head_txid);
					if actual_head_txid != preflight.observed_head_txid {
						return Err(SqliteStorageError::HeadFenceMismatch {
							expected_head_txid: preflight.observed_head_txid,
							actual_head_txid,
						}
						.into());
					}
					if let Some(expected_head_txid) = expected_head_txid {
						if expected_head_txid != actual_head_txid {
							return Err(SqliteStorageError::HeadFenceMismatch {
								expected_head_txid,
								actual_head_txid,
							}
							.into());
						}
					}
					#[cfg(feature = "test-faults")]
					super::apply::maybe_fire_commit_fault(
						&fault_controller,
						&database_id,
						crate::fault::CommitFaultPoint::AfterHeadRead,
						Some(branch_id),
					)
					.await?;

					let object_meta_key =
						keys::branch_delta_object_meta_key(branch_id, object.object_id);
					let object_meta_bytes =
						tx_get_value(&tx, &object_meta_key, Serializable).await?.with_context(
							|| format!("sqlite large delta object meta missing {}", object.object_id),
						)?;
					let object_meta = decode_delta_object_meta(&object_meta_bytes)
						.context("decode sqlite large delta object meta")?;
					ensure!(
						object_meta.object_id == object.object_id
							&& object_meta.stage_id == object.stage_id
							&& object_meta.staged_txid == object.txid
							&& matches!(object_meta.state, DeltaObjectState::StageOwned),
						"sqlite large delta object meta did not match staged object"
					);

					let compaction_root =
						tx_get_value(&tx, &keys::branch_compaction_root_key(branch_id), Snapshot)
							.await?
							.as_deref()
							.map(decode_compaction_root)
							.transpose()
							.context("decode sqlite compaction root for slow dirty admission")?;
					let previous_db_size_pages = previous_head
						.as_ref()
						.map_or(db_size_pages, |head| head.db_size_pages);
					let txid = previous_head
						.as_ref()
						.map_or(Ok(1), |head| {
							head.head_txid
								.checked_add(1)
								.context("sqlite head txid overflowed")
						})?;
					ensure!(
						txid == object.txid,
						"sqlite slow commit staged txid {} did not match publish txid {}",
						object.txid,
						txid
					);

					let truncate_cleanup = collect_truncate_cleanup(
						&tx,
						branch_id,
						previous_db_size_pages,
						db_size_pages,
					)
					.await?;
					test_hooks::maybe_pause_after_truncate_cleanup(&database_id).await;
					#[cfg(feature = "test-faults")]
					super::apply::maybe_fire_commit_fault(
						&fault_controller,
						&database_id,
						crate::fault::CommitFaultPoint::AfterTruncateCleanup,
						Some(branch_id),
					)
					.await?;

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
						versionstamp: udb::INCOMPLETE_VERSIONSTAMP,
						db_size_pages,
						post_apply_checksum: new_head.post_apply_checksum,
					};
					let encoded_commit_row =
						encode_commit_row(commit_row).context("encode sqlite commit row")?;
					let versionstamped_commit_row = udb::append_versionstamp_offset(
						encoded_commit_row.clone(),
						&udb::INCOMPLETE_VERSIONSTAMP,
					)
					.context("prepare versionstamped sqlite commit row")?;
					let commit_key = keys::branch_commit_key(branch_id, txid);
					let vtx_storage_key =
						keys::branch_vtx_key(branch_id, udb::INCOMPLETE_VERSIONSTAMP);
					let versionstamped_vtx_key = udb::append_versionstamp_offset(
						vtx_storage_key.clone(),
						&udb::INCOMPLETE_VERSIONSTAMP,
					)
					.context("prepare versionstamped sqlite vtx key")?;
					let dirty_pgnos = dirty_pages
						.iter()
						.map(|page| page.pgno)
						.collect::<BTreeSet<_>>();

					let encoded_manifest = object.encoded_manifest.clone();
					let manifest_key = keys::branch_delta_manifest_key(branch_id, txid);
					let object_ref_key =
						keys::branch_delta_object_ref_key(branch_id, object.object_id);
					let encoded_committed_meta = object.encoded_committed_meta.clone();
					let stage_finalize_rows = stage_context
						.as_ref()
						.map(|context| {
							let finalized = CommitStageFinalized {
								stage_id: context.stage_id,
								object_id: object.object_id,
								txid,
								finalized_at_ms: now_ms,
							};
							let mut meta = context.stage_meta.clone();
							meta.state = CommitStageState::Finalized { txid };
							Ok::<_, anyhow::Error>((
								keys::branch_commit_stage_finalized_key(branch_id, context.stage_id),
								encode_commit_stage_finalized(finalized)?,
								keys::branch_commit_stage_meta_key(branch_id, context.stage_id),
								encode_commit_stage_meta(meta)?,
							))
						})
						.transpose()?;
					let encoded_page_index_rows = object
						.page_index
						.iter()
						.map(|(pgno, entry)| {
							Ok((
								keys::branch_delta_pageidx_key(branch_id, txid, *pgno),
								encode_delta_page_index_entry(entry.clone())?,
							))
						})
						.collect::<Result<Vec<_>>>()?;

					let added_bytes = tracked_entry_size(&head_key, &encoded_head)?
						+ tracked_entry_size(&commit_key, &encoded_commit_row)?
						+ tracked_entry_size(&vtx_storage_key, &txid_bytes)?
						+ tracked_entry_size(&manifest_key, &encoded_manifest)?
						+ tracked_entry_size(&object_ref_key, &txid_bytes)?
						+ tracked_entry_size(&object_meta_key, &encoded_committed_meta)?
						+ stage_finalize_rows
							.as_ref()
							.map_or(Ok(0), |(finalized_key, finalized_value, meta_key, meta_value)| {
								Ok::<_, anyhow::Error>(
									tracked_entry_size(finalized_key, finalized_value)?
										+ tracked_entry_size(meta_key, meta_value)?,
								)
							})?
						+ object.chunk_tracked_bytes
						+ encoded_page_index_rows
							.iter()
							.map(|(key, value)| tracked_entry_size(key, value))
							.sum::<Result<i64>>()?
						+ truncate_cleanup.added_bytes
						+ dirty_pgnos
							.iter()
							.map(|pgno| {
								tracked_entry_size(
									&keys::branch_pidx_key(branch_id, *pgno),
									&txid_bytes,
								)
							})
							.sum::<Result<i64>>()?;
					let removed_bytes = head_bytes
						.as_ref()
						.map_or(Ok(0), |bytes| tracked_entry_size(&head_key, bytes))?
						+ truncate_cleanup.deleted_bytes;
					let quota_delta = added_bytes
						.checked_sub(removed_bytes)
						.context("sqlite slow commit quota delta overflowed i64")?;
					let would_be = storage_used
						.checked_add(quota_delta)
						.context("sqlite slow commit quota check overflowed i64")?;
					let would_be_with_staged = if let Some(stage_context) = &stage_context {
						would_be
							.checked_add(staged_storage_used)
							.and_then(|value| {
								value.checked_sub(stage_context.reserved_storage_bytes)
							})
							.context("sqlite slow commit staged quota check overflowed i64")?
					} else {
						would_be
							.checked_add(staged_storage_used)
							.context("sqlite slow commit staged quota check overflowed i64")?
					};
					let burst_signal =
						burst_mode::read_branch_signal_for_head(txid, compaction_root.as_ref());
					let deltas_available = admit_deltas_available(
						&tx,
						branch_id,
						txid,
						compaction_root.as_ref(),
						burst_signal.cold_watermark_txid,
						now_ms,
						last_deltas_available_at_ms,
					)
					.await?;
					let hot_quota_cap = burst_mode::adjusted_hot_quota_cap(
						quota::SQLITE_MAX_STORAGE_BYTES,
						burst_signal,
					)?;
					quota::cap_check_with_cap(would_be_with_staged, hot_quota_cap)?;

					#[cfg(feature = "test-faults")]
					super::apply::maybe_fire_commit_fault(
						&fault_controller,
						&database_id,
						crate::fault::CommitFaultPoint::BeforeDeltaWrites,
						Some(branch_id),
					)
					.await?;
					tx.informal().set(&manifest_key, &encoded_manifest);
					tx.informal().set(&object_ref_key, &txid_bytes);
					tx.informal().set(&object_meta_key, &encoded_committed_meta);
					if let Some((finalized_key, finalized_value, meta_key, meta_value)) =
						&stage_finalize_rows
					{
						tx.informal().set(finalized_key, finalized_value);
						tx.informal().set(meta_key, meta_value);
					}
					for (key, value) in &encoded_page_index_rows {
						tx.informal().set(key, value);
					}
					#[cfg(feature = "test-faults")]
					super::apply::maybe_fire_commit_fault(
						&fault_controller,
						&database_id,
						crate::fault::CommitFaultPoint::BeforePidxWrites,
						Some(branch_id),
					)
					.await?;
					for pgno in &dirty_pgnos {
						tx.informal()
							.set(&keys::branch_pidx_key(branch_id, *pgno), &txid_bytes);
					}
					for row in &truncate_cleanup.pidx_clears {
						fence_truncate_cleanup_row(&tx, row).await?;
						tx.informal().clear(&row.key);
					}
					for row in &truncate_cleanup.shard_clears {
						fence_truncate_cleanup_row(&tx, row).await?;
						tx.informal().clear(&row.key);
					}
					for (row, value) in &truncate_cleanup.shard_writes {
						fence_truncate_cleanup_row(&tx, row).await?;
						tx.informal().set(&row.key, value);
					}
					#[cfg(feature = "test-faults")]
					super::apply::maybe_fire_commit_fault(
						&fault_controller,
						&database_id,
						crate::fault::CommitFaultPoint::BeforeHeadWrite,
						Some(branch_id),
					)
					.await?;
					tx.informal().set(&head_key, &encoded_head);
					if head_at_fork_bytes.is_some() {
						tx.informal().clear(&head_at_fork_key);
					}
					if branch_resolution.bucket_initialized {
						branch::write_root_bucket_metadata(
							&tx,
							bucket_id,
							branch_resolution.bucket_branch_id,
							now_ms,
							&udb::INCOMPLETE_VERSIONSTAMP,
						)?;
					}
					if branch_resolution.database_initialized {
						write_root_branch_metadata(
							&tx,
							branch_id,
							branch_resolution.bucket_branch_id,
							&database_id,
							now_ms,
							&udb::INCOMPLETE_VERSIONSTAMP,
							branch_resolution.bucket_initialized,
						)
						.await?;
					}
					#[cfg(feature = "test-faults")]
					super::apply::maybe_fire_commit_fault(
						&fault_controller,
						&database_id,
						crate::fault::CommitFaultPoint::BeforeCommitRows,
						Some(branch_id),
					)
					.await?;
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
					#[cfg(feature = "test-faults")]
					super::apply::maybe_fire_commit_fault(
						&fault_controller,
						&database_id,
						crate::fault::CommitFaultPoint::BeforeQuotaMutation,
						Some(branch_id),
					)
					.await?;
					if quota_delta != 0 {
						quota::atomic_add_branch(&tx, branch_id, quota_delta);
					}
					if let Some(stage_context) = &stage_context {
						if stage_context.reserved_storage_bytes != 0 {
							quota::atomic_add_branch_staged(
								&tx,
								branch_id,
								-stage_context.reserved_storage_bytes,
							);
						}
					}
					let access_bucket = touch_access_if_bucket_advanced(
						&tx,
						branch_id,
						cached_access_bucket,
						now_ms,
					)
					.await?;

					Ok(SlowCommitTxResult {
						branch_id,
						branch_ancestry: preflight.branch_ancestry,
						access_bucket,
						txid,
						deltas_available,
						dirty_pgnos,
						truncated_pgnos: truncate_cleanup.truncated_pgnos,
						added_bytes,
						storage_used: would_be,
					})
				}
			})
			.await
	}
}

async fn write_object_batch(
	udb: &universaldb::Database,
	batch: Vec<(Vec<u8>, Vec<u8>)>,
) -> Result<()> {
	udb.run(move |tx| {
		let batch = batch.clone();
		async move {
			for (key, value) in &batch {
				tx.informal().set(key, value);
			}
			Ok(())
		}
	})
	.await
}

async fn resolve_slow_publish_branch(
	tx: &universaldb::Transaction,
	bucket_id: crate::conveyer::types::BucketId,
	database_id: &str,
	preflight: &SlowCommitPreflight,
) -> Result<BranchResolution> {
	if preflight.database_initialized {
		if let Some(current_branch_id) = branch::resolve_database_branch_in_bucket(
			tx,
			preflight.bucket_branch_id,
			database_id,
			Serializable,
		)
		.await?
		{
			let actual_head_txid = read_branch_head_txid(tx, current_branch_id).await?;
			return Err(SqliteStorageError::HeadFenceMismatch {
				expected_head_txid: preflight.observed_head_txid,
				actual_head_txid,
			}
			.into());
		}

		return Ok(BranchResolution {
			branch_id: preflight.branch_id,
			bucket_branch_id: preflight.bucket_branch_id,
			bucket_initialized: preflight.bucket_initialized,
			database_initialized: true,
		});
	}

	let branch_resolution = resolve_or_allocate_branch(tx, bucket_id, database_id).await?;
	if branch_resolution.branch_id != preflight.branch_id {
		let actual_head_txid = read_branch_head_txid(tx, branch_resolution.branch_id)
			.await
			.unwrap_or(0);
		return Err(SqliteStorageError::HeadFenceMismatch {
			expected_head_txid: preflight.observed_head_txid,
			actual_head_txid,
		}
		.into());
	}

	Ok(branch_resolution)
}

async fn read_branch_head_txid(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
) -> Result<u64> {
	let head_bytes = tx_get_value(tx, &keys::branch_meta_head_key(branch_id), Serializable).await?;
	let head_at_fork_bytes =
		tx_get_value(tx, &keys::branch_meta_head_at_fork_key(branch_id), Serializable).await?;
	let Some(bytes) = head_bytes.as_ref().or(head_at_fork_bytes.as_ref()) else {
		return Ok(0);
	};
	Ok(decode_db_head(bytes)?.head_txid)
}

fn build_delta_object(
	txid: u64,
	db_size_pages: u32,
	now_ms: i64,
	dirty_pages: &[DirtyPage],
) -> Result<LargeDeltaObject> {
	build_delta_object_with_ids(
		txid,
		db_size_pages,
		now_ms,
		Uuid::new_v4(),
		Uuid::new_v4(),
		dirty_pages,
	)
}

fn build_delta_object_with_ids(
	txid: u64,
	db_size_pages: u32,
	now_ms: i64,
	stage_id: Uuid,
	object_id: Uuid,
	dirty_pages: &[DirtyPage],
) -> Result<LargeDeltaObject> {
	let encoded =
		LtxEncoder::new(LtxHeader::delta(txid, db_size_pages, now_ms)).encode_with_index(dirty_pages)?;
	let object_hash = hash_bytes(&encoded.bytes);
	let encoded_len =
		u64::try_from(encoded.bytes.len()).context("sqlite large delta length exceeded u64")?;
	let chunks = encoded
		.bytes
		.chunks(DELTA_OBJECT_CHUNK_BYTES)
		.enumerate()
		.map(|(chunk_idx, chunk)| {
			Ok((
				u32::try_from(chunk_idx).context("sqlite large delta chunk index exceeded u32")?,
				chunk.to_vec(),
			))
		})
		.collect::<Result<Vec<_>>>()?;
	let chunk_count =
		u32::try_from(chunks.len()).context("sqlite large delta chunk count exceeded u32")?;

	let page_hashes = dirty_pages
		.iter()
		.map(|page| (page.pgno, hash_bytes(&page.bytes)))
		.collect::<BTreeMap<_, _>>();
	let page_index = encoded
		.page_index
		.iter()
		.map(|entry| {
			Ok((
				entry.pgno,
				delta_page_index_entry(txid, object_id, entry, &page_hashes)?,
			))
		})
		.collect::<Result<Vec<_>>>()?;

	let stage_meta = DeltaObjectMeta {
		object_id,
		stage_id,
		staged_txid: txid,
		chunk_count,
		encoded_len,
		object_hash,
		state: DeltaObjectState::StageOwned,
		created_at_ms: now_ms,
		expires_after_ms: now_ms.saturating_add(STAGE_TTL_MS),
	};
	let committed_meta = DeltaObjectMeta {
		state: DeltaObjectState::Committed { txid },
		..stage_meta.clone()
	};
	let manifest = DeltaManifest {
		txid,
		object_id,
		chunk_count,
		encoded_len,
		object_hash,
	};
	let encoded_stage_meta = encode_delta_object_meta(stage_meta)?;
	let encoded_committed_meta = encode_delta_object_meta(committed_meta)?;
	let encoded_manifest = encode_delta_manifest(manifest)?;
	let chunk_tracked_bytes = chunks
		.iter()
		.map(|(chunk_idx, chunk)| {
			tracked_entry_size(
				&keys::branch_delta_object_chunk_key(DatabaseBranchId::nil(), object_id, *chunk_idx),
				chunk,
			)
		})
		.sum::<Result<i64>>()?;

	Ok(LargeDeltaObject {
		txid,
		stage_id,
		object_id,
		chunks,
		page_index,
		encoded_len,
		encoded_stage_meta,
		encoded_committed_meta,
		encoded_manifest,
		chunk_tracked_bytes,
	})
}

fn delta_page_index_entry(
	txid: u64,
	object_id: Uuid,
	entry: &LtxPageIndexEntry,
	page_hashes: &BTreeMap<u32, [u8; 32]>,
) -> Result<DeltaPageIndexEntry> {
	Ok(DeltaPageIndexEntry {
		txid,
		object_id,
		encoded_offset: entry.offset,
		encoded_size: u32::try_from(entry.size)
			.context("sqlite large delta page frame size exceeded u32")?,
		page_hash: *page_hashes
			.get(&entry.pgno)
			.context("sqlite large delta page index referenced unknown page")?,
	})
}

fn validate_stage_dirty_pgnos(dirty_pgnos: &[u32]) -> Result<()> {
	if dirty_pgnos.len() > MAX_SQLITE_COMMIT_DIRTY_PAGES {
		return Err(SqliteStorageError::SqliteCommitPageLimitExceeded {
			dirty_page_count: u32::try_from(dirty_pgnos.len()).unwrap_or(u32::MAX),
			max_dirty_pages: MAX_SQLITE_COMMIT_DIRTY_PAGES as u32,
			page_size_bytes: keys::PAGE_SIZE,
		}
		.into());
	}
	let dirty_bytes = dirty_pgnos
		.len()
		.checked_mul(keys::PAGE_SIZE as usize)
		.context("sqlite dirty page byte count overflowed")?;
	if dirty_bytes > MAX_SQLITE_COMMIT_DIRTY_BYTES {
		return Err(SqliteStorageError::CommitTooLarge {
			actual_size_bytes: dirty_bytes as u64,
			max_size_bytes: MAX_SQLITE_COMMIT_DIRTY_BYTES as u64,
		}
		.into());
	}
	let mut last = None;
	for pgno in dirty_pgnos {
		ensure!(*pgno > 0, "sqlite commit stage does not accept page 0");
		if let Some(last) = last {
			ensure!(
				last < *pgno,
				"sqlite commit stage page numbers must be sorted and unique"
			);
		}
		last = Some(*pgno);
	}
	Ok(())
}

fn validate_stage_page_batch(pages: &[DirtyPage]) -> Result<()> {
	ensure!(!pages.is_empty(), "sqlite commit stage batch is empty");
	ensure!(
		pages.len() <= MAX_STAGE_PAGES_PER_BATCH,
		"sqlite commit stage batch had {} pages, limit is {}",
		pages.len(),
		MAX_STAGE_PAGES_PER_BATCH
	);
	let mut last = None;
	for page in pages {
		validate_stage_dirty_page(page)?;
		if let Some(last) = last {
			ensure!(
				last < page.pgno,
				"sqlite commit stage batch pages must be sorted and unique"
			);
		}
		last = Some(page.pgno);
	}
	Ok(())
}

fn validate_stage_dirty_page(page: &DirtyPage) -> Result<()> {
	ensure!(page.pgno > 0, "sqlite commit stage does not accept page 0");
	ensure!(
		page.bytes.len() == keys::PAGE_SIZE as usize,
		"sqlite commit stage page {} had {} bytes, expected {}",
		page.pgno,
		page.bytes.len(),
		keys::PAGE_SIZE
	);
	Ok(())
}

fn hash_dirty_pgnos(dirty_pgnos: &[u32]) -> [u8; 32] {
	let mut hasher = Sha256::new();
	for pgno in dirty_pgnos {
		hasher.update(pgno.to_be_bytes());
	}
	let digest = hasher.finalize();
	let mut out = [0_u8; 32];
	out.copy_from_slice(&digest);
	out
}

fn hash_dirty_pages(pages: &[DirtyPage]) -> [u8; 32] {
	let mut hasher = Sha256::new();
	for page in pages {
		hasher.update(page.pgno.to_be_bytes());
		hasher.update(hash_bytes(&page.bytes));
	}
	let digest = hasher.finalize();
	let mut out = [0_u8; 32];
	out.copy_from_slice(&digest);
	out
}

fn estimate_stage_reservation(
	branch_id: DatabaseBranchId,
	object_id: Uuid,
	staged_txid: u64,
	dirty_pgnos: &[u32],
) -> Result<i64> {
	let dirty_bytes = i64::try_from(dirty_pgnos.len())
		.context("sqlite dirty page count exceeded i64")?
		.checked_mul(i64::from(keys::PAGE_SIZE))
		.context("sqlite dirty page reservation overflowed")?;
	let estimated_object_chunks = dirty_bytes
		.checked_add(i64::try_from(DELTA_OBJECT_CHUNK_BYTES).unwrap_or(i64::MAX) - 1)
		.context("sqlite object chunk reservation overflowed")?
		/ i64::try_from(DELTA_OBJECT_CHUNK_BYTES).unwrap_or(i64::MAX);
	let chunk_key_bytes = (0..estimated_object_chunks)
		.map(|chunk_idx| {
			let chunk_idx = u32::try_from(chunk_idx).unwrap_or(u32::MAX);
			tracked_entry_size(
				&keys::branch_delta_object_chunk_key(branch_id, object_id, chunk_idx),
				&[],
			)
		})
		.sum::<Result<i64>>()?;
	let page_index_bytes = dirty_pgnos
		.iter()
		.map(|pgno| {
			tracked_entry_size(
				&keys::branch_delta_pageidx_key(branch_id, staged_txid, *pgno),
				&[0; 96],
			)
		})
		.sum::<Result<i64>>()?;
	let pidx_bytes = dirty_pgnos
		.iter()
		.map(|pgno| tracked_entry_size(&keys::branch_pidx_key(branch_id, *pgno), &[0; 8]))
		.sum::<Result<i64>>()?;
	dirty_bytes
		.checked_add(chunk_key_bytes)
		.and_then(|value| value.checked_add(page_index_bytes))
		.and_then(|value| value.checked_add(pidx_bytes))
		.and_then(|value| value.checked_add(STAGE_RESERVATION_HEADROOM_BYTES))
		.context("sqlite stage reservation overflowed")
}

fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
	let digest = Sha256::digest(bytes);
	let mut out = [0_u8; 32];
	out.copy_from_slice(&digest);
	out
}

fn map_udb_commit_error(err: anyhow::Error) -> anyhow::Error {
	for source in err.chain() {
		if let Some(universaldb::error::DatabaseError::TransactionTooLarge {
			actual_size_bytes,
			max_size_bytes,
		}) = source.downcast_ref::<universaldb::error::DatabaseError>()
		{
			return SqliteStorageError::CommitTooLarge {
				actual_size_bytes: *actual_size_bytes as u64,
				max_size_bytes: *max_size_bytes as u64,
			}
			.into();
		}
	}

	err
}

#[derive(Clone)]
struct SlowCommitPreflight {
	branch_id: DatabaseBranchId,
	bucket_branch_id: crate::conveyer::types::BucketBranchId,
	bucket_initialized: bool,
	database_initialized: bool,
	branch_ancestry: BranchAncestry,
	observed_head_txid: u64,
}

#[derive(Clone)]
struct StageFinalizeContext {
	stage_id: Uuid,
	stage_meta: CommitStageMeta,
	reserved_storage_bytes: i64,
}

struct LoadedCommitStage {
	branch_id: DatabaseBranchId,
	meta: CommitStageMeta,
}

#[derive(Clone)]
struct LargeDeltaObject {
	txid: u64,
	stage_id: Uuid,
	object_id: Uuid,
	chunks: Vec<(u32, Vec<u8>)>,
	page_index: Vec<(u32, DeltaPageIndexEntry)>,
	encoded_len: u64,
	encoded_stage_meta: Vec<u8>,
	encoded_committed_meta: Vec<u8>,
	encoded_manifest: Vec<u8>,
	chunk_tracked_bytes: i64,
}

struct SlowCommitTxResult {
	branch_id: DatabaseBranchId,
	branch_ancestry: BranchAncestry,
	access_bucket: Option<i64>,
	txid: u64,
	deltas_available: Option<DeltasAvailable>,
	dirty_pgnos: BTreeSet<u32>,
	truncated_pgnos: Vec<u32>,
	added_bytes: i64,
	storage_used: i64,
}
