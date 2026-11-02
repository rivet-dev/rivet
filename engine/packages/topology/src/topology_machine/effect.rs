use super::types::{
	EndpointResult, EndpointTimeoutResult, FenceReport, OperationId, RenewalLateReport,
	RequestError, Timer, WorkerRegistration,
};
use crate::{
	heartbeat::{Heartbeat, Read},
	worker::WorkerId,
};
use serde::{Deserialize, Serialize};

/// A transition's ordered outputs: the runner applies timer operations and delegates `Execute` to the executor.
pub type Effects = rivet_state_machine_util::Effects<Effect, Timer>;

/// Handled by `TopologyExecutor`: completes caller channels, performs I/O, or records metrics.
/// I/O returns the matching Event; timer scheduling stays with the runner.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Effect {
	// Caller responses. The executor delivers each one to the oneshot registered under
	// `request_id` by the `Topology` handle, which then releases its admission permit.
	/// Answers `ResolveEndpointRequest`, possibly for several waiters of one lookup.
	ResolveEndpointResponse {
		/// Matches the caller channel installed by the handle, independently of executor I/O IDs.
		request_id: u64,
		result: EndpointResult,
	},
	/// Answers `InvalidateEndpointRequest` after the invalidation is applied to state.
	InvalidateEndpointResponse {
		/// Matches the caller channel installed by the handle, independently of executor I/O IDs.
		request_id: u64,
		result: Result<(), RequestError>,
	},
	/// Answers `InspectHeartbeatRequest`. Capacity rejection, expiry, and stop all report
	/// `Read::Unknown`.
	InspectHeartbeatResponse {
		/// Matches the caller channel installed by the handle, independently of executor I/O IDs.
		request_id: u64,
		result: Read,
	},

	// Executor requests. Each I/O runs under `io_timeout_ms` and returns its paired response.
	/// Exact-key GET of this worker's heartbeat for registration claims and readbacks. Paired with
	/// `ReadLocalHeartbeatResponse`.
	ReadLocalHeartbeatRequest {
		op: OperationId,
		worker_registration: WorkerRegistration,
	},
	/// Conditional PUT of this worker's heartbeat. Paired with `WriteLocalHeartbeatResponse`.
	WriteLocalHeartbeatRequest {
		op: OperationId,
		worker_registration: WorkerRegistration,
		heartbeat: Heartbeat,
		/// `None` creates the object; `Some` CASes on the confirmed or observed etag.
		expected_etag: Option<String>,
	},
	/// LIST plus exact GETs of storage heartbeats. A stateless worker also reads its own
	/// heartbeat to anchor the store clock. Paired with `ListStorageWorkersResponse`.
	ListStorageWorkersRequest {
		op: OperationId,
		/// Per-heartbeat read budget within the whole discovery deadline.
		read_timeout_ms: u64,
		worker_registration: Option<WorkerRegistration>,
		/// The current view. A listed heartbeat with an unchanged ETag reuses its entry here
		/// instead of being read again.
		known: Vec<crate::heartbeat::Observation>,
	},
	/// Exact-key reads of `worker`'s heartbeat in `first_role`, then in the other role.
	/// Bounded by the executor's read slots. Paired with `LookupEndpointResponse`.
	LookupEndpointRequest {
		worker: WorkerId,
		/// Echoed across the executor request and response; must match this worker's pending read.
		generation: u64,
		/// The role this worker was last seen in, so a known storage worker costs one GET.
		first_role: crate::worker::WorkerRole,
	},
	/// Exact-key read of a storage heartbeat for an inspection. Shares the lookup read
	/// slots. Paired with `ReadStorageHeartbeatResponse`.
	ReadStorageHeartbeatRequest {
		worker: WorkerId,
		/// Echoed across the executor request and response; must match this worker's pending read.
		generation: u64,
	},

	/// **When**
	///
	/// `Timer::WatchPoll` finds a watched stateless process's poll due.
	///
	/// **Executor**
	///
	/// Reads the worker's heartbeat by exact key, stateless role first, within the watch poll
	/// slots and by `deadline_boot_ms`, then returns `ReadWatchedHeartbeatResponse`.
	ReadWatchedHeartbeatRequest {
		process: super::watch::WatchedProcess,
		generation: u64,
		deadline_boot_ms: u64,
	},
	/// **When**
	///
	/// A listing or a poll showed a watched process gone, or the listing current at
	/// `WatchRequest` already did. The watch has been removed.
	///
	/// **Executor**
	///
	/// Resolves every handle of `subscription` and counts `reason`. Returns no event.
	WatchedProcessGone {
		process: super::watch::WatchedProcess,
		subscription: u64,
		reason: super::watch::GoneReason,
	},

	// One-way effects.
	/// Aborts lookup and inspection tasks when release begins, the lease stops, or it fences.
	CancelLookups,
	/// Updates cache-entry and pending-read gauges; warns when cache use crosses the near-capacity threshold.
	RecordLookupUsage {
		/// Cached positive and negative endpoint hints, excluding discovery entries.
		entries: usize,
		/// Pending endpoint lookups plus heartbeat inspections, sharing one concurrency cap.
		reads: usize,
	},
	/// Counts and logs a lookup or inspection rejected by a capacity bound.
	RecordLookupRejected,
	/// **When**
	///
	/// Emitted once for every `EndpointTimedOut` event, whether or not it changed state.
	///
	/// **Executor**
	///
	/// Counts the result and releases the report's inbox slot. Returns no event.
	RecordEndpointTimeout { result: EndpointTimeoutResult },
	/// **When**
	///
	/// `fence` moves a lease that was not already fenced or stopped to Fenced. A process
	/// fences at most once, so this is emitted at most once per process lifetime.
	///
	/// **Executor**
	///
	/// Logs the reason and the lease timing at error and counts the fence by reason. Returns
	/// no event.
	RecordFence { report: FenceReport },
	/// **When**
	///
	/// A live lease's renewal confirms: the store confirmed a heartbeat write, or a readback
	/// found it, before its deadline.
	///
	/// **Executor**
	///
	/// Observes the send-to-confirm latency, sets the remaining-lease gauge, and logs at info
	/// when `recovered` ends a late-renewal episode. Returns no event.
	RecordRenewal {
		latency_ms: u64,
		remaining_lease_ms: u64,
		/// This confirmation arrived before the lease reached half its window, ending an
		/// episode `RecordRenewalLate` reported.
		recovered: bool,
	},
	/// **When**
	///
	/// `Timer::RenewalLate` finds a live lease past half its serving window with no newer
	/// confirmation, and no warning has been emitted since the last timely renewal.
	///
	/// **Executor**
	///
	/// Logs the report at warn with a hint to check object-store latency and sets the
	/// remaining-lease gauge. Returns no event.
	RecordRenewalLate { report: RenewalLateReport },
	/// **When**
	///
	/// A listing response shows that other live storage workers could select a family
	/// version this live storage worker cannot read. Emitted just before the fence.
	///
	/// **Executor**
	///
	/// Logs the family, the selectable version and this binary's range. Returns no event.
	RecordIncompatibleVersion {
		incompatibility: crate::versions::Incompatibility,
	},
}
