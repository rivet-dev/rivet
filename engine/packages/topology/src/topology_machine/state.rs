use super::types::*;
use crate::config::{LeaseTiming, TopologyConfig};
use crate::heartbeat::{HeartbeatState, Observation, StoreClock};
use crate::worker::{Worker, WorkerId, WorkerRole};
use rivet_object_store_util::conditional_write::ConditionalWrite;
use rivet_state_machine::OpSequence;
use rivet_state_machine_util::IoCall;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::Arc};

/// Each variant carries only the data valid in that state. A live lease always has a
/// confirmed write; terminal states retain the registration for diagnostics.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LeaseState {
	/// Started without a registration. Discovers routes but never serves.
	Observer,
	/// Claims the local heartbeat through `ReadLocalHeartbeatRequest` and conditional writes.
	/// `Timer::Registration` bounds startup; serving requires timely confirmation of the original claim.
	Registering {
		worker_registration: WorkerRegistration,
		/// `Draining` if `BeginDrain` arrived during registration.
		participation: Participation,
		/// Set by startup; checked while processing registration alarms.
		registration_deadline_boot_ms: u64,
		phase: RegistrationPhase,
	},
	/// A confirmed claim or renewal permits serving until its send-time deadline.
	/// `Renew` and `Refresh` maintain the lease and placement view; drain still renews.
	Live {
		worker_registration: WorkerRegistration,
		/// Desired participation; `BeginDrain` changes admission before the draining write confirms.
		participation: Participation,
		confirmed_write: ConfirmedHeartbeatWrite,
		/// Writes issued since `confirmed_write`, in ascending sequence. Each CASes on the
		/// same etag, so at most one can land; a readback decides which.
		unconfirmed_writes: Vec<HeartbeatWrite>,
		/// Renewal writes from `unconfirmed_writes` whose requests still run after a fresh
		/// attempt replaced them. At most `ttl_ms / heartbeat_attempt_ms`: each runs only until
		/// the lease deadline at its send. Cleared by every confirmation, since all of them CAS
		/// on the ETag that confirmation replaced.
		superseded_writes: Vec<SupersededWrite>,
		phase: RenewalPhase,
		/// When the registration claim confirmed. Until `ttl_ms` past it, another writer may
		/// still hold a storage view listed before this worker registered, so listings check
		/// this worker's versions against the others during that window only.
		claimed_boot_ms: u64,
		/// Diagnostics only. Set when half the serving window passed since the confirmed
		/// write's send time without a newer confirmation, and cleared by a renewal that
		/// confirms before the lease reaches half its window again. Keeps the late-renewal
		/// warning to one per episode.
		renewal_late: bool,
	},
	/// Cleanup is complete; only release reconciliation may run from this point.
	Releasing {
		worker_registration: WorkerRegistration,
		/// Shared I/O retains the original release request until confirmation or reconciliation.
		progress: ConditionalWrite<ReleaseAttempt, OperationId>,
		/// Original confirmed lease deadline, never extended by release writes or readbacks.
		deadline_boot_ms: u64,
	},
	/// Terminal. Renewal stops and the heartbeat expires naturally.
	Fenced {
		worker_registration: Option<WorkerRegistration>,
		reason: FenceReason,
	},
	/// `StopRequest` ends observer or registering work immediately; a live lease stops after release confirms.
	/// The snapshot alone is not proof that a Released heartbeat was persisted.
	Stopped {
		worker_registration: Option<WorkerRegistration>,
	},
}

impl LeaseState {
	pub(super) fn worker_registration(&self) -> Option<&WorkerRegistration> {
		match self {
			Self::Observer => None,
			Self::Registering {
				worker_registration,
				..
			}
			| Self::Live {
				worker_registration,
				..
			}
			| Self::Releasing {
				worker_registration,
				..
			} => Some(worker_registration),
			Self::Fenced {
				worker_registration,
				..
			}
			| Self::Stopped {
				worker_registration,
			} => worker_registration.as_ref(),
		}
	}

	pub(super) fn heartbeat_state(&self) -> Option<HeartbeatState> {
		match self {
			Self::Registering { participation, .. } | Self::Live { participation, .. } => {
				Some((*participation).into())
			}
			Self::Releasing { .. } | Self::Stopped { .. } => Some(HeartbeatState::Released),
			Self::Observer | Self::Fenced { .. } => None,
		}
	}

	pub(super) fn status(&self, timing: &LeaseTiming) -> LeaseStatus {
		match self {
			Self::Observer => LeaseStatus::Observer,
			Self::Registering { .. } => LeaseStatus::Registering,
			Self::Live {
				confirmed_write, ..
			} => LeaseStatus::Live {
				deadline_boot_ms: confirmed_write
					.write
					.issued_boot_ms
					.saturating_add(timing.serving_ms()),
				degrade_at_boot_ms: confirmed_write
					.write
					.issued_boot_ms
					.saturating_add(timing.degrade_after_ms()),
			},
			Self::Releasing { .. } => LeaseStatus::Releasing,
			Self::Fenced { reason, .. } => LeaseStatus::Fenced { reason: *reason },
			Self::Stopped { .. } => LeaseStatus::Stopped,
		}
	}

	pub(super) fn terminal(&self) -> bool {
		matches!(self, Self::Fenced { .. } | Self::Stopped { .. })
	}
}

/// Storage worker view used by placement. Absence from it is not evidence of expiry.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DiscoveryState {
	/// None before the first successful list; Some([]) is a successful empty list.
	pub entries: Option<Vec<Observation>>,
	/// Live workers anchor on their own confirmed heartbeat; observers use the newest
	/// listed modification time.
	pub store_clock: Option<StoreClock>,
	/// Rate-limits `Event::Refresh`.
	pub(super) last_list_issued_boot_ms: Option<u64>,
	/// When the listing that produced `entries` was issued. An entry older than `ttl_ms` is
	/// dropped only when this listing was issued after the entry's expiry; see
	/// `refresh_discovery`.
	pub(super) entries_listed_boot_ms: Option<u64>,
	/// Some entry is past its expiry but its listing predates that expiry, so a newer listing
	/// must confirm it before discovery drops it. The next listing starts sooner while set.
	pub(super) awaiting_expiry_evidence: bool,
	/// Listings that failed in a row, for the backoff of retries awaiting expiry evidence.
	pub(super) failed_listings: u32,
	/// Validity of the discovery view filtered during the latest transition.
	pub valid_until_boot_ms: u64,
	/// Increments on every listing response, so `Topology::refresh` can wait for one.
	pub round: u64,
	/// Whether the latest listing succeeded.
	pub ok: bool,
	/// The last applied listing retained observations after unreadable heartbeat GETs.
	/// Version selection cannot use its incomplete read floor.
	pub partial: bool,
	/// Read floor over the retained Active and Draining storage entries. Recomputed only
	/// when the retained entries change, so snapshots share it.
	pub read_floor: Arc<crate::versions::ReadFloor>,
}

/// Owns endpoint hints, invalidations, and pending exact-key reads.
/// Cached hints and invalidations are persistent maps: a snapshot shares their structure,
/// and an update while a snapshot is held copies only the path to the changed entry.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LookupState {
	/// Bounded by `endpoint_capacity`; the soonest-expiring entry is evicted first.
	/// Change it only through the `lookup` helpers, which keep `entry_expiry` in step.
	pub entries: imbl::OrdMap<WorkerId, EndpointEntry>,
	/// Each cached entry's `expires_at_boot_ms`, so expiry and eviction visit only due or
	/// head entries instead of the whole cache.
	pub(super) entry_expiry: rivet_state_machine::IndexedDeadlineQueue<WorkerId>,
	/// Listed endpoints hidden by `InvalidateEndpointRequest` until discovery changes them.
	pub invalidated: imbl::OrdMap<WorkerId, Worker>,
	/// Shares the `lookup_concurrency` cap with `heartbeats`.
	pub pending: BTreeMap<WorkerId, PendingLookup>,
	/// Inspection reads from `InspectHeartbeatRequest`.
	pub heartbeats: BTreeMap<WorkerId, PendingInspection>,
	/// Shared generation counter for lookups and inspections.
	pub next_generation: u64,
}

/// Holds no clock reading or timer mirror. Handlers receive the event's `now_boot_ms`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct State {
	/// Validated before the runner starts; bounds discovery, lookup admission, and lease timing.
	pub config: TopologyConfig,
	/// Sole source of local serving permission; discovery entries never grant a lease.
	pub lease: LeaseState,
	/// Live worker deadline alarm; registration and release retain their separate bounds.
	pub worker_lease: rivet_state_machine_util::LeaseWatch<Timer>,
	/// Snapshot placement input, refreshed by `ListStorageWorkersResponse`.
	pub discovery: DiscoveryState,
	/// Endpoint hints and inspection requests stay separate from local lease operations.
	pub lookups: LookupState,
	/// Watched worker processes, for low-urgency cleanup; see `watch`.
	pub watches: super::watch::WatchState,
	/// Rejects stale heartbeat and discovery completions.
	pub(super) sequence: OpSequence<Slot>,

	/// Sole in-flight records for ordinary heartbeat and discovery I/O.
	pub(super) calls: BTreeMap<Slot, IoCall<OperationId>>,

	/// Publication helpers own pending work; this sequence survives claim and release replacement.
	pub(super) publication_sequence: OpSequence<Slot>,
}

impl State {
	/// Returns the correlated ordinary call without duplicating its token or deadline.
	pub(super) fn pending(&self, slot: Slot) -> Option<OperationId> {
		self.calls.get(&slot).map(|call| call.op)
	}

	/// Removes only the current ordinary call; late completions cannot consume a replacement.
	pub(super) fn complete(&mut self, op: OperationId) -> bool {
		if self.pending(op.slot) != Some(op) {
			return false;
		}
		self.calls.remove(&op.slot);
		true
	}

	/// Projects the conditional helper's own call alongside ordinary I/O.
	pub(super) fn pending_calls(&self) -> Vec<IoCall<OperationId>> {
		let publication = match &self.lease {
			LeaseState::Registering {
				phase: RegistrationPhase::Publishing(progress),
				..
			} => progress.pending(),
			LeaseState::Releasing { progress, .. } => progress.pending(),
			LeaseState::Observer
			| LeaseState::Live { .. }
			| LeaseState::Fenced { .. }
			| LeaseState::Stopped { .. }
			| LeaseState::Registering {
				phase:
					RegistrationPhase::ReadingHeartbeat { .. } | RegistrationPhase::Waiting { .. },
				..
			} => None,
		};
		self.calls.values().chain(publication).cloned().collect()
	}

	pub fn new(
		config: TopologyConfig,
		worker_registration: Option<WorkerRegistration>,
	) -> anyhow::Result<Self> {
		config.validate()?;
		anyhow::ensure!(
			worker_registration
				.as_ref()
				.is_none_or(|r| r.storage_capacity_millicores > 0),
			"storage capacity must be positive"
		);

		let lease = match worker_registration {
			Some(worker_registration) => LeaseState::Registering {
				worker_registration,
				participation: Participation::Active,
				registration_deadline_boot_ms: 0,
				phase: RegistrationPhase::ReadingHeartbeat {
					context: HeartbeatReadContext::Initial,
				},
			},
			None => LeaseState::Observer,
		};

		Ok(Self {
			config,
			lease,
			worker_lease: rivet_state_machine_util::LeaseWatch::new(Timer::WorkerLease),
			discovery: DiscoveryState::default(),
			lookups: LookupState::default(),
			watches: Default::default(),
			sequence: OpSequence::default(),
			calls: BTreeMap::new(),
			publication_sequence: OpSequence::default(),
		})
	}

	pub(super) fn discovering(&self) -> bool {
		matches!(self.lease, LeaseState::Observer | LeaseState::Live { .. })
	}

	// Expiration uses the event clock. Snapshots contain deadlines, never a cached clock reading.
	pub(super) fn refresh_discovery(&mut self, now_boot_ms: u64) {
		self.discovery.valid_until_boot_ms = 0;

		if !self.discovering() {
			return;
		}

		let Some(store_clock) = &self.discovery.store_clock else {
			return;
		};
		let Some(entries) = &mut self.discovery.entries else {
			return;
		};

		let store_now = store_clock.lower_bound(now_boot_ms);
		let ttl_ms = self.config.lease.ttl_ms;
		let listed_boot_ms = self.discovery.entries_listed_boot_ms.unwrap_or(0);
		let mut awaiting_expiry_evidence = false;

		let mut valid_until_boot_ms = match self.lease.status(&self.config.lease) {
			LeaseStatus::Live {
				deadline_boot_ms, ..
			} => deadline_boot_ms,
			LeaseStatus::Observer
			| LeaseStatus::Registering
			| LeaseStatus::Fenced { .. }
			| LeaseStatus::Stopped
			| LeaseStatus::Releasing { .. } => now_boot_ms.saturating_add(ttl_ms),
		};

		let retained_before = entries.len();
		entries.retain(|entry| {
			let age_ms = (store_now as i128 - entry.modified_unix_ms as i128).max(0) as u128;

			if age_ms >= ttl_ms as u128 {
				// Age alone is no evidence of expiry: the owner may have renewed after the listing
				// that produced this entry, and a renewal confirmed before its old deadline extends
				// it. Drop the entry only when that listing was issued after the entry expired, so
				// any renewal that landed before the expiry would have shown. A renewal landing
				// later is confirmed only after the owner's old deadline has fenced it, by the
				// safety margin. Until then keep the entry, outside the view's validity bound.
				let expired_unix_ms = entry.modified_unix_ms as i128 + ttl_ms as i128;
				let expired_boot_ms = (store_clock.at_boot_ms as i128 + expired_unix_ms
					- store_clock.lower_unix_ms as i128)
					.max(0) as u128;
				if listed_boot_ms as u128 >= expired_boot_ms {
					return false;
				}
				awaiting_expiry_evidence = true;
				return true;
			}

			if entry.role == WorkerRole::Storage {
				valid_until_boot_ms =
					valid_until_boot_ms.min(now_boot_ms.saturating_add(ttl_ms - age_ms as u64));
			}

			true
		});

		self.discovery.valid_until_boot_ms = valid_until_boot_ms;
		self.discovery.awaiting_expiry_evidence = awaiting_expiry_evidence;
		if entries.len() != retained_before {
			self.recompute_read_floor();
		}
	}

	/// Storage entries that still read persisted data: Active or Draining, not Released.
	/// A draining worker keeps serving reads until it releases, so it still counts.
	pub(super) fn reader_entries(&self) -> impl Iterator<Item = &Observation> + '_ {
		self.discovery.entries.iter().flatten().filter(|entry| {
			entry.role == WorkerRole::Storage && entry.heartbeat.state != HeartbeatState::Released
		})
	}

	/// Recomputes `discovery.read_floor` from the retained entries.
	pub(super) fn recompute_read_floor(&mut self) {
		let readers: Vec<&[crate::versions::FamilyVersions]> = self
			.reader_entries()
			.map(|entry| entry.heartbeat.versions.as_slice())
			.collect();
		let floor = crate::versions::ReadFloor::new(readers.iter().copied());
		self.discovery.read_floor = Arc::new(floor);
	}

	pub(super) fn storage_entries(
		&self,
		now_boot_ms: u64,
	) -> impl Iterator<Item = &Observation> + '_ {
		let store_now = self
			.discovery
			.store_clock
			.as_ref()
			.map(|clock| clock.lower_bound(now_boot_ms));
		let ttl_ms = self.config.lease.ttl_ms;
		self.current_storage_entries().filter(move |entry| {
			store_now.is_some_and(|now| {
				(now as i128 - entry.modified_unix_ms as i128).max(0) < ttl_ms as i128
			})
		})
	}

	/// Entries retained by the last transition; snapshots publish their validity deadline separately.
	pub(super) fn current_storage_entries(&self) -> impl Iterator<Item = &Observation> + '_ {
		let enabled = self.discovering();
		let draining_worker = if self.lease.heartbeat_state() == Some(HeartbeatState::Draining) {
			self.lease
				.worker_registration()
				.map(|registration| &registration.identity.worker_id)
		} else {
			None
		};

		self.discovery
			.entries
			.iter()
			.flatten()
			.filter(move |entry| {
				enabled
					&& entry.role == WorkerRole::Storage
					&& entry.heartbeat.state == HeartbeatState::Active
					&& draining_worker != Some(&entry.worker_id)
			})
	}
}
