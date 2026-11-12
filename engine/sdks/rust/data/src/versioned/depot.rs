//! Version envelopes for Depot records and messages. Log batches use the folder
//! format version instead and are framed by the log shard codec.

/// A replica catch-up barrier, separate from process stop intent.
pub enum DepotReplicaReadinessData {
	V1(crate::generated::depot_replica_readiness_v1::ReplicaReadiness),
}

impl_versioned_v1!(
	DepotReplicaReadinessData,
	DEPOT_REPLICA_READINESS,
	crate::generated::depot_replica_readiness_v1::ReplicaReadiness
);

/// Versioned checkpoint of replica states and tablet grant epochs, without entries.
/// Recovery replays batches above its boundary, bounding restart work. Covered state,
/// forget and grant frames no longer pin old segments. Independent of the cold store.
pub enum DepotLogShardCheckpointData {
	V1(crate::generated::depot_log_shard_v1::LogShardCheckpoint),
}

impl_versioned_v1!(
	DepotLogShardCheckpointData,
	DEPOT_LOG_SHARD,
	crate::generated::depot_log_shard_v1::LogShardCheckpoint
);

/// Length-delimited checkpoint bytes and their CRC32C. Depot verifies the checksum
/// before decoding the enclosed versioned checkpoint.
pub enum DepotLogShardCheckpointEnvelopeData {
	V1(crate::generated::depot_log_shard_v1::LogShardCheckpointEnvelope),
}

impl_versioned_v1!(
	DepotLogShardCheckpointEnvelopeData,
	DEPOT_LOG_SHARD,
	crate::generated::depot_log_shard_v1::LogShardCheckpointEnvelope
);

/// Versioned `StorageWorkerRequest` with strict payload decoding.
pub enum DepotStorageWorkerRequestData {
	V1(crate::generated::depot_storage_worker_protocol_v1::StorageWorkerRequest),
}

impl_versioned_v1!(
	DepotStorageWorkerRequestData,
	DEPOT_STORAGE_WORKER_PROTOCOL,
	crate::generated::depot_storage_worker_protocol_v1::StorageWorkerRequest
);

/// Versioned `StorageWorkerResponse` with strict payload decoding.
pub enum DepotStorageWorkerResponseData {
	V1(crate::generated::depot_storage_worker_protocol_v1::StorageWorkerResponse),
}

impl_versioned_v1!(
	DepotStorageWorkerResponseData,
	DEPOT_STORAGE_WORKER_PROTOCOL,
	crate::generated::depot_storage_worker_protocol_v1::StorageWorkerResponse
);

/// Versioned `ClientRequest` with strict payload decoding.
pub enum DepotClientRequestData {
	V1(crate::generated::depot_client_protocol_v1::ClientRequest),
	V2(crate::generated::depot_client_protocol_v2::ClientRequest),
}

impl_versioned_v2!(
	DepotClientRequestData,
	DEPOT_CLIENT_PROTOCOL,
	crate::generated::depot_client_protocol_v1::ClientRequest,
	crate::generated::depot_client_protocol_v2::ClientRequest
);

/// Versioned `ClientResponse` with strict payload decoding.
pub enum DepotClientResponseData {
	V1(crate::generated::depot_client_protocol_v1::ClientResponse),
	V2(crate::generated::depot_client_protocol_v2::ClientResponse),
}

impl_versioned_v2!(
	DepotClientResponseData,
	DEPOT_CLIENT_PROTOCOL,
	crate::generated::depot_client_protocol_v1::ClientResponse,
	crate::generated::depot_client_protocol_v2::ClientResponse
);

/// Versioned `TabletCoordinatorResponse` with strict payload decoding.
pub enum DepotTabletCoordinatorResponseData {
	V1(crate::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorResponse),
}

impl_versioned_v1!(
	DepotTabletCoordinatorResponseData,
	DEPOT_TABLET_COORDINATOR_PROTOCOL,
	crate::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorResponse
);

/// Versioned `KvCommandRequest` with strict payload decoding.
pub enum DepotKvCommandRequestData {
	V1(crate::generated::depot_kv_command_v1::KvCommandRequest),
}

impl_versioned_v1!(
	DepotKvCommandRequestData,
	DEPOT_KV_COMMAND,
	crate::generated::depot_kv_command_v1::KvCommandRequest
);

/// Versioned `KvCommandResponse` with strict payload decoding.
pub enum DepotKvCommandResponseData {
	V1(crate::generated::depot_kv_command_v1::KvCommandResponse),
}

impl_versioned_v1!(
	DepotKvCommandResponseData,
	DEPOT_KV_COMMAND,
	crate::generated::depot_kv_command_v1::KvCommandResponse
);

/// Versioned `KvGet` with strict payload decoding.
pub enum DepotKvGetData {
	V1(crate::generated::depot_kv_command_v1::KvGet),
}

impl_versioned_v1!(
	DepotKvGetData,
	DEPOT_KV_COMMAND,
	crate::generated::depot_kv_command_v1::KvGet
);

/// Versioned `KvGetResponse` with strict payload decoding.
pub enum DepotKvGetResponseData {
	V1(crate::generated::depot_kv_command_v1::KvGetResponse),
}

impl_versioned_v1!(
	DepotKvGetResponseData,
	DEPOT_KV_COMMAND,
	crate::generated::depot_kv_command_v1::KvGetResponse
);

/// Versioned `KvApply` with strict payload decoding.
pub enum DepotKvApplyData {
	V1(crate::generated::depot_kv_command_v1::KvApply),
}

impl_versioned_v1!(
	DepotKvApplyData,
	DEPOT_KV_COMMAND,
	crate::generated::depot_kv_command_v1::KvApply
);

/// Versioned `KvApplyResponse` with strict payload decoding.
pub enum DepotKvApplyResponseData {
	V1(crate::generated::depot_kv_command_v1::KvApplyResponse),
}

impl_versioned_v1!(
	DepotKvApplyResponseData,
	DEPOT_KV_COMMAND,
	crate::generated::depot_kv_command_v1::KvApplyResponse
);

/// Versioned `CompareAndSetRequest` with strict payload decoding.
pub enum DepotOperatorCompareAndSetRequestData {
	V1(crate::generated::depot_operator_command_v1::CompareAndSetRequest),
}

impl_versioned_v1!(
	DepotOperatorCompareAndSetRequestData,
	DEPOT_OPERATOR_COMMAND,
	crate::generated::depot_operator_command_v1::CompareAndSetRequest
);

/// Versioned `CompareAndSetResponse` with strict payload decoding.
pub enum DepotOperatorCompareAndSetResponseData {
	V1(crate::generated::depot_operator_command_v1::CompareAndSetResponse),
}

impl_versioned_v1!(
	DepotOperatorCompareAndSetResponseData,
	DEPOT_OPERATOR_COMMAND,
	crate::generated::depot_operator_command_v1::CompareAndSetResponse
);

/// Versioned `MutationTag` with strict payload decoding.
pub enum DepotOperatorMutationTagData {
	V1(crate::generated::depot_operator_command_v1::MutationTag),
}

impl_versioned_v1!(
	DepotOperatorMutationTagData,
	DEPOT_OPERATOR_COMMAND,
	crate::generated::depot_operator_command_v1::MutationTag
);

/// Versioned `StopReadiness` with strict payload decoding.
pub enum DepotStopReadinessData {
	V1(crate::generated::depot_storage_worker_protocol_v1::StopReadiness),
}

impl_versioned_v1!(
	DepotStopReadinessData,
	DEPOT_STORAGE_WORKER_PROTOCOL,
	crate::generated::depot_storage_worker_protocol_v1::StopReadiness
);

/// Versioned `HandoffReadinessResponse` with strict payload decoding. A handoff proof has no
/// plain-readiness representation: a `StopReadinessResponse::Ready` never decodes into it.
pub enum DepotHandoffReadinessResponseData {
	V1(crate::generated::depot_storage_worker_protocol_v1::HandoffReadinessResponse),
}

impl_versioned_v1!(
	DepotHandoffReadinessResponseData,
	DEPOT_STORAGE_WORKER_PROTOCOL,
	crate::generated::depot_storage_worker_protocol_v1::HandoffReadinessResponse
);

/// The decoded proof, rather than the caller's retained request purpose, supplies the
/// handoff capability. Every authority field must name the exact request.
pub fn handoff_proof_matches(
	request: &crate::generated::depot_storage_worker_protocol_v1::StopReadiness,
	proof: &crate::generated::depot_storage_worker_protocol_v1::HandoffReadinessResponse,
) -> bool {
	let crate::generated::depot_storage_worker_protocol_v1::ReadinessPurpose::Handoff(purpose) =
		&request.purpose
	else {
		return false;
	};
	request.tablet_id == proof.tablet_id
		&& request.tablet_grant_epoch == proof.tablet_grant_epoch
		&& request.tablet_replica_set_version == proof.tablet_replica_set_version
		&& request.worker_id == proof.worker_id
		&& request.worker_incarnation == proof.worker_incarnation
		&& purpose.tablet_manifest_version == proof.tablet_manifest_version
}

/// Versioned `StopReadinessResponse` with strict payload decoding.
pub enum DepotStopReadinessResponseData {
	V1(crate::generated::depot_storage_worker_protocol_v1::StopReadinessResponse),
}

impl_versioned_v1!(
	DepotStopReadinessResponseData,
	DEPOT_STORAGE_WORKER_PROTOCOL,
	crate::generated::depot_storage_worker_protocol_v1::StopReadinessResponse
);

/// Versioned `MaintainerEntry`, the payload of one tablet maintainer log entry.
pub enum DepotMaintainerEntryData {
	V1(crate::generated::depot_maintainer_log_v1::MaintainerEntry),
	V2(crate::generated::depot_maintainer_log_v2::MaintainerEntry),
}

impl_versioned_v2!(
	DepotMaintainerEntryData,
	DEPOT_MAINTAINER_LOG,
	crate::generated::depot_maintainer_log_v1::MaintainerEntry,
	crate::generated::depot_maintainer_log_v2::MaintainerEntry
);

/// Versioned `MaintainerLogRecord`, the tablet store's record of its maintainer log.
pub enum DepotMaintainerLogRecordData {
	V1(crate::generated::depot_maintainer_log_v1::MaintainerLogRecord),
	V2(crate::generated::depot_maintainer_log_v2::MaintainerLogRecord),
}

impl_versioned_v2!(
	DepotMaintainerLogRecordData,
	DEPOT_MAINTAINER_LOG,
	crate::generated::depot_maintainer_log_v1::MaintainerLogRecord,
	crate::generated::depot_maintainer_log_v2::MaintainerLogRecord
);

/// Versioned `RepairAuditOp`, the stored audit row of one operator repair call.
pub enum DepotRepairAuditData {
	V1(crate::generated::depot_maintainer_log_v1::RepairAuditOp),
	V2(crate::generated::depot_maintainer_log_v2::RepairAuditOp),
}

impl_versioned_v2!(
	DepotRepairAuditData,
	DEPOT_MAINTAINER_LOG,
	crate::generated::depot_maintainer_log_v1::RepairAuditOp,
	crate::generated::depot_maintainer_log_v2::RepairAuditOp
);

/// Versioned `VolumeCreation`, the creation evidence a volume's replicas hold.
pub enum DepotVolumeCreationData {
	V1(crate::generated::depot_maintainer_log_v1::VolumeCreation),
	V2(crate::generated::depot_maintainer_log_v2::VolumeCreation),
}

impl_versioned_v2!(
	DepotVolumeCreationData,
	DEPOT_MAINTAINER_LOG,
	crate::generated::depot_maintainer_log_v1::VolumeCreation,
	crate::generated::depot_maintainer_log_v2::VolumeCreation
);
