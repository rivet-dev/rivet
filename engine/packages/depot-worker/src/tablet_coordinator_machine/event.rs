//! Inputs from the elected service, callers, and manifest adapter.

use super::types::{ReadResult, WriteResult};
use rivet_data::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorRequest;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Event {
	/// **When**
	///
	/// The existing replica transport answers an exact-process admission probe.
	///
	/// **State**
	///
	/// Retains only the matching current observation; failures back off without changing membership.
	///
	/// **Effects**
	///
	/// Cancels its probe alarm and resumes placement or schedules its next deadline.
	DiskProbeResponse {
		worker: [u8; 16],
		operation: u64,
		result: Result<[u8; 16], String>,
	},
	/// **When**
	///
	/// A coordinator receives an alarm split report or index-ready command.
	///
	/// **State**
	///
	/// Validates the exact checkpoint source and grant before changing bootstrap state or holds.
	///
	/// **Effects**
	///
	/// Uses the normal manifest CAS and command response.
	AlarmCommand {
		request_id: u64,
		request:
			rivet_data::generated::depot_tablet_coordinator_protocol_v1::AlarmCoordinatorRequest,
	},
	/// **When**
	///
	/// A live worker requests permission to stop.
	///
	/// **State**
	///
	/// Queues an idempotent intent with expiring topology evidence.
	///
	/// **Effects**
	///
	/// Persists grants/intent, checks readiness, or persists and replies with a permit.
	RequestStop {
		request_id: u64,
		request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::RequestStop,
		topology: super::types::StopTopology,
	},
	/// **When**
	///
	/// A bounded fan-out of fresh maintainer checks completes.
	///
	/// **State**
	///
	/// Retains results only for the correlated stop round.
	///
	/// **Effects**
	///
	/// None; the next retry rechecks the current manifest and fresh topology before CAS.
	StopReadinessResponse {
		operation: u64,
		worker: [u8; 16],
		results: std::collections::BTreeMap<
			[u8; 16],
			rivet_data::generated::depot_storage_worker_protocol_v1::StopReadinessResponse,
		>,
	},
	/// **When**
	///
	/// A V2-only readiness fan-out returns decoded capability proofs.
	///
	/// **State**
	///
	/// Validates every authority field for the correlated handoff; legacy responses
	/// cannot populate this event or authorize the create-admission barrier.
	///
	/// **Effects**
	///
	/// Publishes the immutable prepared CAS, waits to check again, or fences abandonment.
	HandoffReadinessResponse {
		operation: u64,
		worker: [u8; 16],
		results: std::collections::BTreeMap<
			[u8; 16],
			Option<
				rivet_data::generated::depot_storage_worker_protocol_v1::HandoffReadinessResponse,
			>,
		>,
	},
	/// **When**
	///
	/// An admitted command's caller drops its pending reply before completion.
	///
	/// **State**
	///
	/// Removes queued work, or abandons a not-yet-sent handoff with a manifest fence.
	/// A sent manifest CAS keeps its exact identity and readback obligation.
	///
	/// **Effects**
	///
	/// May send the fencing CAS; never replaces a sent mutation.
	CancelCommand { request_id: u64 },

	/// **When**
	///
	/// The elected service admits an operator request under its current authority.
	///
	/// **State**
	///
	/// Queues a bounded command, or rejects it if the coordinator stopped or is full.
	///
	/// **Effects**
	///
	/// Writes the changed manifest, or replies immediately for an idempotent or invalid command.
	Command {
		request_id: u64,
		request: TabletCoordinatorRequest,
	},
	/// **When**
	///
	/// The object store completes a read or readback.
	///
	/// **State**
	///
	/// Installs a confirmed observation or resolves the exact pending write; obsolete operations are ignored.
	///
	/// **Effects**
	///
	/// Writes the takeover barrier, retries an unchanged conditional attempt, or replies after confirmation.
	ReadManifestResponse { operation: u64, result: ReadResult },
	/// **When**
	///
	/// The object store completes a conditional write.
	///
	/// **State**
	///
	/// Installs a confirmed manifest; uncertain outcomes retain the immutable attempt for readback.
	///
	/// **Effects**
	///
	/// Replies after confirmation, reads back uncertainty, and cancels the operation timer.
	WriteManifestResponse { operation: u64, result: WriteResult },
	/// **When**
	///
	/// The runner observes a changed storage discovery snapshot while it holds the tenure.
	///
	/// **State**
	///
	/// Replaces the in-memory discovery observation; it is never persisted. Ignored once
	/// stopped.
	///
	/// **Effects**
	///
	/// While ready and the observation is unexpired, re-grants tablets whose maintainer is
	/// absent from it in one manifest write, beside any queued commands.
	StorageWorkers {
		topology: super::types::StopTopology,
	},
	/// **When**
	///
	/// The runner observes a change of the dynamic `depot.balancing.enabled` setting while
	/// it holds the tenure. The position at start is set on the state directly.
	///
	/// **State**
	///
	/// Flips the balancer's switch and drops its search, so the next drive searches again
	/// with the new position. Off, it keeps observing workers and still repairs, replacing
	/// dead or stopping replicas and growing short replica sets, but chooses no discretionary
	/// action and reports `Disabled`. Ignored once stopped or when no balancer is configured.
	///
	/// **Effects**
	///
	/// Cancels the balance timer; the next drive arms it again.
	BalancingEnabled { enabled: bool },
	/// **When**
	///
	/// The singleton service loses authority or its owner requests shutdown.
	///
	/// **State**
	///
	/// Stops admission and makes all subsequent I/O completions obsolete.
	///
	/// **Effects**
	///
	/// Rejects pending callers and cancels timers; the runtime joins its I/O tasks.
	Stop,
}

impl rivet_state_machine::EventBody for Event {
	fn name(&self) -> &'static str {
		match self {
			Event::DiskProbeResponse { .. } => "disk_probe_response",
			Event::RequestStop { .. } => "request_stop",
			Event::StopReadinessResponse { .. } => "stop_readiness_response",
			Event::HandoffReadinessResponse { .. } => "handoff_readiness_response",
			Event::CancelCommand { .. } => "cancel_command",
			Event::AlarmCommand { .. } => "alarm_command",
			Event::Command { .. } => "command",
			Event::ReadManifestResponse { .. } => "read_manifest_response",
			Event::WriteManifestResponse { .. } => "write_manifest_response",
			Event::StorageWorkers { .. } => "storage_workers",
			Event::BalancingEnabled { .. } => "balancing_enabled",
			Event::Stop => "stop",
		}
	}
}
