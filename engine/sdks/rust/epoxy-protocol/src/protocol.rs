use serde::{Deserialize, Serialize};

use crate::generated::v2 as raw;

pub type ReplicaId = raw::ReplicaId;
pub type ReplicaStatus = raw::ReplicaStatus;
pub type ReplicaConfig = raw::ReplicaConfig;
pub type ClusterConfig = raw::ClusterConfig;
pub type Ballot = raw::Ballot;
pub type CommittedValue = raw::CommittedValue;

pub type UpdateConfigRequest = raw::UpdateConfigRequest;
pub type UpdateConfigResponse = raw::UpdateConfigResponse;

pub type PrepareRequest = raw::PrepareRequest;
pub type PrepareResponse = raw::PrepareResponse;
pub type PrepareResponseOk = raw::PrepareResponseOk;
pub type PrepareResponseAlreadyCommitted = raw::PrepareResponseAlreadyCommitted;
pub type PrepareResponseHigherBallot = raw::PrepareResponseHigherBallot;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct AcceptRequest {
	pub key: Vec<u8>,
	pub value: Vec<u8>,
	pub ballot: Ballot,
	pub mutable: bool,
	pub version: u64,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct AcceptResponseOk {
	pub ballot: Ballot,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct AcceptResponseAlreadyCommitted {
	pub value: Vec<u8>,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct AcceptResponseHigherBallot {
	pub ballot: Ballot,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub enum AcceptResponse {
	AcceptResponseOk(AcceptResponseOk),
	AcceptResponseAlreadyCommitted(AcceptResponseAlreadyCommitted),
	AcceptResponseHigherBallot(AcceptResponseHigherBallot),
}

pub type CommitRequest = raw::CommitRequest;
pub type CommitResponse = raw::CommitResponse;
pub type CommitResponseOk = raw::CommitResponseOk;
pub type CommitResponseAlreadyCommitted = raw::CommitResponseAlreadyCommitted;
pub type CommitResponseStaleCommit = raw::CommitResponseStaleCommit;

pub type CachingBehavior = raw::CachingBehavior;
pub type KvGetRequest = raw::KvGetRequest;
pub type KvGetResponse = raw::KvGetResponse;
pub type KvPurgeCacheEntry = raw::KvPurgeCacheEntry;
pub type KvPurgeCacheRequest = raw::KvPurgeCacheRequest;
pub type KvPurgeCacheResponse = raw::KvPurgeCacheResponse;

pub type ChangelogReadRequest = raw::ChangelogReadRequest;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct ChangelogEntry {
	pub key: Vec<u8>,
	pub value: Vec<u8>,
	#[serde(default)]
	pub version: u64,
	#[serde(default)]
	pub mutable: bool,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct ChangelogReadResponse {
	pub entries: Vec<ChangelogEntry>,
	pub last_versionstamp: Vec<u8>,
}

pub type HealthCheckRequest = raw::HealthCheckRequest;
pub type HealthCheckResponse = raw::HealthCheckResponse;
pub type CoordinatorUpdateReplicaStatusRequest = raw::CoordinatorUpdateReplicaStatusRequest;
pub type CoordinatorUpdateReplicaStatusResponse = raw::CoordinatorUpdateReplicaStatusResponse;
pub type BeginLearningRequest = raw::BeginLearningRequest;
pub type BeginLearningResponse = raw::BeginLearningResponse;

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub enum RequestKind {
	UpdateConfigRequest(UpdateConfigRequest),
	PrepareRequest(PrepareRequest),
	AcceptRequest(AcceptRequest),
	CommitRequest(CommitRequest),
	ChangelogReadRequest(ChangelogReadRequest),
	HealthCheckRequest,
	CoordinatorUpdateReplicaStatusRequest(CoordinatorUpdateReplicaStatusRequest),
	BeginLearningRequest(BeginLearningRequest),
	KvGetRequest(KvGetRequest),
	KvPurgeCacheRequest(KvPurgeCacheRequest),
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct Request {
	pub from_replica_id: ReplicaId,
	pub to_replica_id: ReplicaId,
	pub kind: RequestKind,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub enum ResponseKind {
	UpdateConfigResponse,
	PrepareResponse(PrepareResponse),
	AcceptResponse(AcceptResponse),
	CommitResponse(CommitResponse),
	ChangelogReadResponse(ChangelogReadResponse),
	HealthCheckResponse,
	CoordinatorUpdateReplicaStatusResponse,
	BeginLearningResponse,
	KvGetResponse(KvGetResponse),
	KvPurgeCacheResponse,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone, Hash)]
pub struct Response {
	pub kind: ResponseKind,
}
