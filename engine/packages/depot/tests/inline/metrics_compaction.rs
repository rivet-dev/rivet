//! Inline tests for the background-layer compaction pass metrics. These live inline because the
//! compaction job output types are crate-private.
//!
//! The pass counter is a process-global registry, so each test owns one pass kind end to end. Tests
//! sharing a kind would race on its label series under the default parallel test runner.

use std::time::Instant;

use anyhow::anyhow;

use super::*;
use crate::compaction::types::{
	CompactionJobStatus, HotJobInputRange, HotSliceOutput, HotStageCursor, InstallHotJobOutput,
	ReclaimFdbJobOutput, StageHotSliceOutput, TxidRange,
};

fn fold_bytes(destination: &str) -> u64 {
	SQLITE_COMPACTION_HOT_STAGED_BYTES_TOTAL
		.with_label_values(&[destination])
		.get()
}

fn pass_count(kind: &str, result: &str) -> u64 {
	SQLITE_COMPACTION_PASS_TOTAL
		.with_label_values(&[kind, result])
		.get()
}

fn hot_install(status: CompactionJobStatus, throttled: bool) -> Result<InstallHotJobOutput> {
	Ok(InstallHotJobOutput {
		status,
		resume_cursor: throttled.then_some(1),
		resume_cursor_segment_pgno: None,
		throttled,
		resume_shard_cursor: None,
		installed_shard_count: 2,
		installed_shard_bytes: 8192,
		copied_shard_bytes: 8192,
	})
}

fn hot_stage(status: CompactionJobStatus) -> StageHotSliceOutput {
	StageHotSliceOutput {
		status,
		slice: None,
		throttled: false,
		next_stage_cursor: None,
		admission_blocked: false,
		staged_bytes: 0,
		stalled_at_txid: None,
	}
}

fn hot_slice_output(staged_bytes: u64) -> HotSliceOutput {
	HotSliceOutput {
		input_range: HotJobInputRange {
			txids: TxidRange {
				min_txid: 1,
				max_txid: 2,
			},
			max_pgno_exclusive: None,
			coverage_txids: vec![2],
			max_pages: 1,
			max_bytes: 4096,
		},
		input_fingerprint: [0; 32],
		staged_bytes,
	}
}

fn reclaim(status: CompactionJobStatus, throttled: bool) -> Result<ReclaimFdbJobOutput> {
	Ok(ReclaimFdbJobOutput {
		status,
		output_refs: Vec::new(),
		throttled,
		has_more: false,
		admission_blocked: false,
	})
}

/// A throttled install is a budget no-op and an early-timeout install committed part of its window;
/// both get re-dispatched from a cursor, so neither may land on the `failed` series that alerting
/// reads. The terminal outcomes keep their existing labels so `failed` still means "hit an error".
#[test]
fn hot_install_pass_results() {
	let succeeded_before = pass_count(PASS_HOT_INSTALL, RESULT_SUCCEEDED);
	let copied_bytes_before = SQLITE_COMPACTION_HOT_INSTALL_COPIED_BYTES_TOTAL.get();
	let rejected_before = pass_count(PASS_HOT_INSTALL, RESULT_REJECTED);
	let failed_before = pass_count(PASS_HOT_INSTALL, RESULT_FAILED);
	let error_before = pass_count(PASS_HOT_INSTALL, RESULT_ERROR);
	let throttled_before = pass_count(PASS_HOT_INSTALL, RESULT_THROTTLED);
	let incomplete_before = pass_count(PASS_HOT_INSTALL, RESULT_INCOMPLETE);
	let installed_bytes_before = SQLITE_COMPACTION_HOT_INSTALLED_BYTES_TOTAL.get();

	record_hot_install(
		Instant::now(),
		&hot_install(CompactionJobStatus::Requested, true),
	);
	record_hot_install(
		Instant::now(),
		&hot_install(CompactionJobStatus::Requested, false),
	);
	record_hot_install(
		Instant::now(),
		&hot_install(CompactionJobStatus::Succeeded, false),
	);
	record_hot_install(
		Instant::now(),
		&hot_install(
			CompactionJobStatus::Rejected {
				reason: "branch went stale".to_string(),
			},
			false,
		),
	);
	record_hot_install(
		Instant::now(),
		&hot_install(
			CompactionJobStatus::Failed {
				error: "shard copy failed".to_string(),
			},
			false,
		),
	);
	record_hot_install(Instant::now(), &Err(anyhow!("activity error")));

	// A direct install publishes the same volume having rewritten none of it. Install reports what it
	// published either way, so installed-over-folded is 1.0 in both modes and measures nothing; the
	// copied counter is the one that carries the second write.
	let mut direct = hot_install(CompactionJobStatus::Succeeded, false).expect("output");
	direct.installed_shard_bytes = 8192;
	direct.copied_shard_bytes = 0;
	record_hot_install(Instant::now(), &Ok(direct));

	assert_eq!(
		pass_count(PASS_HOT_INSTALL, RESULT_THROTTLED),
		throttled_before + 1
	);
	assert_eq!(
		pass_count(PASS_HOT_INSTALL, RESULT_INCOMPLETE),
		incomplete_before + 1
	);
	assert_eq!(
		pass_count(PASS_HOT_INSTALL, RESULT_SUCCEEDED),
		succeeded_before + 2
	);
	assert_eq!(
		SQLITE_COMPACTION_HOT_INSTALL_COPIED_BYTES_TOTAL.get(),
		copied_bytes_before + 8192,
		"only the staging install wrote its image a second time"
	);
	assert_eq!(
		pass_count(PASS_HOT_INSTALL, RESULT_REJECTED),
		rejected_before + 1
	);
	assert_eq!(
		pass_count(PASS_HOT_INSTALL, RESULT_FAILED),
		failed_before + 1
	);
	assert_eq!(pass_count(PASS_HOT_INSTALL, RESULT_ERROR), error_before + 1);

	// Only succeeded calls contribute. A throttled or resumable call hands its running total back to
	// the manager to re-report on the call that finishes, so counting it here would count the same
	// bytes once per re-dispatch. Both succeeded calls published 8192, the staging one by rewriting it
	// and the direct one by adopting it in place.
	assert_eq!(
		SQLITE_COMPACTION_HOT_INSTALLED_BYTES_TOTAL.get(),
		installed_bytes_before + 8192 + 8192
	);
}

#[test]
fn reclaim_pass_results() {
	let failed_before = pass_count(PASS_RECLAIM_FDB, RESULT_FAILED);
	let throttled_before = pass_count(PASS_RECLAIM_FDB, RESULT_THROTTLED);

	record_reclaim_fdb(
		Instant::now(),
		&reclaim(CompactionJobStatus::Requested, true),
	);

	assert_eq!(pass_count(PASS_RECLAIM_FDB, RESULT_FAILED), failed_before);
	assert_eq!(
		pass_count(PASS_RECLAIM_FDB, RESULT_THROTTLED),
		throttled_before + 1
	);
}

/// Volume counters must only advance on a successful put so they measure bytes actually moved into
/// cold storage, while the duration histogram covers both outcomes so a slow failing tier stays
/// visible.
#[test]
fn cold_object_upload_volume_counts_successes_only() {
	let objects_before = SQLITE_COMPACTION_COLD_OBJECTS_UPLOADED_TOTAL.get();
	let bytes_before = SQLITE_COMPACTION_COLD_UPLOAD_BYTES_TOTAL.get();
	let failed_duration_before = SQLITE_COMPACTION_COLD_UPLOAD_DURATION
		.with_label_values(&[RESULT_FAILED])
		.get_sample_count();

	record_cold_object_upload(Instant::now(), 4096, &Ok(()));
	record_cold_object_upload(Instant::now(), 8192, &Err(anyhow!("upload failed")));

	assert_eq!(
		SQLITE_COMPACTION_COLD_OBJECTS_UPLOADED_TOTAL.get(),
		objects_before + 1
	);
	assert_eq!(
		SQLITE_COMPACTION_COLD_UPLOAD_BYTES_TOTAL.get(),
		bytes_before + 4096
	);
	assert_eq!(
		SQLITE_COMPACTION_COLD_UPLOAD_DURATION
			.with_label_values(&[RESULT_FAILED])
			.get_sample_count(),
		failed_duration_before + 1
	);
}

/// Staging reports `Succeeded` for three different things, so the pass label has to read the output
/// shape rather than the status alone. A drain-end no-op and a slice left partly staged at the early
/// timeout both landing on `succeeded` would make the success rate meaningless: the first is how
/// every drain ends, and the second makes one wide slice look like several successful passes.
#[test]
fn hot_stage_pass_results() {
	let succeeded_before = pass_count(PASS_HOT_STAGE, RESULT_SUCCEEDED);
	let drained_before = pass_count(PASS_HOT_STAGE, RESULT_DRAINED);
	let incomplete_before = pass_count(PASS_HOT_STAGE, RESULT_INCOMPLETE);
	let throttled_before = pass_count(PASS_HOT_STAGE, RESULT_THROTTLED);
	let admission_before = pass_count(PASS_HOT_STAGE, RESULT_ADMISSION_BLOCKED);
	let stalled_before = pass_count(PASS_HOT_STAGE, RESULT_STALLED);
	let rejected_before = pass_count(PASS_HOT_STAGE, RESULT_REJECTED);
	let error_before = pass_count(PASS_HOT_STAGE, RESULT_ERROR);
	let staged_bytes_before = fold_bytes(FOLD_DESTINATION_STAGING);
	let shard_fold_bytes_before = fold_bytes(FOLD_DESTINATION_SHARD);

	// A slice staged in full.
	let mut staged = hot_stage(CompactionJobStatus::Succeeded);
	staged.slice = Some(hot_slice_output(4096));
	staged.staged_bytes = 4096;
	record_hot_stage(Instant::now(), &Ok(staged), false);

	// The drain asked for one more slice and there was none left.
	record_hot_stage(
		Instant::now(),
		&Ok(hot_stage(CompactionJobStatus::Succeeded)),
		false,
	);

	// A slice left partly staged at the early timeout. Its rows are durable, so its bytes count.
	let mut partial = hot_stage(CompactionJobStatus::Succeeded);
	partial.next_stage_cursor = Some(HotStageCursor {
		as_of_txid: 9,
		shard_id: 2,
	});
	partial.staged_bytes = 1024;
	record_hot_stage(Instant::now(), &Ok(partial), false);

	// A drain window whose first commit the slice budget cannot admit. Shaped exactly like the
	// drained pass above (Succeeded, no slice, no cursor), so only `stalled_at_txid` separates a
	// branch that finished from one whose hot lane can never advance.
	let mut stalled = hot_stage(CompactionJobStatus::Succeeded);
	stalled.stalled_at_txid = Some(4242);
	record_hot_stage(Instant::now(), &Ok(stalled), false);

	let mut throttled = hot_stage(CompactionJobStatus::Requested);
	throttled.throttled = true;
	record_hot_stage(Instant::now(), &Ok(throttled), false);

	let mut admission_blocked = hot_stage(CompactionJobStatus::Requested);
	admission_blocked.admission_blocked = true;
	record_hot_stage(Instant::now(), &Ok(admission_blocked), false);

	record_hot_stage(
		Instant::now(),
		&Ok(hot_stage(CompactionJobStatus::Rejected {
			reason: "branch went stale".to_string(),
		})),
		false,
	);
	record_hot_stage(Instant::now(), &Err(anyhow!("activity error")), false);

	// The same slice folded straight into the shard tier. It stages nothing, so its bytes belong to
	// the other destination even though every other label on the pass is identical.
	let mut direct = hot_stage(CompactionJobStatus::Succeeded);
	direct.slice = Some(hot_slice_output(2048));
	direct.staged_bytes = 2048;
	record_hot_stage(Instant::now(), &Ok(direct), true);

	assert_eq!(
		pass_count(PASS_HOT_STAGE, RESULT_SUCCEEDED),
		succeeded_before + 2
	);
	assert_eq!(
		pass_count(PASS_HOT_STAGE, RESULT_DRAINED),
		drained_before + 1
	);
	// The stalled pass must not land on `drained`; that is the whole point of the label.
	assert_eq!(
		pass_count(PASS_HOT_STAGE, RESULT_STALLED),
		stalled_before + 1
	);
	assert_eq!(
		pass_count(PASS_HOT_STAGE, RESULT_INCOMPLETE),
		incomplete_before + 1
	);
	assert_eq!(
		pass_count(PASS_HOT_STAGE, RESULT_THROTTLED),
		throttled_before + 1
	);
	assert_eq!(
		pass_count(PASS_HOT_STAGE, RESULT_ADMISSION_BLOCKED),
		admission_before + 1
	);
	assert_eq!(
		pass_count(PASS_HOT_STAGE, RESULT_REJECTED),
		rejected_before + 1
	);
	assert_eq!(pass_count(PASS_HOT_STAGE, RESULT_ERROR), error_before + 1);

	// Both the full slice and the early-timeout remainder contribute, and the direct fold does not:
	// it wrote no staging rows, so reporting it here would claim a staging write that never happened.
	assert_eq!(
		fold_bytes(FOLD_DESTINATION_STAGING),
		staged_bytes_before + 4096 + 1024
	);
	assert_eq!(
		fold_bytes(FOLD_DESTINATION_SHARD),
		shard_fold_bytes_before + 2048
	);
}
