use super::types::{EndpointEntry, LeaseStatus, WorkerRegistration};
use crate::{
	heartbeat::{HeartbeatState, StoreClock},
	worker::Worker,
};
use std::{collections::BTreeMap, sync::Arc};

/// One publication joins local lease status, discovery freshness, and endpoint hints.
/// Use the latest publication and a fresh clock at admission and reply; old copies cannot reflect early fencing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TopologySnapshot {
	/// Ordinary and publication deadlines, copied from the sole current I/O records.
	pub(crate) calls: Vec<rivet_state_machine_util::IoCall<super::types::OperationId>>,

	/// Endpoint read generations and deadlines, excluding caller waiters.
	pub(crate) lookups: BTreeMap<crate::worker::WorkerId, rivet_state_machine_util::IoCall<u64>>,

	/// Storage inspection generations and deadlines; independent of endpoint reads.
	pub(crate) inspections:
		BTreeMap<crate::worker::WorkerId, rivet_state_machine_util::IoCall<u64>>,

	/// Absent for an observer; retained after stopping or fencing for diagnostics.
	pub worker_registration: Option<WorkerRegistration>,
	/// Local lease evidence used by both serving gates, independently of discovery freshness.
	pub status: LeaseStatus,
	/// Desired local phase, not confirmation of a persisted write; Draining denies new work.
	/// Observer and Fenced project None. Releasing and Stopped project Released even when no release PUT has confirmed.
	pub heartbeat_state: Option<HeartbeatState>,
	/// Active, unexpired storage workers. Excludes this worker while it drains.
	pub storage_workers: Vec<Worker>,
	/// First instant the view could be wrong: the earliest entry expiry, capped by the lease
	/// deadline. `Topology::storage_workers` refreshes once it passes.
	pub valid_until_boot_ms: u64,
	/// Per-family read floor over the Active and Draining storage workers discovery retains.
	/// Unlike `storage_workers`, version selection keeps using it past `valid_until_boot_ms`;
	/// see `version_for`.
	pub read_floor: Arc<crate::versions::ReadFloor>,
	/// Lower bound on store time for future ownership checks against heartbeat observations.
	pub store_clock: Option<StoreClock>,
	/// Positive and negative hints checked only after storage discovery misses.
	pub(crate) endpoints: imbl::OrdMap<crate::worker::WorkerId, EndpointEntry>,
	/// Failed endpoints hidden until their route changes or a later point read revalidates it.
	pub(crate) invalidated: imbl::OrdMap<crate::worker::WorkerId, Worker>,
	/// Lets `Topology::refresh` wait for the next listing response.
	pub(crate) discovery_round: u64,
	/// Lets `Topology::refresh` reject a failed listing even while prior entries remain cached.
	pub(crate) discovery_ok: bool,
	/// What the last completed listing established about the read floor; see
	/// `Topology::wait_for_read_floor`.
	pub(crate) floor: super::state::FloorEvidence,
	/// Worker processes watched through `Topology::watch`, one per process.
	pub watched_processes: usize,
}

impl TopologySnapshot {
	/// Data-path admission requires Active and an unexpired local lease.
	/// Check the latest snapshot with a fresh clock before starting work.
	pub fn may_admit(&self, now_boot_ms: u64) -> bool {
		self.heartbeat_state == Some(HeartbeatState::Active) && self.status.is_live(now_boot_ms)
	}

	/// Admission of new ownership: tablet grants, replica placements and volume opens. Like
	/// `may_admit`, and also refused once the confirmed renewal is `degrade_after_ms` old.
	pub fn may_take_ownership(&self, now_boot_ms: u64) -> bool {
		self.may_admit(now_boot_ms) && self.status.admits_new_ownership(now_boot_ms)
	}

	/// Whether this snapshot admits work for exactly `worker` at `now_boot_ms`: the lease is
	/// live then and the registration names that worker.
	pub fn permits(&self, worker: &crate::worker::WorkerId, now_boot_ms: u64) -> bool {
		self.may_admit(now_boot_ms)
			&& self
				.worker_registration
				.as_ref()
				.is_some_and(|registration| &registration.identity.worker_id == worker)
	}

	/// Data-path replies require an unexpired lease, including while draining.
	/// Re-read the snapshot and clock after work completes so delayed notifications cannot extend service.
	pub fn may_reply(&self, now_boot_ms: u64) -> bool {
		self.status.is_live(now_boot_ms)
	}

	/// The version to write for persisted `family` data, or for a payload a receiver stores
	/// or forwards verbatim: the minimum `maxRead` over the storage workers discovery retains,
	/// capped by this binary's `maxWrite`. Does no I/O.
	///
	/// The floor is used whenever every storage worker the last listing named has been
	/// observed, by that listing or, when its heartbeat GET failed, by an earlier one whose
	/// observation discovery kept. It stays in use after the storage view expires, while
	/// listings fail. Both are safe because persisted schema upgrades are forward-only: a worker
	/// id only ever restarts on the same or a newer binary, so an older observation can only
	/// under-report what a worker reads, and the floor can only drop if an older binary
	/// registers after every older one is gone, which is a rollback.
	///
	/// Fails closed with `topology.version_unknown` before the first listing completes, and
	/// while the last completed listing named a worker no listing has observed: that worker could
	/// be an older binary.
	pub fn version_for(&self, family: crate::versions::VersionFamily) -> anyhow::Result<u16> {
		if !self.floor_selectable() {
			return Err(crate::versions::VersionUnknown { family: family.0 }.build());
		}
		crate::versions::select(self.own_range(family).as_ref(), &self.read_floor, family)
	}

	/// The version a writer encodes `family` at: `version_for` when the last completed listing
	/// observed every worker, and otherwise this binary's lowest write version. Does no I/O and never waits for a
	/// listing, so a write never fails because a listing is in flight.
	///
	/// The lowest write version is the previous one, which every binary of a one-step rolling
	/// deploy reads, so writing it while the floor is unknown is always safe; it only delays
	/// the switch to the newer version until a listing observes every worker. Fails with
	/// `topology.version_unavailable` when this binary writes nothing every counted storage
	/// worker reads, or does not know the family.
	pub fn write_version(&self, family: crate::versions::VersionFamily) -> anyhow::Result<u16> {
		if self.floor_selectable() {
			return self.version_for(family);
		}
		crate::metrics::WRITE_VERSION_FALLBACKS
			.with_label_values(&[family.schema()])
			.inc();
		crate::versions::lowest_write(self.own_range(family).as_ref(), family)
	}

	/// Whether the last completed listing observed every storage worker it listed.
	fn floor_selectable(&self) -> bool {
		self.floor == super::state::FloorEvidence::Observed
	}

	/// Whether the last completed listing observed every storage worker it listed, so version
	/// selection uses the fleet's read floor rather than the lowest write version.
	pub fn floor_observed(&self) -> bool {
		self.floor_selectable()
	}

	/// Whether a listing has completed in this process's lifetime, whatever it observed. Once
	/// true it stays true. Before it, version selection falls back to the lowest write version
	/// without any listing behind it; after it, the fallback rests on a listing too.
	pub fn floor_listed(&self) -> bool {
		self.floor != super::state::FloorEvidence::Unlisted
	}

	/// This binary's advertised range for `family`: its registration's, or the compiled one
	/// for an observer.
	fn own_range(
		&self,
		family: crate::versions::VersionFamily,
	) -> Option<crate::versions::FamilyVersions> {
		match &self.worker_registration {
			Some(registration) => {
				rivet_data::families::advertised(&registration.versions, family).cloned()
			}
			None => family
				.info()
				.map(rivet_data::families::FamilyInfo::versions),
		}
	}

	/// The version for a message to `peer` that it decodes, acts on and drops:
	/// `min(own maxWrite, peer maxRead)`. Fails with `topology.unsupported_version` when the
	/// peer reads nothing this binary writes.
	pub fn version_for_peer(
		&self,
		family: crate::versions::VersionFamily,
		peer: &Worker,
	) -> anyhow::Result<u16> {
		match &self.worker_registration {
			Some(registration) => {
				crate::versions::for_peer(&registration.versions, &peer.versions, family)
			}
			None => crate::versions::for_peer(
				&crate::versions::compiled_versions(),
				&peer.versions,
				family,
			),
		}
	}

	/// Used by `Topology::resolve_worker` before requesting I/O: discovery wins over cached hints.
	/// `Some(None)` is a cached miss; endpoints never prove liveness or ownership.
	pub fn endpoint(
		&self,
		id: &crate::worker::WorkerId,
		now_boot_ms: u64,
	) -> Option<Option<Worker>> {
		self.endpoint_ref(id, now_boot_ms)
			.map(|worker| worker.cloned())
	}

	/// The version a message of `family` to the worker `id` is encoded at, by this snapshot's
	/// view of that worker: `None` when the view does not know it (a miss, a cached negative
	/// lookup, an expired hint, or a fenced or stopped topology), and otherwise
	/// `version_for_peer` for the worker as discovery or the endpoint cache knows it. Copies
	/// nothing and does no I/O, so hot paths may call it per message.
	///
	/// `now_boot_ms` expires only endpoint cache entries. A worker in the storage discovery list
	/// is read even after the list's `valid_until_boot_ms`, as `version_for` reads the floor.
	/// Upgrades are forward-only: a worker id that restarts runs the same or a
	/// newer binary, so a stale entry can only under-report the version it reads, and a message
	/// encoded at that version still decodes. An under-reported version is not free, though:
	/// `known_older` reads the same value, so a storage worker that hosts envoys and was upgraded
	/// since discovery listed it has forked starts skipped with `HostWithoutForks` (the start
	/// fails, and the actor retries on its next wake) until discovery refreshes.
	pub fn peer_version(
		&self,
		family: crate::versions::VersionFamily,
		id: &crate::worker::WorkerId,
		now_boot_ms: u64,
	) -> Option<anyhow::Result<u16>> {
		let peer = self.endpoint_ref(id, now_boot_ms).flatten()?;

		Some(self.version_for_peer(family, peer))
	}

	/// `endpoint` without copying the worker.
	fn endpoint_ref(
		&self,
		id: &crate::worker::WorkerId,
		now_boot_ms: u64,
	) -> Option<Option<&Worker>> {
		if matches!(
			self.status,
			LeaseStatus::Fenced { .. } | LeaseStatus::Stopped
		) {
			return None;
		}
		endpoint_hint(
			self.storage_workers.iter().find(|worker| &worker.id == id),
			&self.endpoints,
			&self.invalidated,
			id,
			now_boot_ms,
		)
	}
}

pub(crate) fn same_endpoint(a: &Worker, b: &Worker) -> bool {
	a.id == b.id && a.instance_id == b.instance_id && a.api_peer_address == b.api_peer_address
}

/// Whether the discovered `worker` may be used: true unless that exact endpoint was invalidated.
pub(super) fn discovery_usable(
	worker: &Worker,
	invalidated: &imbl::OrdMap<crate::worker::WorkerId, Worker>,
) -> bool {
	!invalidated
		.get(&worker.id)
		.is_some_and(|bad| same_endpoint(bad, worker))
}

/// Discovery wins unless that exact endpoint was invalidated; cached misses remain distinct.
/// Borrows from the snapshot or state, so a caller that needs only a field copies nothing.
pub(super) fn endpoint_hint<'a>(
	storage_worker: Option<&'a Worker>,
	endpoints: &'a imbl::OrdMap<crate::worker::WorkerId, EndpointEntry>,
	invalidated: &imbl::OrdMap<crate::worker::WorkerId, Worker>,
	id: &crate::worker::WorkerId,
	now_boot_ms: u64,
) -> Option<Option<&'a Worker>> {
	if let Some(worker) = storage_worker {
		if discovery_usable(worker, invalidated) {
			return Some(Some(worker));
		}
	}

	endpoints
		.get(id)
		.filter(|entry| now_boot_ms < entry.expires_at_boot_ms)
		.map(|entry| entry.worker.as_ref())
}
