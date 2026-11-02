//! Manifest I/O and caller replies performed by the coordinator executor.

use super::types::ManifestWrite;
use rivet_data::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorResponse;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Effect {
	/// **When**
	///
	/// Placement diagnostics derived from the confirmed manifest change.
	///
	/// **Executor**
	///
	/// Publishes counts, residual width imbalance and the bounded block reason to logs
	/// and global metrics.
	ObservePlacement {
		status: super::state::balancing::Status,
		/// Warn only on entry into a saturated probe window with queued workers.
		probe_limit_reached: bool,
	},
	/// **When**
	///
	/// A fresh topology identity has no successful disk admission observation.
	///
	/// **Executor**
	///
	/// Uses GetDiskIncarnation on the existing replica transport, rechecks topology and
	/// tenure, and returns DiskProbeResponse. It does not inspect volumes.
	DiskProbeRequest {
		worker: [u8; 16],
		instance: [u8; 16],
		capacity: u32,
		operation: u64,
	},
	/// **When**
	///
	/// A manifest mutation also changes alarm initialization or parent holds.
	///
	/// **Executor**
	///
	/// Writes one combined envelope with the retained CAS precondition and returns WriteManifestResponse.
	WriteAlarmManifestRequest {
		operation: u64,
		write: ManifestWrite,
		envelope: rivet_data::generated::depot_tablet_manifest_v1::AlarmTabletManifest,
	},
	/// **When**
	///
	/// A stop decision is confirmed (including CAS readback).
	///
	/// **Executor**
	///
	/// Updates process-global shutdown metrics.
	ObserveStops { entries: u64, new_permit: bool },
	/// **When**
	///
	/// A manifest write that re-granted tablets is confirmed, or a tablet whose maintainer
	/// left discovery is first found to have no eligible live member.
	///
	/// **Executor**
	///
	/// Counts `depot_coordinator_regrants_total` by result and warns once for each listed
	/// tablet that cannot be re-granted.
	ObserveRegrants {
		granted: u64,
		no_member: Vec<[u8; 16]>,
	},
	/// **When**
	///
	/// Discovery removes a stop intent, or a changed configuration or outage invalidates its check.
	///
	/// **Executor**
	///
	/// Cancels its obsolete readiness fan-out; late replies carry the old operation.
	CancelStopReadinessRequest { worker: [u8; 16] },
	/// **When**
	///
	/// A confirmed stop intent needs fresh successor evidence. Progress checks also prompt
	/// closed-volume cleanup while structural blockers remain; only Permit checks authorize stop.
	///
	/// **Executor**
	///
	/// Sends bounded concurrent checks to the captured maintainers, returning correlated results.
	StopReadinessRequest {
		operation: u64,
		check: super::types::StopCheck,
	},
	/// **When**
	///
	/// Startup needs the manifest or a conditional write has an uncertain result.
	///
	/// **Executor**
	///
	/// Reads the formal manifest key and returns ReadManifestResponse with its confirmed ETag.
	ReadManifestRequest { operation: u64 },
	/// **When**
	///
	/// Takeover or a command prepares a new manifest, or readback leaves its precondition unchanged.
	///
	/// **Executor**
	///
	/// Sends the exact versioned manifest with Create or If-Match, then returns WriteManifestResponse.
	WriteManifestRequest {
		operation: u64,
		write: ManifestWrite,
	},
	/// **When**
	///
	/// A command is confirmed in the manifest or cannot be admitted.
	///
	/// **Executor**
	///
	/// Rechecks the original singleton authority and delivers the response to the waiting caller.
	CommandResponse {
		request_id: u64,
		response: TabletCoordinatorResponse,
	},
}

impl rivet_state_machine::Effect for Effect {
	fn name(&self) -> &'static str {
		match self {
			Effect::ObservePlacement { .. } => "observe_placement",
			Effect::DiskProbeRequest { .. } => "disk_probe_request",
			Effect::ObserveStops { .. } => "observe_stops",
			Effect::ObserveRegrants { .. } => "observe_regrants",
			Effect::CancelStopReadinessRequest { .. } => "cancel_stop_readiness_request",
			Effect::StopReadinessRequest { .. } => "stop_readiness_request",
			Effect::ReadManifestRequest { .. } => "read_manifest_request",
			Effect::WriteAlarmManifestRequest { .. } => "write_alarm_manifest_request",
			Effect::WriteManifestRequest { .. } => "write_manifest_request",
			Effect::CommandResponse { .. } => "command_response",
		}
	}
}
