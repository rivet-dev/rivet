//! Explicit field mappings for the v4 read-state protocol.
#![allow(dead_code)]
use crate::generated::{v3, v4};
use anyhow::{Result, bail};
pub(crate) fn ballot_3_to_4(x: v3::Ballot) -> Result<v4::Ballot> {
	Ok(v4::Ballot {
		counter: x.counter,
		replica_id: x.replica_id,
	})
}
pub(crate) fn committed_value_3_to_4(x: v3::CommittedValue) -> Result<v4::CommittedValue> {
	Ok(v4::CommittedValue {
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
		version: x.version,
		mutable: x.mutable,
	})
}
pub(crate) fn cached_value_3_to_4(x: v3::CachedValue) -> Result<v4::CachedValue> {
	Ok(v4::CachedValue {
		value: x
			.value
			.map(|x| x.map(|x| Ok::<_, anyhow::Error>(x)).transpose())
			.transpose()?,
		version: x.version,
	})
}
pub(crate) fn accepted_value_3_to_4(x: v3::AcceptedValue) -> Result<v4::AcceptedValue> {
	Ok(v4::AcceptedValue {
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
		ballot: ballot_3_to_4(x.ballot)?,
		version: x.version,
		mutable: x.mutable,
	})
}
pub(crate) fn replica_status_3_to_4(x: v3::ReplicaStatus) -> Result<v4::ReplicaStatus> {
	Ok(match x {
		v3::ReplicaStatus::Joining => v4::ReplicaStatus::Joining,
		v3::ReplicaStatus::Learning => v4::ReplicaStatus::Learning,
		v3::ReplicaStatus::Active => v4::ReplicaStatus::Active,
	})
}
pub(crate) fn replica_config_3_to_4(x: v3::ReplicaConfig) -> Result<v4::ReplicaConfig> {
	Ok(v4::ReplicaConfig {
		replica_id: x.replica_id,
		status: replica_status_3_to_4(x.status)?,
		api_peer_url: x.api_peer_url,
		guard_url: x.guard_url,
	})
}
pub(crate) fn cluster_config_3_to_4(x: v3::ClusterConfig) -> Result<v4::ClusterConfig> {
	Ok(v4::ClusterConfig {
		coordinator_replica_id: x.coordinator_replica_id,
		epoch: x.epoch,
		replicas: x
			.replicas
			.into_iter()
			.map(|x| replica_config_3_to_4(x))
			.collect::<Result<Vec<_>>>()?,
	})
}
pub(crate) fn update_config_request_3_to_4(
	x: v3::UpdateConfigRequest,
) -> Result<v4::UpdateConfigRequest> {
	Ok(v4::UpdateConfigRequest {
		config: cluster_config_3_to_4(x.config)?,
	})
}
pub(crate) fn prepare_request_3_to_4(x: v3::PrepareRequest) -> Result<v4::PrepareRequest> {
	Ok(v4::PrepareRequest {
		key: x.key,
		ballot: ballot_3_to_4(x.ballot)?,
		mutable: x.mutable,
		version: x.version,
	})
}
pub(crate) fn prepare_response_ok_3_to_4(
	x: v3::PrepareResponseOk,
) -> Result<v4::PrepareResponseOk> {
	Ok(v4::PrepareResponseOk {
		highest_ballot: ballot_3_to_4(x.highest_ballot)?,
		accepted_value: x
			.accepted_value
			.map(|x| committed_value_3_to_4(x))
			.transpose()?,
		accepted_ballot: x.accepted_ballot.map(|x| ballot_3_to_4(x)).transpose()?,
	})
}
pub(crate) fn prepare_response_already_committed_3_to_4(
	x: v3::PrepareResponseAlreadyCommitted,
) -> Result<v4::PrepareResponseAlreadyCommitted> {
	Ok(v4::PrepareResponseAlreadyCommitted {
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
	})
}
pub(crate) fn prepare_response_higher_ballot_3_to_4(
	x: v3::PrepareResponseHigherBallot,
) -> Result<v4::PrepareResponseHigherBallot> {
	Ok(v4::PrepareResponseHigherBallot {
		ballot: ballot_3_to_4(x.ballot)?,
	})
}
pub(crate) fn prepare_response_3_to_4(x: v3::PrepareResponse) -> Result<v4::PrepareResponse> {
	Ok(match x {
		v3::PrepareResponse::PrepareResponseOk(x) => {
			v4::PrepareResponse::PrepareResponseOk(prepare_response_ok_3_to_4(x)?)
		}
		v3::PrepareResponse::PrepareResponseAlreadyCommitted(x) => {
			v4::PrepareResponse::PrepareResponseAlreadyCommitted(
				prepare_response_already_committed_3_to_4(x)?,
			)
		}
		v3::PrepareResponse::PrepareResponseHigherBallot(x) => {
			v4::PrepareResponse::PrepareResponseHigherBallot(prepare_response_higher_ballot_3_to_4(
				x,
			)?)
		}
	})
}
pub(crate) fn pre_accept_request_3_to_4(x: v3::PreAcceptRequest) -> Result<v4::PreAcceptRequest> {
	Ok(v4::PreAcceptRequest {
		key: x.key,
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
		ballot: ballot_3_to_4(x.ballot)?,
		mutable: x.mutable,
		version: x.version,
	})
}
pub(crate) fn pre_accept_response_ok_3_to_4(
	x: v3::PreAcceptResponseOk,
) -> Result<v4::PreAcceptResponseOk> {
	Ok(v4::PreAcceptResponseOk {
		ballot: ballot_3_to_4(x.ballot)?,
	})
}
pub(crate) fn pre_accept_response_already_committed_3_to_4(
	x: v3::PreAcceptResponseAlreadyCommitted,
) -> Result<v4::PreAcceptResponseAlreadyCommitted> {
	Ok(v4::PreAcceptResponseAlreadyCommitted {
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
	})
}
pub(crate) fn pre_accept_response_higher_ballot_3_to_4(
	x: v3::PreAcceptResponseHigherBallot,
) -> Result<v4::PreAcceptResponseHigherBallot> {
	Ok(v4::PreAcceptResponseHigherBallot {
		ballot: ballot_3_to_4(x.ballot)?,
	})
}
pub(crate) fn pre_accept_response_3_to_4(
	x: v3::PreAcceptResponse,
) -> Result<v4::PreAcceptResponse> {
	Ok(match x {
		v3::PreAcceptResponse::PreAcceptResponseOk(x) => {
			v4::PreAcceptResponse::PreAcceptResponseOk(pre_accept_response_ok_3_to_4(x)?)
		}
		v3::PreAcceptResponse::PreAcceptResponseAlreadyCommitted(x) => {
			v4::PreAcceptResponse::PreAcceptResponseAlreadyCommitted(
				pre_accept_response_already_committed_3_to_4(x)?,
			)
		}
		v3::PreAcceptResponse::PreAcceptResponseHigherBallot(x) => {
			v4::PreAcceptResponse::PreAcceptResponseHigherBallot(
				pre_accept_response_higher_ballot_3_to_4(x)?,
			)
		}
	})
}
pub(crate) fn accept_request_3_to_4(x: v3::AcceptRequest) -> Result<v4::AcceptRequest> {
	Ok(v4::AcceptRequest {
		key: x.key,
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
		ballot: ballot_3_to_4(x.ballot)?,
		mutable: x.mutable,
		version: x.version,
	})
}
pub(crate) fn accept_response_ok_3_to_4(x: v3::AcceptResponseOk) -> Result<v4::AcceptResponseOk> {
	Ok(v4::AcceptResponseOk {
		ballot: ballot_3_to_4(x.ballot)?,
	})
}
pub(crate) fn accept_response_already_committed_3_to_4(
	x: v3::AcceptResponseAlreadyCommitted,
) -> Result<v4::AcceptResponseAlreadyCommitted> {
	Ok(v4::AcceptResponseAlreadyCommitted {
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
	})
}
pub(crate) fn accept_response_higher_ballot_3_to_4(
	x: v3::AcceptResponseHigherBallot,
) -> Result<v4::AcceptResponseHigherBallot> {
	Ok(v4::AcceptResponseHigherBallot {
		ballot: ballot_3_to_4(x.ballot)?,
	})
}
pub(crate) fn accept_response_3_to_4(x: v3::AcceptResponse) -> Result<v4::AcceptResponse> {
	Ok(match x {
		v3::AcceptResponse::AcceptResponseOk(x) => {
			v4::AcceptResponse::AcceptResponseOk(accept_response_ok_3_to_4(x)?)
		}
		v3::AcceptResponse::AcceptResponseAlreadyCommitted(x) => {
			v4::AcceptResponse::AcceptResponseAlreadyCommitted(
				accept_response_already_committed_3_to_4(x)?,
			)
		}
		v3::AcceptResponse::AcceptResponseHigherBallot(x) => {
			v4::AcceptResponse::AcceptResponseHigherBallot(accept_response_higher_ballot_3_to_4(x)?)
		}
	})
}
pub(crate) fn commit_request_3_to_4(x: v3::CommitRequest) -> Result<v4::CommitRequest> {
	Ok(v4::CommitRequest {
		key: x.key,
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
		ballot: ballot_3_to_4(x.ballot)?,
		mutable: x.mutable,
		version: x.version,
	})
}
pub(crate) fn commit_response_already_committed_3_to_4(
	x: v3::CommitResponseAlreadyCommitted,
) -> Result<v4::CommitResponseAlreadyCommitted> {
	Ok(v4::CommitResponseAlreadyCommitted {
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
	})
}
pub(crate) fn commit_response_3_to_4(x: v3::CommitResponse) -> Result<v4::CommitResponse> {
	Ok(match x {
		v3::CommitResponse::CommitResponseOk => v4::CommitResponse::CommitResponseOk,
		v3::CommitResponse::CommitResponseAlreadyCommitted(x) => {
			v4::CommitResponse::CommitResponseAlreadyCommitted(
				commit_response_already_committed_3_to_4(x)?,
			)
		}
		v3::CommitResponse::CommitResponseStaleCommit => {
			v4::CommitResponse::CommitResponseStaleCommit
		}
	})
}
pub(crate) fn caching_behavior_3_to_4(x: v3::CachingBehavior) -> Result<v4::CachingBehavior> {
	Ok(match x {
		v3::CachingBehavior::Optimistic => v4::CachingBehavior::Optimistic,
		v3::CachingBehavior::SkipCache => v4::CachingBehavior::SkipCache,
	})
}
pub(crate) fn kv_get_request_3_to_4(x: v3::KvGetRequest) -> Result<v4::KvGetRequest> {
	Ok(v4::KvGetRequest {
		key: x.key,
		caching_behavior: caching_behavior_3_to_4(x.caching_behavior)?,
	})
}
pub(crate) fn kv_get_response_3_to_4(x: v3::KvGetResponse) -> Result<v4::KvGetResponse> {
	Ok(v4::KvGetResponse {
		value: x.value.map(|x| committed_value_3_to_4(x)).transpose()?,
	})
}
pub(crate) fn kv_purge_cache_entry_3_to_4(
	x: v3::KvPurgeCacheEntry,
) -> Result<v4::KvPurgeCacheEntry> {
	Ok(v4::KvPurgeCacheEntry {
		key: x.key,
		version: x.version,
	})
}
pub(crate) fn kv_purge_cache_request_3_to_4(
	x: v3::KvPurgeCacheRequest,
) -> Result<v4::KvPurgeCacheRequest> {
	Ok(v4::KvPurgeCacheRequest {
		entries: x
			.entries
			.into_iter()
			.map(|x| kv_purge_cache_entry_3_to_4(x))
			.collect::<Result<Vec<_>>>()?,
	})
}
pub(crate) fn changelog_read_request_3_to_4(
	x: v3::ChangelogReadRequest,
) -> Result<v4::ChangelogReadRequest> {
	Ok(v4::ChangelogReadRequest {
		after_versionstamp: x
			.after_versionstamp
			.map(|x| Ok::<_, anyhow::Error>(x))
			.transpose()?,
		count: x.count,
	})
}
pub(crate) fn changelog_entry_3_to_4(x: v3::ChangelogEntry) -> Result<v4::ChangelogEntry> {
	Ok(v4::ChangelogEntry {
		key: x.key,
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
		version: x.version,
		mutable: x.mutable,
	})
}
pub(crate) fn changelog_read_response_3_to_4(
	x: v3::ChangelogReadResponse,
) -> Result<v4::ChangelogReadResponse> {
	Ok(v4::ChangelogReadResponse {
		entries: x
			.entries
			.into_iter()
			.map(|x| changelog_entry_3_to_4(x))
			.collect::<Result<Vec<_>>>()?,
		last_versionstamp: x.last_versionstamp,
	})
}
pub(crate) fn coordinator_update_replica_status_request_3_to_4(
	x: v3::CoordinatorUpdateReplicaStatusRequest,
) -> Result<v4::CoordinatorUpdateReplicaStatusRequest> {
	Ok(v4::CoordinatorUpdateReplicaStatusRequest {
		replica_id: x.replica_id,
		status: replica_status_3_to_4(x.status)?,
	})
}
pub(crate) fn begin_learning_request_3_to_4(
	x: v3::BeginLearningRequest,
) -> Result<v4::BeginLearningRequest> {
	Ok(v4::BeginLearningRequest {
		config: cluster_config_3_to_4(x.config)?,
	})
}
pub(crate) fn request_kind_3_to_4(x: v3::RequestKind) -> Result<v4::RequestKind> {
	Ok(match x {
		v3::RequestKind::UpdateConfigRequest(x) => {
			v4::RequestKind::UpdateConfigRequest(update_config_request_3_to_4(x)?)
		}
		v3::RequestKind::PrepareRequest(x) => {
			v4::RequestKind::PrepareRequest(prepare_request_3_to_4(x)?)
		}
		v3::RequestKind::PreAcceptRequest(x) => {
			v4::RequestKind::PreAcceptRequest(pre_accept_request_3_to_4(x)?)
		}
		v3::RequestKind::AcceptRequest(x) => {
			v4::RequestKind::AcceptRequest(accept_request_3_to_4(x)?)
		}
		v3::RequestKind::CommitRequest(x) => {
			v4::RequestKind::CommitRequest(commit_request_3_to_4(x)?)
		}
		v3::RequestKind::ChangelogReadRequest(x) => {
			v4::RequestKind::ChangelogReadRequest(changelog_read_request_3_to_4(x)?)
		}
		v3::RequestKind::HealthCheckRequest => v4::RequestKind::HealthCheckRequest,
		v3::RequestKind::CoordinatorUpdateReplicaStatusRequest(x) => {
			v4::RequestKind::CoordinatorUpdateReplicaStatusRequest(
				coordinator_update_replica_status_request_3_to_4(x)?,
			)
		}
		v3::RequestKind::BeginLearningRequest(x) => {
			v4::RequestKind::BeginLearningRequest(begin_learning_request_3_to_4(x)?)
		}
		v3::RequestKind::KvGetRequest(x) => {
			v4::RequestKind::KvGetRequest(kv_get_request_3_to_4(x)?)
		}
		v3::RequestKind::KvPurgeCacheRequest(x) => {
			v4::RequestKind::KvPurgeCacheRequest(kv_purge_cache_request_3_to_4(x)?)
		}
	})
}
pub(crate) fn request_3_to_4(x: v3::Request) -> Result<v4::Request> {
	Ok(v4::Request {
		from_replica_id: x.from_replica_id,
		to_replica_id: x.to_replica_id,
		kind: request_kind_3_to_4(x.kind)?,
	})
}
pub(crate) fn response_kind_3_to_4(x: v3::ResponseKind) -> Result<v4::ResponseKind> {
	Ok(match x {
		v3::ResponseKind::UpdateConfigResponse => v4::ResponseKind::UpdateConfigResponse,
		v3::ResponseKind::PrepareResponse(x) => {
			v4::ResponseKind::PrepareResponse(prepare_response_3_to_4(x)?)
		}
		v3::ResponseKind::PreAcceptResponse(x) => {
			v4::ResponseKind::PreAcceptResponse(pre_accept_response_3_to_4(x)?)
		}
		v3::ResponseKind::AcceptResponse(x) => {
			v4::ResponseKind::AcceptResponse(accept_response_3_to_4(x)?)
		}
		v3::ResponseKind::CommitResponse(x) => {
			v4::ResponseKind::CommitResponse(commit_response_3_to_4(x)?)
		}
		v3::ResponseKind::ChangelogReadResponse(x) => {
			v4::ResponseKind::ChangelogReadResponse(changelog_read_response_3_to_4(x)?)
		}
		v3::ResponseKind::HealthCheckResponse => v4::ResponseKind::HealthCheckResponse,
		v3::ResponseKind::CoordinatorUpdateReplicaStatusResponse => {
			v4::ResponseKind::CoordinatorUpdateReplicaStatusResponse
		}
		v3::ResponseKind::BeginLearningResponse => v4::ResponseKind::BeginLearningResponse,
		v3::ResponseKind::KvGetResponse(x) => {
			v4::ResponseKind::KvGetResponse(kv_get_response_3_to_4(x)?)
		}
		v3::ResponseKind::KvPurgeCacheResponse => v4::ResponseKind::KvPurgeCacheResponse,
	})
}
pub(crate) fn response_3_to_4(x: v3::Response) -> Result<v4::Response> {
	Ok(v4::Response {
		kind: response_kind_3_to_4(x.kind)?,
	})
}
pub(crate) fn ballot_4_to_3(x: v4::Ballot) -> Result<v3::Ballot> {
	Ok(v3::Ballot {
		counter: x.counter,
		replica_id: x.replica_id,
	})
}
pub(crate) fn committed_value_4_to_3(x: v4::CommittedValue) -> Result<v3::CommittedValue> {
	Ok(v3::CommittedValue {
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
		version: x.version,
		mutable: x.mutable,
	})
}
pub(crate) fn cached_value_4_to_3(x: v4::CachedValue) -> Result<v3::CachedValue> {
	Ok(v3::CachedValue {
		value: x
			.value
			.map(|x| x.map(|x| Ok::<_, anyhow::Error>(x)).transpose())
			.transpose()?,
		version: x.version,
	})
}
pub(crate) fn accepted_value_4_to_3(x: v4::AcceptedValue) -> Result<v3::AcceptedValue> {
	Ok(v3::AcceptedValue {
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
		ballot: ballot_4_to_3(x.ballot)?,
		version: x.version,
		mutable: x.mutable,
	})
}
pub(crate) fn replica_status_4_to_3(x: v4::ReplicaStatus) -> Result<v3::ReplicaStatus> {
	Ok(match x {
		v4::ReplicaStatus::Joining => v3::ReplicaStatus::Joining,
		v4::ReplicaStatus::Learning => v3::ReplicaStatus::Learning,
		v4::ReplicaStatus::Active => v3::ReplicaStatus::Active,
	})
}
pub(crate) fn replica_config_4_to_3(x: v4::ReplicaConfig) -> Result<v3::ReplicaConfig> {
	Ok(v3::ReplicaConfig {
		replica_id: x.replica_id,
		status: replica_status_4_to_3(x.status)?,
		api_peer_url: x.api_peer_url,
		guard_url: x.guard_url,
	})
}
pub(crate) fn cluster_config_4_to_3(x: v4::ClusterConfig) -> Result<v3::ClusterConfig> {
	Ok(v3::ClusterConfig {
		coordinator_replica_id: x.coordinator_replica_id,
		epoch: x.epoch,
		replicas: x
			.replicas
			.into_iter()
			.map(|x| replica_config_4_to_3(x))
			.collect::<Result<Vec<_>>>()?,
	})
}
pub(crate) fn update_config_request_4_to_3(
	x: v4::UpdateConfigRequest,
) -> Result<v3::UpdateConfigRequest> {
	Ok(v3::UpdateConfigRequest {
		config: cluster_config_4_to_3(x.config)?,
	})
}
pub(crate) fn prepare_request_4_to_3(x: v4::PrepareRequest) -> Result<v3::PrepareRequest> {
	Ok(v3::PrepareRequest {
		key: x.key,
		ballot: ballot_4_to_3(x.ballot)?,
		mutable: x.mutable,
		version: x.version,
	})
}
pub(crate) fn prepare_response_ok_4_to_3(
	x: v4::PrepareResponseOk,
) -> Result<v3::PrepareResponseOk> {
	Ok(v3::PrepareResponseOk {
		highest_ballot: ballot_4_to_3(x.highest_ballot)?,
		accepted_value: x
			.accepted_value
			.map(|x| committed_value_4_to_3(x))
			.transpose()?,
		accepted_ballot: x.accepted_ballot.map(|x| ballot_4_to_3(x)).transpose()?,
	})
}
pub(crate) fn prepare_response_already_committed_4_to_3(
	x: v4::PrepareResponseAlreadyCommitted,
) -> Result<v3::PrepareResponseAlreadyCommitted> {
	Ok(v3::PrepareResponseAlreadyCommitted {
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
	})
}
pub(crate) fn prepare_response_higher_ballot_4_to_3(
	x: v4::PrepareResponseHigherBallot,
) -> Result<v3::PrepareResponseHigherBallot> {
	Ok(v3::PrepareResponseHigherBallot {
		ballot: ballot_4_to_3(x.ballot)?,
	})
}
pub(crate) fn prepare_response_4_to_3(x: v4::PrepareResponse) -> Result<v3::PrepareResponse> {
	Ok(match x {
		v4::PrepareResponse::PrepareResponseOk(x) => {
			v3::PrepareResponse::PrepareResponseOk(prepare_response_ok_4_to_3(x)?)
		}
		v4::PrepareResponse::PrepareResponseAlreadyCommitted(x) => {
			v3::PrepareResponse::PrepareResponseAlreadyCommitted(
				prepare_response_already_committed_4_to_3(x)?,
			)
		}
		v4::PrepareResponse::PrepareResponseHigherBallot(x) => {
			v3::PrepareResponse::PrepareResponseHigherBallot(prepare_response_higher_ballot_4_to_3(
				x,
			)?)
		}
	})
}
pub(crate) fn pre_accept_request_4_to_3(x: v4::PreAcceptRequest) -> Result<v3::PreAcceptRequest> {
	Ok(v3::PreAcceptRequest {
		key: x.key,
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
		ballot: ballot_4_to_3(x.ballot)?,
		mutable: x.mutable,
		version: x.version,
	})
}
pub(crate) fn pre_accept_response_ok_4_to_3(
	x: v4::PreAcceptResponseOk,
) -> Result<v3::PreAcceptResponseOk> {
	Ok(v3::PreAcceptResponseOk {
		ballot: ballot_4_to_3(x.ballot)?,
	})
}
pub(crate) fn pre_accept_response_already_committed_4_to_3(
	x: v4::PreAcceptResponseAlreadyCommitted,
) -> Result<v3::PreAcceptResponseAlreadyCommitted> {
	Ok(v3::PreAcceptResponseAlreadyCommitted {
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
	})
}
pub(crate) fn pre_accept_response_higher_ballot_4_to_3(
	x: v4::PreAcceptResponseHigherBallot,
) -> Result<v3::PreAcceptResponseHigherBallot> {
	Ok(v3::PreAcceptResponseHigherBallot {
		ballot: ballot_4_to_3(x.ballot)?,
	})
}
pub(crate) fn pre_accept_response_4_to_3(
	x: v4::PreAcceptResponse,
) -> Result<v3::PreAcceptResponse> {
	Ok(match x {
		v4::PreAcceptResponse::PreAcceptResponseOk(x) => {
			v3::PreAcceptResponse::PreAcceptResponseOk(pre_accept_response_ok_4_to_3(x)?)
		}
		v4::PreAcceptResponse::PreAcceptResponseAlreadyCommitted(x) => {
			v3::PreAcceptResponse::PreAcceptResponseAlreadyCommitted(
				pre_accept_response_already_committed_4_to_3(x)?,
			)
		}
		v4::PreAcceptResponse::PreAcceptResponseHigherBallot(x) => {
			v3::PreAcceptResponse::PreAcceptResponseHigherBallot(
				pre_accept_response_higher_ballot_4_to_3(x)?,
			)
		}
	})
}
pub(crate) fn accept_request_4_to_3(x: v4::AcceptRequest) -> Result<v3::AcceptRequest> {
	Ok(v3::AcceptRequest {
		key: x.key,
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
		ballot: ballot_4_to_3(x.ballot)?,
		mutable: x.mutable,
		version: x.version,
	})
}
pub(crate) fn accept_response_ok_4_to_3(x: v4::AcceptResponseOk) -> Result<v3::AcceptResponseOk> {
	Ok(v3::AcceptResponseOk {
		ballot: ballot_4_to_3(x.ballot)?,
	})
}
pub(crate) fn accept_response_already_committed_4_to_3(
	x: v4::AcceptResponseAlreadyCommitted,
) -> Result<v3::AcceptResponseAlreadyCommitted> {
	Ok(v3::AcceptResponseAlreadyCommitted {
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
	})
}
pub(crate) fn accept_response_higher_ballot_4_to_3(
	x: v4::AcceptResponseHigherBallot,
) -> Result<v3::AcceptResponseHigherBallot> {
	Ok(v3::AcceptResponseHigherBallot {
		ballot: ballot_4_to_3(x.ballot)?,
	})
}
pub(crate) fn accept_response_4_to_3(x: v4::AcceptResponse) -> Result<v3::AcceptResponse> {
	Ok(match x {
		v4::AcceptResponse::AcceptResponseOk(x) => {
			v3::AcceptResponse::AcceptResponseOk(accept_response_ok_4_to_3(x)?)
		}
		v4::AcceptResponse::AcceptResponseAlreadyCommitted(x) => {
			v3::AcceptResponse::AcceptResponseAlreadyCommitted(
				accept_response_already_committed_4_to_3(x)?,
			)
		}
		v4::AcceptResponse::AcceptResponseHigherBallot(x) => {
			v3::AcceptResponse::AcceptResponseHigherBallot(accept_response_higher_ballot_4_to_3(x)?)
		}
	})
}
pub(crate) fn commit_request_4_to_3(x: v4::CommitRequest) -> Result<v3::CommitRequest> {
	Ok(v3::CommitRequest {
		key: x.key,
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
		ballot: ballot_4_to_3(x.ballot)?,
		mutable: x.mutable,
		version: x.version,
	})
}
pub(crate) fn commit_response_already_committed_4_to_3(
	x: v4::CommitResponseAlreadyCommitted,
) -> Result<v3::CommitResponseAlreadyCommitted> {
	Ok(v3::CommitResponseAlreadyCommitted {
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
	})
}
pub(crate) fn commit_response_4_to_3(x: v4::CommitResponse) -> Result<v3::CommitResponse> {
	Ok(match x {
		v4::CommitResponse::CommitResponseOk => v3::CommitResponse::CommitResponseOk,
		v4::CommitResponse::CommitResponseAlreadyCommitted(x) => {
			v3::CommitResponse::CommitResponseAlreadyCommitted(
				commit_response_already_committed_4_to_3(x)?,
			)
		}
		v4::CommitResponse::CommitResponseStaleCommit => {
			v3::CommitResponse::CommitResponseStaleCommit
		}
	})
}
pub(crate) fn caching_behavior_4_to_3(x: v4::CachingBehavior) -> Result<v3::CachingBehavior> {
	Ok(match x {
		v4::CachingBehavior::Optimistic => v3::CachingBehavior::Optimistic,
		v4::CachingBehavior::SkipCache => v3::CachingBehavior::SkipCache,
	})
}
pub(crate) fn kv_get_request_4_to_3(x: v4::KvGetRequest) -> Result<v3::KvGetRequest> {
	Ok(v3::KvGetRequest {
		key: x.key,
		caching_behavior: caching_behavior_4_to_3(x.caching_behavior)?,
	})
}
pub(crate) fn kv_get_response_4_to_3(x: v4::KvGetResponse) -> Result<v3::KvGetResponse> {
	Ok(v3::KvGetResponse {
		value: x.value.map(|x| committed_value_4_to_3(x)).transpose()?,
	})
}
pub(crate) fn kv_purge_cache_entry_4_to_3(
	x: v4::KvPurgeCacheEntry,
) -> Result<v3::KvPurgeCacheEntry> {
	Ok(v3::KvPurgeCacheEntry {
		key: x.key,
		version: x.version,
	})
}
pub(crate) fn kv_purge_cache_request_4_to_3(
	x: v4::KvPurgeCacheRequest,
) -> Result<v3::KvPurgeCacheRequest> {
	Ok(v3::KvPurgeCacheRequest {
		entries: x
			.entries
			.into_iter()
			.map(|x| kv_purge_cache_entry_4_to_3(x))
			.collect::<Result<Vec<_>>>()?,
	})
}
pub(crate) fn changelog_read_request_4_to_3(
	x: v4::ChangelogReadRequest,
) -> Result<v3::ChangelogReadRequest> {
	Ok(v3::ChangelogReadRequest {
		after_versionstamp: x
			.after_versionstamp
			.map(|x| Ok::<_, anyhow::Error>(x))
			.transpose()?,
		count: x.count,
	})
}
pub(crate) fn changelog_entry_4_to_3(x: v4::ChangelogEntry) -> Result<v3::ChangelogEntry> {
	Ok(v3::ChangelogEntry {
		key: x.key,
		value: x.value.map(|x| Ok::<_, anyhow::Error>(x)).transpose()?,
		version: x.version,
		mutable: x.mutable,
	})
}
pub(crate) fn changelog_read_response_4_to_3(
	x: v4::ChangelogReadResponse,
) -> Result<v3::ChangelogReadResponse> {
	Ok(v3::ChangelogReadResponse {
		entries: x
			.entries
			.into_iter()
			.map(|x| changelog_entry_4_to_3(x))
			.collect::<Result<Vec<_>>>()?,
		last_versionstamp: x.last_versionstamp,
	})
}
pub(crate) fn coordinator_update_replica_status_request_4_to_3(
	x: v4::CoordinatorUpdateReplicaStatusRequest,
) -> Result<v3::CoordinatorUpdateReplicaStatusRequest> {
	Ok(v3::CoordinatorUpdateReplicaStatusRequest {
		replica_id: x.replica_id,
		status: replica_status_4_to_3(x.status)?,
	})
}
pub(crate) fn begin_learning_request_4_to_3(
	x: v4::BeginLearningRequest,
) -> Result<v3::BeginLearningRequest> {
	Ok(v3::BeginLearningRequest {
		config: cluster_config_4_to_3(x.config)?,
	})
}
pub(crate) fn request_kind_4_to_3(x: v4::RequestKind) -> Result<v3::RequestKind> {
	Ok(match x {
		v4::RequestKind::UpdateConfigRequest(x) => {
			v3::RequestKind::UpdateConfigRequest(update_config_request_4_to_3(x)?)
		}
		v4::RequestKind::PrepareRequest(x) => {
			v3::RequestKind::PrepareRequest(prepare_request_4_to_3(x)?)
		}
		v4::RequestKind::PreAcceptRequest(x) => {
			v3::RequestKind::PreAcceptRequest(pre_accept_request_4_to_3(x)?)
		}
		v4::RequestKind::AcceptRequest(x) => {
			v3::RequestKind::AcceptRequest(accept_request_4_to_3(x)?)
		}
		v4::RequestKind::CommitRequest(x) => {
			v3::RequestKind::CommitRequest(commit_request_4_to_3(x)?)
		}
		v4::RequestKind::ChangelogReadRequest(x) => {
			v3::RequestKind::ChangelogReadRequest(changelog_read_request_4_to_3(x)?)
		}
		v4::RequestKind::HealthCheckRequest => v3::RequestKind::HealthCheckRequest,
		v4::RequestKind::CoordinatorUpdateReplicaStatusRequest(x) => {
			v3::RequestKind::CoordinatorUpdateReplicaStatusRequest(
				coordinator_update_replica_status_request_4_to_3(x)?,
			)
		}
		v4::RequestKind::BeginLearningRequest(x) => {
			v3::RequestKind::BeginLearningRequest(begin_learning_request_4_to_3(x)?)
		}
		v4::RequestKind::KvGetRequest(x) => {
			v3::RequestKind::KvGetRequest(kv_get_request_4_to_3(x)?)
		}
		v4::RequestKind::KvPurgeCacheRequest(x) => {
			v3::RequestKind::KvPurgeCacheRequest(kv_purge_cache_request_4_to_3(x)?)
		}
		v4::RequestKind::KvReadStateRequest(_) => {
			bail!(crate::READ_STATE_REQUIRES_V4_ERROR)
		}
	})
}
pub(crate) fn request_4_to_3(x: v4::Request) -> Result<v3::Request> {
	Ok(v3::Request {
		from_replica_id: x.from_replica_id,
		to_replica_id: x.to_replica_id,
		kind: request_kind_4_to_3(x.kind)?,
	})
}
pub(crate) fn response_kind_4_to_3(x: v4::ResponseKind) -> Result<v3::ResponseKind> {
	Ok(match x {
		v4::ResponseKind::UpdateConfigResponse => v3::ResponseKind::UpdateConfigResponse,
		v4::ResponseKind::PrepareResponse(x) => {
			v3::ResponseKind::PrepareResponse(prepare_response_4_to_3(x)?)
		}
		v4::ResponseKind::PreAcceptResponse(x) => {
			v3::ResponseKind::PreAcceptResponse(pre_accept_response_4_to_3(x)?)
		}
		v4::ResponseKind::AcceptResponse(x) => {
			v3::ResponseKind::AcceptResponse(accept_response_4_to_3(x)?)
		}
		v4::ResponseKind::CommitResponse(x) => {
			v3::ResponseKind::CommitResponse(commit_response_4_to_3(x)?)
		}
		v4::ResponseKind::ChangelogReadResponse(x) => {
			v3::ResponseKind::ChangelogReadResponse(changelog_read_response_4_to_3(x)?)
		}
		v4::ResponseKind::HealthCheckResponse => v3::ResponseKind::HealthCheckResponse,
		v4::ResponseKind::CoordinatorUpdateReplicaStatusResponse => {
			v3::ResponseKind::CoordinatorUpdateReplicaStatusResponse
		}
		v4::ResponseKind::BeginLearningResponse => v3::ResponseKind::BeginLearningResponse,
		v4::ResponseKind::KvGetResponse(x) => {
			v3::ResponseKind::KvGetResponse(kv_get_response_4_to_3(x)?)
		}
		v4::ResponseKind::KvPurgeCacheResponse => v3::ResponseKind::KvPurgeCacheResponse,
		v4::ResponseKind::KvReadStateResponse(_) => {
			bail!(crate::READ_STATE_REQUIRES_V4_ERROR)
		}
	})
}
pub(crate) fn response_4_to_3(x: v4::Response) -> Result<v3::Response> {
	Ok(v3::Response {
		kind: response_kind_4_to_3(x.kind)?,
	})
}
