//! Process-wide rejection counts include both inbox and core admission outcomes.

use rivet_data::generated::{
	depot_log_shard_v1::{DiskBatch, Frame},
	depot_storage_worker_protocol_v1::{RejectResponse, StorageWorkerResponse},
};
use rivet_metrics::prometheus::*;

lazy_static::lazy_static! {
	static ref BARRIER_WAIT_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"depot_log_shard_barrier_wait_seconds",
			"Time from a request entering the shard's inbox to its reply, by kind: frame (replica commands), read, status, listing, forget, incarnation or reject. Replies queued behind a disk write wait for it here.",
		)
		.buckets(rivet_metrics::START_BUCKETS.to_vec()),
		&["kind"],
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref BATCH_FILL_WAIT_SECONDS: Histogram = register_histogram_with_registry!(
		HistogramOpts::new(
			"depot_log_shard_batch_fill_wait_seconds",
			"Time a write's oldest frame waited in the filling batch before the write went out: behind the previous write and sync. 0 on an idle shard.",
		)
		.buckets(rivet_metrics::MICRO_BUCKETS.to_vec()),
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref DISK_WRITE_SECONDS: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"depot_log_shard_disk_write_seconds",
			"Duration of one segment write, by phase: write (appending the framed batches) or sync (fsync of the segment). Compare with depot_log_shard_batch_fill_wait_seconds: when the fill wait is a large share of write plus sync under load, commits are waiting on the in-flight sync.",
		)
		.buckets(rivet_metrics::MICRO_BUCKETS.to_vec()),
		&["phase"],
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref DISK_BATCH_BYTES: Histogram = register_histogram_with_registry!(
		HistogramOpts::new(
			"depot_log_shard_disk_batch_bytes",
			"Framed bytes of one segment write, every batch included.",
		)
		.buckets(rivet_metrics::BYTE_SIZE_BUCKETS.to_vec()),
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref DISK_BATCH_FRAMES: Histogram = register_histogram_with_registry!(
		HistogramOpts::new(
			"depot_log_shard_disk_batch_frames",
			"Frames in one segment write, every batch included.",
		)
		.buckets(rivet_metrics::PAGE_COUNT_BUCKETS.to_vec()),
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref DISK_BATCH_CAPPED: IntCounter = register_int_counter_with_registry!(
		Opts::new(
			"depot_log_shard_disk_batch_capped_total",
			"Segment writes within one maximum batch of the per-write cap (max_write_bytes), so the filling batch had no room for another full batch and further appends were refused as overloaded.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref STATUS_NOTICE_PENDING: IntGauge = register_int_gauge_with_registry!(
		Opts::new("depot_log_shard_status_notice_pending", "Replicas whose changed state waits for a notice to their maintainer."),
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref INITIAL_ENTRIES_DROPPED: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("depot_log_shard_initial_entries_dropped_total", "Create loads whose initial entries were not written with the new replica, by reason: existing, not_claimed, too_large or malformed. The opener then writes them itself."),
		&["reason"],
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref STATUS_NOTICES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("depot_log_shard_status_notices_total", "Replica status notices sent to maintainers, by result."),
		&["result"],
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref STATUS_NOTICE_REPLICAS: IntCounter = register_int_counter_with_registry!(
		Opts::new("depot_log_shard_status_notice_replicas_total", "Replica states carried by status notices sent to maintainers."),
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref DISK_READ_FAILURES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new("depot_log_shard_disk_read_failures_total", "Disk reads answered Overloaded instead of a page, by reason. The shard keeps serving."),
		&["reason"],
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref USAGE: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new("depot_log_shard_resource_usage", "Current shard resource usage."),
		&["resource"],
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref CAPACITY: IntGaugeVec = register_int_gauge_vec_with_registry!(
		Opts::new("depot_log_shard_resource_capacity", "Configured shard resource capacity."),
		&["resource"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref IO_DURATION: HistogramVec = register_histogram_vec_with_registry!(
		HistogramOpts::new(
			"depot_log_shard_io_duration_seconds",
			"Execution time of one serialized shard disk operation.",
		)
		.buckets(rivet_metrics::BUCKETS.to_vec()),
		&["operation"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref READ_BYTES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_log_shard_read_bytes_total",
			"Entry read bytes scanned from disk batches and returned to callers.",
		),
		&["kind"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref DISK_WRITTEN_BYTES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_log_shard_disk_written_bytes_total",
			"Bytes the shard wrote to its disk, by file kind.",
		),
		&["kind"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref DISK_SYNCS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_log_shard_disk_syncs_total",
			"File and directory syncs the shard issued, by kind.",
		),
		&["kind"],
		*rivet_metrics::REGISTRY
	).unwrap();

	/// Segment tails recovery found ended early: `torn_cut` when it cut an incomplete final
	/// frame, `zero_tail` when the newest segment of a file not sized as a preallocation has
	/// zeros past its last frame, which a disk that lost acknowledged pages also leaves,
	/// `unsynced_write` when a preallocated segment's final write did not fully reach the
	/// disk, and `lost_synced` when its batches end before the length its header records as
	/// synced, which marks the shard corrupt.
	pub(crate) static ref RECOVERY_TAILS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_log_shard_recovery_tail_total",
			"Segment tails log shard recovery found ending early, by kind.",
		),
		&["kind"],
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref RETAINED_ENTRIES: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"depot_log_shard_retained_entries",
			"Log entries above cold that the shard retains. Each keeps its header in memory; payload memory is the retained_payload_bytes resource.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref SEGMENT_PIN_AGE: Gauge = register_gauge_with_registry!(
		Opts::new(
			"depot_log_shard_segment_pin_age_seconds",
			"Time since the batch holding the oldest retained entry was synced: how long the oldest disk segment has been kept from collection. Zero when no retained entry is on disk.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref RETAINED_ENTRY_BYTES: IntGauge = register_int_gauge_with_registry!(
		Opts::new(
			"depot_log_shard_retained_entry_bytes",
			"Encoded bytes of the log entries above cold that the shard retains, payloads included whether held in memory or only on disk.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref DISK_FALLBACK_READS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_log_shard_disk_fallback_reads_total",
			"Entry reads served from disk because memory could not answer them, by reason.",
		),
		&["reason"],
		*rivet_metrics::REGISTRY
	).unwrap();

	pub(crate) static ref DISK_BATCHES: IntCounter = register_int_counter_with_registry!(
		Opts::new(
			"depot_log_shard_disk_batches_total",
			"Memory batches written to segments. Divide by segment syncs for batches per sync.",
		),
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref DISK_FRAMES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_log_shard_disk_frames_total",
			"Frames written to segments, by frame kind. Divide by segment syncs for frames per sync.",
		),
		&["kind"],
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref ADMISSION_WAIT_SECONDS: Histogram = register_histogram_with_registry!(
		HistogramOpts::new(
			"depot_log_shard_admission_wait_seconds",
			"Time a submission waited in the shard's inbox before the runner handled it. Past max_admission_wait_ms (the admission_wait_ms resource capacity) its requests are refused as overloaded; a sustained wait near it means the runner's core is saturated.",
		)
		.buckets(rivet_metrics::BUCKETS.to_vec()),
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref ADMISSION_SHED: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_log_shard_admission_shed_total",
			"Requests refused as overloaded before any work, by reason: queue_wait (waited past max_admission_wait_ms), inbox_full (inbox_capacity) or disk_drain (an append that would make the next disk write longer than max_append_admission_wait_ms at a slow disk's measured rate).",
		),
		&["reason"],
		*rivet_metrics::REGISTRY
	).unwrap();

	static ref REJECTIONS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_log_shard_rejections_total",
			"Log shard requests rejected by reason.",
		),
		&["reason"],
		*rivet_metrics::REGISTRY
	).unwrap();
}

/// Publishes the retained log tail totals from the latest shard snapshot.
pub(crate) fn observe_retained_entries(entries: u64, entry_bytes: u64) {
	RETAINED_ENTRIES.set(entries.min(i64::MAX as u64) as i64);
	RETAINED_ENTRY_BYTES.set(entry_bytes.min(i64::MAX as u64) as i64);
}

/// Uses a bounded label set so capacity, availability and protocol failures stay distinct.
pub(crate) fn record_rejection(reason: &RejectResponse) {
	let label = match reason {
		RejectResponse::DiskIncarnationMismatch(_) => "disk_incarnation_mismatch",
		RejectResponse::VolumeLogReplicaDeleted(_) => "volume_log_replica_deleted",
		RejectResponse::StaleClientEpoch(_) => "stale_client_epoch",
		RejectResponse::ClientConflict(_) => "client_conflict",
		RejectResponse::NotClaimed(_) => "not_claimed",
		RejectResponse::VolumeSeqGap(_) => "volume_seq_gap",
		RejectResponse::Diverged(_) => "diverged",
		RejectResponse::AppendedMismatch(_) => "appended_mismatch",
		RejectResponse::BelowCold(_) => "below_cold_volume_seq",
		RejectResponse::VolumeLogReplicaIncarnationMismatch(_) => {
			"volume_log_replica_incarnation_mismatch"
		}
		RejectResponse::StaleTabletReplicaSet(_) => "stale_tablet_replica_set",
		RejectResponse::StaleTabletGrantEpoch(_) => "stale_tablet_grant_epoch",
		RejectResponse::VolumeNotLoaded => "volume_not_loaded",
		RejectResponse::Overloaded(_) => "overloaded",
		RejectResponse::NotServing => "not_serving",
		RejectResponse::RequestTooLarge(_) => "request_too_large",
		RejectResponse::TooManyLoadedVolumes(_) => "too_many_loaded_volumes",
		RejectResponse::DiskFull(_) => "disk_full",
		RejectResponse::UnsupportedPayloadVersion(_) => "unsupported_payload_version",
	};
	REJECTIONS.with_label_values(&[label]).inc();
	crate::signals::request_refused(reason, label);
}

/// Publishes the age of the oldest pinned disk segment.
pub(crate) fn observe_segment_pin_age(age_ms: u64) {
	SEGMENT_PIN_AGE.set(age_ms as f64 / 1000.0);
}

/// A runner-owned observer publishes gauges and warns only on entry into the near-cap band.
pub(crate) fn observe(
	resources: impl IntoIterator<Item = crate::log_shard_machine::snapshot::ResourceUsage>,
	near: &mut std::collections::BTreeSet<crate::log_shard_machine::snapshot::Resource>,
) {
	for value in resources {
		USAGE
			.with_label_values(&[value.resource.label()])
			.set(value.used.min(i64::MAX as u64) as i64);
		CAPACITY
			.with_label_values(&[value.resource.label()])
			.set(value.capacity.min(i64::MAX as u64) as i64);
		if value.used > 0 && value.used.saturating_mul(10) >= value.capacity.saturating_mul(9) {
			if near.insert(value.resource) {
				tracing::warn!(
					resource = value.resource.label(),
					usage = value.used,
					capacity = value.capacity,
					"log shard resource near capacity"
				);
			}
		} else {
			near.remove(&value.resource);
		}
	}
}

/// Counts each written frame by its kind. The match is exhaustive so a new frame kind
/// cannot go uncounted.
pub(crate) fn record_disk_frames(batches: &[DiskBatch]) {
	let mut entry = 0;
	let mut state = 0;
	let mut forget = 0;
	let mut tablet_grant = 0;
	let mut tablet_replica_set = 0;
	for frame in batches.iter().flat_map(|batch| &batch.frames) {
		match frame {
			Frame::EntryFrame(_) => entry += 1,
			Frame::StateFrame(_) => state += 1,
			Frame::ForgetFrame(_) => forget += 1,
			Frame::TabletGrantFrame(_) => tablet_grant += 1,
			Frame::TabletReplicaSetFrame(_) => tablet_replica_set += 1,
		}
	}
	for (kind, count) in [
		("entry", entry),
		("state", state),
		("forget", forget),
		("tablet_grant", tablet_grant),
		("tablet_replica_set", tablet_replica_set),
	] {
		if count > 0 {
			DISK_FRAMES.with_label_values(&[kind]).inc_by(count);
		}
	}
}

pub(crate) fn observe_batch_fill_wait(fill_wait_ms: u64) {
	BATCH_FILL_WAIT_SECONDS.observe(fill_wait_ms as f64 / 1000.0);
}

/// Records how long one phase of a segment write took, in real time.
/// Starts a metrics-only timer. Browser builds have no std clock, so they time nothing.
pub(crate) fn phase_timer() -> Option<std::time::Instant> {
	#[cfg(target_arch = "wasm32")]
	{
		None
	}
	#[cfg(not(target_arch = "wasm32"))]
	{
		#[allow(clippy::disallowed_methods, reason = "metrics use real time")]
		Some(std::time::Instant::now())
	}
}

/// Seconds since `phase_timer`, or zero where no clock exists.
pub(crate) fn elapsed_seconds(started: Option<std::time::Instant>) -> f64 {
	started.map_or(0.0, |started| started.elapsed().as_secs_f64())
}

pub(crate) fn observe_disk_write_phase(phase: &'static str, seconds: f64) {
	DISK_WRITE_SECONDS
		.with_label_values(&[phase])
		.observe(seconds);
}

/// Records the size of one segment write and whether it reached the per-write cap.
pub(crate) fn observe_disk_write(bytes: u64, frames: usize, capped: bool) {
	DISK_BATCH_BYTES.observe(bytes as f64);
	DISK_BATCH_FRAMES.observe(frames as f64);
	if capped {
		DISK_BATCH_CAPPED.inc();
	}
}

pub(crate) fn observe_status_notices_pending(pending: usize) {
	STATUS_NOTICE_PENDING.set(pending as i64);
}

pub(crate) fn record_status_notice(outcome: &crate::log_shard_machine::types::NoticeOutcome) {
	STATUS_NOTICES.with_label_values(&[outcome.as_str()]).inc();
}

/// Counts the replica states one status notice request carries.
pub(crate) fn record_status_notice_replicas(replicas: usize) {
	STATUS_NOTICE_REPLICAS.inc_by(replicas as u64);
}

/// Counts one disk read that failed alone, without failing the shard.
pub(crate) fn record_disk_read_failure(
	reason: crate::log_shard_machine::state::reads::DiskReadFailure,
) {
	DISK_READ_FAILURES
		.with_label_values(&[reason.label()])
		.inc();
}

pub(crate) fn record_initial_entries_dropped(
	reason: crate::log_shard_machine::types::InitialEntriesDrop,
) {
	INITIAL_ENTRIES_DROPPED
		.with_label_values(&[reason.as_str()])
		.inc();
}

/// Observes one reply's time since its request entered the shard.
pub(crate) fn record_barrier_wait(response: &StorageWorkerResponse, elapsed: std::time::Duration) {
	let kind = match response {
		StorageWorkerResponse::ReplicaStateResponse(_) => "frame",
		StorageWorkerResponse::ReadVolumeLogEntriesResponse(_) => "read",
		StorageWorkerResponse::ReadVolumeLogStatusResponse(_) => "status",
		StorageWorkerResponse::ListTabletReplicasResponse(_) => "listing",
		StorageWorkerResponse::ForgetVolumesResponse(_) => "forget",
		StorageWorkerResponse::TrimVolumesResponse(_) => "trim",
		StorageWorkerResponse::GetDiskIncarnationResponse(_) => "incarnation",
		StorageWorkerResponse::RejectResponse(_) => "reject",
		StorageWorkerResponse::StorageWorkerBatchResponse(_) => "batch",
	};
	BARRIER_WAIT_SECONDS
		.with_label_values(&[kind])
		.observe(elapsed.as_secs_f64());
}

/// Why a request was refused before admission.
#[derive(Clone, Copy, Debug)]
pub(crate) enum AdmissionShed {
	QueueWait,
	InboxFull,
	/// An append that would make the next disk write longer than the append admission bound
	/// at the slow disk's measured rate.
	DiskDrain,
}

/// Records one submission's wait in the inbox.
pub(crate) fn observe_admission_wait(waited_ms: u64) {
	ADMISSION_WAIT_SECONDS.observe(waited_ms as f64 / 1000.0);
}

/// Counts requests refused before admission.
pub(crate) fn record_admission_shed(reason: AdmissionShed, requests: usize) {
	let label = match reason {
		AdmissionShed::QueueWait => "queue_wait",
		AdmissionShed::InboxFull => "inbox_full",
		AdmissionShed::DiskDrain => "disk_drain",
	};
	ADMISSION_SHED
		.with_label_values(&[label])
		.inc_by(requests as u64);
}
