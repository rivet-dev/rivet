//! Versions of `depot.client_protocol` messages: volume requests to a tablet maintainer, replica
//! status notices, operator requests and regional filesystem requests.
//!
//! A sender seals each request at the version its receiver reads, by topology's in-memory view
//! of that worker, so a worker of an older release decodes it while a deploy rolls. A request
//! that version cannot carry is refused before it is sent as the retryable `depot.overloaded`
//! with reason [`VERSION_PENDING_REASON`]. A receiver answers at the version its request was
//! sealed at, which its sender reads, and refuses an answer that version cannot carry the same
//! way.

use crate::metrics;
use rivet_data::{
	DEPOT_CLIENT_PROTOCOL_VERSION,
	families::VersionFamily,
	generated::depot_client_protocol_v2::{
		ClientRequest, ClientResponse, Overloaded as RejectOverloaded, VolumeRejectResponse,
	},
	versioned::{DepotClientRequestData, DepotClientResponseData},
};
use rivet_topology::{versions::PeerVersions, worker::WorkerId};
use vbare::Versioned;

/// The version family of every volume-protocol message.
pub const FAMILY: VersionFamily = VersionFamily::DEPOT_CLIENT_PROTOCOL;

/// The `depot.overloaded` reason of a volume request refused before it was sent, because the
/// version its receiver reads cannot carry it, and of an answer refused because the version its
/// request was sealed at cannot carry it.
pub const VERSION_PENDING_REASON: &str = "version_pending";

/// The first client protocol version that carries `VolumeOpenRequest.lazyFork`.
const LAZY_FORK_VERSION: u16 = 2;

/// The highest client protocol version a released binary reads.
const RELEASED_VERSION: u16 = 1;

/// The version to seal a volume request to `worker` at. See [`PeerVersions::peer_version`]. A
/// fleet view that cannot select a version gets the highest released version, which every
/// release reads, so a request it cannot carry is refused retryably with [`VERSION_PENDING_REASON`]
/// instead of answered `ups.invalid_message` by a released receiver.
pub fn request_version(versions: &dyn PeerVersions, worker: &WorkerId) -> u16 {
	match versions.peer_version(FAMILY, worker) {
		Ok(version) => version.min(DEPOT_CLIENT_PROTOCOL_VERSION),
		Err(error) => {
			tracing::debug!(
				?error,
				"no client protocol version is selectable for the receiver; sealing the released version"
			);
			RELEASED_VERSION
		}
	}
}

/// The version a request's sender sealed it at, from its version header, or `None` for a
/// payload too short to carry one.
pub fn embedded_version(payload: &[u8]) -> Option<u16> {
	let header = payload.get(..2)?;
	Some(u16::from_le_bytes([header[0], header[1]]))
}

/// Seals `request` at `version`. A request `version` cannot carry, such as a base rows read sent
/// to a worker of a release that has none, is refused before it is sent with
/// [`version_pending`] and `retry_after_ms`. Any other encoding failure is a malformed request.
///
/// A volume open's `lazyFork` is dropped below the version that carries it rather than refused:
/// a receiver that reads an older version runs a release from before group forks, and while it
/// is live the fleet's read floor keeps namespace volumes closed, so no group has a base and a
/// lazy fork has nothing to copy. Refusing it would refuse every forkable open, which is every
/// actor database open, for the whole roll.
pub fn seal_request(
	request: &ClientRequest,
	version: u16,
	retry_after_ms: u64,
) -> anyhow::Result<bytes::Bytes> {
	let sealed = match request {
		ClientRequest::VolumeOpenRequest(open) if open.lazy_fork && version < LAZY_FORK_VERSION => {
			let mut open = open.clone();
			open.lazy_fork = false;
			DepotClientRequestData::seal_to_bytes(&ClientRequest::VolumeOpenRequest(open), version)
		}
		ClientRequest::VolumeOpenRequest(_)
		| ClientRequest::FsReadRequest(_)
		| ClientRequest::VolumeCheckpointRequest(_)
		| ClientRequest::FsForkReferenceRequest(_)
		| ClientRequest::FsForkRequest(_)
		| ClientRequest::VolumeCreateRequest(_)
		| ClientRequest::VolumeDeleteRequest(_)
		| ClientRequest::VolumeCloseRequest(_)
		| ClientRequest::ReplicaStatusNoticeRequest(_)
		| ClientRequest::OperatorTierRequest(_)
		| ClientRequest::OperatorDropCachesRequest(_)
		| ClientRequest::RepairInspectRequest(_)
		| ClientRequest::RepairRequest(_)
		| ClientRequest::RepairAuditListRequest(_)
		| ClientRequest::RepairDiagnosticsRequest(_) => {
			DepotClientRequestData::seal_to_bytes(request, version)
		}
	};
	match sealed {
		Ok(body) => Ok(body),
		Err(vbare::EncodeError::Conversion(error)) if version < DEPOT_CLIENT_PROTOCOL_VERSION => {
			let request = request_label(request);
			metrics::COMMAND_REJECTIONS
				.with_label_values(&[request, VERSION_PENDING_REASON])
				.inc();
			tracing::debug!(
				?error,
				request,
				version,
				"refused a volume request the version its receiver reads cannot carry; the receiver runs an older release"
			);
			Err(version_pending(retry_after_ms))
		}
		Err(error) => Err(error.into()),
	}
}

/// Seals `response` at the version its request was sealed at, capped at the latest. A sender
/// seals no higher than the version it reads, so a sender of an older release decodes the
/// answer, and no topology read is needed. An answer that version cannot carry is replaced by
/// the refusal `Overloaded` with `retry_after_ms`, which every version carries.
pub fn seal_response(
	response: &ClientResponse,
	request_version: u16,
	retry_after_ms: u64,
) -> anyhow::Result<Vec<u8>> {
	let version = request_version.clamp(1, DEPOT_CLIENT_PROTOCOL_VERSION);
	match DepotClientResponseData::seal_to_vec(response, version) {
		Ok(body) => Ok(body),
		Err(vbare::EncodeError::Conversion(error)) if version < DEPOT_CLIENT_PROTOCOL_VERSION => {
			let response = response_label(response);
			metrics::COMMAND_REJECTIONS
				.with_label_values(&[response, VERSION_PENDING_REASON])
				.inc();
			tracing::warn!(
				?error,
				response,
				version,
				"refused a volume answer the version its sender reads cannot carry; a sender of an older release asked for state only a newer release writes"
			);
			let refusal = ClientResponse::VolumeRejectResponse(VolumeRejectResponse::Overloaded(
				RejectOverloaded { retry_after_ms },
			));
			Ok(DepotClientResponseData::seal_to_vec(&refusal, version)?)
		}
		Err(error) => Err(error.into()),
	}
}

/// The retryable refusal of a volume request its receiver's version cannot carry. Nothing was
/// sent; the edge sends it again after `retry_after_ms`, when the view may show the receiver
/// upgraded.
pub fn version_pending(retry_after_ms: u64) -> anyhow::Error {
	crate::Overloaded {
		reason: VERSION_PENDING_REASON.to_owned(),
		retry_after_ms,
	}
	.build()
}

/// The hint of a [`version_pending`] refusal, or `None` for any other error.
pub fn version_pending_retry_after_ms(error: &anyhow::Error) -> Option<u64> {
	let error = error.downcast_ref::<rivet_error::RivetError>()?;
	if error.group() != "depot" || error.code() != "overloaded" {
		return None;
	}
	let meta = error.metadata()?;
	if meta.get("reason")?.as_str()? != VERSION_PENDING_REASON {
		return None;
	}
	meta.get("retry_after_ms")?.as_u64()
}

/// The bounded `command` label of a volume request in `depot_client_command_rejections_total`.
pub fn request_label(request: &ClientRequest) -> &'static str {
	match request {
		ClientRequest::FsReadRequest(_) => "volume_fs_read",
		ClientRequest::VolumeCheckpointRequest(_) => "volume_checkpoint",
		ClientRequest::FsForkReferenceRequest(_) => "volume_fork_reference",
		ClientRequest::FsForkRequest(_) => "volume_fork",
		ClientRequest::VolumeCreateRequest(_) => "volume_create",
		ClientRequest::VolumeDeleteRequest(_) => "volume_delete",
		ClientRequest::VolumeOpenRequest(_) => "volume_open",
		ClientRequest::VolumeCloseRequest(_) => "volume_close",
		ClientRequest::ReplicaStatusNoticeRequest(_) => "volume_replica_status",
		ClientRequest::OperatorTierRequest(_) => "volume_operator_tier",
		ClientRequest::OperatorDropCachesRequest(_) => "volume_operator_drop_caches",
		ClientRequest::RepairInspectRequest(_) => "volume_repair_inspect",
		ClientRequest::RepairRequest(_) => "volume_repair",
		ClientRequest::RepairAuditListRequest(_) => "volume_repair_audit_list",
		ClientRequest::RepairDiagnosticsRequest(_) => "volume_repair_diagnostics",
	}
}

/// The bounded `command` label of a volume answer in `depot_client_command_rejections_total`.
fn response_label(response: &ClientResponse) -> &'static str {
	match response {
		ClientResponse::FsReadResponse(_) => "volume_fs_read_answer",
		ClientResponse::VolumeCheckpointResponse(_) => "volume_checkpoint_answer",
		ClientResponse::FsForkReferenceResponse(_) => "volume_fork_reference_answer",
		ClientResponse::FsForkResponse(_) => "volume_fork_answer",
		ClientResponse::VolumeCreateResponse(_) => "volume_create_answer",
		ClientResponse::VolumeDeleteResponse => "volume_delete_answer",
		ClientResponse::VolumeOpenResponse(_) => "volume_open_answer",
		ClientResponse::VolumeCloseResponse => "volume_close_answer",
		ClientResponse::VolumeRejectResponse(_) => "volume_reject_answer",
		ClientResponse::ReplicaStatusNoticeResponse(_) => "volume_replica_status_answer",
		ClientResponse::OperatorTabletReport(_) => "volume_operator_tablet_answer",
		ClientResponse::OperatorResponse(_) => "volume_operator_answer",
		ClientResponse::RepairInspection(_) => "volume_repair_inspect_answer",
		ClientResponse::RepairResponse(_) => "volume_repair_answer",
		ClientResponse::RepairAuditPage(_) => "volume_repair_audit_list_answer",
		ClientResponse::RepairDiagnosticsPage(_) => "volume_repair_diagnostics_answer",
	}
}
