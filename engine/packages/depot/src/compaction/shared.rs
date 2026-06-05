use super::*;

pub(crate) async fn read_manager_fdb_snapshot(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
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
	let pitr_policy = PitrPolicy::default();
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
	let reclaim_inputs =
		read_reclaim_input_snapshot(tx, branch_id, &root, &db_pins, Snapshot, now_ms).await?;
	let hot_lag = head.as_ref().map_or(0, |head| {
		head.head_txid.saturating_sub(root.hot_watermark_txid)
	});
	let has_actionable_lag = hot_lag >= quota::COMPACTION_DELTA_THRESHOLD
		|| reclaim_coverage_is_complete(&reclaim_inputs);
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
) -> Result<PitrPolicy> {
	let Some(branch_record) = branch_record else {
		return Ok(PitrPolicy::default());
	};
	let Some((bucket_id, database_id)) =
		resolve_policy_scope_for_branch(tx, branch_record.branch_id).await?
	else {
		return Ok(PitrPolicy::default());
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

pub(crate) async fn resolve_policy_scope_for_branch(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
) -> Result<Option<(BucketId, String)>> {
	for (key, value) in
		tx_scan_prefix_values(tx, &keys::database_pointer_cur_prefix(), Serializable).await?
	{
		let Ok((bucket_branch_id, database_id)) = keys::decode_database_pointer_cur_key(&key)
		else {
			continue;
		};
		let pointer = decode_database_pointer(&value)
			.context("decode sqlite database pointer for PITR policy")?;
		if pointer.current_branch != branch_id {
			continue;
		}
		let Some(root_bucket_branch_id) = read_bucket_root_branch_id(tx, bucket_branch_id).await?
		else {
			return Ok(None);
		};
		let Some(bucket_id) = resolve_bucket_id_for_root_branch(tx, root_bucket_branch_id).await?
		else {
			return Ok(None);
		};

		return Ok(Some((bucket_id, database_id)));
	}

	Ok(None)
}

pub(crate) async fn read_bucket_root_branch_id(
	tx: &universaldb::Transaction,
	bucket_branch_id: crate::types::BucketBranchId,
) -> Result<Option<crate::types::BucketBranchId>> {
	let mut current = bucket_branch_id;
	for _ in 0..=MAX_BUCKET_DEPTH {
		let Some(record_bytes) =
			tx_get_value(tx, &keys::bucket_branches_list_key(current), Serializable).await?
		else {
			return Ok(None);
		};
		let record = crate::types::decode_bucket_branch_record(&record_bytes)
			.context("decode sqlite bucket branch record for PITR policy")?;
		let Some(parent) = record.parent else {
			return Ok(Some(current));
		};
		current = parent;
	}

	Ok(None)
}

pub(crate) async fn resolve_bucket_id_for_root_branch(
	tx: &universaldb::Transaction,
	root_bucket_branch_id: crate::types::BucketBranchId,
) -> Result<Option<BucketId>> {
	for (key, value) in
		tx_scan_prefix_values(tx, &keys::bucket_pointer_cur_prefix(), Serializable).await?
	{
		let Ok(bucket_id) = keys::decode_bucket_pointer_cur_bucket_id(&key) else {
			continue;
		};
		let pointer = decode_bucket_pointer(&value)
			.context("decode sqlite bucket pointer for PITR policy")?;
		if pointer.current_branch == root_bucket_branch_id {
			return Ok(Some(bucket_id));
		}
	}

	Ok(None)
}

pub(crate) async fn latest_commit_at_or_before_versionstamp(
	tx: &universaldb::Transaction,
	branch_id: DatabaseBranchId,
	versionstamp_cap: [u8; 16],
) -> Result<Option<(u64, [u8; 16], CommitRow)>> {
	let mut selected = None;
	let mut inspected_rows = 0_usize;

	for (key, value) in
		tx_scan_prefix_values(tx, &keys::branch_vtx_prefix(branch_id), Serializable).await?
	{
		let versionstamp = decode_branch_vtx_versionstamp(branch_id, &key)?;
		if versionstamp > versionstamp_cap {
			break;
		}
		inspected_rows = inspected_rows.saturating_add(1);
		if inspected_rows >= CMP_FDB_BATCH_MAX_KEYS {
			tracing::warn!(
				?branch_id,
				row_count = inspected_rows,
				"retaining sqlite history because bucket VTX proof is too large"
			);
			return Ok(None);
		}
		let txid = decode_txid_value(&value)?;
		selected = Some((txid, versionstamp));
	}

	let Some((txid, versionstamp)) = selected else {
		return Ok(None);
	};
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

		if let Ok(txid) = decode_pidx_txid(&value) {
			if txid < min_txid || txid > selected_max_txid {
				continue;
			}
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
	isolation_level: universaldb::utils::IsolationLevel,
	now_ms: i64,
) -> Result<ReclaimInputSnapshot> {
	let (pitr_interval_retention, expired_pitr_interval_rows) =
		read_pitr_interval_reclaim_rows(tx, branch_id, now_ms, isolation_level).await?;
	let Some(max_reclaim_txid) =
		reclaim_delete_upper_bound(root, db_pins, &pitr_interval_retention)
	else {
		return Ok(ReclaimInputSnapshot {
			expired_pitr_interval_rows,
			..ReclaimInputSnapshot::default()
		});
	};

	let mut snapshot = ReclaimInputSnapshot {
		expired_pitr_interval_rows,
		..ReclaimInputSnapshot::default()
	};
	let mut budget = CompactionBatchBudget::fdb();
	let commit_scan_start = keys::branch_commit_key(branch_id, 0);
	let commit_scan_end = max_reclaim_txid
		.checked_add(1)
		.map(|next_txid| keys::branch_commit_key(branch_id, next_txid))
		.unwrap_or_else(|| end_of_key_range(&keys::branch_commit_prefix(branch_id)));
	for (key, value) in
		tx_scan_range_values(tx, &commit_scan_start, &commit_scan_end, isolation_level).await?
	{
		let txid = decode_branch_commit_txid(branch_id, &key)?;
		if txid > max_reclaim_txid {
			break;
		}
		let commit = decode_commit_row(&value).context("decode sqlite commit row for reclaim")?;
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
		snapshot.txid_refs.push(ReclaimTxidRef {
			txid,
			versionstamp: commit.versionstamp,
		});
		snapshot.commits.push((txid, key, value, commit));
		snapshot.delta_chunks.extend(delta_chunks);
	}
	snapshot.total_value_bytes = budget.value_bytes();

	let selected_txids = snapshot
		.txid_refs
		.iter()
		.map(|txid_ref| txid_ref.txid)
		.collect::<BTreeSet<_>>();
	if selected_txids.is_empty() {
		return Ok(snapshot);
	}
	for (key, value) in
		tx_scan_prefix_values(tx, &keys::branch_pidx_prefix(branch_id), isolation_level).await?
	{
		if let Ok(txid) = decode_pidx_txid(&value) {
			if selected_txids.contains(&txid) {
				snapshot.pidx_entries.push((key, value));
			}
		}
	}

	let shard_ids = reclaim_delta_shard_ids(branch_id, &snapshot.delta_chunks)?;
	snapshot.required_coverage_shard_count = shard_ids.len();
	for shard_id in shard_ids {
		let key = keys::branch_shard_key(branch_id, shard_id, root.hot_watermark_txid);
		if let Some(value) = tx_get_value(tx, &key, isolation_level).await? {
			snapshot.coverage_shards.push((key, value));
		}
	}

	Ok(snapshot)
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

pub(crate) fn reclaim_delta_shard_ids(
	branch_id: DatabaseBranchId,
	delta_chunks: &[(Vec<u8>, Vec<u8>)],
) -> Result<BTreeSet<u32>> {
	let deltas = decode_hot_delta_chunks(branch_id, delta_chunks)?;
	let mut shard_ids = BTreeSet::new();
	for delta in deltas.values() {
		for page in &delta.pages {
			shard_ids.insert(page.pgno / keys::SHARD_SIZE);
		}
	}
	Ok(shard_ids)
}

pub(crate) fn reclaim_coverage_is_complete(snapshot: &ReclaimInputSnapshot) -> bool {
	!snapshot.delta_chunks.is_empty()
		&& snapshot.required_coverage_shard_count > 0
		&& snapshot.coverage_shards.len() == snapshot.required_coverage_shard_count
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
	now_ms: i64,
) -> Option<PlannedReclaimCompactionJob> {
	let branch_record = snapshot.branch_record.as_ref()?;
	if snapshot.bucket_proof_blocked_reclaim {
		return None;
	}
	let has_hot_reclaim = !snapshot.reclaim_inputs.txid_refs.is_empty()
		&& snapshot.reclaim_inputs.pidx_entries.is_empty()
		&& reclaim_coverage_is_complete(&snapshot.reclaim_inputs);
	let has_interval_cleanup = !snapshot
		.reclaim_inputs
		.expired_pitr_interval_rows
		.is_empty();
	if !has_hot_reclaim && !has_interval_cleanup {
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
		txid_refs: if has_hot_reclaim {
			snapshot.reclaim_inputs.txid_refs.clone()
		} else {
			Vec::new()
		},
		staged_hot_shards: Vec::new(),
		max_keys: CMP_FDB_BATCH_MAX_KEYS as u32,
		max_bytes: CMP_FDB_BATCH_MAX_VALUE_BYTES as u64,
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
	if snapshot.bucket_proof_blocked_reclaim {
		return "reclaim:bucket-proof-blocked";
	}

	let has_hot_inputs = !snapshot.reclaim_inputs.txid_refs.is_empty();
	if !has_hot_inputs {
		if snapshot
			.reclaim_inputs
			.expired_pitr_interval_rows
			.is_empty()
		{
			return "reclaim:no-actionable-work";
		}
		return "reclaim:expired-pitr-intervals";
	}

	if !snapshot.reclaim_inputs.pidx_entries.is_empty() {
		return "reclaim:pidx-dependencies";
	}

	if has_hot_inputs && !reclaim_coverage_is_complete(&snapshot.reclaim_inputs) {
		return "reclaim:missing-shard-coverage";
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
	for (key, value) in &reclaim_inputs.pidx_entries {
		update_fingerprint(&mut fingerprint, key);
		update_fingerprint(&mut fingerprint, value);
	}
	for (key, value) in &reclaim_inputs.coverage_shards {
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
	let selected_db_size_pages = hot_inputs
		.commits
		.iter()
		.find_map(|(txid, commit)| {
			(*txid == input.input_range.txids.max_txid).then_some(commit.db_size_pages)
		})
		.context("hot compaction selected commit row is missing")?;
	let mut output_refs = Vec::new();

	for as_of_txid in &input.input_range.coverage_txids {
		let pages_by_shard =
			collect_hot_pages_by_shard(selected_db_size_pages, &deltas, *as_of_txid)?;

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
	db_size_pages: u32,
	deltas: &BTreeMap<u64, DecodedLtx>,
	as_of_txid: u64,
) -> Result<BTreeMap<u32, Vec<(u32, Vec<u8>)>>> {
	let mut pages_by_number = BTreeMap::<u32, Vec<u8>>::new();

	for (txid, delta) in deltas {
		if *txid > as_of_txid {
			continue;
		}
		for page in &delta.pages {
			if page.pgno <= db_size_pages {
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
