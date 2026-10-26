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
	/// Per-family read floor over live Active and Draining storage workers, valid until
	/// `valid_until_boot_ms` like `storage_workers`.
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
	/// The last applied listing lacked at least one heartbeat value; its read floor is incomplete.
	pub(crate) discovery_partial: bool,
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
	/// or forwards verbatim: the minimum `maxRead` over live storage workers, capped by this
	/// binary's `maxWrite`. Does no I/O. Fails closed with `topology.version_unknown` once the
	/// storage view has expired, the same rule `Topology::storage_workers` enforces.
	pub fn version_for(
		&self,
		family: crate::versions::VersionFamily,
		now_boot_ms: u64,
	) -> anyhow::Result<u16> {
		if self.discovery_partial || now_boot_ms >= self.valid_until_boot_ms {
			return Err(crate::versions::VersionUnknown { family: family.0 }.build());
		}
		crate::versions::select(self.own_range(family).as_ref(), &self.read_floor, family)
	}

	/// The version a writer encodes `family` at: `version_for` while the storage view is
	/// current, and otherwise this binary's lowest write version. Does no I/O and never waits
	/// for a listing, so a write never fails because a listing is in flight.
	///
	/// The lowest write version is the previous one, which every binary of a one-step rolling
	/// deploy reads, so writing it while the view is unknown is always safe; it only delays the
	/// switch to the newer version until the next listing. Fails with
	/// `topology.version_unavailable` when this binary writes nothing every live storage
	/// worker reads, or does not know the family.
	pub fn write_version(
		&self,
		family: crate::versions::VersionFamily,
		now_boot_ms: u64,
	) -> anyhow::Result<u16> {
		if !self.discovery_partial && now_boot_ms < self.valid_until_boot_ms {
			return self.version_for(family, now_boot_ms);
		}
		crate::metrics::WRITE_VERSION_FALLBACKS
			.with_label_values(&[family.schema()])
			.inc();
		crate::versions::lowest_write(self.own_range(family).as_ref(), family)
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
		if matches!(
			self.status,
			LeaseStatus::Fenced { .. } | LeaseStatus::Stopped
		) {
			return None;
		}
		endpoint_hint(
			self.storage_workers
				.iter()
				.find(|worker| &worker.id == id)
				.cloned(),
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

/// Discovery wins unless that exact endpoint was invalidated; cached misses remain distinct.
pub(super) fn endpoint_hint(
	storage_worker: Option<Worker>,
	endpoints: &imbl::OrdMap<crate::worker::WorkerId, EndpointEntry>,
	invalidated: &imbl::OrdMap<crate::worker::WorkerId, Worker>,
	id: &crate::worker::WorkerId,
	now_boot_ms: u64,
) -> Option<Option<Worker>> {
	if let Some(worker) = storage_worker {
		if !invalidated
			.get(id)
			.is_some_and(|bad| same_endpoint(bad, &worker))
		{
			return Some(Some(worker));
		}
	}

	endpoints
		.get(id)
		.filter(|entry| now_boot_ms < entry.expires_at_boot_ms)
		.map(|entry| entry.worker.clone())
}
