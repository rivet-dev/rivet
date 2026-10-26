//! Serializable transition state, without runtime tasks or storage clients.
use super::types::*;
use rivet_object_store_util::conditional_write::ControlIo;
use rivet_state_machine::OpSequence;
use rivet_topology::{ownership::MissingHeartbeatObservation, worker::WorkerIdentity};

use serde::{Deserialize, Serialize};

/// Common I/O token; its deadline remains available to the executor's dispatch gate.
pub use rivet_state_machine_util::IoCall;
/// Exact conditional request retained until a read proves its outcome or supersedes its base.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mutation {
	/// Retries must never mint another tenure while this claim can still commit.
	Acquire(ClaimAttempt),
	/// Safe scope termination has already completed; preserve the observed ETag and commit ID.
	Release {
		observed: ObservedLease,
		commit_id: [u8; 16],
	},
	/// Announce that candidates may claim this tenure `step_down_ms` after the store stamps the
	/// write. The owner's revocation deadline is armed before the first send and outlives this
	/// request, because the write may land even if no readback ever confirms it.
	StepDown {
		observed: ObservedLease,
		commit_id: [u8; 16],
		step_down_ms: u64,
	},
	/// Cancel a step-down while its revocation deadline is still ahead. It is conditioned on
	/// the latest manifest version, whether or not that version shows the announcement, so once
	/// it lands no earlier announcement conditioned on an older version can land. It fails once
	/// a candidate has claimed.
	WithdrawStepDown {
		observed: ObservedLease,
		commit_id: [u8; 16],
	},
}

/// Ownership inspection and publication share one serial control lane.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ControlState {
	/// No control I/O or unresolved mutation.
	Idle,
	/// Ordinary ownership inspection, before choosing a conditional mutation.
	ReadingManifest { call: IoCall<OpId> },
	/// Prove the previous owner's heartbeat expired before preparing a claim.
	ReadingOwnerHeartbeat {
		call: IoCall<OpId>,
		lease: ObservedLease,
		read_started_store_lower_unix_ms: Option<i64>,
	},
	/// Exact mutation, readback, backoff and suspension are owned by the embedded helper.
	Publishing(rivet_object_store_util::conditional_write::ConditionalWrite<Mutation, OpId>),
	/// Sample jitter for the next check.
	SamplingJitter { call: IoCall<OpId> },
	/// No I/O is pending; the check alarm is armed at `check_at_boot_ms`, the next known
	/// deadline plus `jitter_ms`. An input that moves the deadline earlier re-arms it; a later
	/// deadline never postpones an armed check, or frequent inputs could starve it.
	Waiting {
		jitter_ms: u64,
		check_at_boot_ms: u64,
	},
}

impl ControlState {
	/// Projection used by runtime dispatch guards; never a second operation registry.
	pub fn pending(&self) -> Option<&IoCall<OpId>> {
		self.io().pending()
	}

	/// Maps domain activities without duplicating their operation tokens or publication state.
	fn io(&self) -> ControlIo<'_, Mutation, OpId> {
		match self {
			Self::ReadingManifest { call } => ControlIo::Reading(call),
			Self::ReadingOwnerHeartbeat { call, .. } | Self::SamplingJitter { call, .. } => {
				ControlIo::Other(call)
			}
			Self::Publishing(progress) => ControlIo::Publishing(progress),
			Self::Idle | Self::Waiting { .. } => ControlIo::Idle,
		}
	}

	/// Original conditional request remains available after suspension.
	pub fn mutation(&self) -> Option<&Mutation> {
		match self {
			Self::Publishing(progress) => Some(progress.request()),
			Self::Idle
			| Self::ReadingManifest { .. }
			| Self::ReadingOwnerHeartbeat { .. }
			| Self::SamplingJitter { .. }
			| Self::Waiting { .. } => None,
		}
	}

	/// Claim diagnostics do not maintain a second copy of its identity/precondition.
	pub fn claim(&self) -> Option<&ClaimAttempt> {
		match self.mutation() {
			Some(Mutation::Acquire(claim)) => Some(claim),
			Some(
				Mutation::Release { .. }
				| Mutation::StepDown { .. }
				| Mutation::WithdrawStepDown { .. },
			)
			| None => None,
		}
	}

	/// Read response routing for both ordinary inspection and publication readback.
	pub fn reading(&self) -> bool {
		self.io().reading()
	}

	/// Suspended publication has retained evidence but no local dispatch authority.
	pub fn suspended(&self) -> bool {
		matches!(self, Self::Publishing(progress) if progress.suspended())
	}
}

/// Lifetime of the local singleton services started for one coordinator tenure.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ServicesState {
	/// No singleton services were started for a known tenure.
	Absent,

	/// Services may act only while the latest snapshot and worker lease both permit it.
	Running(CoordinatorTenure),

	/// A step-down announcement has been sent, confirmed or not, and services keep acting until
	/// `revoke_at_boot_ms`, one margin before the deadline counted from the first send; the
	/// step-down alarm then stops the scope. Only a confirmed withdrawal returns to Running, and
	/// a repeated announcement never moves the deadline later.
	SteppingDown {
		tenure: CoordinatorTenure,
		revoke_at_boot_ms: u64,
	},

	/// Permission was revoked before dispatching cancellation/join work.
	Stopping { tenure: CoordinatorTenure },

	/// Cleanup is safe; read a fresh shared manifest before attempting conditional release.
	SafeToRelease(CoordinatorTenure),
}

/// The latest definite manifest read. It is the If-None-Match base for the next read and the
/// origin of the manifest check interval; an Unknown read leaves it untouched.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ManifestView {
	/// None when the read found no manifest, so the next claim may conditionally create it.
	pub lease: Option<ObservedLease>,
	/// Delivery time of the read; the next manifest check is due one interval later.
	pub read_boot_ms: u64,
}

/// Latest evidence about the manifest owner's process. It lets a candidate wait until the
/// first instant the takeover check could pass instead of inspecting on a tick, and marks a
/// handover in progress so the manifest is re-read promptly.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OwnerHeartbeat {
	/// The heartbeat was absent; the takeover wait runs from the first missing observation.
	Missing(MissingHeartbeatObservation),
	/// The heartbeat was present; it cannot expire before `modified_unix_ms` plus the takeover wait.
	/// A draining owner is about to release the manifest, so the manifest is re-read at jitter cadence.
	Live {
		identity: WorkerIdentity,
		modified_unix_ms: i64,
		draining: bool,
	},
	/// The inspection failed before any usable evidence; the next one retries at jitter cadence.
	Unknown { identity: WorkerIdentity },
}

/// A released tenure this candidate leaves to other candidates until `until_boot_ms`, after
/// an operator stepped it down. Without it the holder the operator removed would usually win
/// the election it just vacated, since it reads its own release first.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Yielding {
	pub tenure: CoordinatorTenure,
	pub until_boot_ms: u64,
}

/// Pure coordinator state; application catalogs remain entirely inside the persistence adapter.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct State {
	/// Immutable Topology identity of this candidate worker.
	pub identity: WorkerIdentity,
	/// Validated operational bounds shared with the executor.
	pub config: Config,
	/// Whether the latest topology view permits this worker to seek singleton tenure.
	pub eligible: bool,
	/// Store-clock evidence used when inspecting a previous owner's heartbeat.
	pub store_clock: Option<rivet_topology::heartbeat::StoreClock>,
	/// Clock stamped by the runner at the latest event delivery.
	pub now_boot_ms: u64,
	/// Serialized adapter progress, including any unresolved conditional request.
	pub control: ControlState,
	/// Allocates correlation IDs only; the control phase owns pending work.
	pub sequence: OpSequence<Slot>,
	/// Independent singleton lifetime and its exact coordinator tenure.
	pub services: ServicesState,
	/// Random identity for the next claim, and the prefix of every commit ID.
	pub attempt_id: [u8; 16],
	/// Mutations published by this candidate, which makes each commit ID unique.
	pub commits: u64,
	/// Retained manifest projection; see `ManifestView`.
	pub manifest: Option<ManifestView>,
	/// Heartbeat evidence about the manifest owner; see `OwnerHeartbeat`.
	pub owner_heartbeat: Option<OwnerHeartbeat>,
	/// The owner a fresh storage listing stopped showing as Active, until the runtime reports it
	/// listed again. A hint to re-read the manifest at jitter cadence, kept apart from heartbeat
	/// evidence so an inspection already in flight cannot erase it; never takeover evidence.
	pub owner_unlisted: Option<WorkerIdentity>,
	/// Shutdown is terminal for this candidate, while step-down alone allows later candidacy.
	pub shutdown: bool,
	/// The tenure an operator stepped this candidate down from, which it does not reclaim
	/// for a while; see `Yielding`.
	pub yielding: Option<Yielding>,
	/// Local lease expiry is irreversible even if a delayed view has a later deadline.
	pub lease: rivet_state_machine_util::LeaseWatch<Timer>,
}

impl State {
	/// Construction does not start any background tasks or storage operations.
	pub fn new(identity: WorkerIdentity, config: Config) -> anyhow::Result<Self> {
		config.validate()?;
		Ok(Self {
			identity,
			config,
			eligible: false,
			store_clock: None,
			now_boot_ms: 0,
			control: ControlState::Idle,
			sequence: OpSequence::default(),
			services: ServicesState::Absent,
			attempt_id: [0; 16],
			commits: 0,
			manifest: None,
			owner_heartbeat: None,
			owner_unlisted: None,
			shutdown: false,
			yielding: None,
			lease: rivet_state_machine_util::LeaseWatch::new(Timer::WorkerLease),
		})
	}
}
