//! Immutable manifest observations and conditional attempts.

use rivet_data::generated::{
	depot_tablet_coordinator_protocol_v1::TabletCoordinatorRequest,
	depot_tablet_manifest_v1::{TabletManifest, TabletSplit},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StoredManifest {
	pub manifest: TabletManifest,
	pub etag: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ReadResult {
	Found(StoredManifest),
	FoundAlarm {
		envelope: rivet_data::generated::depot_tablet_manifest_v1::AlarmTabletManifest,
		etag: String,
	},
	Missing,
	Unknown(String),
	Invalid(String),
}

/// The outcome of a read conditioned on an ETag the caller already holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ConditionalRead {
	/// The store answered with the object, or with a failure, as an unconditional read would.
	Read(ReadResult),
	/// The object still carries the caller's ETag, so its content is what the caller last read.
	Unchanged,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WriteResult {
	Confirmed { etag: String },
	Precondition,
	Unknown(String),
	Invalid(String),
}

/// Retained unchanged until readback proves this attempt won or lost its precondition.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ManifestWrite {
	pub origin: WriteOrigin,
	pub manifest: TabletManifest,
	pub expected_etag: Option<String>,
}

/// Why a manifest write was issued, which decides who is answered once it is confirmed.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WriteOrigin {
	/// The takeover barrier that fences the previous coordinator's captured ETag. No caller waits.
	Takeover,
	/// One write that applies a batch of queued commands in order, followed by any automatic
	/// re-grants. Every listed caller is accepted at the written version once the write is
	/// confirmed, and rejected if it is lost.
	Commands {
		request_ids: Vec<u64>,
		/// Tablets this write re-grants because their maintainer left storage discovery.
		regranted: u64,
	},
	/// A pre-publication handoff was cancelled or timed out. This unchanged-assignment
	/// touch fences its old readiness/CAS before callers are rejected.
	HandoffAborted { request_ids: Vec<u64> },
	/// A stop request's own write; its completion decides the reply.
	Stop {
		request_id: u64,
		completion: StopCompletion,
	},
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Command {
	pub topology: Option<StopTopology>,
	pub request_id: u64,
	pub request: TabletCoordinatorRequest,
	/// Drawn at admission, so queueing and I/O retries never redraw a tablet identity.
	pub tablet_id: [u8; 16],
	/// Drawn once at split admission and retained with the command through queueing.
	pub split: Option<TabletSplit>,
	pub requested_at: i64,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Timer {
	Operation(u64),
	Retry,
	Balance,
	/// The earliest deadline of a Learning phase in the confirmed manifest.
	LearningDeadline,
	DiskProbe([u8; 16], u64),
	HandoffRetry(u64),
}

/// A bounded-lifetime Active storage discovery observation, captured by the executor.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StopTopology {
	pub workers: std::collections::BTreeMap<[u8; 16], [u8; 16]>,
	pub capacities: std::collections::BTreeMap<[u8; 16], u32>,
	/// Each worker's advertised per-family version ranges, fixed for its process.
	pub versions: std::collections::BTreeMap<
		[u8; 16],
		std::sync::Arc<[rivet_topology::versions::FamilyVersions]>,
	>,
	pub valid_until_boot_ms: u64,
}
/// Progress checks may wake a maintainer's closed-volume cleanup, but cannot
/// authorize a stop. Clearing structural blockers requires a fresh Permit check.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StopCheckPurpose {
	/// The still-current owner quiesces fresh creates before a voluntary grant handoff.
	Handoff {
		tablet_manifest_version: u64,
	},
	Progress,
	Permit,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StopCheck {
	pub purpose: StopCheckPurpose,
	pub request: rivet_data::generated::depot_tablet_coordinator_protocol_v1::RequestStop,
	pub tablets: Vec<rivet_data::generated::depot_tablet_manifest_v1::Tablet>,
	/// Readiness from a previous peer process cannot authorize its replacement.
	pub instances: std::collections::BTreeMap<[u8; 16], [u8; 16]>,
}
impl StopCheck {
	/// The enhanced wire request carries the exact pre-handoff global CAS fence.
	pub fn handoff_request(
		&self,
		tablet: &rivet_data::generated::depot_tablet_manifest_v1::Tablet,
	) -> Option<rivet_data::generated::depot_storage_worker_protocol_v1::StopReadiness> {
		use rivet_data::generated::depot_storage_worker_protocol_v1 as wire;
		if tablet.maintainer != Some(self.request.worker_id) {
			return None;
		}
		let StopCheckPurpose::Handoff {
			tablet_manifest_version,
		} = self.purpose
		else {
			return None;
		};
		Some(wire::StopReadiness {
			tablet_id: tablet.id,
			tablet_grant_epoch: tablet.tablet_grant_epoch,
			tablet_replica_set_version: tablet.replica_set.tablet_replica_set_version,
			worker_id: self.request.worker_id,
			worker_incarnation: self.request.instance_id,
			purpose: wire::ReadinessPurpose::Handoff(wire::Handoff {
				tablet_manifest_version,
			}),
		})
	}
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StopCompletion {
	pub entries: u64,
	pub new_permit: bool,
	pub response:
		rivet_data::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorResponse,
	pub check: Option<StopCheck>,
}
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StopRound {
	pub operation: u64,
	pub check: StopCheck,
	pub results: Option<
		std::collections::BTreeMap<
			[u8; 16],
			rivet_data::generated::depot_storage_worker_protocol_v1::StopReadinessResponse,
		>,
	>,
}
