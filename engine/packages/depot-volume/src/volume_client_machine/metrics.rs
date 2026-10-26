//! Payload bytes before and after the volume client's payload codec, stall alarms, and how
//! much log an open replays.

use super::state::ReplayStart;
use rivet_data::generated::depot_log_shard_v1::{PayloadCodec, VolumeLogEntryBody};
#[cfg(not(target_arch = "wasm32"))]
use rivet_metrics::prometheus::{
	IntCounter, IntCounterVec, Opts, register_int_counter_vec_with_registry,
	register_int_counter_with_registry,
};

// Metrics are native only; the wasm visualization drives the core without a registry.
#[cfg(not(target_arch = "wasm32"))]
lazy_static::lazy_static! {
	static ref PAYLOAD_BYTES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_volume_payload_bytes_total",
			"Payload bytes the volume client wrote, by stage: raw (before the codec) or encoded (as logged), and by the codec the entry records.",
		),
		&["stage", "codec"],
		*rivet_metrics::REGISTRY
	)
	.unwrap();
	static ref STALLS: IntCounter = register_int_counter_with_registry!(
		Opts::new(
			"depot_volume_client_stalls_total",
			"Stall alarms on serving volume clients: uncommitted history made no quorum progress within the stall timeout, so the client resent it from acknowledged positions and kept its epoch.",
		),
		*rivet_metrics::REGISTRY
	)
	.unwrap();
	static ref OPEN_REPLAYS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_volume_client_open_replays_total",
			"Opens that reached replay, by where replay started: none (the source ends at cold), applied (the maintainer's applied position) or cold (just above cold, because the maintainer reported no usable history).",
		),
		&["start"],
		*rivet_metrics::REGISTRY
	)
	.unwrap();
	static ref OPEN_REPLAY_ENTRIES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_volume_client_open_replay_entries_total",
			"Log entries open replays read from their source replica, by where replay started.",
		),
		&["start"],
		*rivet_metrics::REGISTRY
	)
	.unwrap();
	static ref OPEN_REPLAY_BYTES: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_volume_client_open_replay_bytes_total",
			"Encoded payload bytes open replays read from their source replica and released, by where replay started.",
		),
		&["start"],
		*rivet_metrics::REGISTRY
	)
	.unwrap();
	static ref PERMANENT_REFUSALS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_volume_client_permanent_refusals_total",
			"Replicas a volume client blocked on an entry their log shard refuses for a reason retries cannot change, by reason: entry_too_large (the entry alone exceeds the shard's write limit).",
		),
		&["reason"],
		*rivet_metrics::REGISTRY
	)
	.unwrap();
	static ref SHARD_REFUSED_WRITES: IntCounter = register_int_counter_with_registry!(
		Opts::new(
			"depot_volume_client_shard_refused_writes_total",
			"New writes a volume client refused before taking a position because a quorum of the volume's log shards was refusing appends from this process.",
		),
		*rivet_metrics::REGISTRY
	)
	.unwrap();
	static ref OPEN_CLAIMS: IntCounterVec = register_int_counter_vec_with_registry!(
		Opts::new(
			"depot_volume_client_open_claims_total",
			"Opens whose recovered history matched a quorum, by result: settled (served at once) or claim_entry (appended a claim entry first).",
		),
		&["result"],
		*rivet_metrics::REGISTRY
	)
	.unwrap();
}

#[cfg(not(target_arch = "wasm32"))]
fn replay_label(start: Option<ReplayStart>) -> &'static str {
	match start {
		None => "none",
		Some(ReplayStart::Applied) => "applied",
		Some(ReplayStart::Cold) => "cold",
	}
}

/// Counts one open reaching replay; `None` means there was nothing above cold to read.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn replay_started(start: Option<ReplayStart>) {
	OPEN_REPLAYS.with_label_values(&[replay_label(start)]).inc();
}

#[cfg(target_arch = "wasm32")]
pub(super) fn replay_started(_start: Option<ReplayStart>) {}

/// Counts one accepted replay page.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn replay_page(start: ReplayStart, entries: u64, bytes: u64) {
	let label = replay_label(Some(start));
	OPEN_REPLAY_ENTRIES
		.with_label_values(&[label])
		.inc_by(entries);
	OPEN_REPLAY_BYTES.with_label_values(&[label]).inc_by(bytes);
}

#[cfg(target_arch = "wasm32")]
pub(super) fn replay_page(_start: ReplayStart, _entries: u64, _bytes: u64) {}

/// Counts one open whose history matched a quorum.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn claim_finished(settled: bool) {
	let result = if settled { "settled" } else { "claim_entry" };
	OPEN_CLAIMS.with_label_values(&[result]).inc();
}

#[cfg(target_arch = "wasm32")]
pub(super) fn claim_finished(_settled: bool) {}

/// Counts one stall alarm on a serving volume client.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn stalled() {
	STALLS.inc();
}

#[cfg(target_arch = "wasm32")]
pub(super) fn stalled() {}

/// Counts one replica blocked on an entry its shard permanently refuses, by reason label.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn permanently_refused(reason: &'static str) {
	PERMANENT_REFUSALS.with_label_values(&[reason]).inc();
}

#[cfg(target_arch = "wasm32")]
pub(super) fn permanently_refused(_reason: &'static str) {}

/// Encodes a mutation body's payload with `codec` and counts its bytes before and after. A
/// payload that the codec does not shrink is logged unencoded.
pub(super) fn encode(codec: PayloadCodec, mut body: VolumeLogEntryBody) -> VolumeLogEntryBody {
	if let VolumeLogEntryBody::PayloadWrite(write) = &mut body {
		let raw = write.payload.len() as u64;
		depot_types::payload_codec::encode_write(codec, write);
		count(
			raw,
			write.payload.len() as u64,
			depot_types::payload_codec::label(write.codec),
		);
	}
	body
}

#[cfg(not(target_arch = "wasm32"))]
fn count(raw: u64, encoded: u64, label: &str) {
	PAYLOAD_BYTES.with_label_values(&["raw", label]).inc_by(raw);
	PAYLOAD_BYTES
		.with_label_values(&["encoded", label])
		.inc_by(encoded);
}

#[cfg(target_arch = "wasm32")]
fn count(_raw: u64, _encoded: u64, _label: &str) {}

/// Counts one new write refused because a quorum of its shards was refusing this process.
#[cfg(not(target_arch = "wasm32"))]
pub(super) fn record_shard_refused_write() {
	SHARD_REFUSED_WRITES.inc();
}

#[cfg(target_arch = "wasm32")]
pub(super) fn record_shard_refused_write() {}
