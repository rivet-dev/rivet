//! Leadership protocol shared with the persistence and singleton-scope adapters.
use rivet_state_machine_util::IoTimer;
use rivet_topology::{heartbeat::StoreClock, worker::WorkerIdentity};
use serde::{Deserialize, Serialize};

/// One responsibility per regional cluster, never one lease per singleton service.
pub const COORDINATOR_LEASE_NAME: &str = "coordinator";

/// Authority of one coordinator tenure; target-specific fencing must enforce it at remote effects.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CoordinatorTenure {
	/// Shared Topology worker identity and process incarnation.
	pub owner: WorkerIdentity,
	/// Advances on every new ownership tenure, independently of catalog commits.
	pub coordinator_generation: u64,
	/// Identifies this claim during readback after an uncertain conditional write.
	pub claim_id: [u8; 16],
}

/// Why a running tenure was asked to stop; preserved in serialized coordinator events.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelinquishReason {
	/// An operator requests step-down while the worker may remain eligible for later candidacy.
	StepDownRequested,

	/// The service controller observed singleton services exit or could not start them.
	ServicesExited,
}

/// A projection of the shared manifest. The adapter preserves all application metadata on writes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ObservedLease {
	/// Last ownership tenure, retained after release so generations cannot reset.
	pub tenure: CoordinatorTenure,
	/// True only after the previous scope safely stopped and conditional release committed.
	pub released: bool,
	/// Opaque version of the single publication object, currently the S3 ETag.
	pub version: String,
	/// Generation-matched operator intent: the owner answers a pending request with a
	/// step-down announcement and keeps serving until shortly before its deadline.
	pub release_request: ReleaseRequest,
	/// Unique per write, so no two writes have the same body. The ETag is a hash of the body, so
	/// a repeated body would let a conditional read or write match a version that was replaced.
	/// A readback confirms a mutation by finding its own commit ID.
	pub commit_id: [u8; 16],
	/// Store time at which this version was written, from Last-Modified, which may be truncated
	/// to the second. The step-down anchor.
	pub modified_unix_ms: i64,
	/// A candidate may claim once its store-time lower bound reaches `modified_unix_ms` plus
	/// this; 0 when the owner is not stepping down. See `Config::step_down_ms`.
	pub step_down_ms: u64,
}

impl ObservedLease {
	/// Store time from which a candidate may claim a stepping-down tenure, if one was announced.
	pub fn step_down_at_unix_ms(&self) -> Option<i64> {
		if self.released || self.step_down_ms == 0 {
			return None;
		}
		Some(
			self.modified_unix_ms
				.saturating_add(i64::try_from(self.step_down_ms).unwrap_or(i64::MAX)),
		)
	}
}

/// An operator's durable request that the current tenure hand over. It is written on the
/// exact manifest version the operator read, so it names one holder, and it stays on the
/// manifest until that holder's release clears it. Clearing it before the holder revokes its
/// services withdraws the step-down.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReleaseRequest {
	/// No active request for this tenure.
	None,
	/// The holder announces a step-down at its next manifest read and hands over at the deadline.
	Pending,
}

/// Exact-key persistence response. Unknown includes malformed manifests and transport failures.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LeaseRead {
	/// A validated manifest projection with its conditional version.
	Found(ObservedLease),
	/// An explicit NotFound permits conditional creation.
	Missing,
	/// Never permits acquisition or proves that a previous write failed.
	Unknown,
}

/// Response to a manifest read conditioned on the version the coordinator already holds.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ManifestRead {
	/// The store proved the manifest still has the known version; the body was not transferred.
	Unchanged,
	/// The manifest changed, is missing, or could not be read; same meaning as an unconditional read.
	Read(LeaseRead),
}

/// Shared Topology evidence; notifications update this view but clocks enforce expiration.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct WorkerLeaseView {
	/// Storage role, Active heartbeat, and this process's registration identity all match.
	pub eligible: bool,
	/// The last confirmed worker lease deadline; draining may retain it for safe release.
	pub deadline_boot_ms: Option<u64>,
	/// Topology has permanently fenced/stopped this worker lifetime.
	pub fenced: bool,
	/// Conservative object-store time used by the existing takeover check.
	pub store_clock: Option<StoreClock>,
}

impl WorkerLeaseView {
	/// Projects existing membership state; it neither creates a worker nor renews a second heartbeat.
	pub fn from_topology(
		snapshot: &rivet_topology::TopologySnapshot,
		identity: &WorkerIdentity,
	) -> Self {
		let same_storage = snapshot
			.worker_registration
			.as_ref()
			.is_some_and(|registration| {
				registration.identity == *identity
					&& registration.role == rivet_topology::worker::WorkerRole::Storage
			});
		Self {
			eligible: same_storage
				&& snapshot.heartbeat_state == Some(rivet_topology::HeartbeatState::Active),
			deadline_boot_ms: match snapshot.status {
				rivet_topology::LeaseStatus::Live {
					deadline_boot_ms, ..
				} if same_storage => Some(deadline_boot_ms),
				rivet_topology::LeaseStatus::Live { .. }
				| rivet_topology::LeaseStatus::Observer
				| rivet_topology::LeaseStatus::Registering
				| rivet_topology::LeaseStatus::Releasing { .. }
				| rivet_topology::LeaseStatus::Fenced { .. }
				| rivet_topology::LeaseStatus::Stopped => None,
			},
			fenced: matches!(
				snapshot.status,
				rivet_topology::LeaseStatus::Fenced { .. }
					| rivet_topology::LeaseStatus::Stopped
					| rivet_topology::LeaseStatus::Releasing
			),
			store_clock: snapshot.store_clock.clone(),
		}
	}

	/// A snapshot is insufficient without a fresh monotonic clock.
	pub fn live(&self, now_boot_ms: u64) -> bool {
		!self.fenced && self.deadline_boot_ms.is_some_and(|end| now_boot_ms < end)
	}
}

/// Operational bounds; no extra coordinator heartbeat or service-specific lease interval.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Config {
	/// Timeout for one adapter effect, including uncertainty readback.
	pub io_timeout_ms: u64,
	/// Lower jitter bound between attempts and ownership checks.
	pub retry_min_ms: u64,
	/// Upper jitter bound, sampled through Env and delivered as an event.
	pub retry_max_ms: u64,
	/// Bound for fencing and joining the singleton scope before fail-stop.
	pub stop_timeout_ms: u64,
	/// The shared Topology lease TTL plus zombie grace, not a second lease duration.
	pub takeover_wait_ms: u64,
	/// Interval between conditional manifest reads for changes that have no notification: an
	/// operator step-down request, or a release by a worker whose heartbeat stays live. Owner
	/// expiry and announced step-downs are detected by their own deadlines.
	pub manifest_check_interval_ms: u64,
	/// Delay from a step-down announcement to its deadline. A candidate may claim the announced
	/// version once its store-time lower bound passes the announcement's stamp plus this. It
	/// must exceed one manifest check interval plus its jitter, a read timeout and the margin,
	/// so every candidate has read the announcement before the owner stops serving.
	pub step_down_ms: u64,
	/// How long before its deadline a stepping-down owner revokes its services, counted from
	/// the announcement's first send, which the store's stamp cannot precede by more than the
	/// stamp's truncation. It covers that truncation, the stop timeout within which the scope
	/// joins or the process exits, and clock-rate slack, so the old scope has ended before any
	/// candidate can claim. After the scope joins, the owner releases the manifest at once, so
	/// a candidate usually takes over well before the deadline.
	pub step_down_margin_ms: u64,
	/// Topology's bound on Last-Modified error, including truncation to the second. The margin
	/// must cover it.
	pub store_timestamp_uncertainty_ms: u64,
}

impl Default for Config {
	fn default() -> Self {
		Self::from_topology_timing(&rivet_topology::config::LeaseTiming::default())
	}
}

impl Config {
	/// Derive ownership safety waits from the shared worker lease rather than configuring another lease.
	pub fn from_topology_timing(timing: &rivet_topology::config::LeaseTiming) -> Self {
		let stop_timeout_ms = 5_000;
		let mut config = Self {
			io_timeout_ms: timing.io_timeout_ms,
			retry_min_ms: 250,
			retry_max_ms: 1000,
			stop_timeout_ms,
			takeover_wait_ms: timing.ttl_ms.saturating_add(timing.zombie_grace_ms),
			manifest_check_interval_ms: 30_000,
			step_down_ms: 0,
			step_down_margin_ms: Self::margin_for(
				timing.store_timestamp_uncertainty_ms,
				stop_timeout_ms,
			),
			store_timestamp_uncertainty_ms: timing.store_timestamp_uncertainty_ms,
		};
		config.step_down_ms = config.shortest_step_down_ms();
		config
	}

	/// The step-down margin for a store stamp uncertainty and stop timeout: both, plus one
	/// second of slack for clock-rate drift and the final effect of an exiting process.
	pub fn margin_for(store_timestamp_uncertainty_ms: u64, stop_timeout_ms: u64) -> u64 {
		store_timestamp_uncertainty_ms
			.saturating_add(stop_timeout_ms)
			.saturating_add(1_000)
	}

	/// The shortest valid step-down: every candidate reads the announcement within one check
	/// interval, its jitter and a read timeout, which must all end one second before the owner
	/// revokes its services.
	pub fn shortest_step_down_ms(&self) -> u64 {
		self.announcement_read_bound_ms().saturating_add(1_000)
	}

	/// Longest delay before every candidate has read a newly written manifest version, plus the
	/// margin by which the owner stops serving before its deadline.
	fn announcement_read_bound_ms(&self) -> u64 {
		self.manifest_check_interval_ms
			.saturating_add(self.retry_max_ms)
			.saturating_add(self.io_timeout_ms)
			.saturating_add(self.step_down_margin_ms)
	}

	/// Startup derives takeover_wait_ms from Topology's validated timing configuration.
	pub fn validate(&self) -> anyhow::Result<()> {
		anyhow::ensure!(
			self.io_timeout_ms > 0
				&& self.retry_min_ms > 0
				&& self.retry_max_ms >= self.retry_min_ms
				&& self.stop_timeout_ms > 0
				&& self.takeover_wait_ms > 0
				&& self.manifest_check_interval_ms > 0
				&& self.step_down_margin_ms > 0,
			"invalid coordinator timing"
		);
		// The old scope must have joined, or its process exited, before the deadline.
		anyhow::ensure!(
			self.step_down_margin_ms
				>= Self::margin_for(self.store_timestamp_uncertainty_ms, self.stop_timeout_ms),
			"step_down_margin_ms must cover the store stamp uncertainty, the stop timeout and 1 s of slack"
		);
		// Every candidate reads the manifest within one interval plus jitter and a read timeout,
		// and the owner must still be serving then so no candidate waits out its heartbeat.
		anyhow::ensure!(
			self.step_down_ms > self.announcement_read_bound_ms(),
			"step_down_ms must exceed the manifest check interval, jitter, read timeout and margin"
		);
		Ok(())
	}
}

/// One serialized I/O lane shared with the state machine core.
pub use rivet_state_machine::SingleSlot as Slot;
/// Local executor response correlation, distinct from the durable claim ID.
pub type OpId = rivet_state_machine::OpId<Slot>;

/// The runner delivers due alarms before inputs. Worker authority still requires fresh lease checks.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Timer {
	/// Embedded conditional mutation alarms; no independent runner or lease.
	Publication(rivet_object_store_util::conditional_write::Timer<OpId>),
	/// Starts the next ownership check: a manifest read or an owner heartbeat inspection, armed at
	/// the earliest known deadline plus jitter rather than on a fixed tick.
	Check,
	/// Bounds one adapter attempt; a replaced attempt ignores this operation ID.
	Io(IoTimer<OpId>),
	/// Fences at the worker lease deadline even without a Topology notification.
	WorkerLease,
	/// Revokes a stepping-down owner's services one margin before its announced deadline. Armed
	/// when the announcement is first sent, before any readback confirms it.
	StepDown,
	/// Fails the process if singleton cleanup cannot establish safe termination.
	Stop,
}

/// Retained verbatim across uncertain writes so retries cannot mint competing local tenures.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClaimAttempt {
	/// None only for a definitely absent manifest; otherwise the exact prior manifest version.
	pub expected: Option<ObservedLease>,
	/// Proposed owner/generation/claim identity, reused until its outcome is known.
	pub tenure: CoordinatorTenure,
}
