use super::*;

pub(crate) async fn read_manager_fdb_snapshot(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	shard_gc_cursor: &[u8],
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
		resolve_bucket_fork_pins(tx, branch_id, &mut db_pins, now_ms).await?;
	// Planning must use the same effective policy that stage and install
	// validate against, or every coverage selection mismatches and hot jobs
	// reject forever. The scope is denormalized on the branch record, so this
	// is two point reads.
	let pitr_policy = read_effective_pitr_policy_for_branch(tx, branch_record.as_ref()).await?;
	let hot_inputs = read_hot_input_snapshot(
		tx,
		branch_id,
		head.as_ref(),
		&root,
		Snapshot,
		pitr_policy,
		now_ms,
	)
	.await?;
	let reclaim_inputs = read_reclaim_input_snapshot(
		tx,
		branch_id,
		&root,
		&db_pins,
		bucket_proof_blocked_reclaim,
		shard_gc_cursor,
		Snapshot,
		now_ms,
	)
	.await?;
	let hot_lag = head.as_ref().map_or(0, |head| {
		head.head_txid.saturating_sub(root.hot_watermark_txid)
	});
	let has_actionable_lag =
		hot_lag >= quota::COMPACTION_DELTA_THRESHOLD || reclaim_snapshot_has_work(&reclaim_inputs);
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
	now_ms: i64,
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
		if resolve_bucket_catalog_forks(tx, branch_id, db_pins, &catalog_fact, now_ms).await? {
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
	now_ms: i64,
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
			if materialize_bucket_fork_pin(tx, branch_id, db_pins, &child_fact, now_ms).await? {
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
	now_ms: i64,
) -> Result<bool> {
	// An already-materialized pin is authoritative. Re-snapping would move the
	// fork point as interval rows shift or expire.
	if db_pins.iter().any(|pin| {
		pin.kind == crate::types::DbHistoryPinKind::BucketFork
			&& pin.owner_bucket_branch_id == Some(fork_fact.target_bucket_branch_id)
			&& pin.at_versionstamp <= fork_fact.fork_versionstamp
	}) {
		return Ok(false);
	}

	let watermark_txid = tx_get_value(
		tx,
		&keys::branch_compaction_root_key(branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root for bucket fork pin")?
	.map(|root| root.hot_watermark_txid)
	.unwrap_or(0);

	// Hybrid resolution: when the latest commit at or before the fork point has
	// not been folded yet, pin it exactly; the pin feeds coverage staging at the
	// next install. Commits at or above the watermark are never reclaimed, so
	// the VTX scan result is trustworthy in that range despite keep-set holes
	// below it.
	if let Some((at_txid, at_versionstamp, commit)) =
		latest_commit_at_or_before_versionstamp(tx, branch_id, fork_fact.fork_versionstamp).await?
		&& at_txid >= watermark_txid
	{
		write_materialized_bucket_fork_pin(
			tx,
			branch_id,
			db_pins,
			fork_fact,
			at_txid,
			at_versionstamp,
			commit.wall_clock_ms,
		)?;
		return Ok(false);
	}

	// Historical fork: snap down to the newest covered point at or before the
	// fork versionstamp, drawn from retained PITR interval representatives and
	// existing pins. The fence on pin and fork creation guarantees those points
	// have shard coverage.
	let mut best: Option<(u64, [u8; 16], i64)> = None;
	for (_, coverage) in
		crate::conveyer::pitr_interval::scan_pitr_interval_coverage(tx, branch_id, Serializable)
			.await?
	{
		// Expired rows are about to lose their commit islands to reclaim, so
		// they are not deterministic snap targets even while still present.
		if coverage.expires_at_ms <= now_ms {
			continue;
		}
		if coverage.versionstamp <= fork_fact.fork_versionstamp
			&& best.map_or(true, |(best_txid, _, _)| coverage.txid > best_txid)
		{
			best = Some((coverage.txid, coverage.versionstamp, coverage.wall_clock_ms));
		}
	}
	for pin in db_pins.iter() {
		if pin.at_versionstamp <= fork_fact.fork_versionstamp
			&& best.map_or(true, |(best_txid, _, _)| pin.at_txid > best_txid)
		{
			best = Some((pin.at_txid, pin.at_versionstamp, pin.created_at_ms));
		}
	}

	let Some((at_txid, at_versionstamp, wall_clock_ms)) = best else {
		tracing::warn!(
			?branch_id,
			?fork_fact,
			"retaining sqlite commit history because bucket fork target predates retained coverage"
		);
		return Ok(true);
	};
	write_materialized_bucket_fork_pin(
		tx,
		branch_id,
		db_pins,
		fork_fact,
		at_txid,
		at_versionstamp,
		wall_clock_ms,
	)?;

	Ok(false)
}

fn write_materialized_bucket_fork_pin(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	db_pins: &mut Vec<DbHistoryPin>,
	fork_fact: &BucketForkFact,
	at_txid: u64,
	at_versionstamp: [u8; 16],
	wall_clock_ms: i64,
) -> Result<()> {
	history_pin::write_bucket_fork_pin(
		tx,
		branch_id,
		fork_fact.target_bucket_branch_id,
		at_versionstamp,
		at_txid,
		wall_clock_ms,
	)?;
	db_pins.retain(|pin| pin.owner_bucket_branch_id != Some(fork_fact.target_bucket_branch_id));
	db_pins.push(DbHistoryPin {
		at_versionstamp,
		at_txid,
		kind: crate::types::DbHistoryPinKind::BucketFork,
		owner_database_branch_id: None,
		owner_bucket_branch_id: Some(fork_fact.target_bucket_branch_id),
		owner_restore_point: None,
		created_at_ms: wall_clock_ms,
	});

	Ok(())
}

pub(crate) async fn read_effective_pitr_policy_for_branch(
	tx: &universaldb::Transaction,
	branch_record: Option<&DatabaseBranchRecord>,
) -> Result<PitrPolicy> {
	let Some(branch_record) = branch_record else {
		return Ok(PitrPolicy::default());
	};
	// The policy scope is denormalized onto the branch record at creation so
	// this lookup never scans global pointer indexes. Records predating the
	// scope fields fall back to the default policy.
	let (Some(bucket_id), Some(database_id)) = (
		branch_record.policy_bucket_id,
		branch_record.policy_database_id.as_deref(),
	) else {
		return Ok(PitrPolicy::default());
	};

	if let Some(policy) = tx_get_value(
		tx,
		&keys::database_pitr_policy_key(bucket_id, database_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_pitr_policy)
	.transpose()
	.context("decode sqlite database PITR policy for compaction manager")?
	{
		return Ok(policy);
	}

	tx_get_value(tx, &keys::bucket_policy_pitr_key(bucket_id), Serializable)
		.await?
		.as_deref()
		.map(decode_pitr_policy)
		.transpose()
		.context("decode sqlite bucket PITR policy for compaction manager")
		.map(|policy| policy.unwrap_or_default())
}

pub(crate) async fn latest_commit_at_or_before_versionstamp(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	versionstamp_cap: [u8; 16],
) -> Result<Option<(u64, [u8; 16], CommitRow)>> {
	// Reverse limit-1 scan: the newest surviving VTX row at or below the cap,
	// independent of how many older rows survive. A forward scan with a row cap
	// would structurally miss the head once a branch retains more VTX islands
	// than the cap.
	let prefix = keys::branch_vtx_prefix(branch_id);
	let end = end_of_key_range(&keys::branch_vtx_key(branch_id, versionstamp_cap));
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::Iterator,
			reverse: true,
			limit: Some(1),
			..(prefix.as_slice(), end.as_slice()).into()
		},
		Serializable,
	);
	let Some(entry) = stream.try_next().await? else {
		return Ok(None);
	};
	let versionstamp = decode_branch_vtx_versionstamp(branch_id, entry.key())?;
	let txid = decode_txid_value(entry.value())?;

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

/// Maximum scan pages per row family per reclaim snapshot. Pages are capped at
/// CMP_FDB_BATCH_MAX_KEYS rows, so this bounds rows read per family while
/// letting the scan window slide past dense runs of non-deletable rows.
const CMP_SCAN_MAX_PAGES: usize = 4;

/// Shard GC pages are small because every row carries a full shard blob.
const SHARD_GC_SCAN_PAGE_ROWS: usize = 32;

/// Shard GC stops pulling pages once this many blob bytes were scanned.
const SHARD_GC_SCAN_MAX_VALUE_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
struct CompactionBatchBudget {
	max_keys: usize,
	max_value_bytes: u64,
	key_count: usize,
	value_bytes: u64,
}

impl CompactionBatchBudget {
	fn fdb() -> Self {
		CompactionBatchBudget {
			max_keys: CMP_FDB_BATCH_MAX_KEYS,
			max_value_bytes: CMP_FDB_BATCH_MAX_VALUE_BYTES as u64,
			key_count: 0,
			value_bytes: 0,
		}
	}

	fn can_add(&self, row_count: usize, value_bytes: u64) -> bool {
		self.key_count.saturating_add(row_count) <= self.max_keys
			&& self.value_bytes.saturating_add(value_bytes) <= self.max_value_bytes
	}

	fn add(&mut self, row_count: usize, value_bytes: u64) {
		self.key_count = self.key_count.saturating_add(row_count);
		self.value_bytes = self.value_bytes.saturating_add(value_bytes);
	}

	fn value_bytes(&self) -> u64 {
		self.value_bytes
	}
}

pub(crate) async fn read_hot_input_snapshot(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	head: Option<&DBHead>,
	root: &CompactionRoot,
	isolation_level: universaldb::utils::IsolationLevel,
	pitr_policy: PitrPolicy,
	now_ms: i64,
) -> Result<HotInputSnapshot> {
	let Some(head) = head else {
		return Ok(HotInputSnapshot::default());
	};
	if head.head_txid <= root.hot_watermark_txid {
		return Ok(HotInputSnapshot::default());
	}

	let min_txid = root.hot_watermark_txid.saturating_add(1);
	let max_txid = head.head_txid;
	let mut snapshot = HotInputSnapshot::default();
	let mut budget = CompactionBatchBudget::fdb();

	let commit_scan_start = keys::branch_commit_key(branch_id, min_txid);
	let commit_scan_end = max_txid
		.checked_add(1)
		.map(|next_txid| keys::branch_commit_key(branch_id, next_txid))
		.unwrap_or_else(|| end_of_key_range(&keys::branch_commit_prefix(branch_id)));
	for (key, value) in
		tx_scan_range_values(tx, &commit_scan_start, &commit_scan_end, isolation_level).await?
	{
		let txid = decode_branch_commit_txid(branch_id, &key)?;
		if txid > max_txid {
			break;
		}
		let commit =
			decode_commit_row(&value).context("decode sqlite commit row for hot planning")?;
		let delta_chunks = tx_scan_prefix_values(
			tx,
			&keys::branch_delta_chunk_prefix(branch_id, txid),
			isolation_level,
		)
		.await?;
		let txid_value_bytes = u64::try_from(value.len())
			.unwrap_or(u64::MAX)
			.saturating_add(
				delta_chunks
					.iter()
					.map(|(_, value)| u64::try_from(value.len()).unwrap_or(u64::MAX))
					.fold(0_u64, u64::saturating_add),
			);

		if !budget.can_add(1 + delta_chunks.len(), txid_value_bytes) {
			break;
		}

		budget.add(1 + delta_chunks.len(), txid_value_bytes);
		snapshot.commits.push((txid, commit));
		snapshot.delta_chunks.extend(delta_chunks);
		snapshot.selected_max_txid = Some(txid);
	}

	let Some(selected_max_txid) = snapshot.selected_max_txid else {
		return Ok(snapshot);
	};

	for (key, value) in
		tx_scan_prefix_values(tx, &keys::branch_pidx_prefix(branch_id), isolation_level).await?
	{
		if !budget.can_add(1, u64::try_from(value.len()).unwrap_or(u64::MAX)) {
			break;
		}

		// An undecodable PIDX value must be skipped fail-closed: including it
		// poisons the install activity, which decodes every selected row, into
		// a permanent failure loop.
		let Ok(txid) = decode_pidx_txid(&value) else {
			tracing::error!(
				?branch_id,
				pgno = ?decode_branch_pidx_pgno(branch_id, &key).ok(),
				"skipping undecodable sqlite PIDX row during hot planning"
			);
			continue;
		};
		if txid < min_txid || txid > selected_max_txid {
			continue;
		}
		budget.add(1, u64::try_from(value.len()).unwrap_or(u64::MAX));
		snapshot.pidx_entries.push((key, value));
	}
	snapshot.total_value_bytes = budget.value_bytes();

	snapshot.pitr_interval_coverage =
		select_pitr_interval_coverage(&pitr_policy, &snapshot.commits, now_ms)?;

	Ok(snapshot)
}

pub(crate) fn select_pitr_interval_coverage(
	policy: &PitrPolicy,
	commits: &[(u64, CommitRow)],
	now_ms: i64,
) -> Result<Vec<PitrIntervalSelection>> {
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
		// Commits stamped in the future by clock skew are selected as-is:
		// dropping them leaves their txids permanently unreachable for
		// timestamp forks once the batch folds, and selecting by the commit's
		// own wall clock keeps plan, stage, and install deterministic.
		if commit.wall_clock_ms < retention_floor_ms {
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
	commit_deletes_blocked: bool,
	shard_gc_cursor: &[u8],
	isolation_level: universaldb::utils::IsolationLevel,
	now_ms: i64,
) -> Result<ReclaimInputSnapshot> {
	let (mut pitr_interval_retention, mut expired_pitr_interval_rows) =
		read_pitr_interval_reclaim_rows(tx, branch_id, now_ms, isolation_level).await?;
	let has_retained_pitr_intervals = !pitr_interval_retention.is_empty();
	// Expired-row deletion is batch work like everything else; cap it so a
	// long-idle database does not clear an unbounded row count in one pass.
	// Rows deferred past the cap are still present, so their txids must stay in
	// the keep-set: a row may only leave coverage in the same pass that deletes
	// it, or the fence's expired-but-present acceptance breaks.
	let deferred_expired = expired_pitr_interval_rows
		.split_off(expired_pitr_interval_rows.len().min(CMP_FDB_BATCH_MAX_KEYS));
	for (bucket_start_ms, _, _, coverage) in deferred_expired {
		pitr_interval_retention.push(PitrIntervalSelection {
			bucket_start_ms,
			coverage,
		});
	}
	let mut snapshot = ReclaimInputSnapshot {
		expired_pitr_interval_rows,
		commit_deletes_blocked,
		has_retained_pitr_intervals,
		..ReclaimInputSnapshot::default()
	};
	let watermark_txid = root.hot_watermark_txid;
	if watermark_txid == 0 {
		return Ok(snapshot);
	}

	let mut budget = CompactionBatchBudget::fdb();

	// Every DELTA row at or below the watermark is deletable: the install that
	// advanced the watermark published shard coverage for it atomically. Group
	// chunk rows per txid so a batch boundary never splits one delta blob.
	let delta_scan_start = keys::branch_delta_prefix(branch_id);
	let delta_scan_end = watermark_txid
		.checked_add(1)
		.map(|next_txid| keys::branch_delta_chunk_prefix(branch_id, next_txid))
		.unwrap_or_else(|| {
			universaldb::tuple::Subspace::from_bytes(keys::branch_delta_prefix(branch_id))
				.range()
				.1
		});
	// Every scan below is row-limited so a deep backlog never pulls more than
	// roughly one batch of rows into a refresh or reclaim transaction; the
	// remainder drains across subsequent passes.
	let mut delta_chunks_by_txid = BTreeMap::<u64, Vec<(Vec<u8>, Vec<u8>)>>::new();
	let delta_scan_limit = CMP_FDB_BATCH_MAX_KEYS * 2;
	let delta_rows = tx_scan_range_values_limited(
		tx,
		&delta_scan_start,
		&delta_scan_end,
		isolation_level,
		delta_scan_limit,
	)
	.await?;
	let delta_scan_truncated = delta_rows.len() == delta_scan_limit;
	for (key, value) in delta_rows {
		let txid = keys::decode_branch_delta_chunk_txid(branch_id, &key)?;
		if txid > watermark_txid {
			continue;
		}
		delta_chunks_by_txid
			.entry(txid)
			.or_default()
			.push((key, value));
	}
	// A truncated scan may have split the last delta's chunk rows; drop the
	// trailing group so a batch never deletes part of one blob.
	if delta_scan_truncated {
		delta_chunks_by_txid.pop_last();
	}
	'delta_groups: for (_, chunks) in delta_chunks_by_txid {
		let group_value_bytes = chunks
			.iter()
			.map(|(_, value)| u64::try_from(value.len()).unwrap_or(u64::MAX))
			.fold(0_u64, u64::saturating_add);
		if !budget.can_add(chunks.len(), group_value_bytes) {
			break 'delta_groups;
		}
		budget.add(chunks.len(), group_value_bytes);
		snapshot.delta_chunks.extend(chunks);
	}

	// COMMITS/VTX rows below the watermark are deletable unless a pin or a
	// retained PITR interval representative still resolves through them. When
	// bucket fork proofs are ambiguous the pin set may be incomplete, so
	// commit deletes are skipped entirely for this pass. The scan is paged so a
	// dense run of keep-set islands at the low end cannot permanently hide
	// deletable rows beyond a single page window.
	if !commit_deletes_blocked {
		let keep_txids = reclaim_keep_txids(db_pins, &pitr_interval_retention);
		let mut commit_cursor = keys::branch_commit_key(branch_id, 0);
		let commit_scan_end = keys::branch_commit_key(branch_id, watermark_txid);
		'commit_pages: for _ in 0..CMP_SCAN_MAX_PAGES {
			let rows = tx_scan_range_values_limited(
				tx,
				&commit_cursor,
				&commit_scan_end,
				isolation_level,
				CMP_FDB_BATCH_MAX_KEYS,
			)
			.await?;
			let page_full = rows.len() == CMP_FDB_BATCH_MAX_KEYS;
			for (key, value) in rows {
				commit_cursor = end_of_key_range(&key);
				let txid = decode_branch_commit_txid(branch_id, &key)?;
				if txid >= watermark_txid {
					break 'commit_pages;
				}
				if keep_txids.contains(&txid) {
					continue;
				}
				let commit =
					decode_commit_row(&value).context("decode sqlite commit row for reclaim")?;
				// Each commit deletion also clears its VTX row.
				let row_value_bytes = u64::try_from(value.len()).unwrap_or(u64::MAX);
				if !budget.can_add(2, row_value_bytes) {
					break 'commit_pages;
				}
				budget.add(2, row_value_bytes);
				snapshot.txid_refs.push(ReclaimTxidRef {
					txid,
					versionstamp: commit.versionstamp,
				});
				snapshot.commits.push((txid, key, value, commit));
			}
			if !page_full {
				break;
			}
		}
	}

	// PIDX rows referencing folded txids are stragglers from budget-truncated
	// installs. Reads already fall back to shard coverage for them; reclaim
	// clears the rows so they stop looking like live delta references. The scan
	// always runs at Snapshot isolation and the rows are excluded from the job
	// fingerprint: every clear is a compare-and-clear of a stale-by-definition
	// row, so taking a read conflict on the whole PIDX prefix would only make
	// busy databases reject reclaim passes for no safety gain. Paging keeps
	// dense runs of live rows at low page numbers from hiding stale rows above
	// them.
	let mut pidx_cursor = keys::branch_pidx_prefix(branch_id);
	let (_, pidx_scan_end) =
		universaldb::tuple::Subspace::from_bytes(keys::branch_pidx_prefix(branch_id)).range();
	'pidx_pages: for _ in 0..CMP_SCAN_MAX_PAGES {
		let rows = tx_scan_range_values_limited(
			tx,
			&pidx_cursor,
			&pidx_scan_end,
			Snapshot,
			CMP_FDB_BATCH_MAX_KEYS,
		)
		.await?;
		let page_full = rows.len() == CMP_FDB_BATCH_MAX_KEYS;
		for (key, value) in rows {
			pidx_cursor = end_of_key_range(&key);
			let Ok(txid) = decode_pidx_txid(&value) else {
				continue;
			};
			if txid > watermark_txid {
				continue;
			}
			let row_value_bytes = u64::try_from(value.len()).unwrap_or(u64::MAX);
			if !budget.can_add(1, row_value_bytes) {
				break 'pidx_pages;
			}
			budget.add(1, row_value_bytes);
			snapshot.stale_pidx_entries.push((key, value));
		}
		if !page_full {
			break;
		}
	}

	// SHARD versions are deletable once no covered txid reads through them.
	// Reads resolve "newest version at or below the cap" and every reachable
	// cap is a covered txid or above the watermark, so keeping the newest
	// version at or below each covered point plus everything above the
	// watermark preserves every readable state. The pin set must be complete
	// for this proof, so ambiguous bucket fork proofs suppress it.
	if !commit_deletes_blocked {
		let mut covered_txids = reclaim_keep_txids(db_pins, &pitr_interval_retention);
		covered_txids.insert(watermark_txid);
		// Shard rows carry full blobs, so pages are small and byte-bounded, and
		// the scan starts from a cursor that rotates across passes; otherwise a
		// branch wider than one window would never have its tail shards
		// collected. Deleting a version only needs a scanned newer shadow at or
		// below every covered point, so a window cut at either end of a shard's
		// version list can only cause over-keeping, never a wrong delete.
		let shard_prefix = keys::branch_shard_prefix(branch_id);
		let (_, shard_scan_end) =
			universaldb::tuple::Subspace::from_bytes(keys::branch_shard_prefix(branch_id)).range();
		let mut shard_cursor = if shard_gc_cursor > shard_prefix.as_slice()
			&& shard_gc_cursor < shard_scan_end.as_slice()
		{
			shard_gc_cursor.to_vec()
		} else {
			shard_prefix.clone()
		};
		let mut versions_by_shard = BTreeMap::<u32, Vec<(u64, Vec<u8>, Vec<u8>)>>::new();
		let mut scanned_value_bytes = 0_u64;
		let mut scan_exhausted = false;
		'shard_pages: for _ in 0..CMP_SCAN_MAX_PAGES {
			let rows = tx_scan_range_values_limited(
				tx,
				&shard_cursor,
				&shard_scan_end,
				isolation_level,
				SHARD_GC_SCAN_PAGE_ROWS,
			)
			.await?;
			let page_full = rows.len() == SHARD_GC_SCAN_PAGE_ROWS;
			for (key, value) in rows {
				shard_cursor = end_of_key_range(&key);
				// An unexpected key under the SHARD prefix must not poison
				// manager refresh; skip it and leave the row alone.
				let Ok((shard_id, as_of_txid)) =
					keys::decode_branch_shard_version_key(branch_id, &key)
				else {
					tracing::error!(
						?branch_id,
						"skipping undecodable sqlite SHARD key during reclaim planning"
					);
					continue;
				};
				scanned_value_bytes =
					scanned_value_bytes.saturating_add(u64::try_from(value.len()).unwrap_or(0));
				versions_by_shard
					.entry(shard_id)
					.or_default()
					.push((as_of_txid, key, value));
			}
			if !page_full {
				scan_exhausted = true;
				break;
			}
			if scanned_value_bytes >= SHARD_GC_SCAN_MAX_VALUE_BYTES {
				break 'shard_pages;
			}
		}
		// Rotate the cursor so the next pass continues where this one stopped;
		// an exhausted scan wraps back to the front.
		snapshot.shard_gc_next_cursor = if scan_exhausted {
			Vec::new()
		} else {
			shard_cursor
		};
		'shards: for (_, versions) in versions_by_shard {
			let mut keep = BTreeSet::new();
			for (as_of_txid, _, _) in &versions {
				if *as_of_txid > watermark_txid {
					keep.insert(*as_of_txid);
				}
			}
			for covered_txid in &covered_txids {
				if let Some(keeper) = versions
					.iter()
					.filter(|(as_of_txid, _, _)| as_of_txid <= covered_txid)
					.map(|(as_of_txid, _, _)| *as_of_txid)
					.max()
				{
					keep.insert(keeper);
				}
			}
			for (as_of_txid, key, value) in versions {
				if keep.contains(&as_of_txid) {
					continue;
				}
				let row_value_bytes = u64::try_from(value.len()).unwrap_or(u64::MAX);
				if !budget.can_add(1, row_value_bytes) {
					break 'shards;
				}
				budget.add(1, row_value_bytes);
				snapshot.stale_shard_versions.push((key, value));
			}
		}
	}
	snapshot.total_value_bytes = budget.value_bytes();

	Ok(snapshot)
}

pub(crate) async fn tx_scan_range_values_limited(
	tx: &universaldb::Transaction,
	start: &[u8],
	end: &[u8],
	isolation_level: universaldb::utils::IsolationLevel,
	limit: usize,
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

	Ok(rows)
}

pub(crate) fn reclaim_keep_txids(
	db_pins: &[DbHistoryPin],
	pitr_interval_retention: &[PitrIntervalSelection],
) -> BTreeSet<u64> {
	db_pins
		.iter()
		.map(|pin| pin.at_txid)
		.chain(
			pitr_interval_retention
				.iter()
				.map(|selection| selection.coverage.txid),
		)
		.collect()
}

pub(crate) fn reclaim_snapshot_has_work(snapshot: &ReclaimInputSnapshot) -> bool {
	!snapshot.commits.is_empty()
		|| !snapshot.delta_chunks.is_empty()
		|| !snapshot.stale_pidx_entries.is_empty()
		|| !snapshot.stale_shard_versions.is_empty()
		|| !snapshot.expired_pitr_interval_rows.is_empty()
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
) -> Result<PitrIntervalReclaimRows> {
	let mut retained = Vec::new();
	let mut expired = Vec::new();

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

pub(crate) fn selected_hot_coverage_txids(
	root: &CompactionRoot,
	selected_max_txid: u64,
	db_pins: &[DbHistoryPin],
	pitr_interval_coverage: &[PitrIntervalSelection],
) -> Vec<u64> {
	let mut coverage_txids = BTreeSet::new();
	coverage_txids.insert(selected_max_txid);

	for pin in db_pins {
		if pin.at_txid > root.hot_watermark_txid && pin.at_txid <= selected_max_txid {
			coverage_txids.insert(pin.at_txid);
		}
	}
	for selection in pitr_interval_coverage {
		let txid = selection.coverage.txid;
		if txid > root.hot_watermark_txid && txid <= selected_max_txid {
			coverage_txids.insert(txid);
		}
	}

	coverage_txids.into_iter().collect()
}

pub(crate) fn plan_hot_job(
	database_branch_id: DatabaseBranchId,
	snapshot: &ManagerFdbSnapshot,
	job_id: Id,
	now_ms: i64,
	force: bool,
) -> Option<PlannedHotCompactionJob> {
	let branch_record = snapshot.branch_record.as_ref()?;
	let head = snapshot.head.as_ref()?;
	if head.head_txid <= snapshot.root.hot_watermark_txid {
		return None;
	}
	let hot_lag = head
		.head_txid
		.saturating_sub(snapshot.root.hot_watermark_txid);
	let selected_max_txid = snapshot.hot_inputs.selected_max_txid?;
	let coverage_txids = selected_hot_coverage_txids(
		&snapshot.root,
		selected_max_txid,
		&snapshot.db_pins,
		&snapshot.hot_inputs.pitr_interval_coverage,
	);
	let has_uncovered_pin = coverage_txids
		.iter()
		.any(|txid| *txid != selected_max_txid && *txid > snapshot.root.hot_watermark_txid);
	if hot_lag < quota::COMPACTION_DELTA_THRESHOLD && !has_uncovered_pin && !force {
		return None;
	}

	let input_range = HotJobInputRange {
		txids: TxidRange {
			min_txid: snapshot.root.hot_watermark_txid.saturating_add(1),
			max_txid: selected_max_txid,
		},
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
		planned_at_ms: now_ms,
		attempt: 0,
	})
}

pub(crate) fn plan_reclaim_job(
	database_branch_id: DatabaseBranchId,
	snapshot: &ManagerFdbSnapshot,
	job_id: Id,
	shard_gc_cursor: &[u8],
	now_ms: i64,
) -> Option<PlannedReclaimCompactionJob> {
	let branch_record = snapshot.branch_record.as_ref()?;
	if !reclaim_snapshot_has_work(&snapshot.reclaim_inputs) {
		return None;
	}

	let min_txid = snapshot
		.reclaim_inputs
		.txid_refs
		.first()
		.map(|txid_ref| txid_ref.txid)
		.unwrap_or(snapshot.root.hot_watermark_txid);
	let max_txid = snapshot
		.reclaim_inputs
		.txid_refs
		.last()
		.map(|txid_ref| txid_ref.txid)
		.unwrap_or(snapshot.root.hot_watermark_txid);
	let input_range = ReclaimJobInputRange {
		txids: TxidRange { min_txid, max_txid },
		txid_refs: snapshot.reclaim_inputs.txid_refs.clone(),
		staged_hot_shards: Vec::new(),
		max_keys: CMP_FDB_BATCH_MAX_KEYS as u32,
		max_bytes: CMP_FDB_BATCH_MAX_VALUE_BYTES as u64,
		shard_gc_cursor: shard_gc_cursor.to_vec(),
	};
	let input_fingerprint =
		fingerprint_reclaim_inputs(database_branch_id, &snapshot.root, &snapshot.reclaim_inputs);

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

pub(crate) fn reclaim_noop_reason(snapshot: &ManagerFdbSnapshot) -> &'static str {
	if reclaim_snapshot_has_work(&snapshot.reclaim_inputs) {
		return "reclaim:work-planned";
	}
	if snapshot.bucket_proof_blocked_reclaim {
		return "reclaim:bucket-proof-blocked";
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

pub(crate) fn fingerprint_reclaim_inputs(
	database_branch_id: DatabaseBranchId,
	root: &CompactionRoot,
	reclaim_inputs: &ReclaimInputSnapshot,
) -> CompactionInputFingerprint {
	let mut fingerprint = Sha256::new();
	update_fingerprint(&mut fingerprint, database_branch_id.as_uuid().as_bytes());
	update_fingerprint(&mut fingerprint, &root.manifest_generation.to_be_bytes());
	update_fingerprint(&mut fingerprint, &root.hot_watermark_txid.to_be_bytes());
	for txid_ref in &reclaim_inputs.txid_refs {
		update_fingerprint(&mut fingerprint, &txid_ref.txid.to_be_bytes());
		update_fingerprint(&mut fingerprint, &txid_ref.versionstamp);
	}
	for (txid, key, value, commit) in &reclaim_inputs.commits {
		update_fingerprint(&mut fingerprint, &txid.to_be_bytes());
		update_fingerprint(&mut fingerprint, key);
		update_fingerprint(&mut fingerprint, value);
		update_fingerprint(&mut fingerprint, &commit.versionstamp);
	}
	for (key, value) in &reclaim_inputs.delta_chunks {
		update_fingerprint(&mut fingerprint, key);
		update_fingerprint(&mut fingerprint, value);
	}
	// Stale PIDX rows and stale shard versions are deliberately not
	// fingerprinted: the execute pass recomputes its own stale sets under
	// serializable pin reads and clears them via compare-and-clear, so plan and
	// execute do not need to agree on the exact sets, and fingerprinting them
	// would reject passes on busy databases for no safety gain.
	update_fingerprint(
		&mut fingerprint,
		&[u8::from(reclaim_inputs.commit_deletes_blocked)],
	);
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
	for staged in &input_range.staged_hot_shards {
		update_fingerprint(&mut fingerprint, &staged.job_id.as_bytes());
		update_fingerprint(&mut fingerprint, &staged.output_ref.shard_id.to_be_bytes());
		update_fingerprint(
			&mut fingerprint,
			&staged.output_ref.as_of_txid.to_be_bytes(),
		);
		update_fingerprint(
			&mut fingerprint,
			&staged.output_ref.size_bytes.to_be_bytes(),
		);
		update_fingerprint(&mut fingerprint, &staged.output_ref.content_hash);
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

pub(crate) async fn write_staged_hot_shards(
	tx: &universaldb::Transaction,
	input: &StageHotJobInput,
	_head: &DBHead,
	hot_inputs: &HotInputSnapshot,
) -> Result<Vec<HotShardOutputRef>> {
	let deltas = decode_hot_delta_chunks(input.database_branch_id, &hot_inputs.delta_chunks)?;
	let mut output_refs = Vec::new();

	for as_of_txid in &input.input_range.coverage_txids {
		// Every coverage txid is a commit inside this batch, and folding uses
		// the per-commit database sizes so writes truncated before the coverage
		// point are not resurrected and pages live at an earlier pinned txid
		// are not dropped by a later shrink.
		let pages_by_shard = collect_hot_pages_by_shard(&hot_inputs.commits, &deltas, *as_of_txid)?;

		for (shard_id, page_updates) in pages_by_shard {
			let encoded = build_staged_hot_shard_blob(
				tx,
				input.database_branch_id,
				shard_id,
				*as_of_txid,
				page_updates,
			)
			.await?;
			let key = keys::branch_compaction_stage_hot_shard_key(
				input.database_branch_id,
				input.job_id,
				shard_id,
				*as_of_txid,
				0,
			);
			let content_hash = content_hash(&encoded);

			tx.informal().set(&key, &encoded);
			output_refs.push(HotShardOutputRef {
				shard_id,
				as_of_txid: *as_of_txid,
				min_txid: input.input_range.txids.min_txid,
				max_txid: *as_of_txid,
				size_bytes: u64::try_from(encoded.len()).unwrap_or(u64::MAX),
				content_hash,
			});
		}
	}

	Ok(output_refs)
}

pub(crate) fn decode_hot_delta_chunks(
	branch_id: DatabaseBranchId,
	delta_chunks: &[(Vec<u8>, Vec<u8>)],
) -> Result<BTreeMap<u64, DecodedLtx>> {
	let mut chunks_by_txid = BTreeMap::<u64, BTreeMap<u32, Vec<u8>>>::new();
	for (key, value) in delta_chunks {
		let txid = keys::decode_branch_delta_chunk_txid(branch_id, key)?;
		let chunk_idx = keys::decode_branch_delta_chunk_idx(branch_id, txid, key)?;
		chunks_by_txid
			.entry(txid)
			.or_default()
			.insert(chunk_idx, value.clone());
	}

	chunks_by_txid
		.into_iter()
		.map(|(txid, chunks)| {
			let bytes = chunks.into_values().flatten().collect::<Vec<_>>();
			let decoded =
				decode_ltx_v3(&bytes).with_context(|| format!("decode hot delta {txid}"))?;

			Ok((txid, decoded))
		})
		.collect()
}

pub(crate) fn collect_hot_pages_by_shard(
	batch_commits: &[(u64, CommitRow)],
	deltas: &BTreeMap<u64, DecodedLtx>,
	as_of_txid: u64,
) -> Result<BTreeMap<u32, Vec<(u32, Vec<u8>)>>> {
	let mut pages_by_number = BTreeMap::<u32, Vec<u8>>::new();

	for (txid, delta) in deltas {
		if *txid > as_of_txid {
			continue;
		}
		// A page write survives at the coverage txid only if no commit between
		// the write and the coverage point truncated the database below it.
		// Folding past a truncate would resurrect dead bytes once the database
		// regrows over the page.
		let min_db_size_pages = batch_commits
			.iter()
			.filter(|(commit_txid, _)| *commit_txid >= *txid && *commit_txid <= as_of_txid)
			.map(|(_, commit)| commit.db_size_pages)
			.min()
			.context("hot compaction delta txid is missing its commit row")?;
		for page in &delta.pages {
			if page.pgno <= min_db_size_pages {
				pages_by_number.insert(page.pgno, page.bytes.clone());
			} else {
				// A truncate killed this page after the write; drop any older
				// surviving write of it as well.
				pages_by_number.remove(&page.pgno);
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
	shard_id: u32,
	as_of_txid: u64,
	page_updates: Vec<(u32, Vec<u8>)>,
) -> Result<Vec<u8>> {
	let existing_blob = load_latest_branch_shard_blob(tx, branch_id, shard_id, as_of_txid).await?;
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

	encode_ltx_v3(header, &pages).context("encode staged hot shard blob")
}

pub(crate) async fn load_latest_branch_shard_blob(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	shard_id: u32,
	as_of_txid: u64,
) -> Result<Option<Vec<u8>>> {
	let prefix = keys::branch_shard_version_prefix(branch_id, shard_id);
	let end = end_of_key_range(&keys::branch_shard_key(branch_id, shard_id, as_of_txid));
	let informal = tx.informal();
	let mut stream = informal.get_ranges_keyvalues(
		RangeOption {
			mode: StreamingMode::WantAll,
			..(prefix.as_slice(), end.as_slice()).into()
		},
		Snapshot,
	);

	let mut latest = None;
	while let Some(entry) = stream.try_next().await? {
		latest = Some(entry.value().to_vec());
	}

	Ok(latest)
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

	Ok(rows)
}
