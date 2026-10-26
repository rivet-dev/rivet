use crate::generated::pegboard_actor_v1 as wire;

pub enum ActorData {
	V1(wire::ActorData),
}
impl_versioned_v1!(ActorData, wire::ActorData);

pub enum ActorState {
	V1(wire::ActorState),
}

impl_versioned_v1!(ActorState, wire::ActorState);

pub enum ActorTimer {
	V1(wire::ActorTimer),
}

impl_versioned_v1!(ActorTimer, wire::ActorTimer);

pub enum ActorCreateRequest {
	V1(wire::ActorCreateRequest),
}

impl_versioned_v1!(ActorCreateRequest, wire::ActorCreateRequest);

pub enum ActorCreateResponse {
	V1(wire::ActorCreateResponse),
}
impl_versioned_v1!(ActorCreateResponse, wire::ActorCreateResponse);

pub enum ActorReadRequest {
	V1(wire::ActorReadRequest),
}

impl_versioned_v1!(ActorReadRequest, wire::ActorReadRequest);

pub enum ActorReadResponse {
	V1(wire::ActorReadResponse),
}

impl_versioned_v1!(ActorReadResponse, wire::ActorReadResponse);

pub enum ActorStartNotice {
	V1(wire::ActorStartNotice),
}

impl_versioned_v1!(ActorStartNotice, wire::ActorStartNotice);

pub enum ActorEvent {
	V1(wire::ActorEvent),
}

impl_versioned_v1!(ActorEvent, wire::ActorEvent);

pub enum ActorEventResponse {
	V1(wire::ActorEventResponse),
}
impl_versioned_v1!(ActorEventResponse, wire::ActorEventResponse);

pub enum IndexInsertActorRequest {
	V1(wire::IndexInsertActorRequest),
}

impl_versioned_v1!(IndexInsertActorRequest, wire::IndexInsertActorRequest);

pub enum IndexInsertActorResponse {
	V1(wire::IndexInsertActorResponse),
}

impl_versioned_v1!(IndexInsertActorResponse, wire::IndexInsertActorResponse);

pub enum IndexDeleteActorRequest {
	V1(wire::IndexDeleteActorRequest),
}

impl_versioned_v1!(IndexDeleteActorRequest, wire::IndexDeleteActorRequest);

pub enum IndexDeleteActorResponse {
	V1(wire::IndexDeleteActorResponse),
}

impl_versioned_v1!(IndexDeleteActorResponse, wire::IndexDeleteActorResponse);

pub enum IndexListActorsRequest {
	V1(wire::IndexListActorsRequest),
}

impl_versioned_v1!(IndexListActorsRequest, wire::IndexListActorsRequest);

pub enum IndexListActorsResponse {
	V1(wire::IndexListActorsResponse),
}

impl_versioned_v1!(IndexListActorsResponse, wire::IndexListActorsResponse);

pub enum DestroyedActorIndexEntry {
	V1(wire::DestroyedActorIndexEntry),
}

impl_versioned_v1!(DestroyedActorIndexEntry, wire::DestroyedActorIndexEntry);

pub enum IndexReadDestroyedActorRequest {
	V1(wire::IndexReadDestroyedActorRequest),
}

impl_versioned_v1!(
	IndexReadDestroyedActorRequest,
	wire::IndexReadDestroyedActorRequest
);

pub enum IndexReadDestroyedActorResponse {
	V1(wire::IndexReadDestroyedActorResponse),
}

impl_versioned_v1!(
	IndexReadDestroyedActorResponse,
	wire::IndexReadDestroyedActorResponse
);

pub enum IndexListEnvoysRequest {
	V1(wire::IndexListEnvoysRequest),
}

impl_versioned_v1!(IndexListEnvoysRequest, wire::IndexListEnvoysRequest);

pub enum IndexListEnvoysResponse {
	V1(wire::IndexListEnvoysResponse),
}

impl_versioned_v1!(IndexListEnvoysResponse, wire::IndexListEnvoysResponse);

pub enum EpoxyActorByKeyEntry {
	V1(wire::EpoxyActorByKeyEntry),
}

impl_versioned_v1!(EpoxyActorByKeyEntry, wire::EpoxyActorByKeyEntry);

pub enum KvValue {
	V1(wire::KvValue),
}

impl_versioned_v1!(KvValue, wire::KvValue);

pub enum EnvoyData {
	V1(wire::EnvoyData),
}

impl_versioned_v1!(EnvoyData, wire::EnvoyData);

pub enum EnvoyUpsertRequest {
	V1(wire::EnvoyUpsertRequest),
}

impl_versioned_v1!(EnvoyUpsertRequest, wire::EnvoyUpsertRequest);

pub enum EnvoyUpsertResponse {
	V1(wire::EnvoyUpsertResponse),
}

impl_versioned_v1!(EnvoyUpsertResponse, wire::EnvoyUpsertResponse);

pub enum EnvoyReadRequest {
	V1(wire::EnvoyReadRequest),
}

impl_versioned_v1!(EnvoyReadRequest, wire::EnvoyReadRequest);

pub enum EnvoyReadResponse {
	V1(wire::EnvoyReadResponse),
}

impl_versioned_v1!(EnvoyReadResponse, wire::EnvoyReadResponse);

pub enum IndexInsertEnvoyRequest {
	V1(wire::IndexInsertEnvoyRequest),
}

impl_versioned_v1!(IndexInsertEnvoyRequest, wire::IndexInsertEnvoyRequest);

pub enum IndexInsertEnvoyResponse {
	V1(wire::IndexInsertEnvoyResponse),
}

impl_versioned_v1!(IndexInsertEnvoyResponse, wire::IndexInsertEnvoyResponse);

pub enum IndexUpsertActiveEnvoyRequest {
	V1(wire::IndexUpsertActiveEnvoyRequest),
}

impl_versioned_v1!(
	IndexUpsertActiveEnvoyRequest,
	wire::IndexUpsertActiveEnvoyRequest
);

pub enum IndexUpsertActiveEnvoyResponse {
	V1(wire::IndexUpsertActiveEnvoyResponse),
}

impl_versioned_v1!(
	IndexUpsertActiveEnvoyResponse,
	wire::IndexUpsertActiveEnvoyResponse
);

pub enum IndexDeleteActiveEnvoyRequest {
	V1(wire::IndexDeleteActiveEnvoyRequest),
}

impl_versioned_v1!(
	IndexDeleteActiveEnvoyRequest,
	wire::IndexDeleteActiveEnvoyRequest
);

pub enum IndexDeleteActiveEnvoyResponse {
	V1(wire::IndexDeleteActiveEnvoyResponse),
}

impl_versioned_v1!(
	IndexDeleteActiveEnvoyResponse,
	wire::IndexDeleteActiveEnvoyResponse
);

pub enum IndexPutEnvoyPoolRequest {
	V1(wire::IndexPutEnvoyPoolRequest),
}

impl_versioned_v1!(IndexPutEnvoyPoolRequest, wire::IndexPutEnvoyPoolRequest);

pub enum IndexPutEnvoyPoolResponse {
	V1(wire::IndexPutEnvoyPoolResponse),
}

impl_versioned_v1!(IndexPutEnvoyPoolResponse, wire::IndexPutEnvoyPoolResponse);

pub enum IndexListEnvoyPoolsRequest {
	V1(wire::IndexListEnvoyPoolsRequest),
}

impl_versioned_v1!(IndexListEnvoyPoolsRequest, wire::IndexListEnvoyPoolsRequest);

pub enum IndexListEnvoyPoolsResponse {
	V1(wire::IndexListEnvoyPoolsResponse),
}

impl_versioned_v1!(
	IndexListEnvoyPoolsResponse,
	wire::IndexListEnvoyPoolsResponse
);

pub enum IndexListActiveEnvoysRequest {
	V1(wire::IndexListActiveEnvoysRequest),
}

impl_versioned_v1!(
	IndexListActiveEnvoysRequest,
	wire::IndexListActiveEnvoysRequest
);

pub enum IndexListActiveEnvoysResponse {
	V1(wire::IndexListActiveEnvoysResponse),
}

impl_versioned_v1!(
	IndexListActiveEnvoysResponse,
	wire::IndexListActiveEnvoysResponse
);

pub enum IndexListActiveEnvoyStatesRequest {
	V1(wire::IndexListActiveEnvoyStatesRequest),
}

impl_versioned_v1!(
	IndexListActiveEnvoyStatesRequest,
	wire::IndexListActiveEnvoyStatesRequest
);

pub enum IndexListActiveEnvoyStatesResponse {
	V1(wire::IndexListActiveEnvoyStatesResponse),
}

impl_versioned_v1!(
	IndexListActiveEnvoyStatesResponse,
	wire::IndexListActiveEnvoyStatesResponse
);

pub enum ActiveEnvoyIndexEntry {
	V1(wire::ActiveEnvoyIndexEntry),
}

impl_versioned_v1!(ActiveEnvoyIndexEntry, wire::ActiveEnvoyIndexEntry);

pub enum ActiveEnvoyState {
	V1(wire::ActiveEnvoyState),
}

impl_versioned_v1!(ActiveEnvoyState, wire::ActiveEnvoyState);

pub enum NamespaceRecord {
	V1(wire::NamespaceRecord),
}

impl_versioned_v1!(NamespaceRecord, wire::NamespaceRecord);

pub enum NamespaceCreateRequest {
	V1(wire::NamespaceCreateRequest),
}

impl_versioned_v1!(NamespaceCreateRequest, wire::NamespaceCreateRequest);

pub enum NamespaceCreateResponse {
	V1(wire::NamespaceCreateResponse),
}

impl_versioned_v1!(NamespaceCreateResponse, wire::NamespaceCreateResponse);

pub enum NamespaceReadRequest {
	V1(wire::NamespaceReadRequest),
}

impl_versioned_v1!(NamespaceReadRequest, wire::NamespaceReadRequest);

pub enum NamespaceReadResponse {
	V1(wire::NamespaceReadResponse),
}

impl_versioned_v1!(NamespaceReadResponse, wire::NamespaceReadResponse);

pub enum NamespaceListRequest {
	V1(wire::NamespaceListRequest),
}

impl_versioned_v1!(NamespaceListRequest, wire::NamespaceListRequest);

pub enum NamespaceListResponse {
	V1(wire::NamespaceListResponse),
}

impl_versioned_v1!(NamespaceListResponse, wire::NamespaceListResponse);

pub enum NodePoolConfig {
	V1(wire::NodePoolConfig),
}

impl_versioned_v1!(NodePoolConfig, wire::NodePoolConfig);

pub enum PoolRequest {
	V1(wire::PoolRequest),
}

impl_versioned_v1!(PoolRequest, wire::PoolRequest);

pub enum PoolResponse {
	V1(wire::PoolResponse),
}

impl_versioned_v1!(PoolResponse, wire::PoolResponse);

pub enum PoolLease {
	V1(wire::PoolLease),
}

impl_versioned_v1!(PoolLease, wire::PoolLease);

pub enum PoolListRequest {
	V1(wire::PoolListRequest),
}
impl_versioned_v1!(PoolListRequest, wire::PoolListRequest);

pub enum PoolListResponse {
	V1(wire::PoolListResponse),
}
impl_versioned_v1!(PoolListResponse, wire::PoolListResponse);
