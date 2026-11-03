//! Version envelopes for the actor state machine's state, timers, events and commands, and the
//! index, envoy, namespace and pool records stored beside them.

mod pegboard_actor_v1_to_pegboard_actor_v2;
mod pegboard_actor_v2_to_pegboard_actor_v1;

use super::unchanged;
use crate::generated::{pegboard_actor_v1 as v1, pegboard_actor_v2 as v2};
use pegboard_actor_v1_to_pegboard_actor_v2 as up;
use pegboard_actor_v2_to_pegboard_actor_v1 as down;

/// The envoy version read back from records written at a version that does not carry one.
/// Clients default to version 1 when none is configured, so such an envoy ranks with those.
pub const ENVOY_VERSION_BEFORE_VERSIONS: u32 = 1;

/// The first `pegboard.actor` version whose envoy records and active index state carry the
/// envoy's version.
pub const ENVOY_VERSION_ACTOR_VERSION: u16 = 2;

impl_versioned_v1_v2!(
	ActorData,
	v1::ActorData,
	v2::ActorData,
	up::convert_actor_data_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_data_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	ActorState,
	v1::ActorState,
	v2::ActorState,
	up::convert_actor_state_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_state_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	ActorTimer,
	v1::ActorTimer,
	v2::ActorTimer,
	up::convert_actor_timer_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_timer_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	ActorCreateRequest,
	v1::ActorCreateRequest,
	v2::ActorCreateRequest,
	up::convert_actor_create_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_create_request_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	ActorCreateResponse,
	v1::ActorCreateResponse,
	v2::ActorCreateResponse,
	up::convert_actor_create_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_create_response_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	ActorReadRequest,
	v1::ActorReadRequest,
	v2::ActorReadRequest,
	unchanged,
	unchanged
);

impl_versioned_v1_v2!(
	ActorReadResponse,
	v1::ActorReadResponse,
	v2::ActorReadResponse,
	up::convert_actor_read_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_read_response_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	ActorStartNotice,
	v1::ActorStartNotice,
	v2::ActorStartNotice,
	up::convert_actor_start_notice_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_start_notice_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	ActorEvent,
	v1::ActorEvent,
	v2::ActorEvent,
	up::convert_actor_event_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_event_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	ActorEventResponse,
	v1::ActorEventResponse,
	v2::ActorEventResponse,
	up::convert_actor_event_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_event_response_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	IndexInsertActorRequest,
	v1::IndexInsertActorRequest,
	v2::IndexInsertActorRequest,
	up::convert_index_insert_actor_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_insert_actor_request_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	IndexInsertActorResponse,
	v1::IndexInsertActorResponse,
	v2::IndexInsertActorResponse,
	unchanged,
	unchanged
);

impl_versioned_v1_v2!(
	IndexDeleteActorRequest,
	v1::IndexDeleteActorRequest,
	v2::IndexDeleteActorRequest,
	up::convert_index_delete_actor_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_delete_actor_request_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	IndexDeleteActorResponse,
	v1::IndexDeleteActorResponse,
	v2::IndexDeleteActorResponse,
	unchanged,
	unchanged
);

impl_versioned_v1_v2!(
	IndexListActorsRequest,
	v1::IndexListActorsRequest,
	v2::IndexListActorsRequest,
	up::convert_index_list_actors_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_actors_request_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	IndexListActorsResponse,
	v1::IndexListActorsResponse,
	v2::IndexListActorsResponse,
	up::convert_index_list_actors_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_actors_response_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	DestroyedActorIndexEntry,
	v1::DestroyedActorIndexEntry,
	v2::DestroyedActorIndexEntry,
	up::convert_destroyed_actor_index_entry_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_destroyed_actor_index_entry_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	IndexReadDestroyedActorRequest,
	v1::IndexReadDestroyedActorRequest,
	v2::IndexReadDestroyedActorRequest,
	up::convert_index_read_destroyed_actor_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_read_destroyed_actor_request_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	IndexReadDestroyedActorResponse,
	v1::IndexReadDestroyedActorResponse,
	v2::IndexReadDestroyedActorResponse,
	up::convert_index_read_destroyed_actor_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_read_destroyed_actor_response_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	IndexListEnvoysRequest,
	v1::IndexListEnvoysRequest,
	v2::IndexListEnvoysRequest,
	up::convert_index_list_envoys_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_envoys_request_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	IndexListEnvoysResponse,
	v1::IndexListEnvoysResponse,
	v2::IndexListEnvoysResponse,
	up::convert_index_list_envoys_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_envoys_response_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	EpoxyActorByKeyEntry,
	v1::EpoxyActorByKeyEntry,
	v2::EpoxyActorByKeyEntry,
	up::convert_epoxy_actor_by_key_entry_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_epoxy_actor_by_key_entry_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	KvValue,
	v1::KvValue,
	v2::KvValue,
	up::convert_kv_value_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_kv_value_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	EnvoyData,
	v1::EnvoyData,
	v2::EnvoyData,
	up::convert_envoy_data_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_envoy_data_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	EnvoyUpsertRequest,
	v1::EnvoyUpsertRequest,
	v2::EnvoyUpsertRequest,
	up::convert_envoy_upsert_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_envoy_upsert_request_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	EnvoyUpsertResponse,
	v1::EnvoyUpsertResponse,
	v2::EnvoyUpsertResponse,
	up::convert_envoy_upsert_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_envoy_upsert_response_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	EnvoyReadRequest,
	v1::EnvoyReadRequest,
	v2::EnvoyReadRequest,
	unchanged,
	unchanged
);

impl_versioned_v1_v2!(
	EnvoyReadResponse,
	v1::EnvoyReadResponse,
	v2::EnvoyReadResponse,
	up::convert_envoy_read_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_envoy_read_response_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	IndexInsertEnvoyRequest,
	v1::IndexInsertEnvoyRequest,
	v2::IndexInsertEnvoyRequest,
	up::convert_index_insert_envoy_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_insert_envoy_request_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	IndexInsertEnvoyResponse,
	v1::IndexInsertEnvoyResponse,
	v2::IndexInsertEnvoyResponse,
	unchanged,
	unchanged
);

impl_versioned_v1_v2!(
	IndexUpsertActiveEnvoyRequest,
	v1::IndexUpsertActiveEnvoyRequest,
	v2::IndexUpsertActiveEnvoyRequest,
	up::convert_index_upsert_active_envoy_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_upsert_active_envoy_request_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	IndexUpsertActiveEnvoyResponse,
	v1::IndexUpsertActiveEnvoyResponse,
	v2::IndexUpsertActiveEnvoyResponse,
	up::convert_index_upsert_active_envoy_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_upsert_active_envoy_response_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	IndexDeleteActiveEnvoyRequest,
	v1::IndexDeleteActiveEnvoyRequest,
	v2::IndexDeleteActiveEnvoyRequest,
	up::convert_index_delete_active_envoy_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_delete_active_envoy_request_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	IndexDeleteActiveEnvoyResponse,
	v1::IndexDeleteActiveEnvoyResponse,
	v2::IndexDeleteActiveEnvoyResponse,
	unchanged,
	unchanged
);

impl_versioned_v1_v2!(
	IndexPutEnvoyPoolRequest,
	v1::IndexPutEnvoyPoolRequest,
	v2::IndexPutEnvoyPoolRequest,
	up::convert_index_put_envoy_pool_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_put_envoy_pool_request_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	IndexPutEnvoyPoolResponse,
	v1::IndexPutEnvoyPoolResponse,
	v2::IndexPutEnvoyPoolResponse,
	unchanged,
	unchanged
);

impl_versioned_v1_v2!(
	IndexListEnvoyPoolsRequest,
	v1::IndexListEnvoyPoolsRequest,
	v2::IndexListEnvoyPoolsRequest,
	unchanged,
	unchanged
);

impl_versioned_v1_v2!(
	IndexListEnvoyPoolsResponse,
	v1::IndexListEnvoyPoolsResponse,
	v2::IndexListEnvoyPoolsResponse,
	up::convert_index_list_envoy_pools_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_envoy_pools_response_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	IndexListActiveEnvoysRequest,
	v1::IndexListActiveEnvoysRequest,
	v2::IndexListActiveEnvoysRequest,
	up::convert_index_list_active_envoys_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_active_envoys_request_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	IndexListActiveEnvoysResponse,
	v1::IndexListActiveEnvoysResponse,
	v2::IndexListActiveEnvoysResponse,
	up::convert_index_list_active_envoys_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_active_envoys_response_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	IndexListActiveEnvoyStatesRequest,
	v1::IndexListActiveEnvoyStatesRequest,
	v2::IndexListActiveEnvoyStatesRequest,
	up::convert_index_list_active_envoy_states_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_active_envoy_states_request_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	IndexListActiveEnvoyStatesResponse,
	v1::IndexListActiveEnvoyStatesResponse,
	v2::IndexListActiveEnvoyStatesResponse,
	up::convert_index_list_active_envoy_states_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_active_envoy_states_response_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	ActiveEnvoyIndexEntry,
	v1::ActiveEnvoyIndexEntry,
	v2::ActiveEnvoyIndexEntry,
	up::convert_active_envoy_index_entry_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_active_envoy_index_entry_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	ActiveEnvoyState,
	v1::ActiveEnvoyState,
	v2::ActiveEnvoyState,
	up::convert_active_envoy_state_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_active_envoy_state_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	NamespaceRecord,
	v1::NamespaceRecord,
	v2::NamespaceRecord,
	up::convert_namespace_record_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_record_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	NamespaceCreateRequest,
	v1::NamespaceCreateRequest,
	v2::NamespaceCreateRequest,
	up::convert_namespace_create_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_create_request_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	NamespaceCreateResponse,
	v1::NamespaceCreateResponse,
	v2::NamespaceCreateResponse,
	up::convert_namespace_create_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_create_response_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	NamespaceReadRequest,
	v1::NamespaceReadRequest,
	v2::NamespaceReadRequest,
	up::convert_namespace_read_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_read_request_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	NamespaceReadResponse,
	v1::NamespaceReadResponse,
	v2::NamespaceReadResponse,
	up::convert_namespace_read_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_read_response_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	NamespaceListRequest,
	v1::NamespaceListRequest,
	v2::NamespaceListRequest,
	up::convert_namespace_list_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_list_request_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	NamespaceListResponse,
	v1::NamespaceListResponse,
	v2::NamespaceListResponse,
	up::convert_namespace_list_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_list_response_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	NodePoolConfig,
	v1::NodePoolConfig,
	v2::NodePoolConfig,
	up::convert_node_pool_config_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_node_pool_config_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	PoolRequest,
	v1::PoolRequest,
	v2::PoolRequest,
	up::convert_pool_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_pool_request_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	PoolResponse,
	v1::PoolResponse,
	v2::PoolResponse,
	up::convert_pool_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_pool_response_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	PoolLease,
	v1::PoolLease,
	v2::PoolLease,
	up::convert_pool_lease_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_pool_lease_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	PoolListRequest,
	v1::PoolListRequest,
	v2::PoolListRequest,
	up::convert_pool_list_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_pool_list_request_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v1_v2!(
	PoolListResponse,
	v1::PoolListResponse,
	v2::PoolListResponse,
	up::convert_pool_list_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_pool_list_response_pegboard_actor_v2_to_pegboard_actor_v1
);

impl_versioned_v2_only!(MigrateFromV2Begin, v2::MigrateFromV2Begin);

impl_versioned_v2_only!(
	ActorMigrateFromV2BeginResponse,
	v2::ActorMigrateFromV2BeginResponse
);
