//! Downgrades from version 7 to version 6 of the actor family. Version 6 has no record of when an
//! actor became connectable or went to sleep, so a downgraded phase drops both times. They only
//! report an actor's lifecycle, and nothing decides on them, so a state written at version 6
//! during a rolling deploy reads back without them. Types that are the same at both versions
//! downgrade through the generated `From`.
use crate::{
	generated::{pegboard_actor_v6 as v6, pegboard_actor_v7 as v7},
	versioned::{Downgrade, depot_fs_v2::downgrade_into},
};
use anyhow::Result;

fn downgrade_phase(phase: v7::ActorPhase) -> v6::ActorPhase {
	match phase {
		v7::ActorPhase::Initializing(v) => v6::ActorPhase::Initializing(v.into()),
		v7::ActorPhase::Starting(v) => v6::ActorPhase::Starting(v.into()),
		v7::ActorPhase::Running(v) => v6::ActorPhase::Running(v6::Running {
			allocation: v.allocation.into(),
		}),
		v7::ActorPhase::Stopping(v) => v6::ActorPhase::Stopping(v.into()),
		v7::ActorPhase::Sleeping(v) => v6::ActorPhase::Sleeping(v6::Sleeping {
			wake_retries: v.wake_retries,
		}),
		v7::ActorPhase::Destroying(v) => v6::ActorPhase::Destroying(v.into()),
		v7::ActorPhase::Destroyed => v6::ActorPhase::Destroyed,
		v7::ActorPhase::MigratingFromV2(v) => v6::ActorPhase::MigratingFromV2(v.into()),
		v7::ActorPhase::DelegatedToV2 => v6::ActorPhase::DelegatedToV2,
	}
}

impl Downgrade for v7::ActorState {
	type Older = v6::ActorState;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(v6::ActorState {
			phase: downgrade_phase(self.phase),
			epoch: self.epoch,
			create_unix_ms: self.create_unix_ms,
			storage_origin: self.storage_origin.into(),
			command_index: self.command_index,
			index_bucket_count: self.index_bucket_count,
			pending_action: self.pending_action.into(),
			write_index: self.write_index.into(),
			start_waiters: self.start_waiters.into_iter().map(Into::into).collect(),
			fork_reset: self.fork_reset,
			database_origin: self.database_origin.into(),
			legacy_move: self.legacy_move.map(Into::into),
			legacy_namespace: self.legacy_namespace,
		})
	}
}

impl Downgrade for v7::ActorReadResponse {
	type Older = v6::ActorReadResponse;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(v6::ActorReadResponse {
			data: self.data.map(Into::into),
			phase: self.phase.map(downgrade_phase),
			epoch: self.epoch,
			database_origin: self.database_origin.map(Into::into),
			command_index: self.command_index,
			legacy_move: self.legacy_move.map(Into::into),
			legacy_write_index_phase: self.legacy_write_index_phase.map(Into::into),
		})
	}
}

impl Downgrade for v7::ActorMigrateFromV2BeginResponse {
	type Older = v6::ActorMigrateFromV2BeginResponse;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(v6::ActorMigrateFromV2BeginResponse {
			phase: downgrade_phase(self.phase),
			create_unix_ms: self.create_unix_ms,
			legacy_namespace: self.legacy_namespace,
		})
	}
}

// Everything else is the same at both versions and downgrades through the generated `From`.
downgrade_into! {
	v7::ActorData => v6::ActorData,
	v7::ActorTimer => v6::ActorTimer,
	v7::ActorCreateRequest => v6::ActorCreateRequest,
	v7::ActorCreateResponse => v6::ActorCreateResponse,
	v7::ActorStartNotice => v6::ActorStartNotice,
	v7::ActorEvent => v6::ActorEvent,
	v7::ActorEventResponse => v6::ActorEventResponse,
	v7::IndexInsertActorRequest => v6::IndexInsertActorRequest,
	v7::IndexDeleteActorRequest => v6::IndexDeleteActorRequest,
	v7::IndexListActorsRequest => v6::IndexListActorsRequest,
	v7::IndexListActorsResponse => v6::IndexListActorsResponse,
	v7::DestroyedActorIndexEntry => v6::DestroyedActorIndexEntry,
	v7::IndexReadDestroyedActorRequest => v6::IndexReadDestroyedActorRequest,
	v7::IndexReadDestroyedActorResponse => v6::IndexReadDestroyedActorResponse,
	v7::IndexListEnvoysRequest => v6::IndexListEnvoysRequest,
	v7::IndexListEnvoysResponse => v6::IndexListEnvoysResponse,
	v7::EpoxyActorByKeyEntry => v6::EpoxyActorByKeyEntry,
	v7::KvValue => v6::KvValue,
	v7::EnvoyData => v6::EnvoyData,
	v7::EnvoyUpsertRequest => v6::EnvoyUpsertRequest,
	v7::EnvoyUpsertResponse => v6::EnvoyUpsertResponse,
	v7::EnvoyReadResponse => v6::EnvoyReadResponse,
	v7::IndexInsertEnvoyRequest => v6::IndexInsertEnvoyRequest,
	v7::IndexUpsertActiveEnvoyRequest => v6::IndexUpsertActiveEnvoyRequest,
	v7::IndexUpsertActiveEnvoyResponse => v6::IndexUpsertActiveEnvoyResponse,
	v7::IndexDeleteActiveEnvoyRequest => v6::IndexDeleteActiveEnvoyRequest,
	v7::IndexPutEnvoyPoolRequest => v6::IndexPutEnvoyPoolRequest,
	v7::IndexListEnvoyPoolsResponse => v6::IndexListEnvoyPoolsResponse,
	v7::IndexListActiveEnvoysRequest => v6::IndexListActiveEnvoysRequest,
	v7::IndexListActiveEnvoysResponse => v6::IndexListActiveEnvoysResponse,
	v7::IndexListActiveEnvoyStatesRequest => v6::IndexListActiveEnvoyStatesRequest,
	v7::IndexListActiveEnvoyStatesResponse => v6::IndexListActiveEnvoyStatesResponse,
	v7::ActiveEnvoyIndexEntry => v6::ActiveEnvoyIndexEntry,
	v7::ActiveEnvoyState => v6::ActiveEnvoyState,
	v7::NamespaceRecord => v6::NamespaceRecord,
	v7::NamespaceCreateRequest => v6::NamespaceCreateRequest,
	v7::NamespaceCreateResponse => v6::NamespaceCreateResponse,
	v7::NamespaceReadRequest => v6::NamespaceReadRequest,
	v7::NamespaceReadResponse => v6::NamespaceReadResponse,
	v7::NamespaceListRequest => v6::NamespaceListRequest,
	v7::NamespaceListResponse => v6::NamespaceListResponse,
	v7::NodePoolConfig => v6::NodePoolConfig,
	v7::PoolRequest => v6::PoolRequest,
	v7::PoolResponse => v6::PoolResponse,
	v7::PoolLease => v6::PoolLease,
	v7::PoolListRequest => v6::PoolListRequest,
	v7::PoolListResponse => v6::PoolListResponse,
	v7::MigrateFromV2Begin => v6::MigrateFromV2Begin,
	v7::LegacyActorAlarmReadResponse => v6::LegacyActorAlarmReadResponse,
}
