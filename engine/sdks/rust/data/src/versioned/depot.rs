//! Version envelopes for Depot records and messages. Log batches use the folder
//! format version instead and are framed by the log shard codec.

/// Versioned checkpoint of replica states and tablet grant epochs, without entries.
/// Recovery replays batches above its boundary, bounding restart work. Covered state,
/// forget and grant frames no longer pin old segments. Independent of the cold store.
pub enum DepotLogShardCheckpointData {
	V1(crate::generated::depot_log_shard_v1::LogShardCheckpoint),
}

impl_versioned_v1!(
	DepotLogShardCheckpointData,
	crate::generated::depot_log_shard_v1::LogShardCheckpoint
);

/// Length-delimited checkpoint bytes and their CRC32C. Depot verifies the checksum
/// before decoding the enclosed versioned checkpoint.
pub enum DepotLogShardCheckpointEnvelopeData {
	V1(crate::generated::depot_log_shard_v1::LogShardCheckpointEnvelope),
}

impl_versioned_v1!(
	DepotLogShardCheckpointEnvelopeData,
	crate::generated::depot_log_shard_v1::LogShardCheckpointEnvelope
);

/// Versioned `StorageWorkerRequest` with strict payload decoding.
pub enum DepotStorageWorkerRequestData {
	V1(crate::generated::depot_storage_worker_protocol_v1::StorageWorkerRequest),
}

impl_versioned_v1!(
	DepotStorageWorkerRequestData,
	crate::generated::depot_storage_worker_protocol_v1::StorageWorkerRequest
);

/// Versioned `StorageWorkerResponse` with strict payload decoding.
pub enum DepotStorageWorkerResponseData {
	V1(crate::generated::depot_storage_worker_protocol_v1::StorageWorkerResponse),
}

impl_versioned_v1!(
	DepotStorageWorkerResponseData,
	crate::generated::depot_storage_worker_protocol_v1::StorageWorkerResponse
);

/// Versioned `ClientRequest` with strict payload decoding.
pub enum DepotClientRequestData {
	V1(crate::generated::depot_client_protocol_v1::ClientRequest),
}

impl_versioned_v1!(
	DepotClientRequestData,
	crate::generated::depot_client_protocol_v1::ClientRequest
);

/// Versioned `ClientResponse` with strict payload decoding.
pub enum DepotClientResponseData {
	V1(crate::generated::depot_client_protocol_v1::ClientResponse),
}

impl_versioned_v1!(
	DepotClientResponseData,
	crate::generated::depot_client_protocol_v1::ClientResponse
);

/// Versioned `TabletCoordinatorResponse` with strict payload decoding.
pub enum DepotTabletCoordinatorResponseData {
	V1(crate::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorResponse),
}

impl_versioned_v1!(
	DepotTabletCoordinatorResponseData,
	crate::generated::depot_tablet_coordinator_protocol_v1::TabletCoordinatorResponse
);

/// Versioned `KvCommandRequest` with strict payload decoding.
pub enum DepotKvCommandRequestData {
	V1(crate::generated::depot_kv_command_v1::KvCommandRequest),
}

impl_versioned_v1!(
	DepotKvCommandRequestData,
	crate::generated::depot_kv_command_v1::KvCommandRequest
);

/// Versioned `KvCommandResponse` with strict payload decoding.
pub enum DepotKvCommandResponseData {
	V1(crate::generated::depot_kv_command_v1::KvCommandResponse),
}

impl_versioned_v1!(
	DepotKvCommandResponseData,
	crate::generated::depot_kv_command_v1::KvCommandResponse
);

/// Versioned `KvGet` with strict payload decoding.
pub enum DepotKvGetData {
	V1(crate::generated::depot_kv_command_v1::KvGet),
}

impl_versioned_v1!(DepotKvGetData, crate::generated::depot_kv_command_v1::KvGet);

/// Versioned `KvGetResponse` with strict payload decoding.
pub enum DepotKvGetResponseData {
	V1(crate::generated::depot_kv_command_v1::KvGetResponse),
}

impl_versioned_v1!(
	DepotKvGetResponseData,
	crate::generated::depot_kv_command_v1::KvGetResponse
);

/// Versioned `KvApply` with strict payload decoding.
pub enum DepotKvApplyData {
	V1(crate::generated::depot_kv_command_v1::KvApply),
}

impl_versioned_v1!(
	DepotKvApplyData,
	crate::generated::depot_kv_command_v1::KvApply
);

/// Versioned `KvApplyResponse` with strict payload decoding.
pub enum DepotKvApplyResponseData {
	V1(crate::generated::depot_kv_command_v1::KvApplyResponse),
}

impl_versioned_v1!(
	DepotKvApplyResponseData,
	crate::generated::depot_kv_command_v1::KvApplyResponse
);

/// Versioned `CompareAndSetRequest` with strict payload decoding.
pub enum DepotOperatorCompareAndSetRequestData {
	V1(crate::generated::depot_operator_command_v1::CompareAndSetRequest),
}

impl_versioned_v1!(
	DepotOperatorCompareAndSetRequestData,
	crate::generated::depot_operator_command_v1::CompareAndSetRequest
);

/// Versioned `CompareAndSetResponse` with strict payload decoding.
pub enum DepotOperatorCompareAndSetResponseData {
	V1(crate::generated::depot_operator_command_v1::CompareAndSetResponse),
}

impl_versioned_v1!(
	DepotOperatorCompareAndSetResponseData,
	crate::generated::depot_operator_command_v1::CompareAndSetResponse
);

/// Versioned `MutationTag` with strict payload decoding.
pub enum DepotOperatorMutationTagData {
	V1(crate::generated::depot_operator_command_v1::MutationTag),
}

impl_versioned_v1!(
	DepotOperatorMutationTagData,
	crate::generated::depot_operator_command_v1::MutationTag
);

/// Versioned `StopReadiness` with strict payload decoding.
pub enum DepotStopReadinessData {
	V1(crate::generated::depot_storage_worker_protocol_v1::StopReadiness),
}

impl_versioned_v1!(
	DepotStopReadinessData,
	crate::generated::depot_storage_worker_protocol_v1::StopReadiness
);

/// Versioned `HandoffReadinessResponse` with strict payload decoding. A handoff proof has no
/// plain-readiness representation: a `StopReadinessResponse::Ready` never decodes into it.
pub enum DepotHandoffReadinessResponseData {
	V1(crate::generated::depot_storage_worker_protocol_v1::HandoffReadinessResponse),
}

impl_versioned_v1!(
	DepotHandoffReadinessResponseData,
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
	crate::generated::depot_storage_worker_protocol_v1::StopReadinessResponse
);

/// Versioned `MaintainerEntry`, the payload of one tablet maintainer log entry.
pub enum DepotMaintainerEntryData {
	V1(crate::generated::depot_maintainer_log_v1::MaintainerEntry),
}

impl_versioned_v1!(
	DepotMaintainerEntryData,
	crate::generated::depot_maintainer_log_v1::MaintainerEntry
);

/// Versioned `MaintainerLogRecord`, the tablet store's record of its maintainer log.
pub enum DepotMaintainerLogRecordData {
	V1(crate::generated::depot_maintainer_log_v1::MaintainerLogRecord),
}

impl_versioned_v1!(
	DepotMaintainerLogRecordData,
	crate::generated::depot_maintainer_log_v1::MaintainerLogRecord
);

/// Versioned `RepairAuditOp`, the stored audit row of one operator repair call.
pub enum DepotRepairAuditData {
	V1(crate::generated::depot_maintainer_log_v1::RepairAuditOp),
}

impl_versioned_v1!(
	DepotRepairAuditData,
	crate::generated::depot_maintainer_log_v1::RepairAuditOp
);

/// Versioned `VolumeCreation`, the creation evidence a volume's replicas hold.
pub enum DepotVolumeCreationData {
	V1(crate::generated::depot_maintainer_log_v1::VolumeCreation),
}

impl_versioned_v1!(
	DepotVolumeCreationData,
	crate::generated::depot_maintainer_log_v1::VolumeCreation
);


