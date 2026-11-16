//! No-op stand-ins for the metrics the coordinator and maintainer state machines record.
//! The wasm visualization drives those machines with no Prometheus registry, so every
//! counter and histogram here discards its value. Gauges that a guard holds are real
//! gauges that no registry exports, because the guard type needs one.

use rivet_metrics::prometheus::IntGauge;

/// A counter or histogram, or one labelled series of either, that records nothing.
pub(crate) struct Discard;

impl Discard {
	pub(crate) fn with_label_values(&self, _labels: &[&str]) -> &Self {
		self
	}

	pub(crate) fn inc(&self) {}

	pub(crate) fn inc_by(&self, _value: u64) {}

	pub(crate) fn observe(&self, _value: f64) {}

	pub(crate) fn set(&self, _value: i64) {}
}

lazy_static::lazy_static! {
	pub(crate) static ref GROUP_RECORD_CACHE_ENTRIES: IntGauge = IntGauge::new("depot_group_record_cache_entries", "Unregistered in the wasm build.").unwrap();
	pub(crate) static ref GROUP_RECORD_CACHE_FLOORS: IntGauge = IntGauge::new("depot_group_record_cache_event_floors", "Unregistered in the wasm build.").unwrap();
}

pub(crate) static DELETE_DRAINS_APPLIED: Discard = Discard;
pub(crate) static FILESYSTEM_APPLIES_HELD: Discard = Discard;
pub(crate) static MAINTAINER_GRANT_RETURNS: Discard = Discard;
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

pub(crate) static ALARM_HINTS_DROPPED: Discard = Discard;
pub(crate) static DELETE_FORK_PINS: Discard = Discard;
pub(crate) static DELETE_PIN_TAIL_PROOFS: Discard = Discard;
pub(crate) static GROUP_ACQUIRE_PIN_WAITS: Discard = Discard;
pub(crate) static GROUP_ARCHIVE_CLOSES: Discard = Discard;
pub(crate) static GROUP_EVENT_LISTENERS: Discard = Discard;
pub(crate) static GROUP_EVENTS: Discard = Discard;
pub(crate) static GROUP_EVENTS_DROPPED: Discard = Discard;
pub(crate) static GROUP_EVENTS_FULL_PAGES: Discard = Discard;
pub(crate) static GROUP_EVENTS_POLL_ERRORS: Discard = Discard;
pub(crate) static GROUP_FORK_PINS: Discard = Discard;
pub(crate) static GROUP_LAZY_FORKS: Discard = Discard;
pub(crate) static GROUP_LAZY_FORKS_ABSENT_FORGOTTEN: Discard = Discard;
pub(crate) static GROUP_PIN_WAIT_CHILDREN_SKIPPED: Discard = Discard;
pub(crate) static GROUP_READS: Discard = Discard;
pub(crate) static GROUP_RECORD_CACHE: Discard = Discard;
pub(crate) static GROUP_RECORD_CACHE_CAPACITY: Discard = Discard;
pub(crate) static GROUP_RECORD_CACHE_INVALIDATIONS: Discard = Discard;
pub(crate) static GROUP_RECORD_READ_LAG_ENTRIES: Discard = Discard;
