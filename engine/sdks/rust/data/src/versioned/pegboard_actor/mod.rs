//! Version envelopes for the actor state machine's state, timers, events and commands, and the
//! index, envoy, namespace and pool records stored beside them.

mod pegboard_actor_v1_to_pegboard_actor_v2;
mod pegboard_actor_v2_to_pegboard_actor_v1;
mod pegboard_actor_v2_to_pegboard_actor_v3;
mod pegboard_actor_v3_to_pegboard_actor_v2;

use super::unchanged;
use crate::generated::{pegboard_actor_v1 as v1, pegboard_actor_v2 as v2, pegboard_actor_v3 as v3};
use pegboard_actor_v1_to_pegboard_actor_v2 as up;
use pegboard_actor_v2_to_pegboard_actor_v1 as down;
use pegboard_actor_v2_to_pegboard_actor_v3 as up23;
use pegboard_actor_v3_to_pegboard_actor_v2 as down32;

/// The envoy version read back from records written at a version that does not carry one.
/// Clients default to version 1 when none is configured, so such an envoy ranks with those.
pub const ENVOY_VERSION_BEFORE_VERSIONS: u32 = 1;

/// The first `pegboard.actor` version whose envoy records and active index state carry the
/// envoy's version.
pub const ENVOY_VERSION_ACTOR_VERSION: u16 = 2;

impl_versioned_v1_v2_v3!(
	ActorData,
	PEGBOARD_ACTOR,
	v1::ActorData,
	v2::ActorData,
	v3::ActorData,
	up::convert_actor_data_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_data_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_actor_data_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_data_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	ActorState,
	PEGBOARD_ACTOR,
	v1::ActorState,
	v2::ActorState,
	v3::ActorState,
	up::convert_actor_state_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_state_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_actor_state_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_state_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	ActorTimer,
	PEGBOARD_ACTOR,
	v1::ActorTimer,
	v2::ActorTimer,
	v3::ActorTimer,
	up::convert_actor_timer_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_timer_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_actor_timer_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_timer_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	ActorCreateRequest,
	PEGBOARD_ACTOR,
	v1::ActorCreateRequest,
	v2::ActorCreateRequest,
	v3::ActorCreateRequest,
	up::convert_actor_create_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_create_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_actor_create_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_create_request_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	ActorCreateResponse,
	PEGBOARD_ACTOR,
	v1::ActorCreateResponse,
	v2::ActorCreateResponse,
	v3::ActorCreateResponse,
	up::convert_actor_create_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_create_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_actor_create_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_create_response_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	ActorReadRequest,
	PEGBOARD_ACTOR,
	v1::ActorReadRequest,
	v2::ActorReadRequest,
	v3::ActorReadRequest,
	unchanged,
	unchanged,
	unchanged,
	unchanged
);

impl_versioned_v1_v2_v3!(
	ActorReadResponse,
	PEGBOARD_ACTOR,
	v1::ActorReadResponse,
	v2::ActorReadResponse,
	v3::ActorReadResponse,
	up::convert_actor_read_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_read_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_actor_read_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_read_response_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	ActorStartNotice,
	PEGBOARD_ACTOR,
	v1::ActorStartNotice,
	v2::ActorStartNotice,
	v3::ActorStartNotice,
	up::convert_actor_start_notice_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_start_notice_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_actor_start_notice_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_start_notice_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	ActorEvent,
	PEGBOARD_ACTOR,
	v1::ActorEvent,
	v2::ActorEvent,
	v3::ActorEvent,
	up::convert_actor_event_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_event_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_actor_event_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_event_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	ActorEventResponse,
	PEGBOARD_ACTOR,
	v1::ActorEventResponse,
	v2::ActorEventResponse,
	v3::ActorEventResponse,
	up::convert_actor_event_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_actor_event_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_actor_event_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_event_response_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	IndexInsertActorRequest,
	PEGBOARD_ACTOR,
	v1::IndexInsertActorRequest,
	v2::IndexInsertActorRequest,
	v3::IndexInsertActorRequest,
	up::convert_index_insert_actor_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_insert_actor_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_insert_actor_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_insert_actor_request_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	IndexInsertActorResponse,
	PEGBOARD_ACTOR,
	v1::IndexInsertActorResponse,
	v2::IndexInsertActorResponse,
	v3::IndexInsertActorResponse,
	unchanged,
	unchanged,
	unchanged,
	unchanged
);

impl_versioned_v1_v2_v3!(
	IndexDeleteActorRequest,
	PEGBOARD_ACTOR,
	v1::IndexDeleteActorRequest,
	v2::IndexDeleteActorRequest,
	v3::IndexDeleteActorRequest,
	up::convert_index_delete_actor_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_delete_actor_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_delete_actor_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_delete_actor_request_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	IndexDeleteActorResponse,
	PEGBOARD_ACTOR,
	v1::IndexDeleteActorResponse,
	v2::IndexDeleteActorResponse,
	v3::IndexDeleteActorResponse,
	unchanged,
	unchanged,
	unchanged,
	unchanged
);

impl_versioned_v1_v2_v3!(
	IndexListActorsRequest,
	PEGBOARD_ACTOR,
	v1::IndexListActorsRequest,
	v2::IndexListActorsRequest,
	v3::IndexListActorsRequest,
	up::convert_index_list_actors_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_actors_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_list_actors_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_list_actors_request_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	IndexListActorsResponse,
	PEGBOARD_ACTOR,
	v1::IndexListActorsResponse,
	v2::IndexListActorsResponse,
	v3::IndexListActorsResponse,
	up::convert_index_list_actors_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_actors_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_list_actors_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_list_actors_response_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	DestroyedActorIndexEntry,
	PEGBOARD_ACTOR,
	v1::DestroyedActorIndexEntry,
	v2::DestroyedActorIndexEntry,
	v3::DestroyedActorIndexEntry,
	up::convert_destroyed_actor_index_entry_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_destroyed_actor_index_entry_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_destroyed_actor_index_entry_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_destroyed_actor_index_entry_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	IndexReadDestroyedActorRequest,
	PEGBOARD_ACTOR,
	v1::IndexReadDestroyedActorRequest,
	v2::IndexReadDestroyedActorRequest,
	v3::IndexReadDestroyedActorRequest,
	up::convert_index_read_destroyed_actor_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_read_destroyed_actor_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_read_destroyed_actor_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_read_destroyed_actor_request_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	IndexReadDestroyedActorResponse,
	PEGBOARD_ACTOR,
	v1::IndexReadDestroyedActorResponse,
	v2::IndexReadDestroyedActorResponse,
	v3::IndexReadDestroyedActorResponse,
	up::convert_index_read_destroyed_actor_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_read_destroyed_actor_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_read_destroyed_actor_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_read_destroyed_actor_response_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	IndexListEnvoysRequest,
	PEGBOARD_ACTOR,
	v1::IndexListEnvoysRequest,
	v2::IndexListEnvoysRequest,
	v3::IndexListEnvoysRequest,
	up::convert_index_list_envoys_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_envoys_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_list_envoys_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_list_envoys_request_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	IndexListEnvoysResponse,
	PEGBOARD_ACTOR,
	v1::IndexListEnvoysResponse,
	v2::IndexListEnvoysResponse,
	v3::IndexListEnvoysResponse,
	up::convert_index_list_envoys_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_envoys_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_list_envoys_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_list_envoys_response_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	EpoxyActorByKeyEntry,
	PEGBOARD_ACTOR,
	v1::EpoxyActorByKeyEntry,
	v2::EpoxyActorByKeyEntry,
	v3::EpoxyActorByKeyEntry,
	up::convert_epoxy_actor_by_key_entry_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_epoxy_actor_by_key_entry_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_epoxy_actor_by_key_entry_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_epoxy_actor_by_key_entry_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	KvValue,
	PEGBOARD_ACTOR,
	v1::KvValue,
	v2::KvValue,
	v3::KvValue,
	up::convert_kv_value_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_kv_value_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_kv_value_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_kv_value_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	EnvoyData,
	PEGBOARD_ACTOR,
	v1::EnvoyData,
	v2::EnvoyData,
	v3::EnvoyData,
	up::convert_envoy_data_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_envoy_data_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_envoy_data_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_envoy_data_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	EnvoyUpsertRequest,
	PEGBOARD_ACTOR,
	v1::EnvoyUpsertRequest,
	v2::EnvoyUpsertRequest,
	v3::EnvoyUpsertRequest,
	up::convert_envoy_upsert_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_envoy_upsert_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_envoy_upsert_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_envoy_upsert_request_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	EnvoyUpsertResponse,
	PEGBOARD_ACTOR,
	v1::EnvoyUpsertResponse,
	v2::EnvoyUpsertResponse,
	v3::EnvoyUpsertResponse,
	up::convert_envoy_upsert_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_envoy_upsert_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_envoy_upsert_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_envoy_upsert_response_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	EnvoyReadRequest,
	PEGBOARD_ACTOR,
	v1::EnvoyReadRequest,
	v2::EnvoyReadRequest,
	v3::EnvoyReadRequest,
	unchanged,
	unchanged,
	unchanged,
	unchanged
);

impl_versioned_v1_v2_v3!(
	EnvoyReadResponse,
	PEGBOARD_ACTOR,
	v1::EnvoyReadResponse,
	v2::EnvoyReadResponse,
	v3::EnvoyReadResponse,
	up::convert_envoy_read_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_envoy_read_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_envoy_read_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_envoy_read_response_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	IndexInsertEnvoyRequest,
	PEGBOARD_ACTOR,
	v1::IndexInsertEnvoyRequest,
	v2::IndexInsertEnvoyRequest,
	v3::IndexInsertEnvoyRequest,
	up::convert_index_insert_envoy_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_insert_envoy_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_insert_envoy_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_insert_envoy_request_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	IndexInsertEnvoyResponse,
	PEGBOARD_ACTOR,
	v1::IndexInsertEnvoyResponse,
	v2::IndexInsertEnvoyResponse,
	v3::IndexInsertEnvoyResponse,
	unchanged,
	unchanged,
	unchanged,
	unchanged
);

impl_versioned_v1_v2_v3!(
	IndexUpsertActiveEnvoyRequest,
	PEGBOARD_ACTOR,
	v1::IndexUpsertActiveEnvoyRequest,
	v2::IndexUpsertActiveEnvoyRequest,
	v3::IndexUpsertActiveEnvoyRequest,
	up::convert_index_upsert_active_envoy_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_upsert_active_envoy_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_upsert_active_envoy_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_upsert_active_envoy_request_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	IndexUpsertActiveEnvoyResponse,
	PEGBOARD_ACTOR,
	v1::IndexUpsertActiveEnvoyResponse,
	v2::IndexUpsertActiveEnvoyResponse,
	v3::IndexUpsertActiveEnvoyResponse,
	up::convert_index_upsert_active_envoy_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_upsert_active_envoy_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_upsert_active_envoy_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_upsert_active_envoy_response_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	IndexDeleteActiveEnvoyRequest,
	PEGBOARD_ACTOR,
	v1::IndexDeleteActiveEnvoyRequest,
	v2::IndexDeleteActiveEnvoyRequest,
	v3::IndexDeleteActiveEnvoyRequest,
	up::convert_index_delete_active_envoy_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_delete_active_envoy_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_delete_active_envoy_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_delete_active_envoy_request_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	IndexDeleteActiveEnvoyResponse,
	PEGBOARD_ACTOR,
	v1::IndexDeleteActiveEnvoyResponse,
	v2::IndexDeleteActiveEnvoyResponse,
	v3::IndexDeleteActiveEnvoyResponse,
	unchanged,
	unchanged,
	unchanged,
	unchanged
);

impl_versioned_v1_v2_v3!(
	IndexPutEnvoyPoolRequest,
	PEGBOARD_ACTOR,
	v1::IndexPutEnvoyPoolRequest,
	v2::IndexPutEnvoyPoolRequest,
	v3::IndexPutEnvoyPoolRequest,
	up::convert_index_put_envoy_pool_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_put_envoy_pool_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_put_envoy_pool_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_put_envoy_pool_request_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	IndexPutEnvoyPoolResponse,
	PEGBOARD_ACTOR,
	v1::IndexPutEnvoyPoolResponse,
	v2::IndexPutEnvoyPoolResponse,
	v3::IndexPutEnvoyPoolResponse,
	unchanged,
	unchanged,
	unchanged,
	unchanged
);

impl_versioned_v1_v2_v3!(
	IndexListEnvoyPoolsRequest,
	PEGBOARD_ACTOR,
	v1::IndexListEnvoyPoolsRequest,
	v2::IndexListEnvoyPoolsRequest,
	v3::IndexListEnvoyPoolsRequest,
	unchanged,
	unchanged,
	unchanged,
	unchanged
);

impl_versioned_v1_v2_v3!(
	IndexListEnvoyPoolsResponse,
	PEGBOARD_ACTOR,
	v1::IndexListEnvoyPoolsResponse,
	v2::IndexListEnvoyPoolsResponse,
	v3::IndexListEnvoyPoolsResponse,
	up::convert_index_list_envoy_pools_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_envoy_pools_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_list_envoy_pools_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_list_envoy_pools_response_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	IndexListActiveEnvoysRequest,
	PEGBOARD_ACTOR,
	v1::IndexListActiveEnvoysRequest,
	v2::IndexListActiveEnvoysRequest,
	v3::IndexListActiveEnvoysRequest,
	up::convert_index_list_active_envoys_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_active_envoys_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_list_active_envoys_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_list_active_envoys_request_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	IndexListActiveEnvoysResponse,
	PEGBOARD_ACTOR,
	v1::IndexListActiveEnvoysResponse,
	v2::IndexListActiveEnvoysResponse,
	v3::IndexListActiveEnvoysResponse,
	up::convert_index_list_active_envoys_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_active_envoys_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_list_active_envoys_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_list_active_envoys_response_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	IndexListActiveEnvoyStatesRequest,
	PEGBOARD_ACTOR,
	v1::IndexListActiveEnvoyStatesRequest,
	v2::IndexListActiveEnvoyStatesRequest,
	v3::IndexListActiveEnvoyStatesRequest,
	up::convert_index_list_active_envoy_states_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_active_envoy_states_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_list_active_envoy_states_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_list_active_envoy_states_request_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	IndexListActiveEnvoyStatesResponse,
	PEGBOARD_ACTOR,
	v1::IndexListActiveEnvoyStatesResponse,
	v2::IndexListActiveEnvoyStatesResponse,
	v3::IndexListActiveEnvoyStatesResponse,
	up::convert_index_list_active_envoy_states_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_index_list_active_envoy_states_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_index_list_active_envoy_states_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_index_list_active_envoy_states_response_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	ActiveEnvoyIndexEntry,
	PEGBOARD_ACTOR,
	v1::ActiveEnvoyIndexEntry,
	v2::ActiveEnvoyIndexEntry,
	v3::ActiveEnvoyIndexEntry,
	up::convert_active_envoy_index_entry_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_active_envoy_index_entry_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_active_envoy_index_entry_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_active_envoy_index_entry_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	ActiveEnvoyState,
	PEGBOARD_ACTOR,
	v1::ActiveEnvoyState,
	v2::ActiveEnvoyState,
	v3::ActiveEnvoyState,
	up::convert_active_envoy_state_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_active_envoy_state_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_active_envoy_state_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_active_envoy_state_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	NamespaceRecord,
	PEGBOARD_ACTOR,
	v1::NamespaceRecord,
	v2::NamespaceRecord,
	v3::NamespaceRecord,
	up::convert_namespace_record_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_record_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_namespace_record_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_namespace_record_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	NamespaceCreateRequest,
	PEGBOARD_ACTOR,
	v1::NamespaceCreateRequest,
	v2::NamespaceCreateRequest,
	v3::NamespaceCreateRequest,
	up::convert_namespace_create_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_create_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_namespace_create_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_namespace_create_request_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	NamespaceCreateResponse,
	PEGBOARD_ACTOR,
	v1::NamespaceCreateResponse,
	v2::NamespaceCreateResponse,
	v3::NamespaceCreateResponse,
	up::convert_namespace_create_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_create_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_namespace_create_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_namespace_create_response_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	NamespaceReadRequest,
	PEGBOARD_ACTOR,
	v1::NamespaceReadRequest,
	v2::NamespaceReadRequest,
	v3::NamespaceReadRequest,
	up::convert_namespace_read_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_read_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_namespace_read_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_namespace_read_request_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	NamespaceReadResponse,
	PEGBOARD_ACTOR,
	v1::NamespaceReadResponse,
	v2::NamespaceReadResponse,
	v3::NamespaceReadResponse,
	up::convert_namespace_read_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_read_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_namespace_read_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_namespace_read_response_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	NamespaceListRequest,
	PEGBOARD_ACTOR,
	v1::NamespaceListRequest,
	v2::NamespaceListRequest,
	v3::NamespaceListRequest,
	up::convert_namespace_list_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_list_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_namespace_list_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_namespace_list_request_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	NamespaceListResponse,
	PEGBOARD_ACTOR,
	v1::NamespaceListResponse,
	v2::NamespaceListResponse,
	v3::NamespaceListResponse,
	up::convert_namespace_list_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_namespace_list_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_namespace_list_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_namespace_list_response_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	NodePoolConfig,
	PEGBOARD_ACTOR,
	v1::NodePoolConfig,
	v2::NodePoolConfig,
	v3::NodePoolConfig,
	up::convert_node_pool_config_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_node_pool_config_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_node_pool_config_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_node_pool_config_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	PoolRequest,
	PEGBOARD_ACTOR,
	v1::PoolRequest,
	v2::PoolRequest,
	v3::PoolRequest,
	up::convert_pool_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_pool_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_pool_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_pool_request_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	PoolResponse,
	PEGBOARD_ACTOR,
	v1::PoolResponse,
	v2::PoolResponse,
	v3::PoolResponse,
	up::convert_pool_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_pool_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_pool_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_pool_response_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	PoolLease,
	PEGBOARD_ACTOR,
	v1::PoolLease,
	v2::PoolLease,
	v3::PoolLease,
	up::convert_pool_lease_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_pool_lease_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_pool_lease_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_pool_lease_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	PoolListRequest,
	PEGBOARD_ACTOR,
	v1::PoolListRequest,
	v2::PoolListRequest,
	v3::PoolListRequest,
	up::convert_pool_list_request_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_pool_list_request_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_pool_list_request_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_pool_list_request_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v1_v2_v3!(
	PoolListResponse,
	PEGBOARD_ACTOR,
	v1::PoolListResponse,
	v2::PoolListResponse,
	v3::PoolListResponse,
	up::convert_pool_list_response_pegboard_actor_v1_to_pegboard_actor_v2,
	down::convert_pool_list_response_pegboard_actor_v2_to_pegboard_actor_v1,
	up23::convert_pool_list_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_pool_list_response_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v2_v3!(
	MigrateFromV2Begin,
	PEGBOARD_ACTOR,
	v2::MigrateFromV2Begin,
	v3::MigrateFromV2Begin,
	up23::convert_migrate_from_v2_begin_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_migrate_from_v2_begin_pegboard_actor_v3_to_pegboard_actor_v2
);

impl_versioned_v2_v3!(
	ActorMigrateFromV2BeginResponse,
	PEGBOARD_ACTOR,
	v2::ActorMigrateFromV2BeginResponse,
	v3::ActorMigrateFromV2BeginResponse,
	up23::convert_actor_migrate_from_v2_begin_response_pegboard_actor_v2_to_pegboard_actor_v3,
	down32::convert_actor_migrate_from_v2_begin_response_pegboard_actor_v3_to_pegboard_actor_v2
);
