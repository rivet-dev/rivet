use crate::{
	compaction::{
		companion::{CompanionKind, run_companion_loop},
		shared::*,
		test_hooks, *,
	},
	conveyer::metrics,
	workflows::db_manager::branch_record_is_live_at_generation,
};

#[cfg(feature = "test-faults")]
use crate::fault::ReclaimFaultPoint;

#[workflow(DbReclaimerWorkflow)]
pub async fn db_reclaimer(ctx: &mut WorkflowCtx, input: &DbReclaimerInput) -> Result<()> {
	run_companion_loop(ctx, input.database_branch_id, CompanionKind::Reclaim).await
}

#[activity(ReclaimFdbJob)]
pub async fn reclaim_fdb_job(
	ctx: &ActivityCtx,
	input: &ReclaimFdbJobInput,
) -> Result<ReclaimFdbJobOutput> {
	let input = input.clone();
	let input_for_tx = input.clone();
	let cold_storage_enabled =
		workflow_cold_storage_enabled(ctx.config(), input.database_branch_id);
	let now_ms = ctx.ts();

	let output = ctx
		.udb()?
		.run(move |tx| {
			let input = input_for_tx.clone();
			async move { reclaim_fdb_job_tx(&tx, &input, cold_storage_enabled, now_ms).await }
		})
		.await?;
	record_shard_cache_eviction_metrics(&input, &output);
	Ok(output)
}

fn record_shard_cache_eviction_metrics(input: &ReclaimFdbJobInput, output: &ReclaimFdbJobOutput) {
	if output.status != CompactionJobStatus::Succeeded
		|| input.input_range.shard_cache_evictions.is_empty()
	{
		return;
	}

	let evicted_bytes = input
		.input_range
		.shard_cache_evictions
		.iter()
		.map(|eviction| eviction.size_bytes)
		.fold(0_u64, u64::saturating_add);
	metrics::SQLITE_SHARD_CACHE_EVICTION_TOTAL
		.with_label_values(&[metrics::SHARD_CACHE_EVICTION_CLEARED])
		.inc_by(input.input_range.shard_cache_evictions.len() as u64);
	if evicted_bytes > 0 {
		let evicted_bytes = i64::try_from(evicted_bytes).unwrap_or(i64::MAX);
		let resident_bytes = metrics::SQLITE_SHARD_CACHE_RESIDENT_BYTES.get();
		metrics::SQLITE_SHARD_CACHE_RESIDENT_BYTES
			.set(resident_bytes.saturating_sub(evicted_bytes).max(0));
	}
}

async fn reclaim_fdb_job_tx(
	tx: &universaldb::Transaction,
	input: &ReclaimFdbJobInput,
	cold_storage_enabled: bool,
	now_ms: i64,
) -> Result<ReclaimFdbJobOutput> {
	if input.job_kind != CompactionJobKind::Reclaim {
		return Ok(rejected_reclaim_job("reclaimer received a non-reclaim job"));
	}
	#[cfg(feature = "test-faults")]
	if let Some(output) = reclaim_fdb_fault_output(
		input.database_branch_id,
		ReclaimFaultPoint::PlanBeforeSnapshot,
	)
	.await?
	{
		return Ok(output);
	}
	if input.input_range.txid_refs.is_empty()
		&& input.input_range.cold_objects.is_empty()
		&& input.input_range.shard_cache_evictions.is_empty()
		&& (!input.input_range.staged_hot_shards.is_empty()
			|| !input.input_range.orphan_cold_objects.is_empty())
	{
		return cleanup_repair_fdb_outputs_tx(tx, input).await;
	}

	let branch_record = tx_get_value(
		tx,
		&keys::branches_list_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_database_branch_record)
	.transpose()
	.context("decode sqlite database branch record for FDB reclaim")?;
	if !branch_record_is_live_at_generation(branch_record.as_ref(), input.base_lifecycle_generation)
	{
		return Ok(rejected_reclaim_job("database branch lifecycle changed"));
	}

	let root = tx_get_value(
		tx,
		&keys::branch_compaction_root_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root for FDB reclaim")?
	.unwrap_or(CompactionRoot {
		schema_version: 1,
		manifest_generation: 0,
		hot_watermark_txid: 0,
		cold_watermark_txid: 0,
		cold_watermark_versionstamp: [0; 16],
	});
	if root.manifest_generation != input.base_manifest_generation {
		return Ok(rejected_reclaim_job("base manifest generation changed"));
	}

	let mut db_pins =
		history_pin::read_db_history_pins(tx, input.database_branch_id, Serializable).await?;
	if resolve_bucket_fork_pins(tx, input.database_branch_id, &mut db_pins).await? {
		return Ok(rejected_reclaim_job("bucket fork proof is ambiguous"));
	}
	let snapshot = read_reclaim_input_snapshot(
		tx,
		input.database_branch_id,
		&root,
		&db_pins,
		branch_record.as_ref(),
		read_effective_shard_cache_policy_for_branch(tx, branch_record.as_ref()).await?,
		Serializable,
		cold_storage_enabled,
		now_ms,
	)
	.await?;
	if !input.input_range.txid_refs.is_empty() && snapshot.txid_refs != input.input_range.txid_refs
	{
		return Ok(rejected_reclaim_job("reclaim txid set changed"));
	}
	if snapshot.cold_object_refs != input.input_range.cold_objects {
		return Ok(rejected_reclaim_job("cold object reclaim set changed"));
	}
	if snapshot
		.shard_cache_evictions
		.iter()
		.map(|candidate| candidate.reference.clone())
		.collect::<Vec<_>>()
		!= input.input_range.shard_cache_evictions
	{
		return Ok(rejected_reclaim_job("shard cache eviction set changed"));
	}
	if !input.input_range.txid_refs.is_empty() {
		if !snapshot.pidx_entries.is_empty() {
			return Ok(rejected_reclaim_job("PIDX still references reclaim txids"));
		}
		if !reclaim_coverage_is_complete(&snapshot) {
			return Ok(rejected_reclaim_job(
				"replacement SHARD coverage is missing",
			));
		}
	}

	let input_fingerprint = fingerprint_reclaim_inputs(input.database_branch_id, &root, &snapshot);
	if input_fingerprint != input.input_fingerprint {
		return Ok(rejected_reclaim_job("reclaim input fingerprint changed"));
	}
	#[cfg(feature = "test-faults")]
	if let Some(output) = reclaim_fdb_fault_output(
		input.database_branch_id,
		ReclaimFaultPoint::PlanAfterSnapshot,
	)
	.await?
	{
		return Ok(output);
	}

	let selected_reclaim_txids = input
		.input_range
		.txid_refs
		.iter()
		.map(|txid_ref| txid_ref.txid)
		.collect::<BTreeSet<_>>();
	let mut key_count = 0_u32;
	let mut byte_count = 0_u64;
	#[cfg(feature = "test-faults")]
	if let Some(output) =
		reclaim_fdb_fault_output(input.database_branch_id, ReclaimFaultPoint::BeforeHotDelete)
			.await?
	{
		return Ok(output);
	}
	for (txid, key, value, commit) in &snapshot.commits {
		if !selected_reclaim_txids.contains(txid) {
			continue;
		}
		udb::compare_and_clear(tx, key, value);
		key_count = key_count.saturating_add(1);
		byte_count = byte_count.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));

		let vtx_key = keys::branch_vtx_key(input.database_branch_id, commit.versionstamp);
		if let Some(vtx_value) = tx_get_value(tx, &vtx_key, Serializable).await? {
			if vtx_value == txid.to_be_bytes() {
				udb::compare_and_clear(tx, &vtx_key, &vtx_value);
				key_count = key_count.saturating_add(1);
				byte_count =
					byte_count.saturating_add(u64::try_from(vtx_value.len()).unwrap_or(u64::MAX));
			} else {
				return Ok(rejected_reclaim_job("VTX row changed for reclaim txid"));
			}
		}
	}
	for (key, value) in &snapshot.delta_chunks {
		let txid = keys::decode_branch_delta_chunk_txid(input.database_branch_id, key)?;
		if !selected_reclaim_txids.contains(&txid) {
			continue;
		}
		udb::compare_and_clear(tx, key, value);
		key_count = key_count.saturating_add(1);
		byte_count = byte_count.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));
	}
	for (_, key, value, _) in &snapshot.expired_pitr_interval_rows {
		udb::compare_and_clear(tx, key, value);
		key_count = key_count.saturating_add(1);
		byte_count = byte_count.saturating_add(u64::try_from(value.len()).unwrap_or(u64::MAX));
	}
	for candidate in &snapshot.shard_cache_evictions {
		if !input
			.input_range
			.shard_cache_evictions
			.contains(&candidate.reference)
		{
			continue;
		}
		udb::compare_and_clear(tx, &candidate.shard_key, &candidate.shard_bytes);
		key_count = key_count.saturating_add(1);
		byte_count = byte_count
			.saturating_add(u64::try_from(candidate.shard_bytes.len()).unwrap_or(u64::MAX));
	}
	#[cfg(feature = "test-faults")]
	if let Some(output) =
		reclaim_fdb_fault_output(input.database_branch_id, ReclaimFaultPoint::AfterHotDelete)
			.await?
	{
		return Ok(output);
	}

	Ok(ReclaimFdbJobOutput {
		status: CompactionJobStatus::Succeeded,
		output_refs: vec![ReclaimOutputRef {
			key_count,
			byte_count,
			min_txid: input.input_range.txids.min_txid,
			max_txid: input.input_range.txids.max_txid,
		}],
	})
}

pub(super) async fn cleanup_repair_fdb_outputs_tx(
	tx: &universaldb::Transaction,
	input: &ReclaimFdbJobInput,
) -> Result<ReclaimFdbJobOutput> {
	let input_fingerprint =
		fingerprint_repair_reclaim_range(input.database_branch_id, &input.input_range);
	if input_fingerprint != input.input_fingerprint {
		return Ok(rejected_reclaim_job(
			"repair cleanup input fingerprint changed",
		));
	}

	let branch_record = tx_get_value(
		tx,
		&keys::branches_list_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_database_branch_record)
	.transpose()
	.context("decode sqlite database branch record for repair cleanup")?;
	if !branch_record_is_live_at_generation(branch_record.as_ref(), input.base_lifecycle_generation)
	{
		return Ok(rejected_reclaim_job("database branch lifecycle changed"));
	}

	let root = tx_get_value(
		tx,
		&keys::branch_compaction_root_key(input.database_branch_id),
		Snapshot,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root for repair cleanup")?;
	let manifest_generation = root
		.as_ref()
		.map(|root| root.manifest_generation)
		.unwrap_or(input.base_manifest_generation);

	let mut key_count = 0_u32;
	let mut byte_count = 0_u64;
	for staged in &input.input_range.staged_hot_shards {
		let stage_key = keys::branch_compaction_stage_hot_shard_key(
			input.database_branch_id,
			staged.job_id,
			staged.output_ref.shard_id,
			staged.output_ref.as_of_txid,
			0,
		);
		let Some(stage_value) = tx_get_value(tx, &stage_key, Serializable).await? else {
			continue;
		};
		if staged.output_ref.size_bytes != u64::try_from(stage_value.len()).unwrap_or(u64::MAX)
			|| staged.output_ref.content_hash != content_hash(&stage_value)
		{
			tracing::error!(
				?input.database_branch_id,
				manifest_generation,
				?staged.job_id,
				shard_id = staged.output_ref.shard_id,
				as_of_txid = staged.output_ref.as_of_txid,
				repair_action = "retain_staged_hot_output",
				"staged hot shard cleanup found mismatched bytes"
			);
			return Ok(rejected_reclaim_job(
				"staged hot shard cleanup bytes changed",
			));
		}

		tracing::warn!(
			?input.database_branch_id,
			manifest_generation,
			?staged.job_id,
			shard_id = staged.output_ref.shard_id,
			as_of_txid = staged.output_ref.as_of_txid,
			repair_action = "clear_staged_hot_output",
			"clearing orphan staged hot shard output"
		);
		udb::compare_and_clear(tx, &stage_key, &stage_value);
		key_count = key_count.saturating_add(1);
		byte_count =
			byte_count.saturating_add(u64::try_from(stage_value.len()).unwrap_or(u64::MAX));
	}

	Ok(ReclaimFdbJobOutput {
		status: CompactionJobStatus::Succeeded,
		output_refs: vec![ReclaimOutputRef {
			key_count,
			byte_count,
			min_txid: input.input_range.txids.min_txid,
			max_txid: input.input_range.txids.max_txid,
		}],
	})
}

fn rejected_reclaim_job(reason: impl Into<String>) -> ReclaimFdbJobOutput {
	ReclaimFdbJobOutput {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
		output_refs: Vec::new(),
	}
}

#[cfg(feature = "test-faults")]
async fn reclaim_fdb_fault_output(
	database_branch_id: DatabaseBranchId,
	point: ReclaimFaultPoint,
) -> Result<Option<ReclaimFdbJobOutput>> {
	match test_hooks::maybe_fire_reclaim_fault(database_branch_id, point).await {
		Ok(Some(_)) | Ok(None) => Ok(None),
		Err(err) => Ok(Some(ReclaimFdbJobOutput {
			status: CompactionJobStatus::Failed {
				error: err.to_string(),
			},
			output_refs: Vec::new(),
		})),
	}
}

#[activity(RetireColdObjects)]
pub async fn retire_cold_objects(
	ctx: &ActivityCtx,
	input: &RetireColdObjectsInput,
) -> Result<RetireColdObjectsOutput> {
	let mut input = input.clone();
	input.retired_at_ms = ctx.ts();

	ctx.udb()?
		.run(move |tx| {
			let input = input.clone();
			async move { retire_cold_objects_tx(&tx, &input).await }
		})
		.await
}

async fn retire_cold_objects_tx(
	tx: &universaldb::Transaction,
	input: &RetireColdObjectsInput,
) -> Result<RetireColdObjectsOutput> {
	if input.job_kind != CompactionJobKind::Reclaim {
		return Ok(rejected_cold_object_retire(
			"reclaimer received a non-reclaim job",
		));
	}
	if input.cold_objects.is_empty() {
		return Ok(RetireColdObjectsOutput {
			status: CompactionJobStatus::Succeeded,
			retired_objects: Vec::new(),
			delete_after_ms: None,
		});
	}
	#[cfg(feature = "test-faults")]
	if let Some(output) = retire_cold_fault_output(
		input.database_branch_id,
		ReclaimFaultPoint::BeforeColdRetire,
	)
	.await?
	{
		return Ok(output);
	}

	let branch_record = tx_get_value(
		tx,
		&keys::branches_list_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_database_branch_record)
	.transpose()
	.context("decode sqlite database branch record for cold retire")?;
	if !branch_record_is_live_at_generation(branch_record.as_ref(), input.base_lifecycle_generation)
	{
		return Ok(rejected_cold_object_retire(
			"database branch lifecycle changed",
		));
	}

	let root = tx_get_value(
		tx,
		&keys::branch_compaction_root_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root for cold retire")?
	.unwrap_or(CompactionRoot {
		schema_version: 1,
		manifest_generation: 0,
		hot_watermark_txid: 0,
		cold_watermark_txid: 0,
		cold_watermark_versionstamp: [0; 16],
	});
	if root.manifest_generation != input.base_manifest_generation {
		return Ok(rejected_cold_object_retire(
			"base manifest generation changed",
		));
	}

	let delete_after_ms =
		input
			.retired_at_ms
			.saturating_add(test_hooks::cold_object_delete_grace_ms(
				input.database_branch_id,
			));
	let retired_manifest_generation = root.manifest_generation.saturating_add(1);
	let mut retired_objects = Vec::with_capacity(input.cold_objects.len());

	for cold_object in &input.cold_objects {
		let live_key = keys::branch_compaction_cold_shard_key(
			input.database_branch_id,
			cold_object.shard_id,
			cold_object.as_of_txid,
		);
		let Some(live_value) = tx_get_value(tx, &live_key, Serializable).await? else {
			return Ok(rejected_cold_object_retire(
				"cold shard ref is already absent",
			));
		};
		let live_ref = decode_cold_shard_ref(&live_value)
			.context("decode sqlite cold shard ref for cold retire")?;
		if reclaim_cold_object_ref(&live_ref) != *cold_object {
			return Ok(rejected_cold_object_retire("cold shard ref changed"));
		}

		let retired_key = keys::branch_compaction_retired_cold_object_key(
			input.database_branch_id,
			content_hash(cold_object.object_key.as_bytes()),
		);
		if tx_get_value(tx, &retired_key, Serializable)
			.await?
			.is_some()
		{
			return Ok(rejected_cold_object_retire(
				"cold object is already retired",
			));
		}

		udb::compare_and_clear(tx, &live_key, &live_value);
		let retired = RetiredColdObject {
			object_key: cold_object.object_key.clone(),
			object_generation_id: cold_object.object_generation_id,
			content_hash: cold_object.content_hash,
			retired_manifest_generation,
			retired_at_ms: input.retired_at_ms,
			delete_after_ms,
			delete_state: RetiredColdObjectDeleteState::Retired,
		};
		tx.informal().set(
			&retired_key,
			&encode_retired_cold_object(retired.clone())
				.context("encode sqlite retired cold object")?,
		);
		retired_objects.push(retired);
	}

	let next_root = CompactionRoot {
		schema_version: root.schema_version,
		manifest_generation: retired_manifest_generation,
		hot_watermark_txid: root.hot_watermark_txid,
		cold_watermark_txid: root.cold_watermark_txid,
		cold_watermark_versionstamp: root.cold_watermark_versionstamp,
	};
	tx.informal().set(
		&keys::branch_compaction_root_key(input.database_branch_id),
		&encode_compaction_root(next_root)
			.context("encode sqlite compaction root for cold retire")?,
	);
	#[cfg(feature = "test-faults")]
	if let Some(output) =
		retire_cold_fault_output(input.database_branch_id, ReclaimFaultPoint::AfterColdRetire)
			.await?
	{
		return Ok(output);
	}

	Ok(RetireColdObjectsOutput {
		status: CompactionJobStatus::Succeeded,
		retired_objects,
		delete_after_ms: Some(delete_after_ms),
	})
}

fn rejected_cold_object_retire(reason: impl Into<String>) -> RetireColdObjectsOutput {
	RetireColdObjectsOutput {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
		retired_objects: Vec::new(),
		delete_after_ms: None,
	}
}

#[cfg(feature = "test-faults")]
async fn retire_cold_fault_output(
	database_branch_id: DatabaseBranchId,
	point: ReclaimFaultPoint,
) -> Result<Option<RetireColdObjectsOutput>> {
	match test_hooks::maybe_fire_reclaim_fault(database_branch_id, point).await {
		Ok(Some(_)) | Ok(None) => Ok(None),
		Err(err) => Ok(Some(RetireColdObjectsOutput {
			status: CompactionJobStatus::Failed {
				error: err.to_string(),
			},
			retired_objects: Vec::new(),
			delete_after_ms: None,
		})),
	}
}

#[activity(DeleteRetiredColdObjects)]
pub async fn delete_retired_cold_objects(
	ctx: &ActivityCtx,
	input: &DeleteRetiredColdObjectsInput,
) -> Result<DeleteRetiredColdObjectsOutput> {
	let input = input.clone();
	let Some(cold_tier) = workflow_cold_tier(ctx.config(), input.database_branch_id).await? else {
		return Ok(rejected_cold_object_delete("cold storage is disabled"));
	};

	let marked = ctx
		.udb()?
		.run({
			let input = input.clone();
			move |tx| {
				let input = input.clone();
				async move { mark_retired_cold_objects_delete_issued_tx(&tx, &input).await }
			}
		})
		.await?;
	if !matches!(marked.status, CompactionJobStatus::Succeeded) {
		return Ok(marked);
	}

	#[cfg(feature = "test-faults")]
	if let Some(output) = delete_cold_fault_output(
		input.database_branch_id,
		ReclaimFaultPoint::BeforeColdDelete,
		marked.deleted_object_keys.clone(),
	)
	.await?
	{
		return Ok(output);
	}
	cold_tier
		.delete_objects(&marked.deleted_object_keys)
		.await
		.context("delete retired sqlite cold objects")?;
	#[cfg(feature = "test-faults")]
	if let Some(output) = delete_cold_fault_output(
		input.database_branch_id,
		ReclaimFaultPoint::AfterColdDelete,
		marked.deleted_object_keys.clone(),
	)
	.await?
	{
		return Ok(output);
	}

	Ok(marked)
}

async fn mark_retired_cold_objects_delete_issued_tx(
	tx: &universaldb::Transaction,
	input: &DeleteRetiredColdObjectsInput,
) -> Result<DeleteRetiredColdObjectsOutput> {
	let mut object_keys = Vec::with_capacity(input.cold_objects.len());

	for cold_object in &input.cold_objects {
		let retired_key = keys::branch_compaction_retired_cold_object_key(
			input.database_branch_id,
			content_hash(cold_object.object_key.as_bytes()),
		);
		let Some(retired_value) = tx_get_value(tx, &retired_key, Serializable).await? else {
			return Ok(rejected_cold_object_delete(
				"retired cold object is missing",
			));
		};
		let mut retired = decode_retired_cold_object(&retired_value)
			.context("decode sqlite retired cold object for S3 delete")?;
		if !retired_matches_cold_object(&retired, cold_object) {
			return Ok(rejected_cold_object_delete("retired cold object changed"));
		}
		if retired.delete_after_ms > input.now_ms {
			return Ok(rejected_cold_object_delete(
				"retired cold object is still in grace window",
			));
		}

		let live_key = keys::branch_compaction_cold_shard_key(
			input.database_branch_id,
			cold_object.shard_id,
			cold_object.as_of_txid,
		);
		if tx_get_value(tx, &live_key, Serializable).await?.is_some() {
			tracing::error!(
				?input.database_branch_id,
				object_key = %cold_object.object_key,
				publish_generation = cold_object.expected_publish_generation,
				"live cold ref exists for retired object before S3 delete"
			);
			return Ok(rejected_cold_object_delete(
				"live cold ref exists for retired object",
			));
		}

		if retired.delete_state == RetiredColdObjectDeleteState::Retired {
			retired.delete_state = RetiredColdObjectDeleteState::DeleteIssued;
			tx.informal().set(
				&retired_key,
				&encode_retired_cold_object(retired)
					.context("encode sqlite retired cold object delete state")?,
			);
		}
		object_keys.push(cold_object.object_key.clone());
	}

	Ok(DeleteRetiredColdObjectsOutput {
		status: CompactionJobStatus::Succeeded,
		deleted_object_keys: object_keys,
	})
}

fn rejected_cold_object_delete(reason: impl Into<String>) -> DeleteRetiredColdObjectsOutput {
	DeleteRetiredColdObjectsOutput {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
		deleted_object_keys: Vec::new(),
	}
}

#[cfg(feature = "test-faults")]
async fn delete_cold_fault_output(
	database_branch_id: DatabaseBranchId,
	point: ReclaimFaultPoint,
	deleted_object_keys: Vec<String>,
) -> Result<Option<DeleteRetiredColdObjectsOutput>> {
	match test_hooks::maybe_fire_reclaim_fault(database_branch_id, point).await {
		Ok(Some(_)) | Ok(None) => Ok(None),
		Err(err) => Ok(Some(DeleteRetiredColdObjectsOutput {
			status: CompactionJobStatus::Failed {
				error: err.to_string(),
			},
			deleted_object_keys,
		})),
	}
}

#[activity(CleanupRetiredColdObjects)]
pub async fn cleanup_retired_cold_objects(
	ctx: &ActivityCtx,
	input: &CleanupRetiredColdObjectsInput,
) -> Result<CleanupRetiredColdObjectsOutput> {
	let input = input.clone();

	ctx.udb()?
		.run(move |tx| {
			let input = input.clone();
			async move { cleanup_retired_cold_objects_tx(&tx, &input).await }
		})
		.await
}

async fn cleanup_retired_cold_objects_tx(
	tx: &universaldb::Transaction,
	input: &CleanupRetiredColdObjectsInput,
) -> Result<CleanupRetiredColdObjectsOutput> {
	let mut cleaned = Vec::with_capacity(input.cold_objects.len());
	#[cfg(feature = "test-faults")]
	if let Some(output) = cleanup_cold_fault_output(
		input.database_branch_id,
		ReclaimFaultPoint::BeforeCleanupRows,
	)
	.await?
	{
		return Ok(output);
	}

	for cold_object in &input.cold_objects {
		let live_key = keys::branch_compaction_cold_shard_key(
			input.database_branch_id,
			cold_object.shard_id,
			cold_object.as_of_txid,
		);
		if tx_get_value(tx, &live_key, Serializable).await?.is_some() {
			tracing::error!(
				?input.database_branch_id,
				object_key = %cold_object.object_key,
				publish_generation = cold_object.expected_publish_generation,
				"live cold ref exists for delete-issued retired object"
			);
			return Ok(rejected_cold_object_cleanup(
				"live cold ref exists for delete-issued retired object",
			));
		}

		let retired_key = keys::branch_compaction_retired_cold_object_key(
			input.database_branch_id,
			content_hash(cold_object.object_key.as_bytes()),
		);
		let Some(retired_value) = tx_get_value(tx, &retired_key, Serializable).await? else {
			continue;
		};
		let retired = decode_retired_cold_object(&retired_value)
			.context("decode sqlite retired cold object for cleanup")?;
		if !retired_matches_cold_object(&retired, cold_object) {
			return Ok(rejected_cold_object_cleanup("retired cold object changed"));
		}
		if retired.delete_state != RetiredColdObjectDeleteState::DeleteIssued {
			return Ok(rejected_cold_object_cleanup(
				"retired cold object delete was not issued",
			));
		}

		let completed = RetiredColdObject {
			delete_state: RetiredColdObjectDeleteState::Deleted,
			..retired
		};
		tx.informal().set(
			&retired_key,
			&encode_retired_cold_object(completed)
				.context("encode completed sqlite retired cold object")?,
		);
		cleaned.push(cold_object.object_key.clone());
	}

	Ok(CleanupRetiredColdObjectsOutput {
		status: CompactionJobStatus::Succeeded,
		cleaned_object_keys: cleaned,
	})
}

fn rejected_cold_object_cleanup(reason: impl Into<String>) -> CleanupRetiredColdObjectsOutput {
	CleanupRetiredColdObjectsOutput {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
		cleaned_object_keys: Vec::new(),
	}
}

#[cfg(feature = "test-faults")]
async fn cleanup_cold_fault_output(
	database_branch_id: DatabaseBranchId,
	point: ReclaimFaultPoint,
) -> Result<Option<CleanupRetiredColdObjectsOutput>> {
	match test_hooks::maybe_fire_reclaim_fault(database_branch_id, point).await {
		Ok(Some(_)) | Ok(None) => Ok(None),
		Err(err) => Ok(Some(CleanupRetiredColdObjectsOutput {
			status: CompactionJobStatus::Failed {
				error: err.to_string(),
			},
			cleaned_object_keys: Vec::new(),
		})),
	}
}

#[activity(ValidateReclaimColdObjects)]
pub async fn validate_reclaim_cold_objects(
	ctx: &ActivityCtx,
	input: &ValidateReclaimColdObjectsInput,
) -> Result<ValidateReclaimColdObjectsOutput> {
	let input = input.clone();
	if input.cold_objects.is_empty() {
		return Ok(ValidateReclaimColdObjectsOutput {
			status: CompactionJobStatus::Succeeded,
		});
	}

	let validated = ctx
		.udb()?
		.run({
			let input = input.clone();
			move |tx| {
				let input = input.clone();
				async move { validate_reclaim_cold_objects_tx(&tx, &input).await }
			}
		})
		.await?;
	if !matches!(validated.status, CompactionJobStatus::Succeeded) {
		return Ok(validated);
	}

	let Some(cold_tier) = workflow_cold_tier(ctx.config(), input.database_branch_id).await? else {
		return Ok(rejected_validate_reclaim_cold_objects(
			"cold storage is disabled",
		));
	};
	for cold_object in &input.cold_objects {
		if cold_tier
			.get_object(&cold_object.object_key)
			.await
			.with_context(|| format!("get sqlite workflow cold object {}", cold_object.object_key))?
			.is_none()
		{
			tracing::error!(
				?input.database_branch_id,
				manifest_generation = cold_object.expected_publish_generation,
				object_key = %cold_object.object_key,
				?cold_object.object_generation_id,
				shard_id = cold_object.shard_id,
				as_of_txid = cold_object.as_of_txid,
				repair_action = "retain_live_cold_ref",
				"live cold ref points at missing S3 object"
			);
			return Ok(rejected_validate_reclaim_cold_objects(
				"live cold ref points at missing S3 object",
			));
		}
	}

	Ok(ValidateReclaimColdObjectsOutput {
		status: CompactionJobStatus::Succeeded,
	})
}

async fn validate_reclaim_cold_objects_tx(
	tx: &universaldb::Transaction,
	input: &ValidateReclaimColdObjectsInput,
) -> Result<ValidateReclaimColdObjectsOutput> {
	for cold_object in &input.cold_objects {
		let live_key = keys::branch_compaction_cold_shard_key(
			input.database_branch_id,
			cold_object.shard_id,
			cold_object.as_of_txid,
		);
		let Some(live_value) = tx_get_value(tx, &live_key, Serializable).await? else {
			return Ok(rejected_validate_reclaim_cold_objects(
				"cold shard ref is missing before validation",
			));
		};
		let live_ref = decode_cold_shard_ref(&live_value)
			.context("decode sqlite cold shard ref for validation")?;
		if reclaim_cold_object_ref(&live_ref) != *cold_object {
			return Ok(rejected_validate_reclaim_cold_objects(
				"cold shard ref changed before validation",
			));
		}

		if let Some(retired) = read_retired_cold_object_by_object_key(
			tx,
			input.database_branch_id,
			&cold_object.object_key,
		)
		.await? && retired.delete_state == RetiredColdObjectDeleteState::DeleteIssued
		{
			tracing::error!(
				?input.database_branch_id,
				manifest_generation = cold_object.expected_publish_generation,
				object_key = %cold_object.object_key,
				?cold_object.object_generation_id,
				shard_id = cold_object.shard_id,
				as_of_txid = cold_object.as_of_txid,
				repair_action = "retain_live_cold_ref",
				"live cold ref points at delete-issued retired object"
			);
			return Ok(rejected_validate_reclaim_cold_objects(
				"live cold ref points at delete-issued retired object",
			));
		}
	}

	Ok(ValidateReclaimColdObjectsOutput {
		status: CompactionJobStatus::Succeeded,
	})
}

fn rejected_validate_reclaim_cold_objects(
	reason: impl Into<String>,
) -> ValidateReclaimColdObjectsOutput {
	ValidateReclaimColdObjectsOutput {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
	}
}

#[activity(DeleteOrphanColdObjects)]
pub async fn delete_orphan_cold_objects(
	ctx: &ActivityCtx,
	input: &DeleteOrphanColdObjectsInput,
) -> Result<DeleteOrphanColdObjectsOutput> {
	let input = input.clone();
	if input.orphan_cold_objects.is_empty() {
		return Ok(DeleteOrphanColdObjectsOutput {
			status: CompactionJobStatus::Succeeded,
			deleted_object_keys: Vec::new(),
		});
	}

	let planned = ctx
		.udb()?
		.run({
			let input = input.clone();
			move |tx| {
				let input = input.clone();
				async move { plan_orphan_cold_object_deletes_tx(&tx, &input).await }
			}
		})
		.await?;
	if !matches!(planned.status, CompactionJobStatus::Succeeded)
		|| planned.deleted_object_keys.is_empty()
	{
		return Ok(planned);
	}

	let Some(cold_tier) = workflow_cold_tier(ctx.config(), input.database_branch_id).await? else {
		return Ok(rejected_orphan_cold_object_delete(
			"cold storage is disabled",
		));
	};
	#[cfg(feature = "test-faults")]
	if let Some(output) = delete_orphan_cold_fault_output(
		input.database_branch_id,
		ReclaimFaultPoint::BeforeColdDelete,
		planned.deleted_object_keys.clone(),
	)
	.await?
	{
		return Ok(output);
	}
	cold_tier
		.delete_objects(&planned.deleted_object_keys)
		.await
		.context("delete orphan sqlite workflow cold objects")?;
	#[cfg(feature = "test-faults")]
	if let Some(output) = delete_orphan_cold_fault_output(
		input.database_branch_id,
		ReclaimFaultPoint::AfterColdDelete,
		planned.deleted_object_keys.clone(),
	)
	.await?
	{
		return Ok(output);
	}

	Ok(planned)
}

pub(super) async fn plan_orphan_cold_object_deletes_tx(
	tx: &universaldb::Transaction,
	input: &DeleteOrphanColdObjectsInput,
) -> Result<DeleteOrphanColdObjectsOutput> {
	let branch_record = tx_get_value(
		tx,
		&keys::branches_list_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_database_branch_record)
	.transpose()
	.context("decode sqlite database branch record for orphan cleanup")?;
	if !branch_record_is_live_at_generation(branch_record.as_ref(), input.base_lifecycle_generation)
	{
		return Ok(rejected_orphan_cold_object_delete(
			"database branch lifecycle changed",
		));
	}

	let live_refs = tx_scan_prefix_values(
		tx,
		&keys::branch_compaction_cold_shard_prefix(input.database_branch_id),
		Serializable,
	)
	.await?
	.into_iter()
	.map(|(_, value)| decode_cold_shard_ref(&value))
	.collect::<Result<Vec<_>>>()
	.context("decode sqlite cold shard refs for orphan cleanup")?;
	let root = tx_get_value(
		tx,
		&keys::branch_compaction_root_key(input.database_branch_id),
		Snapshot,
	)
	.await?
	.as_deref()
	.map(decode_compaction_root)
	.transpose()
	.context("decode sqlite compaction root for orphan cleanup")?;
	let manifest_generation = root
		.as_ref()
		.map(|root| root.manifest_generation)
		.unwrap_or_default();
	let mut delete_keys = Vec::new();

	for orphan in &input.orphan_cold_objects {
		let retired = read_retired_cold_object_by_object_key(
			tx,
			input.database_branch_id,
			&orphan.object_key,
		)
		.await?;
		let live_ref = live_refs
			.iter()
			.find(|live_ref| live_ref.object_key == orphan.object_key);
		if let Some(live_ref) = live_ref {
			if retired.as_ref().is_some_and(|retired| {
				retired.delete_state == RetiredColdObjectDeleteState::DeleteIssued
			}) {
				tracing::error!(
					?input.database_branch_id,
					manifest_generation,
					object_key = %orphan.object_key,
					?orphan.object_generation_id,
					shard_id = live_ref.shard_id,
					as_of_txid = live_ref.as_of_txid,
					repair_action = "retain_live_cold_ref",
					"live cold ref points at delete-issued retired object"
				);
				return Ok(rejected_orphan_cold_object_delete(
					"live cold ref points at delete-issued retired object",
				));
			}
			continue;
		}
		if retired.is_some() {
			continue;
		}

		tracing::warn!(
			?input.database_branch_id,
			manifest_generation,
			object_key = %orphan.object_key,
			?orphan.object_generation_id,
			shard_id = orphan.shard_id,
			as_of_txid = orphan.as_of_txid,
			repair_action = "delete_orphan_cold_object",
			"deleting orphan cold object"
		);
		delete_keys.push(orphan.object_key.clone());
		if delete_keys.len() >= CMP_S3_DELETE_MAX_OBJECTS {
			break;
		}
	}

	Ok(DeleteOrphanColdObjectsOutput {
		status: CompactionJobStatus::Succeeded,
		deleted_object_keys: delete_keys,
	})
}

fn rejected_orphan_cold_object_delete(reason: impl Into<String>) -> DeleteOrphanColdObjectsOutput {
	DeleteOrphanColdObjectsOutput {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
		deleted_object_keys: Vec::new(),
	}
}

#[cfg(feature = "test-faults")]
async fn delete_orphan_cold_fault_output(
	database_branch_id: DatabaseBranchId,
	point: ReclaimFaultPoint,
	deleted_object_keys: Vec<String>,
) -> Result<Option<DeleteOrphanColdObjectsOutput>> {
	match test_hooks::maybe_fire_reclaim_fault(database_branch_id, point).await {
		Ok(Some(_)) | Ok(None) => Ok(None),
		Err(err) => Ok(Some(DeleteOrphanColdObjectsOutput {
			status: CompactionJobStatus::Failed {
				error: err.to_string(),
			},
			deleted_object_keys,
		})),
	}
}
