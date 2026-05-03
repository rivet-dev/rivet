use crate::compaction::{
	companion::{CompanionKind, run_companion_loop},
	shared::*,
	*,
};
use crate::workflows::db_manager::branch_record_is_live_at_generation;

#[cfg(feature = "test-faults")]
use crate::compaction::test_hooks;
#[cfg(feature = "test-faults")]
use crate::fault::{DepotFaultAction, HotCompactionFaultPoint};

#[workflow(DbHotCompacterWorkflow)]
pub async fn db_hot_compacter(ctx: &mut WorkflowCtx, input: &DbHotCompacterInput) -> Result<()> {
	run_companion_loop(ctx, input.database_branch_id, CompanionKind::Hot).await
}

#[activity(StageHotJob)]
pub async fn stage_hot_job(
	ctx: &ActivityCtx,
	input: &StageHotJobInput,
) -> Result<StageHotJobOutput> {
	let input = input.clone();
	let now_ms = ctx.ts();

	ctx.udb()?
		.run(move |tx| {
			let input = input.clone();
			async move { stage_hot_job_tx(&tx, &input, now_ms).await }
		})
		.await
}

async fn stage_hot_job_tx(
	tx: &universaldb::Transaction,
	input: &StageHotJobInput,
	now_ms: i64,
) -> Result<StageHotJobOutput> {
	if input.job_kind != CompactionJobKind::Hot {
		return Ok(rejected_hot_job("hot compacter received a non-hot job"));
	}
	#[cfg(feature = "test-faults")]
	if let Some(output) = hot_stage_fault_output(
		input.database_branch_id,
		HotCompactionFaultPoint::StageBeforeInputRead,
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
	.context("decode sqlite database branch record for hot compaction")?;
	if !branch_record_is_live_at_generation(branch_record.as_ref(), input.base_lifecycle_generation)
	{
		return Ok(rejected_hot_job("database branch lifecycle changed"));
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
	.context("decode sqlite compaction root for hot compaction")?
	.unwrap_or(CompactionRoot {
		schema_version: 1,
		manifest_generation: 0,
		hot_watermark_txid: 0,
		cold_watermark_txid: 0,
		cold_watermark_versionstamp: [0; 16],
	});
	if root.manifest_generation != input.base_manifest_generation {
		return Ok(rejected_hot_job("base manifest generation changed"));
	}

	let Some(head) = tx_get_value(
		tx,
		&keys::branch_meta_head_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_db_head)
	.transpose()
	.context("decode sqlite head for hot compaction")?
	else {
		return Ok(rejected_hot_job("database branch head is missing"));
	};

	let db_pins = history_pin::read_db_history_pins(tx, input.database_branch_id, Snapshot).await?;
	let pitr_policy = read_effective_pitr_policy_for_branch(tx, branch_record.as_ref()).await?;
	let hot_inputs = read_hot_input_snapshot(
		tx,
		input.database_branch_id,
		Some(&head),
		&root,
		Snapshot,
		pitr_policy,
		now_ms,
	)
	.await?;
	let coverage_txids =
		selected_hot_coverage_txids(&root, &head, &db_pins, &hot_inputs.pitr_interval_coverage);
	if coverage_txids != input.input_range.coverage_txids {
		return Ok(rejected_hot_job("hot compaction coverage targets changed"));
	}
	let input_fingerprint = fingerprint_hot_inputs(
		input.database_branch_id,
		&root,
		&head,
		&coverage_txids,
		&hot_inputs,
	);
	if input_fingerprint != input.input_fingerprint {
		return Ok(rejected_hot_job("hot compaction input fingerprint changed"));
	}
	#[cfg(feature = "test-faults")]
	if let Some(output) = hot_stage_fault_output(
		input.database_branch_id,
		HotCompactionFaultPoint::StageAfterInputRead,
	)
	.await?
	{
		return Ok(output);
	}

	let output_refs = write_staged_hot_shards(tx, input, &head, &hot_inputs).await?;
	#[cfg(feature = "test-faults")]
	if let Some(output) = hot_stage_after_shard_write_fault_output(tx, input, &output_refs).await? {
		return Ok(output);
	}

	Ok(StageHotJobOutput {
		status: CompactionJobStatus::Succeeded,
		output_refs,
	})
}

fn rejected_hot_job(reason: impl Into<String>) -> StageHotJobOutput {
	StageHotJobOutput {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
		output_refs: Vec::new(),
	}
}

#[cfg(feature = "test-faults")]
async fn hot_stage_fault_output(
	database_branch_id: DatabaseBranchId,
	point: HotCompactionFaultPoint,
) -> Result<Option<StageHotJobOutput>> {
	match test_hooks::maybe_fire_hot_compaction_fault(database_branch_id, point).await {
		Ok(Some(_)) | Ok(None) => Ok(None),
		Err(err) => Ok(Some(failed_hot_job(err))),
	}
}

#[cfg(feature = "test-faults")]
async fn hot_stage_after_shard_write_fault_output(
	tx: &universaldb::Transaction,
	input: &StageHotJobInput,
	output_refs: &[HotShardOutputRef],
) -> Result<Option<StageHotJobOutput>> {
	match test_hooks::maybe_fire_hot_compaction_fault(
		input.database_branch_id,
		HotCompactionFaultPoint::StageAfterShardWrite,
	)
	.await
	{
		Ok(Some(fired)) if fired.action == DepotFaultAction::DropArtifact => {
			for output_ref in output_refs {
				tx.informal()
					.clear(&keys::branch_compaction_stage_hot_shard_key(
						input.database_branch_id,
						input.job_id,
						output_ref.shard_id,
						output_ref.as_of_txid,
						0,
					));
			}
			Ok(Some(StageHotJobOutput {
				status: CompactionJobStatus::Succeeded,
				output_refs: output_refs.to_vec(),
			}))
		}
		Ok(Some(_)) | Ok(None) => Ok(None),
		Err(err) => Ok(Some(failed_hot_job(err))),
	}
}

#[cfg(feature = "test-faults")]
fn failed_hot_job(err: anyhow::Error) -> StageHotJobOutput {
	StageHotJobOutput {
		status: CompactionJobStatus::Failed {
			error: err.to_string(),
		},
		output_refs: Vec::new(),
	}
}

#[activity(InstallHotJob)]
pub async fn install_hot_job(
	ctx: &ActivityCtx,
	input: &InstallHotJobInput,
) -> Result<InstallHotJobOutput> {
	let input = input.clone();
	let now_ms = ctx.ts();

	ctx.udb()?
		.run(move |tx| {
			let input = input.clone();
			async move { install_hot_job_tx(&tx, &input, now_ms).await }
		})
		.await
}

async fn install_hot_job_tx(
	tx: &universaldb::Transaction,
	input: &InstallHotJobInput,
	now_ms: i64,
) -> Result<InstallHotJobOutput> {
	if input.job_kind != CompactionJobKind::Hot {
		return Ok(rejected_hot_install("manager received a non-hot job"));
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
	.context("decode sqlite database branch record for hot install")?;
	if !branch_record_is_live_at_generation(branch_record.as_ref(), input.base_lifecycle_generation)
	{
		return Ok(rejected_hot_install("database branch lifecycle changed"));
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
	.context("decode sqlite compaction root for hot install")?
	.unwrap_or(CompactionRoot {
		schema_version: 1,
		manifest_generation: 0,
		hot_watermark_txid: 0,
		cold_watermark_txid: 0,
		cold_watermark_versionstamp: [0; 16],
	});
	if root.manifest_generation != input.base_manifest_generation {
		return Ok(rejected_hot_install("base manifest generation changed"));
	}

	let Some(head) = tx_get_value(
		tx,
		&keys::branch_meta_head_key(input.database_branch_id),
		Serializable,
	)
	.await?
	.as_deref()
	.map(decode_db_head)
	.transpose()
	.context("decode sqlite head for hot install")?
	else {
		return Ok(rejected_hot_install("database branch head is missing"));
	};

	let mut db_pins =
		history_pin::read_db_history_pins(tx, input.database_branch_id, Serializable).await?;
	if resolve_bucket_fork_pins(tx, input.database_branch_id, &mut db_pins).await? {
		return Ok(rejected_hot_install("bucket fork proof is ambiguous"));
	}
	let pitr_policy = read_effective_pitr_policy_for_branch(tx, branch_record.as_ref()).await?;
	let hot_inputs = read_hot_input_snapshot(
		tx,
		input.database_branch_id,
		Some(&head),
		&root,
		Serializable,
		pitr_policy,
		now_ms,
	)
	.await?;
	let coverage_txids =
		selected_hot_coverage_txids(&root, &head, &db_pins, &hot_inputs.pitr_interval_coverage);
	if coverage_txids != input.input_range.coverage_txids {
		return Ok(rejected_hot_install(
			"hot compaction coverage targets changed",
		));
	}
	let input_fingerprint = fingerprint_hot_inputs(
		input.database_branch_id,
		&root,
		&head,
		&coverage_txids,
		&hot_inputs,
	);
	if input_fingerprint != input.input_fingerprint {
		return Ok(rejected_hot_install(
			"hot compaction input fingerprint changed",
		));
	}

	let mut staged_blobs = Vec::with_capacity(input.output_refs.len());
	let mut staged_outputs = BTreeSet::new();
	let mut latest_staged_shards = BTreeSet::new();
	let coverage_txids = input
		.input_range
		.coverage_txids
		.iter()
		.copied()
		.collect::<BTreeSet<_>>();
	#[cfg(feature = "test-faults")]
	if let Some(output) = hot_install_before_staged_read_fault_output(tx, input).await? {
		return Ok(output);
	}
	for output_ref in &input.output_refs {
		if !coverage_txids.contains(&output_ref.as_of_txid)
			|| output_ref.min_txid != input.input_range.txids.min_txid
			|| output_ref.max_txid != output_ref.as_of_txid
		{
			return Ok(rejected_hot_install(
				"hot output ref does not match planned txid range",
			));
		}
		if !staged_outputs.insert((output_ref.shard_id, output_ref.as_of_txid)) {
			return Ok(rejected_hot_install(
				"duplicate staged hot shard output ref",
			));
		}
		if output_ref.as_of_txid == input.input_range.txids.max_txid
			&& !latest_staged_shards.insert(output_ref.shard_id)
		{
			return Ok(rejected_hot_install(
				"duplicate latest hot shard output ref",
			));
		}

		let stage_key = keys::branch_compaction_stage_hot_shard_key(
			input.database_branch_id,
			input.job_id,
			output_ref.shard_id,
			output_ref.as_of_txid,
			0,
		);
		let Some(staged_blob) = tx_get_value(tx, &stage_key, Serializable).await? else {
			return Ok(rejected_hot_install("staged hot shard is missing"));
		};
		if output_ref.size_bytes != u64::try_from(staged_blob.len()).unwrap_or(u64::MAX)
			|| output_ref.content_hash != content_hash(&staged_blob)
		{
			return Ok(rejected_hot_install("staged hot shard checksum mismatch"));
		}
		staged_blobs.push((output_ref.clone(), staged_blob));
	}
	#[cfg(feature = "test-faults")]
	if let Some(output) = hot_install_fault_output(
		input.database_branch_id,
		HotCompactionFaultPoint::InstallAfterStagedRead,
	)
	.await?
	{
		return Ok(output);
	}

	#[cfg(feature = "test-faults")]
	if let Some(output) = hot_install_fault_output(
		input.database_branch_id,
		HotCompactionFaultPoint::InstallBeforeShardPublish,
	)
	.await?
	{
		return Ok(output);
	}
	for (output_ref, staged_blob) in &staged_blobs {
		tx.informal().set(
			&keys::branch_shard_key(
				input.database_branch_id,
				output_ref.shard_id,
				output_ref.as_of_txid,
			),
			staged_blob,
		);
	}
	#[cfg(feature = "test-faults")]
	if let Some(output) = hot_install_fault_output(
		input.database_branch_id,
		HotCompactionFaultPoint::InstallAfterShardPublishBeforePidxClear,
	)
	.await?
	{
		return Ok(output);
	}

	for (key, value) in &hot_inputs.pidx_entries {
		let pgno = decode_branch_pidx_pgno(input.database_branch_id, key)?;
		let shard_id = pgno / keys::SHARD_SIZE;
		if !latest_staged_shards.contains(&shard_id) {
			return Ok(rejected_hot_install(
				"missing staged hot shard for PIDX row",
			));
		}
		decode_pidx_txid(value)?;
	}

	for (key, value) in &hot_inputs.pidx_entries {
		udb::compare_and_clear(tx, key, value);
	}

	for selection in &hot_inputs.pitr_interval_coverage {
		tx.informal().set(
			&keys::branch_pitr_interval_key(input.database_branch_id, selection.bucket_start_ms),
			&encode_pitr_interval_coverage(selection.coverage.clone())
				.context("encode sqlite PITR interval coverage for hot install")?,
		);
	}

	#[cfg(feature = "test-faults")]
	if let Some(output) = hot_install_fault_output(
		input.database_branch_id,
		HotCompactionFaultPoint::InstallBeforeRootUpdate,
	)
	.await?
	{
		return Ok(output);
	}
	let next_root = CompactionRoot {
		schema_version: root.schema_version,
		manifest_generation: root.manifest_generation.saturating_add(1),
		hot_watermark_txid: root
			.hot_watermark_txid
			.max(input.input_range.txids.max_txid),
		cold_watermark_txid: root.cold_watermark_txid,
		cold_watermark_versionstamp: root.cold_watermark_versionstamp,
	};
	tx.informal().set(
		&keys::branch_compaction_root_key(input.database_branch_id),
		&encode_compaction_root(next_root)
			.context("encode sqlite compaction root for hot install")?,
	);
	#[cfg(feature = "test-faults")]
	if let Some(output) = hot_install_fault_output(
		input.database_branch_id,
		HotCompactionFaultPoint::InstallAfterRootUpdate,
	)
	.await?
	{
		return Ok(output);
	}

	Ok(InstallHotJobOutput {
		status: CompactionJobStatus::Succeeded,
		output_refs: input.output_refs.clone(),
	})
}

fn rejected_hot_install(reason: impl Into<String>) -> InstallHotJobOutput {
	InstallHotJobOutput {
		status: CompactionJobStatus::Rejected {
			reason: reason.into(),
		},
		output_refs: Vec::new(),
	}
}

#[cfg(feature = "test-faults")]
async fn hot_install_before_staged_read_fault_output(
	tx: &universaldb::Transaction,
	input: &InstallHotJobInput,
) -> Result<Option<InstallHotJobOutput>> {
	match test_hooks::maybe_fire_hot_compaction_fault(
		input.database_branch_id,
		HotCompactionFaultPoint::InstallBeforeStagedRead,
	)
	.await
	{
		Ok(Some(fired)) if fired.action == DepotFaultAction::DropArtifact => {
			for output_ref in &input.output_refs {
				tx.informal()
					.clear(&keys::branch_compaction_stage_hot_shard_key(
						input.database_branch_id,
						input.job_id,
						output_ref.shard_id,
						output_ref.as_of_txid,
						0,
					));
			}
			Ok(None)
		}
		Ok(Some(_)) | Ok(None) => Ok(None),
		Err(err) => Ok(Some(failed_hot_install(err))),
	}
}

#[cfg(feature = "test-faults")]
async fn hot_install_fault_output(
	database_branch_id: DatabaseBranchId,
	point: HotCompactionFaultPoint,
) -> Result<Option<InstallHotJobOutput>> {
	match test_hooks::maybe_fire_hot_compaction_fault(database_branch_id, point).await {
		Ok(Some(_)) | Ok(None) => Ok(None),
		Err(err) => Ok(Some(failed_hot_install(err))),
	}
}

#[cfg(feature = "test-faults")]
fn failed_hot_install(err: anyhow::Error) -> InstallHotJobOutput {
	InstallHotJobOutput {
		status: CompactionJobStatus::Failed {
			error: err.to_string(),
		},
		output_refs: Vec::new(),
	}
}
