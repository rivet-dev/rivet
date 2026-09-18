use std::time::Instant;

use super::*;

pub(crate) async fn read_manager_fdb_snapshot(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	default_pitr_policy: Option<PitrPolicy>,
	now_ms: i64,
) -> Result<ManagerFdbSnapshot> {
	let branch_record = tx_get_value(tx, &keys::branches_list_key(branch_id), Serializable)
		.await?
		.as_deref()
		.map(decode_database_branch_record)
		.transpose()
		.context("decode sqlite database branch record for compaction manager")?;
	let head = tx_get_value(tx, &keys::branch_meta_head_key(branch_id), Serializable)
		.await?
		.as_deref()
		.map(decode_db_head)
		.transpose()
		.context("decode sqlite head for compaction manager")?;
	let root = tx_get_value(
		tx,
		&keys::branch_compaction_root_key(branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root for manager refresh")?
	.unwrap_or(CompactionRoot {
		schema_version: 1,
		manifest_generation: 0,
		hot_watermark_txid: 0,
		cold_watermark_txid: 0,
		cold_watermark_versionstamp: [0; 16],
	});
	let dirty_key = keys::sqlite_cmp_dirty_key(branch_id);
	let dirty_bytes = tx_get_value(tx, &dirty_key, Serializable).await?;
	let dirty = dirty_bytes
		.as_deref()
		.map(decode_sqlite_cmp_dirty)
		.transpose()
		.context("decode sqlite dirty marker for compaction manager")?;
	let mut db_pins = history_pin::read_db_history_pins(tx, branch_id, Serializable).await?;
	let bucket_proof_blocked_reclaim =
		resolve_bucket_fork_pins(tx, branch_id, &mut db_pins).await?;
	// Policy overrides currently require resolving a branch back to its bucket and
	// database by scanning global pointer indexes. Doing that inside manager
	// refresh can age out the FDB transaction when many actors cross the hot
	// compaction threshold together.
	let pitr_policy = default_pitr_policy;
	let shard_cache_policy = ShardCachePolicy::default();
	let hot_inputs = read_hot_input_snapshot(
		tx,
		branch_id,
		head.as_ref(),
		&root,
		None,
		None,
		Snapshot,
		pitr_policy,
		now_ms,
	)
	.await?;
	let mut reclaim_budget = CompactionBatchBudget::fdb();
	let mut reclaim_inputs = read_reclaim_input_snapshot(
		tx,
		branch_id,
		&root,
		&db_pins,
		branch_record.as_ref(),
		shard_cache_policy,
		// The manager refresh classifies from the start of the cold prefix and the start of commit
		// history; the reclaim drain owns the per-pass cursor advance.
		None,
		0,
		None,
		Snapshot,
		now_ms,
		&mut reclaim_budget,
		// The refresh has no cursor to hand back, so a truncated scan would cost it a full re-read and
		// buy nothing.
		None,
		true,
	)
	.await?;
	// Bounded first dead-shard chunk, used only to decide whether to dispatch a reclaim job: its real
	// candidates (supersessions detected within this one chunk) signal that dead-shard work exists. The
	// candidates themselves are discarded here, not deleted. The companion runs the full
	// `SweepDeadShardVersions` walk, which detects cross-chunk supersessions and holds `prev` in local
	// memory. `has_more` is deliberately not used as the trigger: a branch with more than one chunk of
	// folds is always "has more", so it would keep `plan_reclaim_job` perpetually `Some` and a forced
	// reclaim could never settle.
	let dead_shard_chunk = read_dead_shard_versions_chunk(
		tx,
		branch_id,
		&db_pins,
		&reclaim_inputs.pitr_interval_retention,
		&DeadShardScanState::default(),
		Snapshot,
		&mut reclaim_budget,
	)
	.await?;
	reclaim_inputs.dead_shard_sweep_needed = !dead_shard_chunk.candidates.is_empty();
	let hot_lag = head.as_ref().map_or(0, |head| {
		head.head_txid.saturating_sub(root.hot_watermark_txid)
	});
	let has_actionable_lag = hot_lag >= quota::COMPACTION_DELTA_THRESHOLD
		|| !reclaim_inputs.delta_reclaim_segments.is_empty()
		|| !reclaim_inputs.commit_reclaim_txids.is_empty();
	let cleared_dirty = if !has_actionable_lag {
		if let Some(expected_dirty) = dirty_bytes {
			udb::compare_and_clear(tx, &dirty_key, &expected_dirty);
			true
		} else {
			false
		}
	} else {
		false
	};

	Ok(ManagerFdbSnapshot {
		branch_record,
		head,
		root,
		dirty,
		db_pins,
		hot_inputs,
		reclaim_inputs,
		bucket_proof_blocked_reclaim,
		cleared_dirty,
	})
}

pub(crate) async fn resolve_bucket_fork_pins(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	db_pins: &mut Vec<DbHistoryPin>,
) -> Result<bool> {
	let catalog_rows = tx_scan_prefix_values(
		tx,
		&keys::bucket_catalog_by_db_prefix(branch_id),
		Serializable,
	)
	.await?;
	if catalog_rows.len() >= CMP_FDB_BATCH_MAX_KEYS {
		tracing::warn!(
			?branch_id,
			row_count = catalog_rows.len(),
			"retaining sqlite history because bucket catalog proof is too large"
		);
		return Ok(true);
	}

	for (_, value) in catalog_rows {
		let catalog_fact = decode_bucket_catalog_db_fact(&value)
			.context("decode sqlite bucket catalog proof fact")?;
		if catalog_fact.database_branch_id != branch_id {
			tracing::warn!(
				?branch_id,
				?catalog_fact,
				"retaining sqlite history because bucket catalog proof has wrong branch"
			);
			return Ok(true);
		}
		if resolve_bucket_catalog_forks(tx, branch_id, db_pins, &catalog_fact).await? {
			return Ok(true);
		}
	}

	Ok(false)
}

pub(crate) async fn resolve_bucket_catalog_forks(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	db_pins: &mut Vec<DbHistoryPin>,
	catalog_fact: &BucketCatalogDbFact,
) -> Result<bool> {
	let mut queue = vec![catalog_fact.bucket_branch_id];
	let mut visited = BTreeSet::new();
	let mut inspected_rows = 0_usize;

	for depth in 0..=MAX_BUCKET_DEPTH {
		let Some(source_bucket_branch_id) = queue.pop() else {
			return Ok(false);
		};
		if !visited.insert(source_bucket_branch_id) {
			continue;
		}

		let child_rows = tx_scan_prefix_values(
			tx,
			&keys::bucket_child_prefix(source_bucket_branch_id),
			Serializable,
		)
		.await?;
		inspected_rows = inspected_rows.saturating_add(child_rows.len());
		if inspected_rows >= CMP_FDB_BATCH_MAX_KEYS {
			tracing::warn!(
				?branch_id,
				?source_bucket_branch_id,
				row_count = inspected_rows,
				"retaining sqlite history because bucket child proof is too large"
			);
			return Ok(true);
		}

		for (_, value) in child_rows {
			let child_fact =
				decode_bucket_fork_fact(&value).context("decode sqlite bucket child fact")?;
			if child_fact.source_bucket_branch_id != source_bucket_branch_id {
				tracing::warn!(
					?branch_id,
					?child_fact,
					"retaining sqlite history because bucket child proof has wrong source"
				);
				return Ok(true);
			}
			if !bucket_fork_can_inherit_database(&child_fact, catalog_fact) {
				continue;
			}
			if bucket_fork_pin_fact_is_missing_or_changed(tx, &child_fact).await? {
				tracing::warn!(
					?branch_id,
					?child_fact,
					"retaining sqlite history because bucket fork proof is missing"
				);
				return Ok(true);
			}
			if materialize_bucket_fork_pin(tx, branch_id, db_pins, &child_fact).await? {
				return Ok(true);
			}
			queue.push(child_fact.target_bucket_branch_id);
		}

		if depth == MAX_BUCKET_DEPTH && !queue.is_empty() {
			tracing::warn!(
				?branch_id,
				"retaining sqlite history because bucket proof exceeded max depth"
			);
			return Ok(true);
		}
	}

	Ok(false)
}

pub(crate) fn bucket_fork_can_inherit_database(
	fork_fact: &BucketForkFact,
	catalog_fact: &BucketCatalogDbFact,
) -> bool {
	fork_fact.fork_versionstamp >= catalog_fact.catalog_versionstamp
		&& catalog_fact
			.tombstone_versionstamp
			.map_or(true, |tombstone_versionstamp| {
				fork_fact.fork_versionstamp < tombstone_versionstamp
			})
}

pub(crate) async fn bucket_fork_pin_fact_is_missing_or_changed(
	tx: &universaldb::Transaction,
	child_fact: &BucketForkFact,
) -> Result<bool> {
	let Some(fork_pin_bytes) = tx_get_value(
		tx,
		&keys::bucket_fork_pin_key(
			child_fact.source_bucket_branch_id,
			child_fact.fork_versionstamp,
			child_fact.target_bucket_branch_id,
		),
		Serializable,
	)
	.await?
	else {
		return Ok(true);
	};
	let fork_pin_fact =
		decode_bucket_fork_fact(&fork_pin_bytes).context("decode sqlite bucket fork fact")?;

	Ok(fork_pin_fact != *child_fact)
}

pub(crate) async fn materialize_bucket_fork_pin(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	db_pins: &mut Vec<DbHistoryPin>,
	fork_fact: &BucketForkFact,
) -> Result<bool> {
	let Some((at_txid, at_versionstamp, commit)) =
		latest_commit_at_or_before_versionstamp(tx, branch_id, fork_fact.fork_versionstamp).await?
	else {
		tracing::warn!(
			?branch_id,
			?fork_fact,
			"retaining sqlite history because bucket fork versionstamp could not be resolved"
		);
		return Ok(true);
	};

	history_pin::write_bucket_fork_pin(
		tx,
		branch_id,
		fork_fact.target_bucket_branch_id,
		at_versionstamp,
		at_txid,
		commit.wall_clock_ms,
	)?;
	db_pins.retain(|pin| pin.owner_bucket_branch_id != Some(fork_fact.target_bucket_branch_id));
	db_pins.push(DbHistoryPin {
		at_versionstamp,
		at_txid,
		kind: crate::types::DbHistoryPinKind::BucketFork,
		owner_database_branch_id: None,
		owner_bucket_branch_id: Some(fork_fact.target_bucket_branch_id),
		owner_restore_point: None,
		created_at_ms: commit.wall_clock_ms,
	});

	Ok(false)
}

pub(crate) async fn read_effective_pitr_policy_for_branch(
	tx: &universaldb::Transaction,
	branch_record: Option<&DatabaseBranchRecord>,
	default_policy: Option<PitrPolicy>,
) -> Result<Option<PitrPolicy>> {
	// A disabled cluster ignores stored overrides, so short-circuit before reading them. Leaving the
	// override rows in place means re-enabling PITR restores each scope's settings as they were.
	let Some(default_policy) = default_policy else {
		return Ok(None);
	};
	let Some(branch_record) = branch_record else {
		return Ok(Some(default_policy));
	};
	let Some((bucket_id, database_id)) =
		resolve_policy_scope_for_branch(tx, branch_record.branch_id).await?
	else {
		return Ok(Some(default_policy));
	};

	if let Some(policy) = tx_get_value(
		tx,
		&keys::database_pitr_policy_key(bucket_id, &database_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_pitr_policy)
	.transpose()
	.context("decode sqlite database PITR policy for compaction manager")?
	{
		return Ok(Some(policy));
	}

	tx_get_value(tx, &keys::bucket_policy_pitr_key(bucket_id), Serializable)
		.await?
		.as_deref()
		.map(decode_pitr_policy)
		.transpose()
		.context("decode sqlite bucket PITR policy for compaction manager")
		.map(|policy| Some(policy.unwrap_or(default_policy)))
}

pub(crate) async fn read_effective_shard_cache_policy_for_branch(
	tx: &universaldb::Transaction,
	branch_record: Option<&DatabaseBranchRecord>,
) -> Result<ShardCachePolicy> {
	let Some(branch_record) = branch_record else {
		return Ok(ShardCachePolicy::default());
	};
	let Some((bucket_id, database_id)) =
		resolve_policy_scope_for_branch(tx, branch_record.branch_id).await?
	else {
		return Ok(ShardCachePolicy::default());
	};

	if let Some(policy) = tx_get_value(
		tx,
		&keys::database_shard_cache_policy_key(bucket_id, &database_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_shard_cache_policy)
	.transpose()
	.context("decode sqlite database shard cache policy for compaction manager")?
	{
		return Ok(policy);
	}

	tx_get_value(
		tx,
		&keys::bucket_policy_shard_cache_key(bucket_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_shard_cache_policy)
	.transpose()
	.context("decode sqlite bucket shard cache policy for compaction manager")
	.map(|policy| policy.unwrap_or_default())
}

/// Resolves the `(bucket, database_id)` policy scope a database branch belongs to.
///
/// One point read of the branch's owner index row, which every `DBPTR` write maintains. A branch with
/// no row resolves to `None` and both callers fall back to the cluster default.
///
/// Do not reintroduce a fallback that derives the scope when the row is missing. The only way to map a
/// branch back to its database is to scan the `DBPTR` partition looking for the pointer that names it,
/// which is unbounded in cluster size inside a transaction bounded to five seconds. Measured on a
/// production cluster that was 173 MB of pointers, paid per call by roughly 1.3M branches predating
/// the index, against per-scope override rows that did not exist.
///
/// The scope exists only to look up policy overrides (`database_pitr_policy_key`,
/// `bucket_policy_pitr_key`, `database_shard_cache_policy_key`). Defaulting costs a branch with no
/// owner row its per-database override, which is a retention or cache-sizing difference on branches
/// created before the index existed, not a correctness property of compaction.
pub(crate) async fn resolve_policy_scope_for_branch(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
) -> Result<Option<(BucketId, String)>> {
	let Some(owner_bytes) = tx_get_value(
		tx,
		&keys::database_branch_owner_key(branch_id),
		Serializable,
	)
	.await?
	else {
		return Ok(None);
	};

	let owner = decode_database_branch_owner(&owner_bytes)
		.context("decode sqlite database branch owner for policy scope")?;

	Ok(Some((owner.bucket_id, owner.database_id)))
}

pub(crate) async fn latest_commit_at_or_before_versionstamp(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	versionstamp_cap: [u8; 16],
) -> Result<Option<(u64, [u8; 16], CommitRow)>> {
	// One descending read capped at a single row, bounded above by the cap itself. Scanning the whole
	// VTX prefix and keeping the last row under the cap read one key per commit the branch has ever
	// made, and the loop's row cap could not help: the scan was already fully materialized before the
	// first comparison ran, so the cap bounded CPU rather than FDB reads.
	let range_end = end_of_key_range(&keys::branch_vtx_key(branch_id, versionstamp_cap));
	let Some((key, value)) = tx_get_range_last(
		tx,
		&keys::branch_vtx_prefix(branch_id),
		&range_end,
		Serializable,
	)
	.await?
	else {
		return Ok(None);
	};
	let versionstamp = decode_branch_vtx_versionstamp(branch_id, &key)?;
	let txid = decode_txid_value(&value)?;
	let Some(commit_bytes) =
		tx_get_value(tx, &keys::branch_commit_key(branch_id, txid), Serializable).await?
	else {
		return Ok(None);
	};
	let commit = decode_commit_row(&commit_bytes).context("decode sqlite bucket pin commit row")?;

	Ok(Some((txid, versionstamp, commit)))
}

pub(crate) fn decode_branch_vtx_versionstamp(
	branch_id: DatabaseBranchId,
	key: &[u8],
) -> Result<[u8; 16]> {
	let prefix = keys::branch_vtx_prefix(branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("branch VTX key did not start with expected prefix")?;
	ensure!(
		suffix.len() == 16,
		"branch VTX versionstamp suffix had {} bytes, expected 16",
		suffix.len()
	);

	suffix
		.try_into()
		.context("branch VTX versionstamp suffix should decode as 16 bytes")
}

pub(crate) fn decode_txid_value(value: &[u8]) -> Result<u64> {
	let bytes = <[u8; 8]>::try_from(value)
		.map_err(|_| anyhow::anyhow!("txid value had {} bytes, expected 8", value.len()))?;

	Ok(u64::from_be_bytes(bytes))
}

/// Key-count and value-byte budget for one compaction FDB transaction. Reclaim deletes use
/// `COMPARE_AND_CLEAR`, whose mutation carries the full compared value, so every candidate a plan
/// collects must count its value bytes here or the delete transaction can exceed FDB's transaction
/// size limit. One budget instance is shared across all reclaim lanes of a slice; the plan and
/// delete paths consume it in the same deterministic order so both derive identical sets.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CompactionBatchBudget {
	max_keys: usize,
	max_value_bytes: u64,
	key_count: usize,
	value_bytes: u64,
}

impl CompactionBatchBudget {
	pub(crate) fn fdb() -> Self {
		Self::with_limits(CMP_FDB_BATCH_MAX_KEYS, CMP_FDB_BATCH_MAX_VALUE_BYTES as u64)
	}

	/// Tests exercise budget capping with small limits so fixtures stay far below the production
	/// key and byte caps; production code uses `fdb()`.
	pub(crate) fn with_limits(max_keys: usize, max_value_bytes: u64) -> Self {
		CompactionBatchBudget {
			max_keys,
			max_value_bytes,
			key_count: 0,
			value_bytes: 0,
		}
	}

	pub(crate) fn can_add(&self, row_count: usize, value_bytes: u64) -> bool {
		self.key_count.saturating_add(row_count) <= self.max_keys
			&& self.value_bytes.saturating_add(value_bytes) <= self.max_value_bytes
	}

	pub(crate) fn add(&mut self, row_count: usize, value_bytes: u64) {
		self.key_count = self.key_count.saturating_add(row_count);
		self.value_bytes = self.value_bytes.saturating_add(value_bytes);
	}

	pub(crate) fn value_bytes(&self) -> u64 {
		self.value_bytes
	}

	pub(crate) fn key_count(&self) -> usize {
		self.key_count
	}
}

/// Reads the branch compaction manifest root, returning the genesis default when absent.
pub(crate) async fn read_compaction_root_or_default(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
) -> Result<CompactionRoot> {
	Ok(tx_get_value(
		tx,
		&keys::branch_compaction_root_key(branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root")?
	.unwrap_or(CompactionRoot {
		schema_version: 1,
		manifest_generation: 0,
		hot_watermark_txid: 0,
		cold_watermark_txid: 0,
		cold_watermark_versionstamp: [0; 16],
	}))
}

pub(crate) async fn read_hot_input_snapshot(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	head: Option<&DBHead>,
	root: &CompactionRoot,
	cursor_min_txid: Option<u64>,
	cursor_min_segment_pgno: Option<u32>,
	isolation_level: universaldb::utils::IsolationLevel,
	pitr_policy: Option<PitrPolicy>,
	now_ms: i64,
) -> Result<HotInputSnapshot> {
	let Some(head) = head else {
		return Ok(HotInputSnapshot::default());
	};
	// The cursor lets a companion plan a slice past already-staged-but-not-installed work
	// during an internal drain. When absent the slice starts at the installed hot watermark.
	let min_txid = cursor_min_txid.unwrap_or_else(|| root.hot_watermark_txid.saturating_add(1));
	if head.head_txid < min_txid {
		return Ok(HotInputSnapshot::default());
	}

	let max_txid = head.head_txid;
	let mut snapshot = HotInputSnapshot::default();
	let mut budget = CompactionBatchBudget::fdb();
	// Pages the selected commits touched, in ascending order. Built during commit selection so each
	// candidate can reserve budget for the PIDX rows it will make this slice clear, and reused as the
	// PIDX read set below.
	let mut slice_pgnos: BTreeSet<u32> = BTreeSet::new();
	// Budget the PIDX lane has reserved but not yet spent. A commit is admitted only if the budget
	// still holds its own rows plus every reserved PIDX row, so the clear lane below always fits.
	let mut reserved_pidx_keys = 0_usize;
	let mut reserved_pidx_bytes = 0_u64;

	let commit_scan_start = keys::branch_commit_key(branch_id, min_txid);
	let commit_scan_end = max_txid
		.checked_add(1)
		.map(|next_txid| keys::branch_commit_key(branch_id, next_txid))
		.unwrap_or_else(|| end_of_key_range(&keys::branch_commit_prefix(branch_id)));
	for (key, value) in tx_scan_range_values_limited(
		tx,
		&commit_scan_start,
		&commit_scan_end,
		CMP_FDB_BATCH_MAX_KEYS,
		isolation_level,
	)
	.await?
	{
		let txid = decode_branch_commit_txid(branch_id, &key)?;
		if txid > max_txid {
			break;
		}
		let commit =
			decode_commit_row(&value).context("decode sqlite commit row for hot planning")?;
		// Resume inside this commit only when it is the one the cursor stopped in. Every later commit
		// starts from its first page.
		let segment_cursor = (txid == min_txid)
			.then_some(cursor_min_segment_pgno)
			.flatten();
		// A resume page number only addresses a segmented commit. In the segmented layout a row is
		// keyed `prefix + first_pgno + '/' + chunk_idx`, in the legacy layout it is
		// `prefix + chunk_idx`, and any page number at or above one shard sorts above every chunk
		// index, so beginning at the segment prefix would read a legacy commit's rows as zero rows
		// and admit it as coverage with none of its pages folded. A cursor pointing into a legacy
		// commit cannot be produced any more, but one recorded before that changed can still be
		// resumed, so the layout is probed rather than assumed. The probe reads a single key, and
		// only for the one commit a cursor resumes into.
		let mut delta_scan_begin = keys::branch_delta_chunk_prefix(branch_id, txid);
		if let Some(first_pgno) = segment_cursor {
			if commit_is_segmented(tx, branch_id, txid, isolation_level).await? {
				delta_scan_begin = keys::branch_delta_segment_prefix(branch_id, txid, first_pgno);
			}
		}
		let delta_chunks = tx_scan_range_values_limited(
			tx,
			&delta_scan_begin,
			&keys::branch_delta_txid_scan_end(branch_id, txid),
			CMP_FDB_BATCH_MAX_KEYS,
			isolation_level,
		)
		.await?;
		// A scan that filled its limit may have stopped inside a blob, leaving it short of its own
		// chunks. Reassembling that would report a torn delta, so the trailing blob is dropped and
		// re-read whole by the next slice.
		//
		// Dropping every row it read is different from a commit that has no delta at all: the first
		// means the commit's next blob does not fit this slice, the second is an ordinary commit whose
		// delta was already reclaimed. Only the first defers the commit; the second is admitted with no
		// pages, which is what makes it a coverage txid.
		let scan_truncated = delta_chunks.len() >= CMP_FDB_BATCH_MAX_KEYS;
		let delta_chunks = if scan_truncated {
			drop_trailing_partial_delta_blob(branch_id, txid, delta_chunks)?
		} else {
			delta_chunks
		};
		let blob_too_large_for_slice = scan_truncated && delta_chunks.is_empty();
		let txid_value_bytes = u64::try_from(value.len())
			.unwrap_or(u64::MAX)
			.saturating_add(
				delta_chunks
					.iter()
					.map(|(_, value)| u64::try_from(value.len()).unwrap_or(u64::MAX))
					.fold(0_u64, u64::saturating_add),
			);

		// Reject on the commit's own cost before decoding its delta. A commit that cannot fit even
		// without the PIDX reservation is deferred to a later slice, and deferring it must not depend on
		// its delta being decodable.
		if blob_too_large_for_slice
			|| !budget.can_add(
				1 + delta_chunks.len() + reserved_pidx_keys,
				txid_value_bytes.saturating_add(reserved_pidx_bytes),
			) {
			if snapshot.selected_max_txid.is_none() {
				snapshot.oversized_commit_txid = Some(txid);
			}
			break;
		}

		// The page ranges of this commit the slice may admit, ascending and disjoint. A segmented
		// commit contributes one per blob it stored; a pre-segmentation commit stores everything in one
		// blob, so its ranges are cut from the decoded pages instead. Either way the unit of admission
		// is a shard-aligned page range, which is what lets a slice take part of a commit and still
		// fold every shard it touches to a complete image.
		let candidate_units = hot_admission_units(branch_id, txid, &delta_chunks, segment_cursor)?;

		// Pages each unit adds to the slice, and with them the PIDX rows the clear lane may have to
		// hold. Pages an earlier commit in this slice already touched share one PIDX row, so only the
		// new ones cost anything. This is an upper bound: a page with no PIDX row, or one a later commit
		// took ownership of, yields no row, and reserving for it only ends the slice marginally early.
		//
		// Units are admitted in order and the first one that does not fit ends the commit, because a
		// later unit's pages cannot be folded without the earlier ones having been.
		let mut admitted_pgnos: BTreeSet<u32> = BTreeSet::new();
		let mut admitted_pidx_keys = 0_usize;
		let mut stopped_at_pgno = None;
		for unit in &candidate_units {
			let unit_pgnos = unit
				.pgnos
				.iter()
				.copied()
				.filter(|pgno| !slice_pgnos.contains(pgno) && !admitted_pgnos.contains(pgno))
				.collect::<BTreeSet<u32>>();
			let unit_pidx_keys = admitted_pidx_keys + unit_pgnos.len();
			let unit_pidx_bytes = (unit_pidx_keys as u64).saturating_mul(PIDX_VALUE_BYTES);

			// Re-check with the PIDX rows this commit would add. Without the reservation a run of small
			// commits spends the whole budget on commits, the clear lane below finds no room, and the
			// slice folds pages whose PIDX rows survive. A surviving row's owner sits below the next
			// slice's `min_txid` forever, so it pins its delta and its commit against reclaim
			// permanently.
			if !budget.can_add(
				1 + delta_chunks.len() + reserved_pidx_keys + unit_pidx_keys,
				txid_value_bytes
					.saturating_add(reserved_pidx_bytes)
					.saturating_add(unit_pidx_bytes),
			) {
				stopped_at_pgno = Some(unit.first_pgno);
				break;
			}

			admitted_pidx_keys = unit_pidx_keys;
			admitted_pgnos.extend(unit_pgnos);
		}

		// Not even this commit's first page range fits an otherwise-empty slice. Nothing can make it
		// fit, so the drain stalls here rather than reporting itself finished. A commit with no pages
		// at all reaches this with no units and no stop, and is admitted as pure coverage.
		if admitted_pgnos.is_empty() && stopped_at_pgno.is_some() {
			if snapshot.selected_max_txid.is_none() {
				snapshot.oversized_commit_txid = Some(txid);
			}
			break;
		}

		let admitted_pidx_bytes = (admitted_pidx_keys as u64).saturating_mul(PIDX_VALUE_BYTES);
		budget.add(1 + delta_chunks.len(), txid_value_bytes);
		reserved_pidx_keys += admitted_pidx_keys;
		reserved_pidx_bytes = reserved_pidx_bytes.saturating_add(admitted_pidx_bytes);
		slice_pgnos.extend(admitted_pgnos);
		snapshot.commits.push((txid, commit));
		snapshot.delta_chunks.extend(delta_chunks);
		snapshot.selected_max_txid = Some(txid);

		// A partially admitted commit is always the last entry in a slice: the pages after the cut
		// belong to the same txid, and mixing a later txid in would fold pages above them.
		if let Some(first_pgno) = stopped_at_pgno {
			snapshot.selected_max_pgno_exclusive = Some(first_pgno);
			break;
		}
	}

	let Some(selected_max_txid) = snapshot.selected_max_txid else {
		return Ok(snapshot);
	};

	// PIDX entries to clear after staging are exactly the still-slice-owned pages of the selected
	// slice deltas (T1): a commit at txid `T` sets `PIDX[pgno] = T` for the pages it changed, so a page
	// the slice touched still has a slice-owned PIDX entry iff a later commit has not overwritten it.
	// Deriving them from the already-read slice deltas point-reads only `branch_pidx_key(pgno)` per
	// touched page (bounded by slice page count) instead of scanning the whole PIDX prefix (which scales
	// with database size and can age out the FDB transaction). The owner window `[min_txid,
	// selected_max_txid]` keeps exactly the rows the full-prefix filter would have kept, in the same
	// ascending-pgno order, so this is behavior-preserving.
	//
	// Commit selection reserved budget for every one of these rows at `PIDX_VALUE_BYTES` each, and
	// `decode_pidx_txid` below rejects any other width, so the reservation covers exactly what this
	// loop spends and `can_add` cannot reject a row the slice needs. It stays as a structural guard so
	// the lane keeps its own bound if the reservation above is ever changed or dropped.
	for pgno in slice_pgnos {
		let pidx_key = keys::branch_pidx_key(branch_id, pgno);
		let Some(value) = tx_get_value(tx, &pidx_key, isolation_level).await? else {
			continue;
		};
		let owner_txid = decode_pidx_txid(&value)?;
		if owner_txid < min_txid || owner_txid > selected_max_txid {
			continue;
		}
		if !budget.can_add(1, u64::try_from(value.len()).unwrap_or(u64::MAX)) {
			break;
		}
		budget.add(1, u64::try_from(value.len()).unwrap_or(u64::MAX));
		snapshot.pidx_entries.push((pidx_key, value));
	}
	snapshot.total_value_bytes = budget.value_bytes();

	snapshot.pitr_interval_coverage =
		select_pitr_interval_coverage(pitr_policy, &snapshot.commits, now_ms)?;

	Ok(snapshot)
}

pub(crate) fn select_pitr_interval_coverage(
	policy: Option<PitrPolicy>,
	commits: &[(u64, CommitRow)],
	now_ms: i64,
) -> Result<Vec<PitrIntervalSelection>> {
	// PITR disabled selects no coverage, so hot compaction folds only the drain's own max txid and
	// writes no `PITR_INTERVAL` rows. Rows an earlier enabled period wrote still expire on their own
	// stamps and keep pinning until then.
	let Some(policy) = policy else {
		return Ok(Vec::new());
	};
	ensure!(
		policy.interval_ms > 0,
		"sqlite PITR interval policy must be positive"
	);
	ensure!(
		policy.retention_ms > 0,
		"sqlite PITR retention policy must be positive"
	);

	let retention_floor_ms = now_ms.saturating_sub(policy.retention_ms);
	let mut selected_by_bucket = BTreeMap::<i64, PitrIntervalSelection>::new();
	for (txid, commit) in commits {
		if commit.wall_clock_ms < retention_floor_ms || commit.wall_clock_ms > now_ms {
			continue;
		}
		let bucket_start_ms =
			commit.wall_clock_ms.div_euclid(policy.interval_ms) * policy.interval_ms;
		let coverage = PitrIntervalCoverage {
			txid: *txid,
			versionstamp: commit.versionstamp,
			wall_clock_ms: commit.wall_clock_ms,
			expires_at_ms: commit.wall_clock_ms.saturating_add(policy.retention_ms),
		};
		let replace = selected_by_bucket
			.get(&bucket_start_ms)
			.map_or(true, |existing| {
				coverage.wall_clock_ms > existing.coverage.wall_clock_ms
					|| (coverage.wall_clock_ms == existing.coverage.wall_clock_ms
						&& coverage.txid > existing.coverage.txid)
			});
		if replace {
			selected_by_bucket.insert(
				bucket_start_ms,
				PitrIntervalSelection {
					bucket_start_ms,
					coverage,
				},
			);
		}
	}

	Ok(selected_by_bucket.into_values().collect())
}

pub(crate) async fn read_reclaim_input_snapshot(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	root: &CompactionRoot,
	db_pins: &[DbHistoryPin],
	_branch_record: Option<&DatabaseBranchRecord>,
	_shard_cache_policy: ShardCachePolicy,
	cold_scan_cursor: Option<ColdScanCursor>,
	commit_scan_cursor: u64,
	cursor_segment_pgno: Option<u32>,
	isolation_level: universaldb::utils::IsolationLevel,
	now_ms: i64,
	budget: &mut CompactionBatchBudget,
	// Wall-clock bound on the commit scan. `Some` only for callers that can hand a cursor back and be
	// re-dispatched; the scan gives up and sets `scan_truncated` rather than running past it.
	scan_deadline: Option<Instant>,
	// Whether to classify the commit/delta lane at all. False for the v2 reclaim drain, where
	// `SweepCommitDeltaChunk` derives and clears that lane itself, so deriving it here would scan the
	// same `COMMITS` window a second time per pass.
	derive_commit_delta: bool,
) -> Result<ReclaimInputSnapshot> {
	let (pitr_interval_retention, expired_pitr_interval_rows) =
		read_pitr_interval_reclaim_rows(tx, branch_id, now_ms, isolation_level, budget).await?;
	let cold_window = ColdObjectReclaimWindow::default();
	let (shard_cache_evictions, shard_lru_cleanup_keys) = (Vec::new(), Vec::new());
	let ColdObjectReclaimWindow {
		refs: cold_object_refs,
		next_cursor: next_cold_scan_cursor,
		..
	} = cold_window;
	// The dead-shard version-retention walk runs regardless of cold storage: it is the sole shard GC when
	// cold is off, and the structural counterpart to cold-only demotion when cold is on. It runs in the
	// standalone `SweepDeadShardVersions` activity (`read_dead_shard_versions_chunk` +
	// `delete_dead_shard_versions_tx`), which holds its cross-chunk context in local memory. The PITR
	// interval retention read here is exposed so that sweep can build its coverage set without re-reading.
	let mut snapshot = ReclaimInputSnapshot {
		cold_object_refs,
		shard_cache_evictions,
		shard_lru_cleanup_keys,
		expired_pitr_interval_rows,
		cold_scan_cursor,
		next_cold_scan_cursor,
		commit_scan_cursor,
		cursor_segment_pgno,
		next_commit_scan_cursor: commit_scan_cursor,
		next_segment_pgno: cursor_segment_pgno,
		pitr_interval_retention,
		..ReclaimInputSnapshot::default()
	};
	if !derive_commit_delta {
		// Not this caller's lane to report on. The sweep owns the cursor and the completion flag.
		snapshot.commit_scan_complete = true;
		return Ok(snapshot);
	}
	let window = read_commit_delta_reclaim_window(
		tx,
		branch_id,
		root,
		db_pins,
		&snapshot.pitr_interval_retention,
		commit_scan_cursor,
		cursor_segment_pgno,
		isolation_level,
		budget,
		scan_deadline,
	)
	.await?;
	snapshot.scan_truncated = window.scan_truncated;
	snapshot.commits = window.commits;
	snapshot.delta_chunks = window.delta_chunks;
	snapshot.delta_reclaim_segments = window.delta_reclaim_segments;
	snapshot.commit_reclaim_txids = window.commit_reclaim_txids;
	snapshot.next_commit_scan_cursor = window.next_commit_scan_cursor;
	snapshot.next_segment_pgno = window.next_segment_pgno;
	snapshot.commit_scan_complete = window.commit_scan_complete;
	snapshot.total_value_bytes = window.total_value_bytes;

	Ok(snapshot)
}

/// One bounded window of commit/delta reclaim candidates, derived from `commit_scan_cursor`.
#[derive(Default)]
pub(crate) struct CommitDeltaReclaimWindow {
	/// Every commit row the window scanned, not only the reclaimable ones. The delete needs the exact
	/// value it read to `compare_and_clear`, and the scan charges for all of them either way.
	pub(crate) commits: Vec<(u64, Vec<u8>, Vec<u8>, CommitRow)>,
	pub(crate) delta_chunks: Vec<(Vec<u8>, Vec<u8>)>,
	/// Reclaimable delta segments, identified individually rather than by txid. A large commit is
	/// stored as several shard-aligned segments whose pages can become droppable at different times,
	/// so classifying a whole txid at once would hold every segment hostage to its slowest one.
	pub(crate) delta_reclaim_segments: Vec<DeltaSegmentRef>,
	pub(crate) commit_reclaim_txids: Vec<u64>,
	pub(crate) next_commit_scan_cursor: u64,
	/// Where to resume inside `next_commit_scan_cursor`'s txid when its segments did not all fit this
	/// window. `None` means the txid was taken whole and the cursor points past it.
	pub(crate) next_segment_pgno: Option<u32>,
	pub(crate) commit_scan_complete: bool,
	pub(crate) total_value_bytes: u64,
	/// Set when the scan gave up on `scan_deadline`. The sets are partial, so they classify nothing.
	pub(crate) scan_truncated: bool,
}

/// Classifies one budget-bounded window of `COMMITS`/`DELTA` history for reclaim.
///
/// Split out of [`read_reclaim_input_snapshot`] so the sweep can derive and clear a window inside one
/// transaction. Deriving in one transaction and clearing in another forces the clearing side to
/// re-derive the whole window and reject on any mismatch, which doubles the read volume and turns
/// every race into discarded work. Derived and cleared together, the `Serializable` reads here are
/// themselves the fence: a racing pin or commit touches `DB_PIN`/`PIDX`/`COMMITS` and aborts the
/// transaction, so nothing downstream has to compare sets.
///
/// The caller owns the cursor. This reads one window forward from `commit_scan_cursor` and reports
/// where the next one starts, so a sweep advances by committing rather than by planning.
pub(crate) async fn read_commit_delta_reclaim_window(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	root: &CompactionRoot,
	db_pins: &[DbHistoryPin],
	pitr_interval_retention: &[PitrIntervalSelection],
	commit_scan_cursor: u64,
	cursor_segment_pgno: Option<u32>,
	isolation_level: universaldb::utils::IsolationLevel,
	budget: &mut CompactionBatchBudget,
	scan_deadline: Option<Instant>,
) -> Result<CommitDeltaReclaimWindow> {
	let mut window = CommitDeltaReclaimWindow {
		next_commit_scan_cursor: commit_scan_cursor,
		..CommitDeltaReclaimWindow::default()
	};
	// Nothing is folded until hot compaction has advanced, so there is no commit/delta history to
	// classify for reclaim.
	if root.hot_watermark_txid == 0 {
		window.commit_scan_complete = true;
		return Ok(window);
	}

	// Coverage txid set (the folds): pins, unexpired PITR reps, and head. A read can only land on a
	// fold, so a non-fold commit's metadata is unreachable and a folded delta is droppable once its
	// shards are materialized at every coverage fold that still needs them.
	let head_txid = tx_get_value(tx, &keys::branch_meta_head_key(branch_id), isolation_level)
		.await?
		.as_deref()
		.map(decode_db_head)
		.transpose()
		.context("decode sqlite head for reclaim classification")?
		.map(|head| head.head_txid);
	let mut coverage: BTreeSet<u64> = BTreeSet::new();
	coverage.extend(db_pins.iter().map(|pin| pin.at_txid));
	coverage.extend(
		pitr_interval_retention
			.iter()
			.map(|selection| selection.coverage.txid),
	);
	coverage.extend(head_txid);

	// COMMITS/VTX delete bound: below the lowest fold, capped at the cold watermark while cold is on so
	// cold still has the commit metadata it needs to publish past it (finding #8). DELTA deletion is
	// independent of this bound (cold never reads deltas).
	let commit_reclaim_bound = reclaim_delete_upper_bound(root, db_pins, pitr_interval_retention);

	let mut delta_reclaim_segments = Vec::new();
	let mut commit_reclaim_txids = Vec::new();
	// Commits whose every segment must pass the deferred gate before the row can be deleted, paired
	// with how many segments that is.
	let mut commit_gate_candidates: Vec<(u64, usize)> = Vec::new();
	// Folded-delta candidates whose shard-materialization gate is deferred until after the commit loop,
	// each carrying its shard set. Deferring lets a single bounded fold-window scan replace the global
	// fold-prefix scan (T2). Collected in ascending txid order, so `delta_reclaim_txids` keeps its order.
	let mut delta_gate_candidates: Vec<(DeltaSegmentRef, BTreeSet<u32>)> = Vec::new();
	// The `COMMITS` range is swept in budget-bounded windows that advance across drain passes. Starting
	// every pass at txid 0 instead lets retained rows at the head of the range (pinned, above the delete
	// bound, or still PIDX-owned) consume the whole budget on every pass, so reclaimable history behind
	// them is never reached and the drain reads the empty window as "nothing left to reclaim".
	if commit_scan_cursor > root.hot_watermark_txid {
		window.commit_scan_complete = true;
		return Ok(window);
	}
	let commit_scan_start = keys::branch_commit_key(branch_id, commit_scan_cursor);
	let commit_scan_end = root
		.hot_watermark_txid
		.checked_add(1)
		.map(|next_txid| keys::branch_commit_key(branch_id, next_txid))
		.unwrap_or_else(|| end_of_key_range(&keys::branch_commit_prefix(branch_id)));
	let commit_rows = tx_scan_range_values_limited(
		tx,
		&commit_scan_start,
		&commit_scan_end,
		CMP_FDB_BATCH_MAX_KEYS,
		isolation_level,
	)
	.await?;
	// A short window reached the end of the range. A full one means there is more history past it, and
	// the budget can still cut the window shorter below.
	let mut commit_scan_complete = commit_rows.len() < CMP_FDB_BATCH_MAX_KEYS;
	let mut next_commit_scan_cursor = commit_scan_cursor;
	// An unchanged cursor has to round-trip, so a pass that stops before completing the txid it
	// resumed into hands back the same segment it was given.
	let mut next_segment_pgno = cursor_segment_pgno;
	for (key, value) in commit_rows {
		// Give up before pulling another txid's delta rather than after, so the bound covers the read
		// it is meant to prevent. The partial sets built so far classify nothing, so the cursor stays
		// where the scan started and the caller re-derives this same window on its next pass.
		if scan_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
			tracing::warn!(
				?branch_id,
				commit_scan_cursor,
				scanned_txids = window.commits.len(),
				"reclaim commit scan hit its elapsed bound"
			);
			window.scan_truncated = true;
			window.next_commit_scan_cursor = commit_scan_cursor;
			window.commit_scan_complete = false;
			return Ok(window);
		}
		let txid = decode_branch_commit_txid(branch_id, &key)?;
		if txid > root.hot_watermark_txid {
			commit_scan_complete = true;
			break;
		}
		let commit = decode_commit_row(&value).context("decode sqlite commit row for reclaim")?;
		// Resume inside a txid whose segments did not all fit an earlier window. Only the txid the
		// cursor stopped on carries a segment cursor; every later txid is scanned whole.
		let resume_segment_pgno = (txid == commit_scan_cursor)
			.then_some(cursor_segment_pgno)
			.flatten();
		let delta_scan_begin = match resume_segment_pgno {
			Some(first_pgno) => keys::branch_delta_segment_prefix(branch_id, txid, first_pgno),
			None => keys::branch_delta_chunk_prefix(branch_id, txid),
		};
		let (_, delta_scan_end) = keys::branch_delta_txid_range(branch_id, txid);
		let delta_chunks =
			tx_scan_range_values(tx, &delta_scan_begin, &delta_scan_end, isolation_level).await?;
		// Admission is per segment. A large commit is many shard-aligned segments, and a window forced
		// to take all of them at once would stall on any commit bigger than one batch and never
		// advance past it. Segments are admitted in key order, so the window can stop on a segment
		// boundary and the next pass resumes inside the same txid from `next_segment_pgno`.
		let mut admitted_chunks: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
		let mut stopped_at_segment: Option<Option<u32>> = None;
		// The commit row rides along with the first segment admitted, charged once.
		let mut commit_row_bytes = u64::try_from(value.len()).unwrap_or(u64::MAX);
		let segment_groups = group_delta_chunks_by_segment(branch_id, txid, &delta_chunks)?;
		// A commit whose delta is already reclaimed has no segment to carry its row, so it is charged
		// on its own. Admitting it with no pages is what makes it a coverage txid.
		if segment_groups.is_empty() {
			if !budget.can_add(1, commit_row_bytes) && budget.key_count() != 0 {
				commit_scan_complete = false;
				break;
			}
			budget.add(1, commit_row_bytes);
			commit_row_bytes = 0;
		}
		for (segment_first_pgno, chunks) in segment_groups {
			let segment_value_bytes = chunks
				.iter()
				.map(|(_, value)| u64::try_from(value.len()).unwrap_or(u64::MAX))
				.fold(0_u64, u64::saturating_add)
				.saturating_add(commit_row_bytes);
			let segment_row_count = chunks.len() + usize::from(commit_row_bytes != 0);
			if !budget.can_add(segment_row_count, segment_value_bytes) {
				// This segment cannot share the window with what is already admitted, so stop and let
				// the next pass start here with a clean budget.
				if budget.key_count() != 0 || !admitted_chunks.is_empty() {
					stopped_at_segment = Some(segment_first_pgno);
					break;
				}
				// An untouched budget that still cannot hold a single segment means the segment itself
				// is oversized. Admit it alone, bounded well under FDB's transaction limit. Only a
				// legacy whole-commit blob can reach this: a segment spans at most
				// `COMMIT_SEGMENT_MAX_SHARDS` shards, so it is far smaller than the ceiling.
				if segment_value_bytes > CMP_FDB_OVERSIZED_TXID_MAX_VALUE_BYTES as u64 {
					tracing::warn!(
						?branch_id,
						txid,
						?segment_first_pgno,
						segment_value_bytes,
						"skipping oversized delta segment in reclaim commit scan"
					);
					stopped_at_segment = Some(segment_first_pgno);
					break;
				}
			}
			budget.add(segment_row_count, segment_value_bytes);
			commit_row_bytes = 0;
			admitted_chunks.extend(chunks.into_iter().cloned());
		}
		// Nothing fit for this txid, so leave it whole for a pass with a clean budget rather than
		// counting a commit whose delta this window never admitted. The cursor still points here.
		if admitted_chunks.is_empty() && commit_row_bytes != 0 {
			commit_scan_complete = false;
			break;
		}
		let delta_chunks = admitted_chunks;
		// A txid is fully covered only when this window read it from the start and admitted every
		// segment. Resuming mid-txid leaves the earlier segments outside this window, and those may
		// have been retained rather than cleared, so the commit row cannot be judged against a partial
		// set. It is classified on a later pass that sees the txid whole.
		let txid_fully_covered = stopped_at_segment.is_none() && resume_segment_pgno.is_none();

		// Whether a live PIDX entry still points at this commit's delta (T1). `live_owned(T)` is true iff
		// some page in `T`'s delta still has `PIDX[pgno] == T`, which is provably equivalent to the old
		// global "is `T` in the PIDX-owner set?" scan because a PIDX owner `T` can only sit on a page `T`
		// wrote. Point-reading the commit's own delta pages bounds the read by delta size instead of
		// scanning the whole PIDX prefix (which scales with database size and can age out the FDB
		// transaction). The point reads run under the same isolation as the snapshot, so a racing commit
		// overwriting one of these pages still aborts the delete tx via OCC.
		//
		// This scan only reaches txids at or below `hot_watermark_txid`, so in a settled branch every
		// one of them is folded and `live_owned` should be false: install clears the PIDX row of every
		// page it folds. A settled branch where this keeps returning true is the stale-PIDX defect (a
		// hot slice folded a page without clearing its owner row), not normal retention, and the rows
		// it pins are unreachable until `SweepStalePidx` clears them. Do not treat retained DELTA here
		// as an expected steady-state footprint; see
		// `docs-internal/engine/sqlite/compaction-flow.md` settled state.
		let segment_units = delta_segment_units(branch_id, txid, &delta_chunks)?;
		let mut any_segment_owned = false;
		for unit in &segment_units {
			if segment_pages_still_owned(
				tx,
				branch_id,
				txid,
				&unit.pgnos,
				head_txid,
				isolation_level,
			)
			.await?
			{
				any_segment_owned = true;
				continue;
			}

			// DELTA: a folded segment whose shards are all materialized at the smallest coverage fold
			// that still needs them. Cold never reads deltas, so this ignores the cold watermark. The
			// gate is evaluated after the loop against a fold window bounded to the candidates' txid
			// span.
			delta_gate_candidates.push((
				DeltaSegmentRef {
					txid,
					first_pgno: unit.first_pgno,
				},
				unit.shards.clone(),
			));
		}

		// COMMITS/VTX: a non-fold commit at or below the delete bound. `txid <= bound` already implies
		// the commit is below the lowest fold, so it is never a coverage txid. A commit whose pages a
		// live PIDX entry still owns is withheld (the safety gate): in real states `txid <= bound`
		// always implies the commit is folded, so this only guards an inconsistent state where the
		// watermark advanced without clearing PIDX.
		//
		// The commit row is also withheld until every one of its segments is droppable. The sweep
		// enumerates history by scanning `COMMITS`, so deleting the commit while any segment survives
		// would strand that segment: no later pass would ever visit its txid again.
		if commit_reclaim_bound.is_some_and(|bound| txid <= bound)
			&& !any_segment_owned
			&& txid_fully_covered
		{
			commit_gate_candidates.push((txid, segment_units.len()));
		}

		window.commits.push((txid, key, value, commit));
		window.delta_chunks.extend(delta_chunks);

		// Stopping inside a txid holds the commit cursor on it and resumes at the segment that did not
		// fit. A legacy blob has no boundary to resume from, so its txid is stepped over instead of
		// being retried forever.
		if let Some(segment_first_pgno) = stopped_at_segment {
			match segment_first_pgno {
				Some(first_pgno) => {
					next_commit_scan_cursor = txid;
					next_segment_pgno = Some(first_pgno);
				}
				None => {
					next_commit_scan_cursor = txid.saturating_add(1);
					next_segment_pgno = None;
				}
			}
			commit_scan_complete = false;
			break;
		}

		next_commit_scan_cursor = txid.saturating_add(1);
		next_segment_pgno = None;
	}
	window.next_commit_scan_cursor = next_commit_scan_cursor;
	window.next_segment_pgno = next_segment_pgno;
	window.commit_scan_complete = commit_scan_complete;

	// Per-shard fold versions for just the window the deferred gate can query (T2). The gate for a
	// candidate at `txid` inspects fold txids in `[txid, F0]` where `F0` is the smallest coverage fold
	// `>= txid`; `F0` is monotonic in `txid`, so one range scan over `[min_gate_txid, F0(max_gate_txid)]`
	// supplies every fold any candidate can ask about, identical to restricting the old global fold map
	// to that window. The scan is bounded by the candidates' txid span plus one coverage gap instead of
	// the whole fold prefix (which scales with compaction history). Candidates at or below the cold
	// watermark short-circuit the gate, so they are excluded from the scan window. The window read is
	// capped at `CMP_FDB_BATCH_MAX_KEYS`: a truncated window can only drop high folds, which makes the
	// gate fail closed (retain), never a false drop.
	let mut windowed_fold_versions: BTreeMap<u32, BTreeSet<u64>> = BTreeMap::new();
	let gate_scan_txids = delta_gate_candidates
		.iter()
		.map(|(segment, _)| segment.txid)
		.filter(|txid| *txid > root.cold_watermark_txid)
		.collect::<Vec<_>>();
	if let (Some(&min_gate_txid), Some(&max_gate_txid)) =
		(gate_scan_txids.first(), gate_scan_txids.last())
	{
		if let Some(f0_max) = coverage.range(max_gate_txid..).next().copied() {
			let fold_scan_start = keys::branch_compaction_fold_key(branch_id, min_gate_txid);
			let fold_scan_end = f0_max
				.checked_add(1)
				.map(|next_txid| keys::branch_compaction_fold_key(branch_id, next_txid))
				.unwrap_or_else(|| {
					end_of_key_range(&keys::branch_compaction_fold_prefix(branch_id))
				});
			for (key, value) in tx_scan_range_values_limited(
				tx,
				&fold_scan_start,
				&fold_scan_end,
				CMP_FDB_BATCH_MAX_KEYS,
				isolation_level,
			)
			.await?
			{
				let fold_txid = keys::decode_branch_compaction_fold_txid(branch_id, &key)?;
				let entry = decode_fold_index_entry(&value)
					.context("decode sqlite fold index entry for reclaim classification")?;
				for shard_id in entry.shard_ids {
					windowed_fold_versions
						.entry(shard_id)
						.or_default()
						.insert(fold_txid);
				}
			}
		}
	}
	let mut passing_segments_by_txid: BTreeMap<u64, usize> = BTreeMap::new();
	for (segment, shards) in &delta_gate_candidates {
		if delta_materialization_gate_passes(
			segment.txid,
			shards,
			&coverage,
			&windowed_fold_versions,
			root.cold_watermark_txid,
		) {
			delta_reclaim_segments.push(*segment);
			*passing_segments_by_txid.entry(segment.txid).or_default() += 1;
		}
	}

	// A commit row may only go once every segment of its txid goes with it. The sweep finds history
	// by scanning `COMMITS`, so a commit deleted ahead of a surviving segment would leave that
	// segment unreachable to every later pass. A commit that wrote no delta has nothing to wait for.
	for (txid, segment_count) in commit_gate_candidates {
		if passing_segments_by_txid
			.get(&txid)
			.copied()
			.unwrap_or_default()
			== segment_count
		{
			commit_reclaim_txids.push(txid);
		}
	}

	window.total_value_bytes = budget.value_bytes();
	window.delta_reclaim_segments = delta_reclaim_segments;
	window.commit_reclaim_txids = commit_reclaim_txids;

	Ok(window)
}

/// Shard-materialization gate for a folded delta at `txid` (#1). The delta is safe to drop only if,
/// for the smallest coverage fold `F0 >= txid`, every shard it touched has a materialized version in
/// `[txid, F0]`. The reduction to the single smallest fold is sound: if the newest version `<= F0` is
/// `>= txid`, every larger coverage fold `F1 > F0` is automatically covered because its newest version
/// `<= F1` is `>= V >= txid`.
///
/// A delta at or below the cold watermark is always covered: cold compaction has published its pages
/// (or they were dead and skipped via watermark-only publish), so a read refills from the cold tier.
///
/// Soundness (#6): every fork/restore target is either head or already carries a live pin/rep at that
/// exact txid (fork points self-pin a `DB_PIN` in the same tx; `Latest` resolves to a self-pinned capped
/// parent, never an arbitrary txid). So a future reader can only land on a txid already in the coverage
/// set, and a delta whose gate passes now stays droppable: the materialization that covers `F0` keeps
/// covering it as new (necessarily higher) folds appear.
pub(crate) fn delta_materialization_gate_passes(
	txid: u64,
	shards: &BTreeSet<u32>,
	coverage: &BTreeSet<u64>,
	shard_fold_versions: &BTreeMap<u32, BTreeSet<u64>>,
	cold_watermark_txid: u64,
) -> bool {
	if txid <= cold_watermark_txid {
		return true;
	}
	let Some(f0) = coverage.range(txid..).next().copied() else {
		// Head is always a coverage fold and `txid <= hot_watermark <= head`, so this is unreachable in
		// practice; fail closed (retain the delta) rather than drop it.
		return false;
	};
	shards.iter().all(|shard_id| {
		shard_fold_versions
			.get(shard_id)
			.is_some_and(|versions| versions.range(txid..=f0).next().is_some())
	})
}

/// Whether any page in `txid`'s delta still has `PIDX[pgno] == txid` (T1). Point-reads only the
/// commit's own delta pages, so it is bounded by delta size rather than the whole-branch PIDX prefix.
/// Returns `false` for a commit with no delta (it owns no PIDX entry). The reads run under the
/// caller's isolation level, so under `Serializable` a racing PIDX overwrite on one of these pages
/// aborts the enclosing tx via OCC.
/// Whether a live PIDX entry still points at this commit's delta.
///
/// `head_txid` bounds which owners count as a supersede. An owner at or below head is published, so
/// an owner other than `txid` proves a later commit took the page and `txid` no longer needs its
/// delta. An owner *above* head is not published: it belongs to a commit that has staged its rows
/// but not finalized, and it may never become visible. Treating it as a supersede would let reclaim
/// drop `txid`'s delta on the strength of a commit that later aborts, leaving the page with no
/// reachable content. So an owner above head withholds the delta instead, and the next pass
/// reclassifies once the staged commit either lands or is cleared.
///
/// No writer stages PIDX above head today, so this is a guard against a state the current commit path
/// cannot produce. It is the correctness precondition for ever staging PIDX rows before finalize
/// (see `~/.agents/specs/depot-commit-size-cap.md` §4.1), and it mirrors the read path, which already
/// discards owners above its head-derived cap.
/// Groups a txid's delta chunk rows into their segments, in key order.
///
/// A legacy commit yields a single group keyed `None`, so callers need no separate path for it.
/// Chunks arrive in key order, and a segment's chunks are contiguous, so grouping preserves that
/// order and each group is admitted or deferred as a unit.
pub(crate) fn group_delta_chunks_by_segment<'a>(
	branch_id: DatabaseBranchId,
	txid: u64,
	delta_chunks: &'a [(Vec<u8>, Vec<u8>)],
) -> Result<Vec<(Option<u32>, Vec<&'a (Vec<u8>, Vec<u8>)>)>> {
	let mut groups: Vec<(Option<u32>, Vec<&'a (Vec<u8>, Vec<u8>)>)> = Vec::new();
	for row in delta_chunks {
		let (key, _) = row;
		let first_pgno = match keys::decode_branch_delta_chunk_ref(branch_id, txid, key)? {
			keys::DeltaChunkRef::Legacy { .. } => None,
			keys::DeltaChunkRef::Segment { first_pgno, .. } => Some(first_pgno),
		};
		match groups.last_mut() {
			Some((group_pgno, chunks)) if *group_pgno == first_pgno => chunks.push(row),
			_ => groups.push((first_pgno, vec![row])),
		}
	}

	Ok(groups)
}

/// One delta segment's reclaim inputs: the pages it wrote and the shards those pages live in.
pub(crate) struct DeltaSegmentUnit {
	pub(crate) first_pgno: Option<u32>,
	pub(crate) pgnos: BTreeSet<u32>,
	pub(crate) shards: BTreeSet<u32>,
}

/// Splits a txid's delta rows into per-segment reclaim units.
///
/// A legacy commit yields a single unit with `first_pgno: None`, so callers need no separate path
/// for it.
pub(crate) fn delta_segment_units(
	branch_id: DatabaseBranchId,
	txid: u64,
	delta_chunks: &[(Vec<u8>, Vec<u8>)],
) -> Result<Vec<DeltaSegmentUnit>> {
	let by_txid = delta_blob::reassemble_delta_segments_by_txid(branch_id, delta_chunks)?;
	let Some(segments) = by_txid.get(&txid) else {
		return Ok(Vec::new());
	};

	segments
		.iter()
		.map(|segment| {
			let decoded = decode_ltx_v3(&segment.blob)
				.with_context(|| format!("decode delta {txid} segment {:?}", segment.first_pgno))?;
			let mut pgnos = BTreeSet::new();
			let mut shards = BTreeSet::new();
			for page in &decoded.pages {
				pgnos.insert(page.pgno);
				shards.insert(page.pgno / keys::SHARD_SIZE);
			}
			Ok(DeltaSegmentUnit {
				first_pgno: segment.first_pgno,
				pgnos,
				shards,
			})
		})
		.collect()
}

/// Whether a live PIDX entry still points at this segment's pages.
///
/// Ownership is a per-page property, so it is evaluated per segment: one segment of a large commit
/// can still own pages while its siblings are fully superseded and safe to drop.
///
/// `head_txid` bounds which owners count as a supersede. An owner at or below head is published, so
/// an owner other than `txid` proves a later commit took the page and this segment no longer needs
/// its copy. An owner *above* head is not published: it belongs to a commit that has staged its rows
/// but not finalized, and it may never become visible. Treating it as a supersede would let reclaim
/// drop the segment on the strength of a commit that later aborts, leaving the page with no
/// reachable content. So an owner above head withholds the segment instead, and the next pass
/// reclassifies once the staged commit either lands or is cleared.
///
/// Point-reads only the segment's own pages, so the read is bounded by segment size rather than the
/// whole-branch PIDX prefix. The reads run under the caller's isolation level, so under
/// `Serializable` a racing PIDX overwrite on one of these pages aborts the enclosing tx via OCC.
pub(crate) async fn segment_pages_still_owned(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	txid: u64,
	pgnos: &BTreeSet<u32>,
	head_txid: Option<u64>,
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<bool> {
	for pgno in pgnos {
		let Some(value) = tx_get_value(
			tx,
			&keys::branch_pidx_key(branch_id, *pgno),
			isolation_level,
		)
		.await?
		else {
			continue;
		};
		let owner_txid = decode_pidx_txid(&value)?;
		if owner_txid == txid {
			return Ok(true);
		}
		if head_txid.is_some_and(|head_txid| owner_txid > head_txid) {
			tracing::warn!(
				?branch_id,
				txid,
				pgno,
				owner_txid,
				head_txid,
				"withholding delta segment from reclaim: PIDX owner sits above head"
			);
			return Ok(true);
		}
	}
	Ok(false)
}

type PitrIntervalReclaimRows = (
	Vec<PitrIntervalSelection>,
	Vec<(i64, Vec<u8>, Vec<u8>, PitrIntervalCoverage)>,
);

pub(crate) async fn read_pitr_interval_reclaim_rows(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	now_ms: i64,
	isolation_level: universaldb::utils::IsolationLevel,
	budget: &mut CompactionBatchBudget,
) -> Result<PitrIntervalReclaimRows> {
	let mut retained = Vec::new();
	let mut expired = Vec::new();

	// Retained rows are always collected in full: they are the coverage set for reclaim
	// classification and truncating them would misclassify retained history as dead. Expired rows
	// are delete candidates whose compared values ride in the delete transaction, so their
	// collection is capped by the shared slice budget. Collection stops at the first row that does
	// not fit, keeping the collected set a deterministic prefix of the scan order; the remaining
	// tail drains across later slices as each delete clears the rows ahead of it.
	let mut expired_budget_exhausted = false;
	for (key, value) in tx_scan_prefix_values(
		tx,
		&keys::branch_pitr_interval_prefix(branch_id),
		isolation_level,
	)
	.await?
	{
		let bucket_start_ms = keys::decode_branch_pitr_interval_bucket(branch_id, &key)?;
		let coverage = decode_pitr_interval_coverage(&value)
			.context("decode sqlite PITR interval coverage for reclaim")?;
		if coverage.expires_at_ms <= now_ms {
			if expired_budget_exhausted {
				continue;
			}
			let value_bytes = u64::try_from(value.len()).unwrap_or(u64::MAX);
			if !budget.can_add(1, value_bytes) {
				expired_budget_exhausted = true;
				continue;
			}
			budget.add(1, value_bytes);
			expired.push((bucket_start_ms, key, value, coverage));
		} else {
			retained.push(PitrIntervalSelection {
				bucket_start_ms,
				coverage,
			});
		}
	}

	Ok((retained, expired))
}

/// One bounded cold-ref scan window: the reclaim-eligible refs and the resume cursor.
#[derive(Default)]
pub(crate) struct ColdObjectReclaimWindow {
	pub(crate) refs: Vec<ReclaimColdObjectRef>,
	pub(crate) next_cursor: Option<ColdScanCursor>,
}

/// Discovers dead `SHARD` versions for the version-retention sweep (C4): a `SHARD/{s}/X` superseded by
/// a newer version `SHARD/{s}/Y` (Y > X) of the same shard with no live coverage txid in `[X, Y)`. Such
/// a version sits at a txid that is no longer a fold, so no read (head, pin, or unexpired PITR rep) can
/// land on it. This is the only shard GC when cold storage is off and the structural counterpart to
/// C5's temperature demotion: C5 touches *live* versions, C4 only *dead* ones, and they never overlap.
///
/// Coverage is the txid SET, not fold-index membership (#4). A shard unchanged across a PITR rep has no
/// `CMP/fold` row at that rep, so deadness is tested against `{DB_PIN targets} ∪ {unexpired PITR reps} ∪
/// {head}`; `CMP/fold` is used only to enumerate `(shard, version, successor)` triples.
///
/// Soundness (#6): every fork/restore target is either head or already carries a live pin/rep at that
/// exact txid (fork points self-pin a `DB_PIN(DatabaseFork)` in the same tx). `Latest` resolves to that
/// self-pinned capped parent, never an arbitrary txid. So a future pin can only land on a txid already
/// in the coverage set, which makes deadness monotonic: a version dead now stays dead.
///
/// The walk reads only the blob-free `CMP/fold` rows. With cold on those are bounded by cold lag
/// (`publish_cold_finalize_tx` clears folds `<= cold_wm`); with cold off C4 is the sole fold clearer, so
/// the live fold set is `O(live shard count)`. Each chunk collects up to `CMP_FDB_BATCH_MAX_KEYS`
/// deletions. The `SweepDeadShardVersions` activity drives this chunk by chunk in its own FDB
/// transactions, carrying `prev` across chunks in local memory and deleting each chunk's candidates in
/// the same transaction that found them, so the walk-and-delete is one Serializable tx per chunk and no
/// plan/delete OCC fence is needed.
///
/// Note: a shard's newest version is the leftover `prev` after the walk and is never returned, so C4
/// never deletes a shard's last version. The per-shard `SHARD_ACCESS`/`SHARD_LRU` rows therefore stay
/// valid and need no cleanup here.
#[derive(Default)]
struct OwnerDeltaPageProbe {
	page: Option<Vec<u8>>,
	rows: usize,
	value_bytes: u64,
}

/// What one owner delta's segment can say about the page a stale PIDX row owns.
///
/// Absent and unusable are held apart even though both retain the row, because they call for
/// different responses. An absent segment is an ordinary miss. An unusable one means no row this
/// delta owns can ever be confirmed, so the branch never writes `CMP/pidx_repair` and re-walks its
/// whole PIDX prefix on every reclaim job from then on.
enum OwnerDeltaSegment {
	/// Decoded, so its pages can be compared against the shard image.
	Decoded(crate::conveyer::ltx::LtxBlob),
	/// No segment covering the page's range exists under this owner txid.
	Missing,
	/// Present but either too large to reassemble inside one window or holding bytes that did not
	/// decode.
	Unusable,
}

/// Reassembles scanned chunk rows into the segment that can hold `first_pgno`.
///
/// Errors are the caller's to classify. A delta that does not reassemble or decode proves nothing
/// about the page, and this walk exists to delete rows, so the caller turns that into a retained row
/// rather than failing the branch's whole reclaim lane.
fn decode_owner_delta_segment(
	branch_id: DatabaseBranchId,
	owner_txid: u64,
	first_pgno: Option<u32>,
	delta_rows: Vec<(Vec<u8>, Vec<u8>)>,
) -> Result<OwnerDeltaSegment> {
	let Some(segment) = delta_blob::reassemble_delta_segments(branch_id, owner_txid, delta_rows)?
		.into_iter()
		.find(|segment| segment.first_pgno == first_pgno)
	else {
		return Ok(OwnerDeltaSegment::Missing);
	};

	Ok(OwnerDeltaSegment::Decoded(
		crate::conveyer::ltx::LtxBlob::decode_index(segment.blob)?,
	))
}

async fn read_stale_pidx_owner_page(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	owner_txid: u64,
	pgno: u32,
	isolation_level: universaldb::utils::IsolationLevel,
	segment_cache: &mut BTreeMap<(u64, Option<u32>), OwnerDeltaSegment>,
) -> Result<OwnerDeltaPageProbe> {
	let delta_prefix = keys::branch_delta_chunk_prefix(branch_id, owner_txid);
	let (_, delta_end) = keys::branch_delta_txid_range(branch_id, owner_txid);
	let locate_end = pgno
		.checked_add(1)
		.map(|next_pgno| keys::branch_delta_segment_prefix(branch_id, owner_txid, next_pgno))
		.unwrap_or_else(|| delta_end.clone());
	let Some((key, value)) =
		tx_get_range_last(tx, &delta_prefix, &locate_end, isolation_level).await?
	else {
		return Ok(OwnerDeltaPageProbe {
			page: None,
			rows: 0,
			value_bytes: 0,
		});
	};

	let first_pgno = keys::decode_branch_delta_chunk_ref(branch_id, owner_txid, &key)?.first_pgno();
	let mut rows = 1;
	let mut value_bytes = u64::try_from(value.len()).unwrap_or(u64::MAX);
	let cache_key = (owner_txid, first_pgno);
	if !segment_cache.contains_key(&cache_key) {
		let (segment_start, segment_end) = match first_pgno {
			Some(first_pgno) => (
				keys::branch_delta_segment_prefix(branch_id, owner_txid, first_pgno),
				first_pgno
					.checked_add(1)
					.map(|next_pgno| {
						keys::branch_delta_segment_prefix(branch_id, owner_txid, next_pgno)
					})
					.unwrap_or_else(|| delta_end.clone()),
			),
			None => (delta_prefix, delta_end),
		};
		let delta_rows = tx_scan_range_values_limited(
			tx,
			&segment_start,
			&segment_end,
			CMP_FDB_BATCH_MAX_KEYS,
			isolation_level,
		)
		.await?;
		rows += delta_rows.len();
		value_bytes = delta_rows
			.iter()
			.map(|(_, value)| u64::try_from(value.len()).unwrap_or(u64::MAX))
			.fold(value_bytes, u64::saturating_add);
		let segment = if delta_rows.len() >= CMP_FDB_BATCH_MAX_KEYS {
			// The delta holds more chunk rows than one window may read, so its blob cannot be
			// reassembled from what was scanned and nothing it owns is ever confirmable. Retaining
			// is the safe outcome, but it also retires nothing, so it is worth chasing.
			tracing::warn!(
				?branch_id,
				owner_txid,
				?first_pgno,
				scanned_rows = delta_rows.len(),
				"owner delta is too large to reassemble in one stale pidx window, so its rows can never be confirmed"
			);
			OwnerDeltaSegment::Unusable
		} else {
			match decode_owner_delta_segment(branch_id, owner_txid, first_pgno, delta_rows) {
				Ok(segment) => segment,
				Err(err) => {
					// An unreadable delta proves nothing about the page. Treat it as unusable so the
					// sweep fails closed instead of wedging the branch's whole reclaim lane on a
					// decode error, matching how an unreadable shard image is treated.
					tracing::warn!(
						?branch_id,
						owner_txid,
						?first_pgno,
						?err,
						"could not read an owner delta while confirming stale pidx contents"
					);
					OwnerDeltaSegment::Unusable
				}
			}
		};
		segment_cache.insert(cache_key, segment);
	}

	let page = match segment_cache.get(&cache_key) {
		Some(OwnerDeltaSegment::Decoded(segment)) => match segment.get_page(pgno) {
			Ok(page) => page,
			Err(err) => {
				tracing::warn!(
					?branch_id,
					owner_txid,
					pgno,
					?err,
					"could not decode an owner delta page while confirming stale pidx contents"
				);
				None
			}
		},
		Some(OwnerDeltaSegment::Missing) | Some(OwnerDeltaSegment::Unusable) | None => None,
	};
	Ok(OwnerDeltaPageProbe {
		page,
		rows,
		value_bytes,
	})
}

/// One bounded window of the stale-PIDX repair walk. Walks the pgno-major `PIDX` prefix ascending
/// starting at `pgno_cursor`, reading at most `CMP_FDB_BATCH_MAX_KEYS` rows.
///
/// A row is stale when its owner txid is at or below the hot watermark. Hot install advances the
/// watermark only after copying a complete image of every shard the drained slices folded, so such a
/// page is already materialized in `SHARD` and its PIDX row only forces reads down the delta chain and
/// pins the delta and commit against reclaim. The row should have been cleared by the install that
/// folded it; a slice whose budget ran out before its PIDX lane left it behind, and the owner-window
/// filter in `read_hot_input_snapshot` means no later slice will ever revisit it.
///
/// Staleness alone is not enough to clear: the newest `SHARD` version at or below the watermark must
/// be at least as new as the owner and carry bytes identical to the owner delta's page. Page-number
/// presence is insufficient: an old hot pass could skip the owner commit yet copy an older version of
/// the same page into a later shard. Clearing PIDX then makes reads serve those older bytes, which is
/// silent database corruption. Shard images and owner-delta segments are memoized within the window.
///
/// Every row scanned is charged against the budget whether or not it is clearable. Live rows are the
/// overwhelming majority on a healthy branch, so a walk that only charged for candidates would read an
/// unbounded number of rows per transaction; charging every row is what keeps the window bounded, and
/// the cursor is what keeps the next window from re-reading them.
pub(crate) async fn read_stale_pidx_chunk(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	root: &CompactionRoot,
	pgno_cursor: Option<u32>,
	isolation_level: universaldb::utils::IsolationLevel,
	budget: &mut CompactionBatchBudget,
) -> Result<StalePidxScanChunk> {
	let prefix = keys::branch_pidx_prefix(branch_id);
	let scan_start = match pgno_cursor {
		Some(cursor) => keys::branch_pidx_key(branch_id, cursor),
		None => prefix.clone(),
	};
	// The prefix range end, not `end_of_key_range` of a key: the latter appends a single zero byte and
	// would stop short of the rest of the prefix.
	let (_, scan_end) = universaldb::tuple::Subspace::from_bytes(prefix.as_slice()).range();
	let rows = tx_scan_range_values_limited(
		tx,
		&scan_start,
		&scan_end,
		CMP_FDB_BATCH_MAX_KEYS,
		isolation_level,
	)
	.await?;
	let chunk_full = rows.len() >= CMP_FDB_BATCH_MAX_KEYS;

	let mut candidates = Vec::new();
	let mut last_pgno: Option<u32> = None;
	// The newest folded image of each shard, or `None` when the shard has no version at
	// all. Keyed by shard alone because that one version is what every read of every page in the
	// shard resolves through, so one image read answers the whole shard.
	let mut latest_shard_pages = BTreeMap::new();
	let mut owner_delta_segments = BTreeMap::new();
	let mut budget_capped = false;
	let mut retained_unconfirmed = false;
	for (key, value) in rows {
		let pgno = decode_branch_pidx_pgno(branch_id, &key)?;
		let row_bytes = u64::try_from(key.len().saturating_add(value.len())).unwrap_or(u64::MAX);
		let owner_txid = decode_pidx_txid(&value)?;
		if owner_txid > root.hot_watermark_txid {
			// A live row: the page's newest write has not been folded yet, so its PIDX row is the
			// correct owner and reads must keep resolving through it.
			if !budget.can_add(1, row_bytes) {
				// Stop before charging, and resume at this row rather than past it.
				budget_capped = true;
				break;
			}
			budget.add(1, row_bytes);
			last_pgno = Some(pgno);
			continue;
		}

		// The content probe reads a whole shard image, so its cost is charged with the row that
		// caused it and both are checked before the cursor advances past this page. A shard already
		// probed in this window costs nothing more.
		let shard_id = pgno / keys::SHARD_SIZE;
		let mut probe_rows = 0_usize;
		let mut probe_value_bytes = 0_u64;
		if !latest_shard_pages.contains_key(&shard_id) {
			let probe = shard_blob::latest_shard_version_page_set(
				tx,
				branch_id,
				shard_id,
				root.hot_watermark_txid,
				isolation_level,
			)
			.await;
			let latest = match probe {
				Ok(Some(probe)) => {
					probe_rows = probe.rows;
					probe_value_bytes = probe.value_bytes;
					Some(probe)
				}
				Ok(None) => None,
				Err(err) => {
					// An unreadable image proves nothing about the page, and this walk exists to
					// delete rows. Treat it as no coverage so the sweep fails closed instead of
					// wedging the branch's whole reclaim lane on a decode error.
					tracing::warn!(
						?branch_id,
						shard_id,
						?err,
						"could not read a shard image while confirming stale pidx coverage"
					);
					None
				}
			};
			latest_shard_pages.insert(shard_id, latest);
		}

		let latest = latest_shard_pages
			.get(&shard_id)
			.and_then(|latest| latest.as_ref());
		let shard_page = latest
			.filter(|latest| latest.as_of_txid >= owner_txid)
			.and_then(|latest| latest.blob.get_page(pgno).ok().flatten());
		let owner_probe = if shard_page.is_some() {
			read_stale_pidx_owner_page(
				tx,
				branch_id,
				owner_txid,
				pgno,
				isolation_level,
				&mut owner_delta_segments,
			)
			.await?
		} else {
			OwnerDeltaPageProbe::default()
		};
		probe_rows += owner_probe.rows;
		probe_value_bytes = probe_value_bytes.saturating_add(owner_probe.value_bytes);

		// A window that has charged nothing yet always admits its first row, image and all, so a shard
		// image larger than the whole budget cannot stall the walk.
		if last_pgno.is_some()
			&& !budget.can_add(1 + probe_rows, row_bytes.saturating_add(probe_value_bytes))
		{
			budget_capped = true;
			break;
		}
		budget.add(1 + probe_rows, row_bytes.saturating_add(probe_value_bytes));
		last_pgno = Some(pgno);

		let confirmed = shard_page
			.as_ref()
			.zip(owner_probe.page.as_ref())
			.is_some_and(|(shard_page, owner_page)| shard_page == owner_page);
		if !confirmed {
			tracing::warn!(
				?branch_id,
				pgno,
				owner_txid,
				hot_watermark_txid = root.hot_watermark_txid,
				latest_shard_version_txid = latest.map(|latest| latest.as_of_txid),
				shard_page_present = shard_page.is_some(),
				owner_page_present = owner_probe.page.is_some(),
				"retaining stale pidx row whose owner and shard page contents are not confirmed equal"
			);
			retained_unconfirmed = true;
			continue;
		}

		candidates.push((key, value));
	}

	// A window that stopped early must have charged at least one row, or the walk cannot advance and
	// the caller would re-read the same rows forever. Each chunk transaction supplies a fresh budget
	// and a PIDX row is one fixed-width key and value, so this is unreachable outside a caller that
	// hands over an already-spent budget.
	ensure!(
		!(budget_capped && last_pgno.is_none()),
		"stale pidx window could not admit a single row"
	);
	// Resume strictly after the last row charged. A window that cleared nothing still advances, so the
	// live rows filling it cannot stall the stale rows sitting behind them.
	let next_pgno_cursor = match last_pgno {
		Some(pgno) => pgno.checked_add(1),
		None => pgno_cursor,
	};
	// `has_more` covers both boundaries the window can stop on. A `None` cursor means the walk ran off
	// the end of the page space, which is the end of the prefix either way.
	let has_more = (chunk_full || budget_capped) && next_pgno_cursor.is_some();

	Ok(StalePidxScanChunk {
		candidates,
		next_pgno_cursor,
		has_more,
		retained_unconfirmed,
	})
}

/// Builds the dead-shard coverage set: any txid a read can land on and resolve to a version, so a shard
/// version superseded across such a txid is still reachable and not dead. Pins, unexpired PITR interval
/// reps, and head.
fn dead_shard_coverage(
	db_pins: &[DbHistoryPin],
	pitr_interval_retention: &[PitrIntervalSelection],
	head_txid: Option<u64>,
) -> BTreeSet<u64> {
	let mut coverage: BTreeSet<u64> = BTreeSet::new();
	coverage.extend(db_pins.iter().map(|pin| pin.at_txid));
	coverage.extend(
		pitr_interval_retention
			.iter()
			.map(|selection| selection.coverage.txid),
	);
	coverage.extend(head_txid);
	coverage
}

async fn read_dead_shard_head_txid(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<Option<u64>> {
	Ok(
		tx_get_value(tx, &keys::branch_meta_head_key(branch_id), isolation_level)
			.await?
			.as_deref()
			.map(decode_db_head)
			.transpose()
			.context("decode sqlite head for dead shard version walk")?
			.map(|head| head.head_txid),
	)
}

/// One bounded chunk of the dead-shard version-retention walk. Walks the txid-major `CMP/fold` index
/// ascending starting strictly after `scan.fold_cursor`, reading at most `CMP_FDB_BATCH_MAX_KEYS` fold
/// rows. `scan.prev` carries the last fold txid seen per shard from earlier chunks so a version
/// superseded by a fold in this chunk is still detected across the boundary. A shard version
/// `SHARD/{s}/X` is dead when a later fold `Y` lists `s` again with no coverage txid in `[X, Y)`; each
/// such candidate carries `superseded_by_txid = Y` so the delete can re-validate it locally instead of
/// re-walking. Returns the candidates found this chunk, the next `DeadShardScanState` (`prev` updated
/// through every processed fold and `fold_cursor` advanced past the last fold read, or left unchanged
/// when the chunk was empty so it never rewinds), and a `has_more` flag that is true while the walk
/// stopped at a chunk or candidate-budget boundary (more folds to read) and false once it drained the
/// prefix. The cursor never resets to `None` after starting, so a later pass that resumes a drained walk
/// reads an empty range rather than restarting from the beginning against a fully-populated `prev`.
pub(crate) async fn read_dead_shard_versions_chunk(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	db_pins: &[DbHistoryPin],
	pitr_interval_retention: &[PitrIntervalSelection],
	scan: &DeadShardScanState,
	isolation_level: universaldb::utils::IsolationLevel,
	budget: &mut CompactionBatchBudget,
) -> Result<DeadShardScanChunk> {
	let head_txid = read_dead_shard_head_txid(tx, branch_id, isolation_level).await?;
	let coverage = dead_shard_coverage(db_pins, pitr_interval_retention, head_txid);

	let prefix = keys::branch_compaction_fold_prefix(branch_id);
	let scan_start = match scan.fold_cursor {
		Some(cursor) => keys::branch_compaction_fold_key(branch_id, cursor.saturating_add(1)),
		None => prefix.clone(),
	};
	let (_, scan_end) = universaldb::tuple::Subspace::from_bytes(prefix.as_slice()).range();
	let rows = tx_scan_range_values_limited(
		tx,
		&scan_start,
		&scan_end,
		CMP_FDB_BATCH_MAX_KEYS,
		isolation_level,
	)
	.await?;
	let chunk_full = rows.len() >= CMP_FDB_BATCH_MAX_KEYS;

	let mut prev = scan.prev.clone();
	let mut candidates = Vec::new();
	let mut last_fold_txid: Option<u64> = None;
	let mut budget_capped = false;
	// `CMP/fold` rows scan in ascending big-endian txid order, which is exactly the order the
	// supersession walk needs.
	'folds: for (key, value) in rows {
		let fold_txid = keys::decode_branch_compaction_fold_txid(branch_id, &key)?;
		let entry = decode_fold_index_entry(&value)
			.context("decode sqlite fold index entry for dead shard version walk")?;
		// This fold's candidates are collected before `prev` advances so a budget stop resumes
		// cleanly. Deleting a candidate clears its shard rows with `COMPARE_AND_CLEAR`, whose
		// mutations carry the compared value bytes, so each candidate's rows count against the
		// shared slice budget. A candidate that does not fit stops the walk with `prev` and the
		// cursor still pointing before this fold; the next pass re-walks it with a fresh budget,
		// and candidates this slice already deletes read as absent on the re-walk.
		for shard_id in &entry.shard_ids {
			let Some(&prev_txid) = prev.get(shard_id) else {
				continue;
			};
			// `prev_txid` is dead when no coverage txid lands in `[prev_txid, fold_txid)`: no read
			// resolves to a version at a non-fold txid in that span.
			let has_coverage = coverage.range(prev_txid..fold_txid).next().is_some();
			if has_coverage {
				continue;
			}
			// The version is absent only if a concurrent demote/delete already removed it; skip it
			// and let the next pass re-derive. The live version is normally present here, so this
			// also keeps quota crediting to genuinely-freed bytes.
			let Some(version) = shard_blob::read_shard_blob_at(
				tx,
				branch_id,
				*shard_id,
				prev_txid,
				isolation_level,
			)
			.await?
			else {
				continue;
			};
			let row_value_bytes = version
				.rows
				.iter()
				.map(|(_, value)| u64::try_from(value.len()).unwrap_or(u64::MAX))
				.fold(0_u64, u64::saturating_add);
			if !budget.can_add(version.rows.len(), row_value_bytes) {
				budget_capped = true;
				break 'folds;
			}
			budget.add(version.rows.len(), row_value_bytes);
			candidates.push(DeadShardVersionCandidate {
				reference: DeadShardVersionRef {
					shard_id: *shard_id,
					as_of_txid: prev_txid,
					superseded_by_txid: fold_txid,
				},
				shard_rows: version.rows,
			});
		}
		for shard_id in entry.shard_ids {
			prev.insert(shard_id, fold_txid);
		}
		last_fold_txid = Some(fold_txid);
	}

	// Advance the cursor past the last fold read so the next pass resumes strictly after it. An empty
	// chunk (the walk already drained past the prefix end) leaves the cursor where it was so it never
	// rewinds to the start. More folds remain while the read filled the chunk or the candidate budget
	// capped this pass.
	let next_fold_cursor = match last_fold_txid {
		Some(txid) => Some(txid),
		None => scan.fold_cursor,
	};
	let has_more = budget_capped || chunk_full;

	Ok(DeadShardScanChunk {
		candidates,
		next_scan: DeadShardScanState {
			prev,
			fold_cursor: next_fold_cursor,
		},
		has_more,
	})
}

/// Deletes a batch of dead `SHARD` versions (C4 version-retention sweep) in the current transaction:
/// clears each version's live rows with `COMPARE_AND_CLEAR`, credits the freed bytes back to quota,
/// and drops the shard from its own `CMP/fold/{as_of_txid}` entry (clearing an emptied entry so the
/// fold index stays bounded). Returns the shard-row volume this call freed, counting key plus value
/// bytes exactly as `ReclaimRowVolume::clear` does for every other reclaim row kind. The fold index
/// rewrites are index bookkeeping rather than reclaimed data, so they stay out of that volume. The
/// caller must have found `candidates` in this same Serializable transaction so no plan/delete OCC
/// fence is needed; the fold reads here conflict on any racing fold write.
pub(crate) async fn delete_dead_shard_versions_tx(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	candidates: &[DeadShardVersionCandidate],
) -> Result<ReclaimRowVolume> {
	let mut volume = ReclaimRowVolume::default();
	for candidate in candidates {
		volume.scan(candidate.shard_rows.len());
		// Credit the freed live SHARD bytes back to quota (#10). C4 only deletes live versions in its
		// `(cold_wm, hot_wm]` walk range, so there is no already-demoted S3-only version to double-credit.
		let mut freed = 0_usize;
		for (key, value) in &candidate.shard_rows {
			udb::compare_and_clear(tx, key, value);
			freed = freed.saturating_add(key.len().saturating_add(value.len()));
			volume.clear(key.len(), value.len());
		}
		quota::atomic_add_branch(
			tx,
			branch_id,
			i64::try_from(freed).unwrap_or(i64::MAX).saturating_neg(),
		);

		// Drop the shard from its own fold entry so `CMP/fold/{X}` keeps recording exactly the shards
		// with a live SHARD row at fold txid `X`. An emptied entry is cleared so the index stays bounded
		// (C4 is the sole fold clearer when cold is off). This whole delete is one Serializable tx, so the
		// read here conflicts on `fold_key` and a concurrent fold write aborts the job.
		let fold_key = keys::branch_compaction_fold_key(branch_id, candidate.reference.as_of_txid);
		if let Some(fold_value) = tx_get_value(tx, &fold_key, Serializable).await? {
			let mut entry = decode_fold_index_entry(&fold_value)
				.context("decode sqlite fold index entry for dead shard version delete")?;
			let before = entry.shard_ids.len();
			entry
				.shard_ids
				.retain(|shard_id| *shard_id != candidate.reference.shard_id);
			if entry.shard_ids.len() != before {
				if entry.shard_ids.is_empty() {
					udb::compare_and_clear(tx, &fold_key, &fold_value);
				} else {
					tx.informal().set(
						&fold_key,
						&encode_fold_index_entry(entry).context(
							"encode sqlite fold index entry for dead shard version delete",
						)?,
					);
				}
			}
		}
	}
	Ok(volume)
}

/// Whether a shard version is still the image some retained point in history reads through. A read
/// at txid `c` resolves a shard through the newest version at or below `c`, so version `X` serves
/// exactly the retained txids in `[X, superseded_by)`, and `[X, ∞)` when nothing supersedes it.
///
/// The truncate cleanup and the cold object reclaim both decide retention by this; do not add a
/// third variant. The dead-shard sweep applies the same interval but derives it inline from its
/// `CMP/fold` walk, which already carries each version's successor, so it does not call through here.
pub(crate) fn shard_version_is_retained(
	retained_txids: &BTreeSet<u64>,
	as_of_txid: u64,
	superseded_by_txid: Option<u64>,
) -> bool {
	match superseded_by_txid {
		Some(superseded_by_txid) => retained_txids
			.range(as_of_txid..superseded_by_txid)
			.next()
			.is_some(),
		None => retained_txids.range(as_of_txid..).next().is_some(),
	}
}

pub(crate) fn reclaim_delete_upper_bound(
	root: &CompactionRoot,
	db_pins: &[DbHistoryPin],
	pitr_interval_retention: &[PitrIntervalSelection],
) -> Option<u64> {
	if root.hot_watermark_txid == 0 {
		return None;
	}

	let pinned_floor = db_pins
		.iter()
		.filter(|pin| pin.at_txid <= root.hot_watermark_txid)
		.map(|pin| pin.at_txid)
		.chain(
			pitr_interval_retention
				.iter()
				.filter(|selection| selection.coverage.txid <= root.hot_watermark_txid)
				.map(|selection| selection.coverage.txid),
		)
		.min();
	let max_reclaim_txid = pinned_floor
		.map(|txid| txid.saturating_sub(1))
		.unwrap_or(root.hot_watermark_txid);

	(max_reclaim_txid > 0).then_some(max_reclaim_txid)
}

/// The txids this slice folds a complete image at.
///
/// Coverage is what licenses delta reclaim, so a commit folded only in part must never appear here:
/// its delta still holds the pages above the slice's page bound, and naming it as coverage would let
/// reclaim drop a delta whose pages no shard image carries. `selected_max_pgno_exclusive` is `Some`
/// exactly when the slice's last commit is partial, and a partial commit is always the last.
pub(crate) fn selected_hot_coverage_txids(
	root: &CompactionRoot,
	selected_max_txid: u64,
	selected_max_pgno_exclusive: Option<u32>,
	db_pins: &[DbHistoryPin],
	pitr_interval_coverage: &[PitrIntervalSelection],
) -> Vec<u64> {
	let mut coverage_txids = BTreeSet::new();
	if selected_max_pgno_exclusive.is_none() {
		coverage_txids.insert(selected_max_txid);
	}
	// A partial commit is still folded: `hot_fold_txids` adds it back as a fold target. The two lists
	// must stay separate, because folding is the work and coverage is the licence to reclaim.

	// A pin or PITR representative strictly below the partial commit was folded whole by this slice,
	// so it is still real coverage. Only the partial commit itself is excluded.
	let coverage_ceiling = match selected_max_pgno_exclusive {
		Some(_) => selected_max_txid.saturating_sub(1),
		None => selected_max_txid,
	};
	for pin in db_pins {
		if pin.at_txid > root.hot_watermark_txid && pin.at_txid <= coverage_ceiling {
			coverage_txids.insert(pin.at_txid);
		}
	}
	for selection in pitr_interval_coverage {
		let txid = selection.coverage.txid;
		if txid > root.hot_watermark_txid && txid <= coverage_ceiling {
			coverage_txids.insert(txid);
		}
	}

	coverage_txids.into_iter().collect()
}

/// The txid a hot drain runs to: the live head, capped to one drain window past the watermark, then
/// rounded down to a `HOT_DRAIN_HEAD_GRAIN_TXIDS` boundary.
///
/// Rounding is what makes a drain reproducible. Every other input to slice selection is already
/// stable between two drains from the same watermark, so pinning the head to a grid point means an
/// abandoned job's successor picks the same boundaries, writes the same `(shard_id, as_of_txid)`
/// keys, and overwrites its predecessor's images instead of stranding them at txids nothing revisits.
///
/// Returns a value at or below the watermark when the backlog has not yet crossed the next grid
/// point, which the caller reads as "nothing to drain yet".
pub(crate) fn snapped_drain_head_txid(
	hot_watermark_txid: u64,
	head_txid: u64,
	max_hot_drain_span_txids: u64,
) -> u64 {
	let capped = head_txid.min(hot_watermark_txid.saturating_add(max_hot_drain_span_txids));
	let span = capped.saturating_sub(hot_watermark_txid);
	let snapped_span = span - (span % HOT_DRAIN_HEAD_GRAIN_TXIDS);
	hot_watermark_txid.saturating_add(snapped_span)
}

/// The txid a hot drain runs to.
///
/// Grid-snapping the head is what makes a drain reproducible, so an abandoned job's successor picks
/// the same slice boundaries, writes the same `(shard_id, as_of_txid)` keys, and overwrites its
/// predecessor's images instead of stranding them at txids nothing revisits.
///
/// `highest_stable_coverage_txid` is the largest pin or PITR representative in the window, excluding
/// the drain head itself. The drain must reach it, so it raises the head when it sits above the grid
/// point. Doing that keeps reproducibility: a pin txid does not move between drains, so two drains
/// from the same watermark still agree. Taking the live head here instead would put the final slice's
/// boundary back on a moving target, and that boundary is itself a coverage txid
/// (`selected_hot_coverage_txids` inserts `selected_max_txid`), so it is exactly the version that
/// would be stranded.
///
/// `force` is the one case that genuinely needs the live head: it means "compact what is there now",
/// including a tail shorter than a grain.
pub(crate) fn plan_drain_head_txid(
	hot_watermark_txid: u64,
	head_txid: u64,
	highest_stable_coverage_txid: Option<u64>,
	force: bool,
	max_hot_drain_span_txids: u64,
) -> u64 {
	let capped = head_txid.min(hot_watermark_txid.saturating_add(max_hot_drain_span_txids));
	if force {
		return capped;
	}
	let snapped = snapped_drain_head_txid(hot_watermark_txid, head_txid, max_hot_drain_span_txids);
	match highest_stable_coverage_txid {
		Some(coverage_txid) if coverage_txid <= capped => snapped.max(coverage_txid),
		_ => snapped,
	}
}

pub(crate) fn plan_hot_job(
	database_branch_id: DatabaseBranchId,
	snapshot: &ManagerFdbSnapshot,
	job_id: Id,
	now_ms: i64,
	force: bool,
	max_hot_drain_span_txids: u64,
) -> Option<PlannedHotCompactionJob> {
	let branch_record = snapshot.branch_record.as_ref()?;
	let head = snapshot.head.as_ref()?;
	if head.head_txid <= snapshot.root.hot_watermark_txid {
		return None;
	}

	let hot_lag = head
		.head_txid
		.saturating_sub(snapshot.root.hot_watermark_txid);
	let Some(selected_max_txid) = snapshot.hot_inputs.selected_max_txid else {
		// A window whose first commit does not fit the slice budget can never be drained by a
		// smaller slice, so the branch's hot lane is stuck here until the budget or the commit cap
		// changes. That is the safe outcome (the commit stays readable through its delta) but it
		// must not be silent.
		if let Some(txid) = snapshot.hot_inputs.oversized_commit_txid {
			tracing::warn!(
				?database_branch_id,
				txid,
				hot_watermark_txid = snapshot.root.hot_watermark_txid,
				"hot compaction cannot plan a drain: commit exceeds the hot slice budget"
			);
		}
		return None;
	};
	let coverage_txids = selected_hot_coverage_txids(
		&snapshot.root,
		selected_max_txid,
		snapshot.hot_inputs.selected_max_pgno_exclusive,
		&snapshot.db_pins,
		&snapshot.hot_inputs.pitr_interval_coverage,
	);
	let has_uncovered_pin = coverage_txids
		.iter()
		.any(|txid| *txid != selected_max_txid && *txid > snapshot.root.hot_watermark_txid);
	if hot_lag < quota::COMPACTION_DELTA_THRESHOLD && !has_uncovered_pin && !force {
		return None;
	}

	// The largest pin or PITR representative this drain has to reach, excluding the drain head
	// itself. `selected_max_txid` is head-derived and would drag the boundary back onto a moving
	// target, which is the whole thing the grid exists to prevent.
	let highest_stable_coverage_txid = coverage_txids
		.iter()
		.copied()
		.filter(|txid| *txid != selected_max_txid && *txid > snapshot.root.hot_watermark_txid)
		.max();
	let drain_head_txid = plan_drain_head_txid(
		snapshot.root.hot_watermark_txid,
		head.head_txid,
		highest_stable_coverage_txid,
		force,
		max_hot_drain_span_txids,
	);
	if drain_head_txid <= snapshot.root.hot_watermark_txid {
		return None;
	}

	// `selected_max_txid` was read against the live head, so this range can extend past
	// `drain_head_txid` once the head is snapped. That is deliberate and inert: the drain and the
	// install both bound themselves by `drain_head_txid` (`db_manager.rs` builds the install range
	// from it, and every slice pins `head.head_txid` to it before reading), so nothing folds or
	// advances the watermark past it. This range feeds the plan fingerprint and the manager's
	// bookkeeping, both of which recompute from the same inputs on either side.
	let input_range = HotJobInputRange {
		txids: TxidRange {
			min_txid: snapshot.root.hot_watermark_txid.saturating_add(1),
			max_txid: selected_max_txid,
		},
		max_pgno_exclusive: snapshot.hot_inputs.selected_max_pgno_exclusive,
		coverage_txids: coverage_txids.clone(),
		max_pages: u32::try_from(snapshot.hot_inputs.pidx_entries.len()).unwrap_or(u32::MAX),
		max_bytes: snapshot.hot_inputs.total_value_bytes,
	};
	let input_fingerprint = fingerprint_hot_inputs(
		database_branch_id,
		&snapshot.root,
		head,
		&coverage_txids,
		&snapshot.hot_inputs,
	);

	Some(PlannedHotCompactionJob {
		database_branch_id,
		job_id,
		base_lifecycle_generation: branch_record.lifecycle_generation,
		base_manifest_generation: snapshot.root.manifest_generation,
		input_fingerprint,
		input_range,
		// Capture the head and clock once so the companion drains up to this head and every staged
		// slice + the bulk install bind the same fingerprint inputs. The head is capped to a bounded
		// txid window past the hot watermark so a large unfolded backlog drains incrementally across
		// manager refresh cycles instead of one unbounded job, then snapped down to a grid so the
		// boundaries this drain picks do not depend on where the live head happened to be.
		drain_head_txid,
		drain_now_ms: now_ms,
		planned_at_ms: now_ms,
		attempt: 0,
	})
}

pub(crate) fn plan_reclaim_job(
	database_branch_id: DatabaseBranchId,
	snapshot: &ManagerFdbSnapshot,
	job_id: Id,
	now_ms: i64,
	skip_commit_delta: bool,
) -> Option<PlannedReclaimCompactionJob> {
	let branch_record = snapshot.branch_record.as_ref()?;
	if snapshot.bucket_proof_blocked_reclaim {
		return None;
	}
	let has_delta_reclaim = !snapshot.reclaim_inputs.delta_reclaim_segments.is_empty();
	let has_commit_reclaim = !snapshot.reclaim_inputs.commit_reclaim_txids.is_empty();
	let has_cold_reclaim = !snapshot.reclaim_inputs.cold_object_refs.is_empty();
	let has_shard_cache_eviction = !snapshot.reclaim_inputs.shard_cache_evictions.is_empty();
	let has_dead_shard_sweep = snapshot.reclaim_inputs.dead_shard_sweep_needed;
	let has_shard_lru_cleanup = !snapshot.reclaim_inputs.shard_lru_cleanup_keys.is_empty();
	let has_interval_cleanup = !snapshot
		.reclaim_inputs
		.expired_pitr_interval_rows
		.is_empty();
	// The cold-object reclaim scan is windowed (R5): when its cursor can still advance there may be
	// eligible refs sitting behind the ineligible (pinned / above-watermark) rows that filled this
	// window, so plan a (possibly cold-empty) slice to keep the drain advancing rather than stalling.
	let cold_scan_can_advance = snapshot.reclaim_inputs.next_cold_scan_cursor.is_some();
	if !has_delta_reclaim
		&& !has_commit_reclaim
		&& !has_cold_reclaim
		&& !has_shard_cache_eviction
		&& !has_dead_shard_sweep
		&& !has_shard_lru_cleanup
		&& !has_interval_cleanup
		&& !cold_scan_can_advance
	{
		return None;
	}

	// A slice that skips the commit/delta lane must carry it empty everywhere: in the input range the
	// delete compares against, and in the fingerprint, which hashes both txid lists. Leaving them
	// populated while the flag says skip makes the delete derive nothing, compare it against a
	// non-empty plan, and reject every slice. Enforced here rather than left to each caller to arrange
	// by handing in a snapshot that happens to be empty.
	let (delta_reclaim_segments, commit_reclaim_txids) = if skip_commit_delta {
		(Vec::new(), Vec::new())
	} else {
		(
			snapshot.reclaim_inputs.delta_reclaim_segments.clone(),
			snapshot.reclaim_inputs.commit_reclaim_txids.clone(),
		)
	};
	// The range this job covers spans every txid its segments touch plus every commit row it
	// reclaims. Segments of one commit share a txid, so the duplicates a plain iterator yields are
	// harmless to a min/max.
	let reclaim_txid_bounds = delta_reclaim_segments
		.iter()
		.map(|segment| segment.txid)
		.chain(commit_reclaim_txids.iter().copied());
	let min_txid = reclaim_txid_bounds
		.clone()
		.min()
		.unwrap_or(snapshot.root.cold_watermark_txid);
	let max_txid = reclaim_txid_bounds
		.max()
		.unwrap_or(snapshot.root.cold_watermark_txid);
	let input_range = ReclaimJobInputRange {
		txids: TxidRange { min_txid, max_txid },
		delta_reclaim_segments: delta_reclaim_segments.clone(),
		cursor_segment_pgno: snapshot.reclaim_inputs.cursor_segment_pgno,
		commit_reclaim_txids: commit_reclaim_txids.clone(),
		cold_objects: snapshot.reclaim_inputs.cold_object_refs.clone(),
		shard_cache_evictions: snapshot
			.reclaim_inputs
			.shard_cache_evictions
			.iter()
			.map(|candidate| candidate.reference.clone())
			.collect(),
		stale_hot_job_ids: Vec::new(),
		stale_commit_stage_txids: Vec::new(),
		stale_cold_job_ids: Vec::new(),
		skip_commit_delta,
		// The exact cold-scan window this slice was derived from, so the delete re-derives `cold_objects`
		// identically under Serializable (R5).
		cold_scan_cursor: snapshot.reclaim_inputs.cold_scan_cursor,
		// Likewise for the commit window, so the delete re-derives the same commit/delta classification.
		commit_scan_cursor: snapshot.reclaim_inputs.commit_scan_cursor,
		max_keys: CMP_FDB_BATCH_MAX_KEYS as u32,
		max_bytes: CMP_FDB_BATCH_MAX_VALUE_BYTES as u64,
	};
	let input_fingerprint = fingerprint_reclaim_inputs_scoped(
		database_branch_id,
		&snapshot.root,
		&snapshot.reclaim_inputs,
		!skip_commit_delta,
	);

	Some(PlannedReclaimCompactionJob {
		database_branch_id,
		job_id,
		base_lifecycle_generation: branch_record.lifecycle_generation,
		base_manifest_generation: snapshot.root.manifest_generation,
		input_fingerprint,
		input_range,
		planned_at_ms: now_ms,
		attempt: 0,
	})
}

/// Explains why a refresh planned no reclaim job. Only call this when `plan_reclaim_job` returned
/// `None`: a reason reported next to a planned job reads as an idle lane when reclaim is in fact
/// dispatching every cycle, which is the wrong conclusion to hand an operator.
///
/// The first two arms mirror the planner's two hard early-outs. The rest split the case where every
/// lane came back empty, because "this window found nothing" and "this branch has nothing left" are
/// different situations and only the second one means the drain is finished.
pub(crate) fn reclaim_noop_reason(snapshot: &ManagerFdbSnapshot) -> &'static str {
	if snapshot.branch_record.is_none() {
		return "reclaim:no-branch-record";
	}
	if snapshot.bucket_proof_blocked_reclaim {
		return "reclaim:bucket-proof-blocked";
	}

	// The window read rows and classified all of them retained, so history is present and something is
	// holding it: a pin, unexpired PITR coverage, or the watermark cap.
	if !snapshot.reclaim_inputs.commits.is_empty()
		|| !snapshot.reclaim_inputs.delta_chunks.is_empty()
	{
		return "reclaim:window-fully-retained";
	}
	// An empty window below the end of the range. The scan is still walking toward reclaimable rows
	// that sit behind it, so this is a drain in progress, not a settled branch.
	if !snapshot.reclaim_inputs.commit_scan_complete {
		return "reclaim:scan-window-empty";
	}

	"reclaim:no-actionable-work"
}

pub(crate) fn fingerprint_hot_inputs(
	database_branch_id: DatabaseBranchId,
	root: &CompactionRoot,
	head: &DBHead,
	coverage_txids: &[u64],
	hot_inputs: &HotInputSnapshot,
) -> CompactionInputFingerprint {
	let mut fingerprint = Sha256::new();
	update_fingerprint(&mut fingerprint, database_branch_id.as_uuid().as_bytes());
	update_fingerprint(&mut fingerprint, &root.manifest_generation.to_be_bytes());
	update_fingerprint(&mut fingerprint, &root.hot_watermark_txid.to_be_bytes());
	update_fingerprint(&mut fingerprint, &head.head_txid.to_be_bytes());
	for txid in coverage_txids {
		update_fingerprint(&mut fingerprint, &txid.to_be_bytes());
	}
	for selection in &hot_inputs.pitr_interval_coverage {
		update_fingerprint(&mut fingerprint, &selection.bucket_start_ms.to_be_bytes());
		update_fingerprint(&mut fingerprint, &selection.coverage.txid.to_be_bytes());
		update_fingerprint(&mut fingerprint, &selection.coverage.versionstamp);
		update_fingerprint(
			&mut fingerprint,
			&selection.coverage.wall_clock_ms.to_be_bytes(),
		);
		update_fingerprint(
			&mut fingerprint,
			&selection.coverage.expires_at_ms.to_be_bytes(),
		);
	}
	for (txid, commit) in &hot_inputs.commits {
		update_fingerprint(&mut fingerprint, &txid.to_be_bytes());
		update_fingerprint(&mut fingerprint, &commit.wall_clock_ms.to_be_bytes());
		update_fingerprint(&mut fingerprint, &commit.versionstamp);
		update_fingerprint(&mut fingerprint, &commit.db_size_pages.to_be_bytes());
		update_fingerprint(&mut fingerprint, &commit.post_apply_checksum.to_be_bytes());
	}
	for (key, value) in &hot_inputs.delta_chunks {
		update_fingerprint(&mut fingerprint, key);
		update_fingerprint(&mut fingerprint, value);
	}
	for (key, value) in &hot_inputs.pidx_entries {
		update_fingerprint(&mut fingerprint, key);
		update_fingerprint(&mut fingerprint, value);
	}
	finish_fingerprint(fingerprint)
}

/// Fingerprints a reclaim slice, optionally excluding the commit/delta lane.
///
/// The lane contributes four fields: both txid lists and the whole scanned `commits` / `delta_chunks`
/// window. A slice that does not own the lane must exclude all four, because the side that re-derives
/// it skips the scan entirely and so has them empty. Excluding only the txid lists still leaves the
/// scanned window hashed on the deriving side and empty on the other, which rejects every slice with
/// `reclaim input fingerprint changed`.
pub(crate) fn fingerprint_reclaim_inputs_scoped(
	database_branch_id: DatabaseBranchId,
	root: &CompactionRoot,
	reclaim_inputs: &ReclaimInputSnapshot,
	include_commit_delta: bool,
) -> CompactionInputFingerprint {
	let empty_txids: &[u64] = &[];
	let empty_segments: &[DeltaSegmentRef] = &[];
	let (delta_reclaim_segments, commit_reclaim_txids) = if include_commit_delta {
		(
			reclaim_inputs.delta_reclaim_segments.as_slice(),
			reclaim_inputs.commit_reclaim_txids.as_slice(),
		)
	} else {
		(empty_segments, empty_txids)
	};
	let mut fingerprint = Sha256::new();
	update_fingerprint(&mut fingerprint, database_branch_id.as_uuid().as_bytes());
	update_fingerprint(&mut fingerprint, &root.manifest_generation.to_be_bytes());
	update_fingerprint(&mut fingerprint, &root.hot_watermark_txid.to_be_bytes());
	update_fingerprint(&mut fingerprint, &root.cold_watermark_txid.to_be_bytes());
	for segment in delta_reclaim_segments {
		update_fingerprint(&mut fingerprint, &segment.txid.to_be_bytes());
		// A legacy blob and a segment starting at page 0 are different rows, so the discriminator has
		// to be part of the hash rather than the page number alone.
		match segment.first_pgno {
			Some(first_pgno) => {
				update_fingerprint(&mut fingerprint, &[1]);
				update_fingerprint(&mut fingerprint, &first_pgno.to_be_bytes());
			}
			None => update_fingerprint(&mut fingerprint, &[0]),
		}
	}
	for txid in commit_reclaim_txids {
		update_fingerprint(&mut fingerprint, &txid.to_be_bytes());
	}
	for cold_object in &reclaim_inputs.cold_object_refs {
		update_fingerprint(&mut fingerprint, cold_object.object_key.as_bytes());
		update_fingerprint(
			&mut fingerprint,
			&cold_object.object_generation_id.as_bytes(),
		);
		update_fingerprint(&mut fingerprint, &cold_object.content_hash);
		update_fingerprint(
			&mut fingerprint,
			&cold_object.expected_publish_generation.to_be_bytes(),
		);
		update_fingerprint(&mut fingerprint, &cold_object.shard_id.to_be_bytes());
		update_fingerprint(&mut fingerprint, &cold_object.as_of_txid.to_be_bytes());
	}
	for candidate in &reclaim_inputs.shard_cache_evictions {
		update_fingerprint(
			&mut fingerprint,
			&candidate.reference.shard_id.to_be_bytes(),
		);
		update_fingerprint(
			&mut fingerprint,
			&candidate.reference.as_of_txid.to_be_bytes(),
		);
		update_fingerprint(
			&mut fingerprint,
			&candidate.reference.size_bytes.to_be_bytes(),
		);
		update_fingerprint(&mut fingerprint, &candidate.reference.content_hash);
		for (key, value) in &candidate.shard_rows {
			update_fingerprint(&mut fingerprint, key);
			update_fingerprint(&mut fingerprint, value);
		}
		update_fingerprint(&mut fingerprint, &candidate.cold_ref_key);
		update_fingerprint(&mut fingerprint, &candidate.cold_ref_bytes);
	}
	for (txid, key, value, commit) in reclaim_inputs
		.commits
		.iter()
		.filter(|_| include_commit_delta)
	{
		update_fingerprint(&mut fingerprint, &txid.to_be_bytes());
		update_fingerprint(&mut fingerprint, key);
		update_fingerprint(&mut fingerprint, value);
		update_fingerprint(&mut fingerprint, &commit.versionstamp);
	}
	for (key, value) in reclaim_inputs
		.delta_chunks
		.iter()
		.filter(|_| include_commit_delta)
	{
		update_fingerprint(&mut fingerprint, key);
		update_fingerprint(&mut fingerprint, value);
	}
	for (bucket_start_ms, key, value, coverage) in &reclaim_inputs.expired_pitr_interval_rows {
		update_fingerprint(&mut fingerprint, &bucket_start_ms.to_be_bytes());
		update_fingerprint(&mut fingerprint, key);
		update_fingerprint(&mut fingerprint, value);
		update_fingerprint(&mut fingerprint, &coverage.txid.to_be_bytes());
		update_fingerprint(&mut fingerprint, &coverage.versionstamp);
		update_fingerprint(&mut fingerprint, &coverage.wall_clock_ms.to_be_bytes());
		update_fingerprint(&mut fingerprint, &coverage.expires_at_ms.to_be_bytes());
	}
	finish_fingerprint(fingerprint)
}

pub(crate) fn fingerprint_repair_reclaim_range(
	database_branch_id: DatabaseBranchId,
	input_range: &ReclaimJobInputRange,
) -> CompactionInputFingerprint {
	let mut fingerprint = Sha256::new();
	update_fingerprint(&mut fingerprint, database_branch_id.as_uuid().as_bytes());
	update_fingerprint(&mut fingerprint, &input_range.txids.min_txid.to_be_bytes());
	update_fingerprint(&mut fingerprint, &input_range.txids.max_txid.to_be_bytes());
	for job_id in &input_range.stale_hot_job_ids {
		update_fingerprint(&mut fingerprint, &job_id.as_bytes());
	}
	for job_id in &input_range.stale_cold_job_ids {
		update_fingerprint(&mut fingerprint, &job_id.as_bytes());
	}
	finish_fingerprint(fingerprint)
}

pub(crate) fn update_fingerprint(fingerprint: &mut Sha256, bytes: &[u8]) {
	fingerprint.update((bytes.len() as u64).to_be_bytes());
	fingerprint.update(bytes);
}

pub(crate) fn finish_fingerprint(fingerprint: Sha256) -> CompactionInputFingerprint {
	let digest = fingerprint.finalize();
	let mut output = [0_u8; 32];
	output.copy_from_slice(&digest);
	output
}

/// Staged shard images written by one hot stage transaction. `next_stage_cursor` is `Some` when the
/// transaction stopped at `CMP_STAGE_MAX_WRITE_BYTES` with images left to stage, so the caller must
/// run another transaction from that cursor before advancing its txid cursor.
pub(crate) struct StagedHotShardsOutput {
	// Read only by the fault-injection path, which clears the images this transaction wrote.
	#[cfg_attr(not(feature = "test-faults"), allow(dead_code))]
	pub(crate) output_refs: Vec<HotShardOutputRef>,
	pub(crate) staged_bytes: u64,
	pub(crate) next_stage_cursor: Option<HotStageCursor>,
}

pub(crate) enum StagedHotShards {
	Staged(StagedHotShardsOutput),
}

/// Stages the slice's folded shard images, starting at `stage_cursor` and stopping once this
/// transaction's staged bytes reach `CMP_STAGE_MAX_WRITE_BYTES`.
///
/// Hot staging writes a complete shard image per `(coverage txid, touched shard)` pair, so its write
/// volume scales with how widely the slice's pages scatter across shards, not with the bytes the input
/// read budget admitted. Without this cap a scattered slice, or even a single wide commit, commits far
/// past FDB's transaction limit and fails the activity with `transaction_too_large` on every retry. The
/// walk is ordered (coverage txids ascending, shards ascending within each), so a later transaction
/// re-derives the same sequence and starts exactly where the previous one stopped. Restaging a pair is
/// harmless anyway: shard images are content-addressed and the writes are idempotent.
/// The txids this slice folds an image at, ascending.
///
/// This is coverage plus the slice's partially admitted commit, when it has one. The two lists are
/// deliberately different: coverage is what licenses delta reclaim and so must never name a commit
/// whose delta still holds pages no image carries, while folding is the work the slice exists to do
/// and must reach the partial commit or the cursor bounds nothing. `selected_hot_coverage_txids`
/// drops the partial commit for exactly the reason this adds it back.
///
/// Derived from the range rather than carried as a field, so the plan fingerprint stays unchanged and
/// the staging and install sides cannot disagree about it.
pub(crate) fn hot_fold_txids(input_range: &HotJobInputRange) -> Vec<u64> {
	let mut fold_txids = input_range.coverage_txids.clone();
	if input_range.max_pgno_exclusive.is_some() {
		fold_txids.push(input_range.txids.max_txid);
	}
	fold_txids.sort_unstable();
	fold_txids.dedup();

	fold_txids
}

pub(crate) async fn write_staged_hot_shards(
	tx: &universaldb::Transaction,
	input: &StageHotJobInput,
	deltas: &BTreeMap<u64, DecodedLtx>,
	db_size_pages_by_txid: &[(u64, u32)],
	stage_cursor: Option<HotStageCursor>,
	direct_to_shard: bool,
) -> Result<StagedHotShards> {
	let mut output_refs = Vec::new();
	let mut staged_bytes = 0_u64;

	let fold_txids = hot_fold_txids(&input.input_range);
	for as_of_txid in &fold_txids {
		// Fold txids an earlier transaction finished are fully staged.
		if stage_cursor.is_some_and(|cursor| *as_of_txid < cursor.as_of_txid) {
			continue;
		}
		// Only the slice's last commit can be partial, and only its own pages are bounded. Every
		// other fold target is folded whole.
		let max_pgno_exclusive = (*as_of_txid == input.input_range.txids.max_txid)
			.then_some(input.input_range.max_pgno_exclusive)
			.flatten();
		// The image folded for a coverage txid must hold every page the database had at that txid,
		// not at the slice maximum. A shrink later in the slice would otherwise drop pages this txid
		// still has, the install would clear their page index rows, and a read pinned here would
		// resolve them to zeros.
		let db_size_pages = db_size_pages_at_txid(db_size_pages_by_txid, *as_of_txid)
			.with_context(|| {
				format!("no commit at or below coverage txid {as_of_txid} records a database size")
			})?;
		let pages_by_shard =
			collect_hot_pages_by_shard(db_size_pages, deltas, *as_of_txid, max_pgno_exclusive)?;

		for (shard_id, page_updates) in pages_by_shard {
			if stage_cursor.is_some_and(|cursor| {
				*as_of_txid == cursor.as_of_txid && shard_id < cursor.shard_id
			}) {
				continue;
			}
			// Checked before encoding the image, so a transaction that has already staged something
			// stops here and one that has not always writes at least one image. That is what keeps the
			// drain making progress on a slice too wide for a single transaction.
			if staged_bytes >= CMP_STAGE_MAX_WRITE_BYTES {
				return Ok(StagedHotShards::Staged(StagedHotShardsOutput {
					output_refs,
					staged_bytes,
					next_stage_cursor: Some(HotStageCursor {
						as_of_txid: *as_of_txid,
						shard_id,
					}),
				}));
			}
			let encoded = match build_staged_hot_shard_blob(
				tx,
				input.database_branch_id,
				input.job_id,
				shard_id,
				*as_of_txid,
				page_updates,
			)
			.await?
			{
				StagedHotShardBlob::Encoded(encoded) => encoded,
				// Report the ref so the caller can fetch the image and re-enter at the same cursor.
			};
			let content_hash = content_hash(&encoded);

			// Writing straight into `SHARD` skips the install's byte-for-byte copy, which is the
			// whole point: the image is written once instead of twice.
			//
			// This image is readable immediately, before the install and before the watermark moves.
			// Do not assume otherwise. The read path caps version selection at the source's head
			// (`conveyer/read/plan.rs`), not at the hot watermark, so any page in this shard that has
			// no live PIDX row resolves through this version as soon as the transaction commits. Only
			// pages that still own a PIDX row keep reading from their delta.
			//
			// What keeps that correct is the completeness invariant on `load_merge_base_shard_blob`:
			// every version written must be a complete image of its shard, because the read path
			// resolves a page from exactly one version and zero-fills whatever it is missing. Direct
			// folding makes that invariant load-bearing for data no install has published yet, which
			// it was not before. So the merge base must be read even though this fold "is not live
			// yet", and a slice-local sparse image is never acceptable.
			//
			// Two caps bound how far an unpublished version can reach, and both key on the watermark
			// rather than on this version: `tx_load_source_shard_fold_floors` mins the DELTA-walk
			// floor with `hot_watermark_txid`, and the stale-PIDX sweep caps its page set the same
			// way. Deltas above the watermark are therefore still replayed over this image.
			if direct_to_shard {
				shard_blob::write_shard_blob(
					tx,
					input.database_branch_id,
					shard_id,
					*as_of_txid,
					&encoded,
				)?;
			} else {
				// Stage the blob as chunk rows so no staged value exceeds the FDB value cap. A retry
				// clears the version's chunk range first so a shorter re-encode leaves no stale tail.
				let (stage_begin, stage_end) = keys::branch_compaction_stage_hot_shard_txid_range(
					input.database_branch_id,
					input.job_id,
					shard_id,
					*as_of_txid,
				);
				tx.informal().clear_range(&stage_begin, &stage_end);
				for (chunk_idx, chunk) in shard_blob::split_shard_blob(&encoded)? {
					tx.informal().set(
						&keys::branch_compaction_stage_hot_shard_key(
							input.database_branch_id,
							input.job_id,
							shard_id,
							*as_of_txid,
							chunk_idx,
						),
						chunk,
					);
				}
			}
			let size_bytes = u64::try_from(encoded.len()).unwrap_or(u64::MAX);
			staged_bytes = staged_bytes.saturating_add(size_bytes);
			// Persist the ref metadata alongside the blob so the manager install and reclaimer cleanup
			// can re-derive the drained ref set from FDB. The companion no longer accumulates refs in
			// workflow state. Keyed by the slice `min_txid` so the install scans exactly this slice's
			// refs per chunk. Overwriting on a retry is idempotent.
			tx.informal().set(
				&keys::branch_compaction_stage_hot_ref_key(
					input.database_branch_id,
					input.job_id,
					input.input_range.txids.min_txid,
					shard_id,
					*as_of_txid,
				),
				&encode_staged_hot_shard_ref(StagedHotShardRef {
					shard_id,
					as_of_txid: *as_of_txid,
					min_txid: input.input_range.txids.min_txid,
					size_bytes,
					content_hash,
				})
				.context("encode staged hot shard ref")?,
			);
			output_refs.push(HotShardOutputRef {
				shard_id,
				as_of_txid: *as_of_txid,
				min_txid: input.input_range.txids.min_txid,
				max_txid: *as_of_txid,
				size_bytes,
				content_hash,
			});
		}
	}

	Ok(StagedHotShards::Staged(StagedHotShardsOutput {
		output_refs,
		staged_bytes,
		next_stage_cursor: None,
	}))
}

fn hot_shard_output_ref_from_staged(staged: StagedHotShardRef) -> HotShardOutputRef {
	HotShardOutputRef {
		shard_id: staged.shard_id,
		as_of_txid: staged.as_of_txid,
		min_txid: staged.min_txid,
		// A staged hot shard always has `max_txid == as_of_txid`, so it is not persisted.
		max_txid: staged.as_of_txid,
		size_bytes: staged.size_bytes,
		content_hash: staged.content_hash,
	}
}

/// Reads the staged hot shard refs for one drain slice (grouped by the slice's `min_txid`) and
/// reconstructs the `HotShardOutputRef` set. The install re-derives the same cursor sequence, so it
/// calls this per chunk to get exactly that slice's refs from FDB instead of workflow state.
pub(crate) async fn read_staged_hot_shard_refs_for_slice(
	tx: &universaldb::Transaction,
	database_branch_id: DatabaseBranchId,
	job_id: Id,
	min_txid: u64,
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<Vec<HotShardOutputRef>> {
	let prefix =
		keys::branch_compaction_stage_hot_ref_slice_prefix(database_branch_id, job_id, min_txid);
	let rows = tx_scan_prefix_values(tx, &prefix, isolation_level).await?;
	let mut refs = Vec::with_capacity(rows.len());
	for (_key, value) in &rows {
		let staged = decode_staged_hot_shard_ref(value)
			.context("decode staged hot shard ref for install")?;
		refs.push(hot_shard_output_ref_from_staged(staged));
	}
	Ok(refs)
}

/// One job's staged hot shard refs, capped by the caller's batch budget. `has_more` is true when the
/// scan stopped early, so the caller reports that its slice left work behind.
pub(crate) struct StagedHotShardRefsPage {
	pub(crate) refs: Vec<HotShardOutputRef>,
	pub(crate) has_more: bool,
}

/// Reads one job's staged hot shard refs under the caller's batch budget. A job's staging area scales
/// with the shard images its drain staged, so cleanup cannot enumerate the whole thing in a single
/// transaction; this caps the enumeration the same way the other reclaim lanes cap their candidates.
pub(crate) async fn read_staged_hot_shard_refs_limited(
	tx: &universaldb::Transaction,
	database_branch_id: DatabaseBranchId,
	job_id: Id,
	isolation_level: universaldb::utils::IsolationLevel,
	budget: &mut CompactionBatchBudget,
) -> Result<StagedHotShardRefsPage> {
	let prefix = keys::branch_compaction_stage_hot_ref_prefix(database_branch_id, job_id);
	// `end_of_key_range` appends a zero byte, which is just past one whole key and would exclude
	// everything under the prefix, so bound the scan with the prefix subspace's own range.
	let (scan_begin, scan_end) =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix.clone()))
			.range();
	let rows = tx_scan_range_values_limited(
		tx,
		&scan_begin,
		&scan_end,
		CMP_STAGE_CLEANUP_REF_PAGE_KEYS,
		isolation_level,
	)
	.await?;
	let has_more = rows.len() >= CMP_STAGE_CLEANUP_REF_PAGE_KEYS;

	let mut refs = Vec::with_capacity(rows.len());
	for (_key, value) in &rows {
		let value_bytes = u64::try_from(value.len()).unwrap_or(u64::MAX);
		if !budget.can_add(1, value_bytes) {
			return Ok(StagedHotShardRefsPage {
				refs,
				has_more: true,
			});
		}
		budget.add(1, value_bytes);
		let staged = decode_staged_hot_shard_ref(value)
			.context("decode staged hot shard ref for cleanup")?;
		refs.push(hot_shard_output_ref_from_staged(staged));
	}

	Ok(StagedHotShardRefsPage { refs, has_more })
}

/// Enumerates staged commits that look abandoned: above head, and untouched for longer than the
/// grace window.
///
/// "Above head" is what makes a staged commit distinguishable from a finalized one: finalize clears
/// the row in the same transaction that moves head, so a `CSTAGE` row at or below head is a row
/// whose commit already landed and whose clear is simply not visible to this snapshot yet. Those are
/// skipped rather than cleared, because clearing one would refund quota for bytes that are now live
/// commit data.
///
/// The grace window is the other half: a live staged commit is written across many transactions, and
/// clearing one mid-write would destroy a commit still in progress.
pub(crate) async fn scan_orphan_commit_stages(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	head_txid: u64,
	now_ms: i64,
	limit: usize,
) -> Result<Vec<u64>> {
	let rows =
		tx_scan_prefix_values(tx, &keys::branch_commit_stage_prefix(branch_id), Snapshot).await?;

	let mut orphans = Vec::new();
	for (key, value) in rows {
		if orphans.len() >= limit {
			break;
		}
		let txid = keys::decode_branch_commit_stage_txid(branch_id, &key)?;
		if txid <= head_txid {
			continue;
		}
		let stage = crate::conveyer::types::decode_commit_stage_row(&value)
			.context("decode sqlite commit stage row for orphan scan")?;
		if now_ms.saturating_sub(stage.started_at_ms)
			< crate::conveyer::constants::COMMIT_STAGE_ORPHAN_GRACE_MS
		{
			continue;
		}
		orphans.push(txid);
	}

	Ok(orphans)
}

/// A job-id subspace found resident under a branch's `CMP/stage/` prefix.
pub(crate) struct StagedJobSubspace {
	pub(crate) job_id: Id,
	pub(crate) lane: keys::StagedJobLane,
}

/// Enumerates up to `limit` job-id subspaces holding staging on this branch, skipping `active`.
///
/// Reads one row per subspace, not the subspace itself: after finding a job's first staged row the
/// scan jumps straight past that job's whole prefix. A branch with no staging therefore costs a
/// single empty range read, and a branch mid-drain costs one read plus one skip. Enumerating the
/// rows instead would read the staged shard blobs, which are the largest values depot writes.
pub(crate) async fn scan_staged_job_subspaces(
	tx: &universaldb::Transaction,
	database_branch_id: DatabaseBranchId,
	active: &[Id],
	limit: usize,
) -> Result<Vec<StagedJobSubspace>> {
	let prefix = keys::branch_compaction_stage_prefix(database_branch_id);
	let (mut scan_begin, scan_end) =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix.clone()))
			.range();
	let mut found = Vec::new();
	// Skipped subspaces do not consume `limit`, so bound the walk by how many skips are legitimate:
	// one per job whose cleanup is already accounted for. Anything beyond that is reported next
	// refresh instead of growing this transaction.
	let mut remaining_reads = limit.saturating_add(active.len());

	while found.len() < limit && remaining_reads > 0 {
		remaining_reads -= 1;
		let Some((key, _value)) = tx_get_range_first(tx, &scan_begin, &scan_end, Snapshot).await?
		else {
			break;
		};
		let Some((job_id, lane)) =
			keys::decode_branch_compaction_stage_job(database_branch_id, &key)
		else {
			// An unrecognized key shape under the staging prefix is not something to guess at, and
			// stepping one key at a time through it would be unbounded. Stop the scan and let the
			// next refresh try again.
			tracing::warn!(
				?database_branch_id,
				"unrecognized key under compaction staging prefix"
			);
			break;
		};

		// Resume past this job's whole subspace whether or not it is reported, so one skip per job
		// keeps the walk bounded by job count rather than by staged rows.
		let job_prefix = keys::branch_compaction_stage_job_prefix(database_branch_id, job_id);
		let (_, job_end) =
			universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(job_prefix))
				.range();
		scan_begin = job_end;

		if !active.contains(&job_id) {
			found.push(StagedJobSubspace { job_id, lane });
		}
	}

	Ok(found)
}

/// Decodes a window of DELTA rows into one logical delta per txid.
///
/// A segmented commit stores its pages across several self-contained blobs, but every consumer here
/// reasons about a txid's whole page set (which shards it touched, which of its pages PIDX still
/// owns), so the segments are merged back into one value. Segments are ascending and disjoint, so
/// concatenating their pages in key order keeps `pages` sorted by page number, which is what
/// `DecodedLtx::get_page` binary-searches on.
///
/// The merged value's `page_index` is cleared: its offsets address one backing blob and a merged
/// delta has none. Read `pages`.
/// One shard-aligned page range of a commit that a hot slice may admit on its own.
pub(crate) struct HotAdmissionUnit {
	pub(crate) first_pgno: u32,
	pub(crate) pgnos: Vec<u32>,
}

/// The page ranges of one commit a slice may admit, ascending, starting at `min_first_pgno`.
///
/// A segmented commit already stores its pages as shard-aligned blobs, so each blob is one unit and
/// a slice may take part of the commit.
///
/// A pre-segmentation commit yields exactly one unit covering everything it wrote, so a slice takes
/// it whole or defers it. Cutting one into shard-aligned units instead would let a slice stop
/// mid-commit, and the cursor that records where it stopped is a page number: the resume scan then
/// begins at `branch_delta_segment_prefix`, which is `prefix + pgno + '/'`, while a legacy row is
/// `prefix + chunk_idx`. For any resume page at or above one shard, the page number sorts strictly
/// above every chunk index, so the resume scan reads nothing at all. The commit is then admitted as
/// pure coverage, its remaining pages are never folded, and their PIDX rows keep the delta from ever
/// being reclaimed. Deferring costs a slice; cutting loses the pages.
pub(crate) fn hot_admission_units(
	branch_id: DatabaseBranchId,
	txid: u64,
	delta_chunks: &[(Vec<u8>, Vec<u8>)],
	min_first_pgno: Option<u32>,
) -> Result<Vec<HotAdmissionUnit>> {
	let segments = delta_blob::reassemble_delta_segments(branch_id, txid, delta_chunks.to_vec())?;
	let mut units = Vec::new();
	for segment in &segments {
		let decoded = decode_ltx_v3(&segment.blob)
			.with_context(|| format!("decode hot delta {txid} segment {:?}", segment.first_pgno))?;
		match segment.first_pgno {
			Some(first_pgno) => units.push(HotAdmissionUnit {
				first_pgno,
				pgnos: decoded.pages.iter().map(|page| page.pgno).collect(),
			}),
			None => units.push(HotAdmissionUnit {
				first_pgno: delta_blob::segment_first_pgno(&decoded.pages)?,
				pgnos: decoded.pages.iter().map(|page| page.pgno).collect(),
			}),
		}
	}

	if let Some(min_first_pgno) = min_first_pgno {
		units.retain(|unit| unit.first_pgno >= min_first_pgno);
	}

	Ok(units)
}

/// Drops the rows of the last delta blob in a limit-truncated scan.
///
/// A scan that filled its row limit may have stopped partway through a blob. Those rows cannot be
/// reassembled (the blob is short its own chunks) and admitting them would fold a delta missing
/// pages, so they are discarded and the next slice reads that blob whole.
fn drop_trailing_partial_delta_blob(
	branch_id: DatabaseBranchId,
	txid: u64,
	mut rows: Vec<(Vec<u8>, Vec<u8>)>,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let Some((last_key, _)) = rows.last() else {
		return Ok(rows);
	};
	let last_first_pgno =
		keys::decode_branch_delta_chunk_ref(branch_id, txid, last_key)?.first_pgno();
	// A legacy commit has exactly one blob, so dropping it leaves nothing and the commit is deferred
	// whole. That is the correct outcome: its blob does not fit this slice's remaining budget.
	rows.retain(|(key, _)| {
		keys::decode_branch_delta_chunk_ref(branch_id, txid, key)
			.map(|chunk_ref| chunk_ref.first_pgno() != last_first_pgno)
			.unwrap_or(false)
	});

	Ok(rows)
}

pub(crate) fn decode_hot_delta_chunks(
	branch_id: DatabaseBranchId,
	delta_chunks: &[(Vec<u8>, Vec<u8>)],
) -> Result<BTreeMap<u64, DecodedLtx>> {
	delta_blob::reassemble_delta_segments_by_txid(branch_id, delta_chunks)?
		.into_iter()
		.filter(|(_, segments)| !segments.is_empty())
		.map(|(txid, segments)| {
			let mut decoded = segments
				.iter()
				.map(|segment| {
					decode_ltx_v3(&segment.blob).with_context(|| {
						format!("decode hot delta {txid} segment {:?}", segment.first_pgno)
					})
				})
				.collect::<Result<Vec<_>>>()?;

			let merged = if decoded.len() == 1 {
				decoded.remove(0)
			} else {
				let mut merged = decoded.remove(0);
				merged.page_index.clear();
				for rest in decoded {
					merged.pages.extend(rest.pages);
				}
				merged
			};

			Ok((txid, merged))
		})
		.collect()
}

/// The database size in pages as of `as_of_txid`, which is the size recorded by the newest commit
/// at or below it. `entries` must be sorted ascending by txid.
pub(crate) fn db_size_pages_at_txid(entries: &[(u64, u32)], as_of_txid: u64) -> Option<u32> {
	entries
		.iter()
		.rev()
		.find_map(|(txid, db_size_pages)| (*txid <= as_of_txid).then_some(*db_size_pages))
}

/// The pages each shard holds at `as_of_txid`, folded from every delta at or below it.
///
/// `max_pgno_exclusive` bounds the fold to the pages the slice admitted from `as_of_txid` itself. A
/// pre-segmentation commit is read whole even when only part of it is admitted, so without the bound
/// the fold would write images for shards the slice never reserved PIDX budget for. Those images
/// would be complete and harmless, but the PIDX rows behind them would not be cleared, and a page
/// whose owner survives its fold pins its delta against reclaim permanently.
///
/// The bound applies only to `as_of_txid`'s own pages. Deltas below it are history this image must
/// carry in full regardless of where the slice cut.
pub(crate) fn collect_hot_pages_by_shard(
	db_size_pages: u32,
	deltas: &BTreeMap<u64, DecodedLtx>,
	as_of_txid: u64,
	max_pgno_exclusive: Option<u32>,
) -> Result<BTreeMap<u32, Vec<(u32, Vec<u8>)>>> {
	let mut pages_by_number = BTreeMap::<u32, Vec<u8>>::new();

	for (txid, delta) in deltas {
		if *txid > as_of_txid {
			continue;
		}
		let page_bound = (*txid == as_of_txid)
			.then_some(max_pgno_exclusive)
			.flatten();
		for page in &delta.pages {
			if page.pgno <= db_size_pages && page_bound.is_none_or(|bound| page.pgno < bound) {
				pages_by_number.insert(page.pgno, page.bytes.clone());
			}
		}
	}

	let mut pages_by_shard = BTreeMap::<u32, Vec<(u32, Vec<u8>)>>::new();
	for (pgno, bytes) in pages_by_number {
		pages_by_shard
			.entry(pgno / keys::SHARD_SIZE)
			.or_default()
			.push((pgno, bytes));
	}
	Ok(pages_by_shard)
}

pub(crate) async fn build_staged_hot_shard_blob(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	job_id: Id,
	shard_id: u32,
	as_of_txid: u64,
	page_updates: Vec<(u32, Vec<u8>)>,
) -> Result<StagedHotShardBlob> {
	let existing_blob =
		load_merge_base_shard_blob(tx, branch_id, job_id, shard_id, as_of_txid).await?;
	let mut merged_pages = BTreeMap::<u32, Vec<u8>>::new();
	let mut timestamp_ms = 0;

	if let Some(existing_blob) = existing_blob {
		let decoded = decode_ltx_v3(&existing_blob).context("decode existing branch shard blob")?;
		timestamp_ms = decoded.header.timestamp_ms;
		for page in decoded.pages {
			if page.pgno / keys::SHARD_SIZE == shard_id {
				ensure!(
					page.bytes.len() == keys::PAGE_SIZE as usize,
					"page {} had {} bytes, expected {}",
					page.pgno,
					page.bytes.len(),
					keys::PAGE_SIZE
				);
				merged_pages.insert(page.pgno, page.bytes);
			}
		}
	}

	for (pgno, bytes) in page_updates {
		ensure!(pgno > 0, "page number must be greater than zero");
		ensure!(
			pgno / keys::SHARD_SIZE == shard_id,
			"page {} does not belong to shard {}",
			pgno,
			shard_id
		);
		ensure!(
			bytes.len() == keys::PAGE_SIZE as usize,
			"page {} had {} bytes, expected {}",
			pgno,
			bytes.len(),
			keys::PAGE_SIZE
		);
		merged_pages.insert(pgno, bytes);
	}

	let pages = merged_pages
		.into_iter()
		.map(|(pgno, bytes)| DirtyPage { pgno, bytes })
		.collect::<Vec<_>>();
	let commit = pages.iter().map(|page| page.pgno).max().unwrap_or(1);
	let header = LtxHeader::delta(as_of_txid, commit, timestamp_ms);

	Ok(StagedHotShardBlob::Encoded(
		encode_ltx_v3(header, &pages).context("encode staged hot shard blob")?,
	))
}

/// Picks the merge base for a fold: the newest complete shard image at or below `as_of_txid`,
/// whether it is already installed under `SHARD` or still sitting in this job's staging area.
///
/// The staging area has to be consulted because the companion drains every slice of a job before the
/// manager installs any of them. An earlier slice's image for the same shard is therefore not
/// installed yet while a later slice folds, and using only `SHARD` would fold that shard's untouched
/// pages away. Every written version must stay a complete image of its shard: the read path resolves
/// a page from exactly one shard version and zero-fills whatever that version is missing.
///
/// Both sources live in FDB, so neither sees a version shard-cache eviction has demoted to S3. A
/// shard whose newest image is cold-only therefore reads here as "no base" (or as a stale older
/// version), and folding on that would encode an image holding only the drain's own pages. Since that
/// image is newer than the cold ref, the read path would then select it over the cold image and
/// zero-fill every page the fold dropped. Fail closed when a cold ref outranks what the hot tier
/// holds: the fold has no complete base to merge onto, so the slice must not be staged at all.
async fn load_merge_base_shard_blob(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	job_id: Id,
	shard_id: u32,
	as_of_txid: u64,
) -> Result<Option<Vec<u8>>> {
	let installed = load_latest_branch_shard_blob(tx, branch_id, shard_id, as_of_txid).await?;
	let staged =
		load_latest_staged_hot_shard_blob(tx, branch_id, job_id, shard_id, as_of_txid).await?;

	let merge_base = match (installed, staged) {
		(Some((installed_txid, installed_blob)), Some((staged_txid, staged_blob))) => {
			if staged_txid >= installed_txid {
				Some((staged_txid, staged_blob))
			} else {
				Some((installed_txid, installed_blob))
			}
		}
		(Some(version), None) | (None, Some(version)) => Some(version),
		(None, None) => None,
	};

	Ok(merge_base.map(|(_, blob)| blob))
}

/// The outcome of folding one shard image.
pub(crate) enum StagedHotShardBlob {
	Encoded(Vec<u8>),
}

pub(crate) async fn load_latest_branch_shard_blob(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	shard_id: u32,
	as_of_txid: u64,
) -> Result<Option<(u64, Vec<u8>)>> {
	let load =
		shard_blob::read_latest_shard_blob(tx, branch_id, shard_id, as_of_txid, Snapshot).await?;
	Ok(load
		.version
		.map(|(version_txid, version)| (version_txid, version.blob)))
}

/// Loads the newest shard version this job has already staged at or below `as_of_txid`. One reverse
/// single-row read finds the version, then its own chunk range is read forward and reassembled, so
/// the scan is bounded by one shard image rather than the job's whole staging area.
async fn load_latest_staged_hot_shard_blob(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	job_id: Id,
	shard_id: u32,
	as_of_txid: u64,
) -> Result<Option<(u64, Vec<u8>)>> {
	let version_prefix =
		keys::branch_compaction_stage_hot_shard_version_prefix(branch_id, job_id, shard_id);
	let (_, scan_end) =
		keys::branch_compaction_stage_hot_shard_txid_range(branch_id, job_id, shard_id, as_of_txid);
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::Iterator,
			limit: Some(1),
			reverse: true,
			..(version_prefix.as_slice(), scan_end.as_slice()).into()
		},
		Snapshot,
	);

	let Some(entry) = stream.try_next().await? else {
		return Ok(None);
	};
	let version_txid = decode_staged_hot_shard_row_txid(&version_prefix, entry.key())?;
	drop(stream);

	let chunk_prefix = keys::branch_compaction_stage_hot_shard_txid_prefix(
		branch_id,
		job_id,
		shard_id,
		version_txid,
	);
	let rows = tx_scan_prefix_values(tx, &chunk_prefix, Snapshot).await?;
	let blob = shard_blob::assemble_chunked_rows(&chunk_prefix, &rows)
		.context("assemble staged hot shard merge base")?;

	Ok(Some((version_txid, blob)))
}

/// Decodes the `as_of_txid` of a staged hot shard chunk row, whose key is the shard's version
/// prefix followed by the big-endian txid, a separator, and the big-endian chunk index.
fn decode_staged_hot_shard_row_txid(version_prefix: &[u8], key: &[u8]) -> Result<u64> {
	let suffix = key
		.strip_prefix(version_prefix)
		.context("staged hot shard key did not start with its version prefix")?;
	let txid_bytes: [u8; std::mem::size_of::<u64>()] = suffix
		.get(..std::mem::size_of::<u64>())
		.context("staged hot shard key was too short to hold a txid")?
		.try_into()
		.map_err(|_| anyhow::anyhow!("staged hot shard key had an invalid txid segment"))?;

	Ok(u64::from_be_bytes(txid_bytes))
}

pub(crate) fn decode_branch_pidx_pgno(branch_id: DatabaseBranchId, key: &[u8]) -> Result<u32> {
	let prefix = keys::branch_pidx_prefix(branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("branch PIDX key did not start with expected prefix")?;
	let bytes: [u8; std::mem::size_of::<u32>()] = suffix
		.try_into()
		.map_err(|_| anyhow::anyhow!("branch PIDX key suffix had invalid length"))?;

	Ok(u32::from_be_bytes(bytes))
}

pub(crate) fn content_hash(bytes: &[u8]) -> [u8; 32] {
	let digest = Sha256::digest(bytes);
	let mut hash = [0_u8; 32];
	hash.copy_from_slice(&digest);
	hash
}

pub(crate) fn hex_lower(bytes: &[u8]) -> String {
	const HEX: &[u8; 16] = b"0123456789abcdef";
	let mut out = String::with_capacity(bytes.len() * 2);
	for byte in bytes {
		out.push(HEX[(byte >> 4) as usize] as char);
		out.push(HEX[(byte & 0x0f) as usize] as char);
	}
	out
}

pub(crate) fn decode_branch_commit_txid(branch_id: DatabaseBranchId, key: &[u8]) -> Result<u64> {
	let prefix = keys::branch_commit_prefix(branch_id);
	let suffix = key
		.strip_prefix(prefix.as_slice())
		.context("branch commit key did not start with expected prefix")?;
	let bytes: [u8; std::mem::size_of::<u64>()] = suffix
		.try_into()
		.map_err(|_| anyhow::anyhow!("branch commit key suffix had invalid length"))?;

	Ok(u64::from_be_bytes(bytes))
}

/// Width of a PIDX value, a raw big-endian `u64` txid. Hot planning reserves budget per PIDX row
/// before it has read the row, so it needs the width up front.
pub(crate) const PIDX_VALUE_BYTES: u64 = std::mem::size_of::<u64>() as u64;

pub(crate) fn decode_pidx_txid(value: &[u8]) -> Result<u64> {
	let bytes: [u8; std::mem::size_of::<u64>()] = value
		.try_into()
		.map_err(|_| anyhow::anyhow!("branch pidx value had invalid length"))?;

	Ok(u64::from_be_bytes(bytes))
}

pub(crate) async fn tx_get_value(
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

pub(crate) async fn tx_scan_prefix_values(
	tx: &universaldb::Transaction,
	prefix: &[u8],
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let prefix_subspace =
		universaldb::Subspace::from(universaldb::tuple::Subspace::from_bytes(prefix.to_vec()));
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..RangeOption::from(&prefix_subspace)
		},
		isolation_level,
	);
	let mut rows = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		rows.push((entry.key().to_vec(), entry.value().to_vec()));
	}

	#[cfg(feature = "test-faults")]
	crate::compaction::test_hooks::scan_probe::record(
		crate::compaction::test_hooks::scan_probe::SCAN_PREFIX,
		rows.len() as u64,
	);

	Ok(rows)
}

pub(crate) async fn tx_scan_range_values(
	tx: &universaldb::Transaction,
	start: &[u8],
	end: &[u8],
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..(start, end).into()
		},
		isolation_level,
	);
	let mut rows = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		rows.push((entry.key().to_vec(), entry.value().to_vec()));
	}

	#[cfg(feature = "test-faults")]
	crate::compaction::test_hooks::scan_probe::record(
		crate::compaction::test_hooks::scan_probe::SCAN_RANGE,
		rows.len() as u64,
	);

	Ok(rows)
}

/// Like `tx_scan_range_values` but caps the FDB read at `limit` rows. Used by the hot and reclaim
/// commit-range scans, whose loops already stop adding at `CompactionBatchBudget::fdb()` (at most
/// `CMP_FDB_BATCH_MAX_KEYS` keys). Passing that same cap as the read limit keeps the transaction from
/// materializing the entire commit backlog (which can age out the FDB transaction) while reading
/// every row the budget could actually consume. Rows past the limit are exactly the rows the budget
/// would have discarded, so this is behavior-preserving for those callers.
/// Whether a commit's delta is stored as shard-aligned segments rather than one legacy blob.
///
/// Decided from the commit's first delta row, since both layouts share a prefix and are told apart
/// only by their key suffix. Reads one key.
async fn commit_is_segmented(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	txid: u64,
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<bool> {
	let rows = tx_scan_range_values_limited(
		tx,
		&keys::branch_delta_chunk_prefix(branch_id, txid),
		&keys::branch_delta_txid_scan_end(branch_id, txid),
		1,
		isolation_level,
	)
	.await?;
	let Some((key, _)) = rows.first() else {
		// No delta rows at all, so there is nothing to resume into either way.
		return Ok(false);
	};

	Ok(keys::decode_branch_delta_chunk_ref(branch_id, txid, key)?
		.first_pgno()
		.is_some())
}

pub(crate) async fn tx_scan_range_values_limited(
	tx: &universaldb::Transaction,
	start: &[u8],
	end: &[u8],
	limit: usize,
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			limit: Some(limit),
			..(start, end).into()
		},
		isolation_level,
	);
	let mut rows = Vec::new();

	while let Some(entry) = stream.try_next().await? {
		rows.push((entry.key().to_vec(), entry.value().to_vec()));
	}

	#[cfg(feature = "test-faults")]
	crate::compaction::test_hooks::scan_probe::record(
		crate::compaction::test_hooks::scan_probe::SCAN_RANGE_LIMITED,
		rows.len() as u64,
	);

	Ok(rows)
}

/// Reads the last key/value in `[start, end)`, or `None` if the range is empty.
///
/// A descending scan capped at one row, so the read is one key rather than the whole range. The
/// ascending equivalent cannot do this: finding the newest row by scanning forward means reading
/// every older row first, which is what made resolving one fork pin cost a branch's entire commit
/// history.
pub(crate) async fn tx_get_range_last(
	tx: &universaldb::Transaction,
	start: &[u8],
	end: &[u8],
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::Iterator,
			limit: Some(1),
			reverse: true,
			..(start, end).into()
		},
		isolation_level,
	);

	let last = stream.try_next().await?;

	#[cfg(feature = "test-faults")]
	crate::compaction::test_hooks::scan_probe::record(
		crate::compaction::test_hooks::scan_probe::GET_RANGE_LAST,
		last.is_some() as u64,
	);

	match last {
		Some(entry) => Ok(Some((entry.key().to_vec(), entry.value().to_vec()))),
		None => Ok(None),
	}
}

/// Reads the first key/value in `[start, end)`, or `None` if the range is empty. Used to select the
/// oldest fold past the cold cursor with a single small read instead of a full prefix scan.
pub(crate) async fn tx_get_range_first(
	tx: &universaldb::Transaction,
	start: &[u8],
	end: &[u8],
	isolation_level: universaldb::utils::IsolationLevel,
) -> Result<Option<(Vec<u8>, Vec<u8>)>> {
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::Iterator,
			limit: Some(1),
			..(start, end).into()
		},
		isolation_level,
	);

	let first = stream.try_next().await?;

	#[cfg(feature = "test-faults")]
	crate::compaction::test_hooks::scan_probe::record(
		crate::compaction::test_hooks::scan_probe::GET_RANGE_FIRST,
		first.is_some() as u64,
	);

	match first {
		Some(entry) => Ok(Some((entry.key().to_vec(), entry.value().to_vec()))),
		None => Ok(None),
	}
}

/// Deterministic percentage-based admission for compaction on a database branch. Hashes the branch
/// id to a stable bucket in `[0.0, 1.0)` and admits when it falls under the configured fraction, so a
/// fixed subset of branches compacts at any given percent. Branch ids are v4 UUIDs, so using the full
/// 128-bit value keeps the mapping uniform.
///
/// `admission_fraction` is expected pre-clamped to `[0.0, 1.0]` by `compaction_admission_fraction()`.
/// The bucket is always in `[0.0, 0.9999]`, so `< 1.0` admits everything and `< 0.0` admits nothing;
/// no extra branches for the extremes are needed.
pub(crate) fn compaction_admitted(
	admission_fraction: f64,
	database_branch_id: DatabaseBranchId,
) -> bool {
	let bucket = (database_branch_id.as_uuid().as_u128() % 10_000) as f64 / 10_000.0;
	bucket < admission_fraction
}

/// Whether the branch is admitted for the FDB-only lanes (hot and reclaim) under the percent
/// currently in effect.
///
/// Reads the dynamic config, so this may only be called from an activity. Calling it from a workflow
/// body would make the decision non-deterministic and diverge the workflow on replay.
pub(crate) fn branch_admitted_now(
	config: &rivet_config::Config,
	database_branch_id: DatabaseBranchId,
) -> bool {
	compaction_admitted(
		config.dynamic().sqlite().compaction_admission_fraction(),
		database_branch_id,
	)
}

#[cfg(test)]
#[path = "../../tests/inline/compaction_hot_fold_bounds.rs"]
mod hot_fold_bounds_tests;
