use super::types::{EndpointResult, FenceReason, OperationId};
use crate::{
	heartbeat::{Observation, Read, Write},
	worker::{Worker, WorkerId},
};
use serde::{Deserialize, Serialize};

/// Sent by the `Topology` handle or executor and processed by `TopologyStateMachine`.
/// Caller `request_id`s identify waiting responses, independently of I/O operation IDs and generations.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Event {
	// Caller requests.
	/// Sent by `Topology::resolve_worker` only on a snapshot miss. Concurrent requests for
	/// one worker share a `LookupEndpointRequest`; answered by `ResolveEndpointResponse`.
	ResolveEndpointRequest {
		/// Matches the caller channel installed by the handle, independently of executor I/O IDs.
		request_id: u64,
		worker: WorkerId,
	},
	/// Sent by `Topology::invalidate_endpoint` after a request to `worker` failed. Hides
	/// that exact endpoint until discovery reports a different instance or address or a
	/// later point read revalidates it. A new invalidation fails an in-flight lookup for the
	/// worker with `Invalidated`; repeating one for an already hidden endpoint does not.
	InvalidateEndpointRequest {
		/// Matches the caller channel installed by the handle, independently of executor I/O IDs.
		request_id: u64,
		worker: Worker,
	},
	/// **When**
	///
	/// Sent one-way by `Topology::report_endpoint_timeout` after a stream opening to a
	/// stateless worker timed out. It takes no admission permit and has no response.
	///
	/// **State**
	///
	/// Drops the cached endpoint for `worker` only when it is the same endpoint and a point
	/// read last confirmed it at least `endpoint_timeout_invalidate_after_ms` ago. A missing,
	/// different or recently confirmed entry, or one with a lookup in flight, is unchanged.
	/// A stopped topology changes nothing.
	///
	/// **Effects**
	///
	/// Always exactly one `RecordEndpointTimeout`, which also releases the report's inbox slot.
	EndpointTimedOut { worker: Worker },
	/// Sent by `Topology::inspect_storage_heartbeat` for fresh owner evidence. Bypasses the
	/// endpoint cache and never joins a read issued before the request arrived.
	InspectHeartbeatRequest {
		/// Matches the caller channel installed by the handle, independently of executor I/O IDs.
		request_id: u64,
		worker: WorkerId,
	},

	/// **When**
	///
	/// Sent by `Topology::watch` when the first subscriber watches `process`, or when every
	/// earlier subscription ended and a new one starts.
	///
	/// **State**
	///
	/// Adds one watch for `process`, or replaces the subscription of the existing one. A
	/// worker the current listing names as a storage worker is judged by listings; any other
	/// is polled, first at once and then every `watch.poll_interval_ms`. Ignored once
	/// topology stops.
	///
	/// **Effects**
	///
	/// `WatchedProcessGone` when the current listing already shows the process gone, and the
	/// `WatchPoll` timer re-armed at the head of the poll queue.
	WatchRequest {
		process: super::watch::WatchedProcess,
		/// Identifies this subscription across `UnwatchRequest` and `WatchedProcessGone`.
		subscription: u64,
	},
	/// **When**
	///
	/// Sent by the last `WatchHandle` of a subscription as it drops.
	///
	/// **State**
	///
	/// Removes the watch only when it still carries `subscription`, so an unwatch that
	/// arrives after a newer subscription leaves the newer one in place.
	///
	/// **Effects**
	///
	/// Only the `WatchPoll` timer re-armed or cancelled.
	UnwatchRequest {
		process: super::watch::WatchedProcess,
		subscription: u64,
	},

	// Executor responses.
	/// Answers `ReadLocalHeartbeatRequest`. Drives claims and readbacks; ignored unless `op`
	/// is still pending in `Slot::Heartbeat`. A `Timeout` timer synthesizes `Read::Unknown`.
	ReadLocalHeartbeatResponse { op: OperationId, result: Read },
	/// Answers `WriteLocalHeartbeatRequest`: confirmation validates a claim or renewal, or completes release.
	/// A conflict triggers readback. An unknown renewal outcome sends a fresh CAS write, and an
	/// unknown claim or release reads back; `Timeout(Heartbeat)` synthesizes `Write::Unknown`.
	WriteLocalHeartbeatResponse { op: OperationId, result: Write },
	/// Answers `ListStorageWorkersRequest`. `None` means the listing failed or timed out and
	/// keeps observed entries subject to expiry; matching local confirmation re-anchors the store clock.
	ListStorageWorkersResponse {
		op: OperationId,
		result: Option<Vec<Observation>>,
	},
	/// The listing completed but at least one heartbeat GET was unreadable. Healthy observations
	/// are applied without treating the listing as expiry evidence for retained observations.
	ListStorageWorkersPartialResponse {
		op: OperationId,
		entries: Vec<Observation>,
	},
	/// Answers `LookupEndpointRequest`. Ignored unless `generation` matches the pending
	/// lookup for `worker`. A found endpoint is cached until its heartbeat's lease would end;
	/// a miss is cached for `lookup_ttl_ms`.
	LookupEndpointResponse {
		worker: WorkerId,
		/// Echoed across the executor request and response; must match this worker's pending read.
		generation: u64,
		result: EndpointResult,
		/// `Last-Modified` of the heartbeat a found endpoint came from.
		#[serde(default)]
		heartbeat_modified_unix_ms: Option<i64>,
	},
	/// Answers `ReadStorageHeartbeatRequest`; stale generations are ignored and overdue results become Unknown.
	/// Queued inspections start a newer read when this one completes.
	ReadStorageHeartbeatResponse {
		worker: WorkerId,
		/// Echoed across the executor request and response; must match this worker's pending read.
		generation: u64,
		result: Read,
	},

	/// **When**
	///
	/// Answers `ReadWatchedHeartbeatRequest`; a timed-out, failed or over-capacity read
	/// returns `Read::Unknown`.
	///
	/// **State**
	///
	/// Ignored unless `generation` is the read in flight for a still-watched process. A
	/// heartbeat of a different process instance, a released or expired heartbeat, or none in
	/// either role ends the watch. A heartbeat in the storage role moves the watch to listing
	/// detection. `Unknown` keeps the watch for its next poll.
	///
	/// **Effects**
	///
	/// `WatchedProcessGone` when the process is gone.
	ReadWatchedHeartbeatResponse {
		process: super::watch::WatchedProcess,
		generation: u64,
		result: Read,
	},

	// Lifecycle notifications.
	/// **When**
	///
	/// Sent by `Topology::refresh` when a caller needs a fresh listing, for example after a
	/// lookup miss or an expired view.
	///
	/// **State**
	///
	/// None. It is ignored while a listing is in flight or within `refresh_hint_interval_ms`
	/// of the last one, and never extends the lease.
	///
	/// **Effects**
	///
	/// `ListStorageWorkersRequest` when not ignored.
	Refresh,
	/// Sent by `Topology::begin_drain`: stops admission immediately while pending writes keep their payload.
	/// Renewals publish Draining while dependents clean up; replies remain allowed only until lease expiry.
	BeginDrain,
	/// Sent by `Topology::fence`, typically by a dependent that can no longer guarantee
	/// safe service. Terminal.
	Fence(FenceReason),
	/// Sent by `Topology::shutdown` only after safe dependent cleanup; a live lease stops renewal and CASes Released.
	/// Release reconciliation retains the old deadline; observers or unconfirmed registrations stop directly.
	StopRequest,
}

impl Event {
	pub(super) fn operation_id(&self) -> Option<OperationId> {
		match self {
			Self::ReadLocalHeartbeatResponse { op, .. }
			| Self::WriteLocalHeartbeatResponse { op, .. }
			| Self::ListStorageWorkersResponse { op, .. }
			| Self::ListStorageWorkersPartialResponse { op, .. } => Some(*op),
			Self::ResolveEndpointRequest { .. }
			| Self::InvalidateEndpointRequest { .. }
			| Self::EndpointTimedOut { .. }
			| Self::InspectHeartbeatRequest { .. }
			| Self::WatchRequest { .. }
			| Self::UnwatchRequest { .. }
			| Self::ReadWatchedHeartbeatResponse { .. }
			| Self::LookupEndpointResponse { .. }
			| Self::ReadStorageHeartbeatResponse { .. }
			| Self::Refresh
			| Self::BeginDrain
			| Self::Fence(_)
			| Self::StopRequest => None,
		}
	}
}
