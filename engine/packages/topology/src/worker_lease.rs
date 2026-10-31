//! The lease status of one worker, read by exact id. Endpoint lookups return any heartbeat that
//! is not released, which a crashed worker keeps until heartbeat GC removes it, so callers that
//! must tell a live worker from a dead one read its lease status here instead.
//!
//! A verdict is judged by store timestamps. `Expired` follows the same-id takeover rule: the
//! heartbeat's `Last-Modified` is older than `ttl_ms + zombie_grace_ms` plus the store timestamp
//! uncertainty, measured against a lower bound of store time. A delayed reply or a late
//! heartbeat cannot turn a cached `Expired` back into `Live`. This is a cached read, never a
//! topology state machine transition, and must not feed lease renewal.
use crate::{
	heartbeat::{HeartbeatState, Read, read_heartbeat},
	worker::{WorkerId, WorkerInstanceId, WorkerRole},
};
use object_store::ObjectStore;
use rivet_metrics::{GaugeGuardExt, IntGaugeGuard};
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorkerLease {
	/// The heartbeat is within its lease. A hint for routing, cached briefly.
	Live,
	/// The heartbeat is older than the takeover bound, or gone.
	Expired,
	/// The worker released its lease.
	Released,
	/// The read failed, the store clock is not known yet, or the lookup was over capacity.
	Unknown,
}

impl WorkerLease {
	fn label(self) -> &'static str {
		match self {
			WorkerLease::Live => "live",
			WorkerLease::Expired => "expired",
			WorkerLease::Released => "released",
			WorkerLease::Unknown => "unknown",
		}
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct WorkerLeaseConfig {
	/// Cached verdicts. Reaching it evicts the least recently used.
	pub cache_capacity: u64,
	/// Lifetime of a cached `Expired` or `Released` verdict.
	pub terminal_ttl_ms: u64,
	/// Lease reads in flight. A lookup above this returns `Unknown` instead of waiting.
	pub request_capacity: usize,
}

impl Default for WorkerLeaseConfig {
	fn default() -> Self {
		Self {
			cache_capacity: 4096,
			terminal_ttl_ms: 60_000,
			request_capacity: 64,
		}
	}
}

/// A cached verdict, the process instance of the heartbeat it judged, and the boot time it
/// expires at. `_entry` counts the cache entry on `WORKER_LEASE_CACHE_ENTRIES` for as long as
/// any copy of it is alive; a replacement carries the old entry's guard forward.
#[derive(Clone)]
struct Cached {
	lease: WorkerLease,
	instance_id: Option<WorkerInstanceId>,
	expires_at_boot_ms: u64,
	_entry: Arc<IntGaugeGuard>,
}

pub(crate) struct WorkerLeases {
	cache: moka::future::Cache<WorkerId, Cached>,
	admission: Arc<Semaphore>,
	config: WorkerLeaseConfig,
	_capacity_metric: IntGaugeGuard,
}

impl WorkerLeases {
	pub(crate) fn new(config: WorkerLeaseConfig) -> Self {
		let capacity_metric =
			crate::metrics::WORKER_LEASE_CACHE_CAPACITY.add_guard(config.cache_capacity as i64);
		Self {
			cache: moka::future::Cache::builder()
				.max_capacity(config.cache_capacity)
				.build(),
			admission: Arc::new(Semaphore::new(config.request_capacity.max(1))),
			config,
			_capacity_metric: capacity_metric,
		}
	}

	/// Returns `id`'s cached verdict while it is fresh, without any I/O.
	pub(crate) async fn cached(&self, id: &WorkerId, now_boot_ms: u64) -> Option<WorkerLease> {
		self.cache
			.get(id)
			.await
			.filter(|cached| now_boot_ms < cached.expires_at_boot_ms)
			.map(|cached| cached.lease)
	}

	/// Returns `id`'s lease status, from the cache while its verdict is fresh, with the process
	/// instance recorded in the heartbeat that verdict judged, when one was found. The two come
	/// from one read, so a caller can tell whether a live lease belongs to a particular process.
	///
	/// `store_now_unix_ms` is a lower bound of store time, or `None` before topology has one.
	/// The heartbeat is read in `first_role` first. A `Live` verdict is cached until the
	/// heartbeat's lease would end (`LeaseTiming::hint_ttl_ms`), and at least `live_ttl_ms`.
	#[allow(clippy::too_many_arguments)]
	pub(crate) async fn lookup_instance(
		&self,
		store: &dyn ObjectStore,
		id: &WorkerId,
		now_boot_ms: u64,
		store_now_unix_ms: Option<i64>,
		lease: &crate::config::LeaseTiming,
		live_ttl_ms: u64,
		first_role: WorkerRole,
	) -> (WorkerLease, Option<WorkerInstanceId>) {
		if let Some(cached) = self.cache.get(id).await
			&& now_boot_ms < cached.expires_at_boot_ms
		{
			record(cached.lease);
			return (cached.lease, cached.instance_id);
		}
		let Ok(_permit) = self.admission.clone().try_acquire_owned() else {
			record(WorkerLease::Unknown);
			return (WorkerLease::Unknown, None);
		};

		let (verdict, modified_unix_ms, instance_id) =
			read_lease(store, id, store_now_unix_ms, lease, first_role).await;
		let ttl_ms = match verdict {
			WorkerLease::Live => Some(match (modified_unix_ms, store_now_unix_ms) {
				(Some(modified), Some(store_now)) => {
					lease.hint_ttl_ms(modified, store_now, live_ttl_ms)
				}
				_ => live_ttl_ms,
			}),
			WorkerLease::Expired | WorkerLease::Released => Some(self.config.terminal_ttl_ms),
			WorkerLease::Unknown => None,
		};
		if let Some(ttl_ms) = ttl_ms {
			let expires_at_boot_ms = now_boot_ms.saturating_add(ttl_ms);
			self.cache
				.entry(id.clone())
				.and_upsert_with(|old| async move {
					let entry = match old {
						Some(old) => old.into_value()._entry,
						None => Arc::new(crate::metrics::WORKER_LEASE_CACHE_ENTRIES.inc_guard()),
					};
					Cached {
						lease: verdict,
						instance_id,
						expires_at_boot_ms,
						_entry: entry,
					}
				})
				.await;
		}
		record(verdict);
		(verdict, instance_id)
	}
}

fn record(lease: WorkerLease) {
	crate::metrics::WORKER_LEASE_LOOKUPS
		.with_label_values(&[lease.label()])
		.inc();
}

/// Reads the heartbeat of `id` by exact key in `first_role` and then the other role, and
/// judges it. Also returns the heartbeat's `Last-Modified` and process instance when one was
/// found.
pub(crate) async fn read_lease(
	store: &dyn ObjectStore,
	id: &WorkerId,
	store_now_unix_ms: Option<i64>,
	lease: &crate::config::LeaseTiming,
	first_role: WorkerRole,
) -> (WorkerLease, Option<i64>, Option<WorkerInstanceId>) {
	for role in crate::discovery::roles_from(first_role) {
		match read_heartbeat(store, role, id).await {
			Read::Missing => continue,
			Read::Unknown => return (WorkerLease::Unknown, None, None),
			Read::Found(entry) => {
				let verdict = judge(
					entry.heartbeat.state,
					entry.modified_unix_ms,
					store_now_unix_ms,
					lease,
				);
				return (
					verdict,
					Some(entry.modified_unix_ms),
					Some(WorkerInstanceId::from_bytes(entry.heartbeat.instance_id)),
				);
			}
		}
	}
	// Heartbeat GC removes only long-expired heartbeats, so a missing one is gone.
	(WorkerLease::Expired, None, None)
}

/// Judges one heartbeat by its store timestamp against a lower bound of store time.
pub fn judge(
	state: HeartbeatState,
	modified_unix_ms: i64,
	store_now_unix_ms: Option<i64>,
	lease: &crate::config::LeaseTiming,
) -> WorkerLease {
	match state {
		HeartbeatState::Released => WorkerLease::Released,
		HeartbeatState::Active | HeartbeatState::Draining => {
			let Some(store_now) = store_now_unix_ms else {
				return WorkerLease::Unknown;
			};
			let bound = lease
				.ttl_ms
				.saturating_add(lease.zombie_grace_ms)
				.saturating_add(lease.store_timestamp_uncertainty_ms);
			let age_ms = (store_now as i128 - modified_unix_ms as i128).max(0);
			if age_ms >= bound as i128 {
				WorkerLease::Expired
			} else {
				WorkerLease::Live
			}
		}
	}
}


