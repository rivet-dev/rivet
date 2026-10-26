//! Routing inputs shared by the client and the tablet command service.

use rivet_data::generated::depot_client_protocol_v1::{
	ClientRequest, NotMaintainer, Overloaded, VolumeRecordMissing, VolumeRejectResponse,
};

/// How long a caller waits before resending a lifecycle request the maintainer refused at
/// admission. Maintainer admission limits drain as queued opens and lifecycle operations
/// finish, which settle on this scale.
pub const MAINTAINER_OVERLOAD_RETRY_AFTER_MS: u64 = 100;

/// How long a caller waits before resending a request the answering worker refused because it
/// does not hold the tablet's maintainer grant. The refusal also triggers a route refresh,
/// which completes on this scale, so the next attempt finds the current maintainer.
pub const NOT_MAINTAINER_RETRY_AFTER_MS: u64 = 100;

/// The refusal of a lifecycle request the maintainer did not admit under load.
pub fn overloaded() -> VolumeRejectResponse {
	VolumeRejectResponse::Overloaded(Overloaded {
		retry_after_ms: MAINTAINER_OVERLOAD_RETRY_AFTER_MS,
	})
}

/// The refusal of a request sent to a worker that does not hold the tablet's maintainer grant.
pub fn not_maintainer() -> VolumeRejectResponse {
	VolumeRejectResponse::NotMaintainer(NotMaintainer {
		retry_after_ms: NOT_MAINTAINER_RETRY_AFTER_MS,
	})
}

/// The refusal of a request on a name whose current incarnation, `volume_incarnation`, has no
/// stored record the maintainer can recover. It never clears by itself: the caller must delete
/// that incarnation to free the name.
pub fn record_missing(volume_incarnation: [u8; 16]) -> VolumeRejectResponse {
	VolumeRejectResponse::VolumeRecordMissing(VolumeRecordMissing { volume_incarnation })
}

/// The retry hint a lifecycle refusal carries, or `None` when it carries none.
pub fn retry_after_ms(reject: &VolumeRejectResponse) -> Option<u64> {
	match reject {
		VolumeRejectResponse::NotMaintainer(reject) => Some(reject.retry_after_ms),
		VolumeRejectResponse::Overloaded(reject) => Some(reject.retry_after_ms),
		VolumeRejectResponse::VolumeUnderRepair(reject) => Some(reject.retry_after_ms),
		VolumeRejectResponse::TabletSplitting
		| VolumeRejectResponse::VolumeNotFound
		| VolumeRejectResponse::VolumeExists
		| VolumeRejectResponse::VolumeDeleted
		| VolumeRejectResponse::VolumeRecordMissing(_) => None,
	}
}

/// Lifecycle and cold reads route by the logical volume key, independently of incarnation.
pub fn volume_key(request: &ClientRequest) -> Option<&crate::VolumeKey> {
	Some(match request {
		ClientRequest::FsForkReferenceRequest(request) => request.volume_key(),
		ClientRequest::FsForkRequest(request) => &request.spec.destination_volume_key,
		ClientRequest::VolumeCheckpointRequest(request) => &request.volume_key,
		ClientRequest::FsReadRequest(request) => request.volume_key(),
		ClientRequest::VolumeCreateRequest(request) => &request.volume_key,
		ClientRequest::VolumeOpenRequest(request) => &request.volume_key,
		ClientRequest::VolumeDeleteRequest(request) => &request.volume_log_key.volume_key,
		ClientRequest::VolumeCloseRequest(request) => &request.volume_log_key.volume_key,
		ClientRequest::RepairInspectRequest(request) => &request.volume_key,
		ClientRequest::RepairDiagnosticsRequest(request) => &request.volume_log_key.volume_key,
		ClientRequest::RepairRequest(request) => &request.envelope.volume_log_key.volume_key,
		ClientRequest::RepairAuditListRequest(request) => &request.volume_key,
		// A notice names many volumes, possibly of several tablets.
		ClientRequest::ReplicaStatusNoticeRequest(_) => return None,
		// Operator requests name tablets of the worker, not volumes.
		ClientRequest::OperatorTierRequest(_) | ClientRequest::OperatorDropCachesRequest(_) => {
			return None;
		}
	})
}
