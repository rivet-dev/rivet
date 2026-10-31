use crate::{
	heartbeat::HeartbeatState,
	worker::{Worker, WorkerIdentity, WorkerRole},
};
use rivet_object_store_util::conditional_write::ConditionalWrite;
use rivet_state_machine::OpId;
use serde::{Deserialize, Serialize};

/// Desired local participation while registering or live; release is a separate lease phase.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Participation {
	/// Admit new work only with a fresh, confirmed heartbeat lease.
	Active,
	/// Deny new admissions while finishing owned work and continuing lease renewal.
	Draining,
}

impl From<Participation> for HeartbeatState {
	fn from(participation: Participation) -> Self {
		match participation {
			Participation::Active => Self::Active,
			Participation::Draining => Self::Draining,
		}
	}
}

/// Published lease status used by the snapshot admission and reply gates.
/// `is_live` requires a fresh clock reading, including when notifications are delayed.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LeaseStatus {
	/// Discovers storage workers without a lease. Never serves.
	Observer,
	/// Registration is accepted, but serving waits for a claim confirmed before the registration deadline.
	Registering,
	/// May serve while `now_boot_ms < deadline_boot_ms`, and take on new ownership only while
	/// `now_boot_ms < degrade_at_boot_ms`. Both count from the confirmed write's send time, so a
	/// later confirmed renewal moves both forward and ends a degraded phase.
	Live {
		deadline_boot_ms: u64,
		degrade_at_boot_ms: u64,
	},
	/// Terminal. Late responses cannot revive the lease.
	Fenced { reason: FenceReason },
	/// No longer serving or renewing; a conditional release is in progress.
	Releasing,
	/// Terminal after stopping. A clean lease leaves a Released heartbeat.
	Stopped,
}

/// Published when lease validation or a dependent ends service permanently.
/// Fencing cancels renewal and issues no clean release write.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FenceReason {
	/// The deadline passed, or a confirmation arrived after its deadline.
	Expired,
	/// A renewal or release readback found no heartbeat object.
	MissingHeartbeat,
	/// A readback found another instance or a write sequence this worker never issued.
	ForeignHeartbeat,
	/// The registration deadline (`boot_timeout_ms`) passed before a claim confirmed.
	RegistrationFailed,
	/// A dependent called `Topology::fence` because safe service or cleanup failed.
	DependentFailed,
	/// Other live storage workers could write a family at a version this binary cannot read.
	IncompatibleVersion,
}

impl FenceReason {
	/// A stable label for logs and the fence counter.
	pub fn as_str(self) -> &'static str {
		match self {
			Self::Expired => "expired",
			Self::MissingHeartbeat => "missing_heartbeat",
			Self::ForeignHeartbeat => "foreign_heartbeat",
			Self::RegistrationFailed => "registration_failed",
			Self::DependentFailed => "dependent_failed",
			Self::IncompatibleVersion => "incompatible_version",
		}
	}
}

/// What a worker knew about its own lease when it fenced, for the fence log. Every instant is
/// on the boot clock.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FenceReport {
	pub reason: FenceReason,
	/// `None` for an observer, which has no lease to fence.
	pub worker_id: Option<crate::worker::WorkerId>,
	pub now_boot_ms: u64,
	/// Send time of the last renewal or claim the store confirmed.
	pub last_confirmed_issued_boot_ms: Option<u64>,
	/// How long the store took to confirm that write.
	pub last_confirmed_latency_ms: Option<u64>,
	/// The serving deadline that write established.
	pub deadline_boot_ms: Option<u64>,
	/// How long the oldest heartbeat write still unconfirmed has been outstanding: the store
	/// latency this worker was waiting on when it fenced.
	pub unconfirmed_write_age_ms: Option<u64>,
	/// Heartbeat writes issued since the last confirmed one.
	pub unconfirmed_writes: usize,
}

/// A live lease that reached half its serving window without a confirmed renewal, for the
/// late-renewal warning. Every instant is on the boot clock.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RenewalLateReport {
	pub worker_id: crate::worker::WorkerId,
	/// Time since the send time of the last write the store confirmed.
	pub since_confirmed_send_ms: u64,
	/// How long the oldest heartbeat write still unconfirmed has waited on the store. `None`
	/// when no renewal is outstanding, for example while waiting to retry.
	pub pending_renewal_wait_ms: Option<u64>,
	/// Heartbeat writes issued since the last confirmed one.
	pub unconfirmed_writes: usize,
	/// Time left before the lease deadline fences this worker.
	pub remaining_lease_ms: u64,
}

/// Immutable for the process lifetime. Written into every heartbeat this worker claims
/// or renews.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkerRegistration {
	/// Positive immutable placement weight, advertised without changing lease semantics.
	pub storage_capacity_millicores: u32,
	/// Stable worker key plus this process incarnation; claims may replace older incarnations.
	pub identity: WorkerIdentity,
	/// Selects the heartbeat namespace. Only `Storage` workers are listed for placement.
	pub role: WorkerRole,
	/// Advertised to peers through the heartbeat for direct requests.
	pub api_peer_address: String,
	/// Diagnostics only.
	pub build_version: String,
	/// Per-family version ranges this binary reads and writes, advertised in every heartbeat.
	/// Usually `versions::compiled_versions()`; tests advertise other ranges.
	pub versions: std::sync::Arc<[crate::versions::FamilyVersions]>,
	/// Diagnostics only.
	pub build_git_sha: String,
	/// Worker-incarnation startup time for diagnostics, never lease expiration.
	pub started_at_unix_ms: i64,
	/// The scope of the log directory lock this process took before registering and holds
	/// until its storage services have stopped, or `None` without one. Advertised in every
	/// heartbeat. A registering process with the same scope that finds a previous process's
	/// Active or Draining heartbeat naming it may claim that heartbeat without the takeover
	/// wait: holding the lock proves the previous holder exited.
	pub directory_lock: Option<crate::heartbeat::DirectoryLock>,
}

/// Cached endpoint hint. Hints never establish ownership.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EndpointEntry {
	/// `None` caches a miss.
	pub worker: Option<Worker>,
	/// Snapshot lookups reject the hint at this instant, even before `LookupExpiry` runs.
	pub expires_at_boot_ms: u64,
	/// When the point read that produced this hint completed.
	pub confirmed_at_boot_ms: u64,
}

/// How the state machine handled an `EndpointTimedOut` report.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EndpointTimeoutResult {
	/// The cached endpoint was confirmed long enough ago, so it was dropped.
	Invalidated,
	/// The route was recently confirmed, already gone, or being read again.
	Ignored,
}

impl LeaseStatus {
	/// Tests only the local lease deadline; admission must also reject draining workers.
	pub fn is_live(&self, now_boot_ms: u64) -> bool {
		matches!(self, Self::Live { deadline_boot_ms, .. } if now_boot_ms < *deadline_boot_ms)
	}

	/// Whether the lease is live and its confirmed renewal is recent enough to take on new
	/// ownership. A degraded lease still serves and replies; it refuses only new tablet grants,
	/// replica placements and volume opens, with a retryable refusal.
	pub fn admits_new_ownership(&self, now_boot_ms: u64) -> bool {
		matches!(
			self,
			Self::Live { deadline_boot_ms, degrade_at_boot_ms }
				if now_boot_ms < *deadline_boot_ms && now_boot_ms < *degrade_at_boot_ms
		)
	}
}

/// Returned through `ResolveEndpointResponse`; errors are not cached.
/// `Ok(None)` caches either an absent heartbeat in both roles or a Released heartbeat.
pub type EndpointResult = Result<Option<Worker>, RequestError>;

/// Failure returned to `Topology` handle callers through caller responses.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RequestError {
	/// The endpoint read failed, timed out, or finished past its deadline.
	Unavailable,
	/// A per-worker waiter cap or the shared `lookup_concurrency` cap was reached.
	Overloaded,
	/// `InvalidateEndpointRequest` arrived while this lookup was in flight.
	Invalidated,
	/// The lease is releasing, fenced, or stopped; caller work is no longer accepted.
	Stopped,
}

/// Scheduled by runner effects and delivered as `RuntimeEvent::Timer`.
/// Due alarms run before ordinary inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Timer {
	/// Bounds registration; expiry fences with RegistrationFailed.
	Registration,
	/// Issues `WriteLocalHeartbeatRequest` for a live lease, including while draining.
	Renew,
	/// Issues `ListStorageWorkersRequest` while observing or holding a live lease. Armed
	/// after each listing response at `refresh_interval_ms`, or earlier when the listed
	/// view would start to expire.
	Refresh,
	/// Expires the current live lease through its watch.
	WorkerLease,
	/// Fires half the serving window after the confirmed write's send time, so a renewal
	/// that has not confirmed by then is reported before the lease deadline fences. Each
	/// confirmation rearms it. Diagnostics only; it never changes lease authority.
	RenewalLate,
	/// Bounds release reconciliation by the last confirmed lease's original deadline.
	ReleaseDeadline,
	/// Re-reads after an `Unknown` read, or retries a claim whose readback was missing.
	Retry,
	/// Re-reads the observed heartbeat before attempting CAS, once its lease can no longer
	/// serve: `ttl_ms + zombie_grace_ms` after the read, or earlier once a store-time lower
	/// bound is that far plus `store_timestamp_uncertainty_ms` past its `Last-Modified`.
	Takeover,
	/// Evicts expired hints and fails due lookup attempts before the runner delivers responses.
	LookupExpiry,
	/// Reads the heartbeats of the watched stateless processes whose poll is due.
	WatchPoll,
	/// The current renewal write has waited `heartbeat_attempt_ms`. It becomes a superseded
	/// write that keeps running, and a fresh CAS write on the same ETag starts beside it.
	RenewalAttempt,
	/// Synthesizes `Read::Unknown` or `Write::Unknown` for heartbeat I/O, or `None` for a listing.
	/// A live renewal write's call runs until the lease deadline; `RenewalAttempt` paces retries.
	Io(rivet_state_machine_util::IoTimer<OperationId>),
	/// Correlated alarms for conditional claim/release progress embedded in this state machine.
	Publication(rivet_object_store_util::conditional_write::Timer<OperationId>),
}

/// Registration I/O activity; the read context retains why it was started and any uncertain claim.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RegistrationPhase {
	/// `ReadLocalHeartbeatResponse` is interpreted using the retained evidence.
	ReadingHeartbeat { context: HeartbeatReadContext },
	/// Shared conditional I/O retains the claim through writes, readbacks and retries.
	Publishing(ConditionalWrite<ClaimAttempt, OperationId>),
	/// No heartbeat I/O is pending; the runner owns the retry/takeover delay.
	Waiting { next: RegistrationAction },
}

/// Purpose of a heartbeat read, independent of the activity's name.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HeartbeatReadContext {
	/// No claim has been issued yet.
	Initial,
	/// The full takeover wait elapsed; only this unchanged observation permits CAS.
	AfterTakeoverWait {
		/// Version and store timestamp checked together after the wait.
		observed: TakeoverObservation,
		/// Boot time the observation was read. The wait never ends later than
		/// `ttl_ms + zombie_grace_ms` after it, even when store time is unknown.
		observed_boot_ms: u64,
		/// An earlier uncertain claim still anchors its original payload and deadline.
		retained_claim: Option<HeartbeatWrite>,
	},
	/// A claim may have landed; readback must confirm its original send time.
	ResolveClaim(ClaimAttempt),
}

impl HeartbeatReadContext {
	pub(super) fn retained_claim(&self) -> Option<&HeartbeatWrite> {
		match self {
			Self::Initial => None,
			Self::AfterTakeoverWait { retained_claim, .. } => retained_claim.as_ref(),
			Self::ResolveClaim(attempt) => Some(&attempt.write),
		}
	}
}

/// Continuation retained during registration backoff or the takeover safety wait.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RegistrationAction {
	/// Repeat a read with unchanged context, or inspect after the takeover wait.
	ReadHeartbeat(HeartbeatReadContext),
	/// An uncertain claim's readback was missing; retry the original create payload.
	WriteHeartbeat(HeartbeatWrite),
}

impl RegistrationPhase {
	pub(super) fn retained_claim(&self) -> Option<&HeartbeatWrite> {
		match self {
			Self::ReadingHeartbeat { context }
			| Self::Waiting {
				next: RegistrationAction::ReadHeartbeat(context),
				..
			} => context.retained_claim(),
			Self::Publishing(progress) => Some(&progress.request().write),
			Self::Waiting {
				next: RegistrationAction::WriteHeartbeat(write),
				..
			} => Some(write),
		}
	}
}

/// Renewal activity; the confirmed lease deadline bounds every read, write and wait.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RenewalPhase {
	/// Waiting for the normal `Renew` timer.
	Idle,
	/// CAS on the confirmed ETag for the unconfirmed write `write_sequence`, the slot's current
	/// call. Superseded earlier writes may still be running and may still land. A confirmation
	/// is credited to exactly this write and its send time. A conflict requires a heartbeat
	/// read; an unknown outcome sends a fresh CAS write.
	WritingHeartbeat { write_sequence: u64 },
	/// Resolve a conflicting renewal without extending the confirmed lease deadline.
	ReadingHeartbeat,
	/// No heartbeat I/O is in flight; the runner delivers `Retry`, which starts `next`.
	Waiting { next: RenewalRetry },
}

/// A renewal write replaced by a fresh attempt while its request still runs in the executor.
/// Its answer still counts: a confirmation credits exactly this write at its own send time.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SupersededWrite {
	/// The write's executor operation, no longer the slot's current call.
	pub op: OperationId,
	/// The unconfirmed write this request carried.
	pub write_sequence: u64,
}

/// What a renewal does when its `Retry` timer fires.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RenewalRetry {
	/// A readback was unknown; read the heartbeat again.
	ReadHeartbeat,
	/// A write failed quickly with an unknown outcome; send a fresh CAS write. Spacing the
	/// retry from the failed write's send time keeps instant failures from looping.
	WriteHeartbeat,
}

/// Retained across claim readbacks to match store evidence without resetting the original lease deadline.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClaimAttempt {
	/// Always write sequence 1. Retries keep the original `issued_boot_ms`.
	pub write: HeartbeatWrite,
	/// None creates a heartbeat; Some replaces this observed version.
	/// Retaining both ETag and modification time avoids repeating an already-completed takeover wait.
	pub observed: Option<TakeoverObservation>,
}

/// One heartbeat write attempt, kept until a confirmation supersedes it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HeartbeatWrite {
	/// Matched during readback to a write actually issued by this process.
	pub write_sequence: u64,
	/// Claims and renewals anchor expiration at this instant plus TTL minus the margin;
	/// a release write never establishes a new serving deadline.
	pub issued_boot_ms: u64,
	/// Retained across uncertain retries even if the local worker starts draining meanwhile.
	pub state: HeartbeatState,
}

/// The latest write known to have landed. The next renewal CASes on its etag.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConfirmedHeartbeatWrite {
	/// The original send time, rather than confirmation time, bounds serving.
	pub write: HeartbeatWrite,
	/// CAS target for the next renewal or clean release.
	pub etag: String,
	/// Reply instant. The store write happened before it, so this anchors the store clock.
	pub confirmed_boot_ms: u64,
}

/// One in-flight `LookupEndpointRequest` shared by every waiter for a worker.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PendingLookup {
	/// Must match `LookupEndpointResponse::generation`; stale responses are ignored.
	pub generation: u64,
	/// A new invalidation arrived during the read, so waiters receive `Invalidated`. A repeated
	/// invalidation of an endpoint that is already hidden leaves this unset, because the read
	/// was started to recheck exactly that endpoint.
	pub invalidated: bool,
	/// Late responses fail as `Unavailable`; `LookupExpiry` also wakes at this deadline.
	pub deadline_boot_ms: u64,
	/// Caller IDs answered together by separate `ResolveEndpointResponse` effects, capped at 128.
	pub waiters: Vec<u64>,
}

/// One in-flight `ReadStorageHeartbeatRequest` for heartbeat inspections of a worker.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PendingInspection {
	/// Must match `ReadStorageHeartbeatResponse::generation`; stale responses are ignored.
	pub generation: u64,
	/// Late observations become `Read::Unknown`; `LookupExpiry` also wakes at this deadline.
	pub deadline_boot_ms: u64,
	/// Caller IDs answered through `InspectHeartbeatResponse` by the read in flight.
	pub waiters: Vec<u64>,
	/// Arrived after the in-flight read began; their evidence must come from a newer read.
	pub queued: Vec<u64>,
}

/// Independent correlation lanes reject stale responses; pending work is not superseded.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Slot {
	/// Ordinary local heartbeat reads and renewals; old completions cannot revive renewal.
	Heartbeat,
	/// Independent of heartbeat I/O; `Timeout(Discovery)` fails a listing without cancelling renewal.
	Discovery,

	/// Claim and release helper I/O; separate IDs prevent collisions with ordinary heartbeat work.
	Publication,
}

/// Executor correlation token checked against the pending slot before applying a response.
pub type OperationId = OpId<Slot>;

impl rivet_state_machine::Slot for Slot {}

/// Comparison evidence retained across takeover waits and uncertain claim readbacks.
/// Active or Draining heartbeats must stay unchanged for the wait; Released bypasses it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TakeoverObservation {
	/// CAS target for replacing the observed heartbeat.
	pub etag: String,
	/// Compared together with `etag`; any change restarts the takeover wait.
	pub modified_unix_ms: i64,
}

/// One release CAS prepared after cleanup. A changed ETag requires explicit domain reconciliation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReleaseAttempt {
	/// Fixed Released payload above every renewal issued before cleanup.
	pub write: HeartbeatWrite,
	/// Readback may replace this only after validating a preceding renewal from this incarnation.
	pub expected_etag: String,
}
