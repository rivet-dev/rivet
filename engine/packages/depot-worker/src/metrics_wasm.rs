//! No-op stand-ins for the metrics the coordinator and maintainer state machines record.
//! The wasm visualization drives those machines with no Prometheus registry, so every
//! counter and histogram here discards its value.

/// A counter or histogram, or one labelled series of either, that records nothing.
pub(crate) struct Discard;

impl Discard {
	pub(crate) fn with_label_values(&self, _labels: &[&str]) -> &Self {
		self
	}

	pub(crate) fn inc(&self) {}

	pub(crate) fn inc_by(&self, _value: u64) {}

	pub(crate) fn observe(&self, _value: f64) {}
}

pub(crate) static DELETE_DRAINS_APPLIED: Discard = Discard;
pub(crate) static DELETE_DRAIN_DURATION: Discard = Discard;
pub(crate) static RETIRED_LOG_REPLICAS_FORGOTTEN: Discard = Discard;
pub(crate) static TABLET_FLUSH_PIGGYBACKED: Discard = Discard;
pub(crate) static TABLET_FLUSH_RECORDS: Discard = Discard;
pub(crate) static TABLET_FS_READ_REJECTED: Discard = Discard;
pub(crate) static TABLET_SESSION_READS_PASSED_OVER: Discard = Discard;
pub(crate) static TABLET_FS_READ_EXPIRED: Discard = Discard;
pub(crate) static TABLET_VOLUME_OPEN_REJECTED: Discard = Discard;
pub(crate) static TABLET_MAINTAINER_FENCES: Discard = Discard;
pub(crate) static TABLET_FS_OPEN_APPLY_WAITS: Discard = Discard;
pub(crate) static TABLET_UNLOAD_FORGETS: Discard = Discard;
pub(crate) static TABLET_VOLUME_OPEN_EXPIRED: Discard = Discard;

pub(crate) fn record_notice_apply() {}

pub(crate) fn record_member_repair(_result: &'static str) {}

pub(crate) fn record_recovery_scan(_scan: &'static str, _trigger: &'static str) {}

pub(crate) fn record_status_observation(_source: &'static str) {}

pub(crate) fn record_idle_visits(_component: &'static str, _visits: usize) {}

pub(crate) static TABLET_FS_VOLUME_RECORD_MISSING: Discard = Discard;
pub(crate) static TABLET_FORK_REPLAY_RECORD_MISSING: Discard = Discard;
pub(crate) static TABLET_STORE_OPERATIONS_SLOW: Discard = Discard;

pub(crate) fn record_retention_check(_result: &'static str) {}
