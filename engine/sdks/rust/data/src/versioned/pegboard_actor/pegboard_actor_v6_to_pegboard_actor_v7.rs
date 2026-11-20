//! Upgrades from version 6 to version 7 of the actor family. Version 7 records when a running
//! actor became connectable and when a sleeping actor went to sleep. A version 6 state did not
//! record either, so its phase upgrades with neither time, and the actor's next start or sleep
//! sets it. Types that are the same at both versions upgrade through the generated `From`.
use crate::generated::{pegboard_actor_v6 as v6, pegboard_actor_v7 as v7};

impl From<v6::Running> for v7::Running {
	fn from(value: v6::Running) -> Self {
		Self {
			allocation: value.allocation.into(),
			connectable_unix_ms: None,
		}
	}
}

impl From<v6::Sleeping> for v7::Sleeping {
	fn from(value: v6::Sleeping) -> Self {
		Self {
			wake_retries: value.wake_retries,
			sleep_unix_ms: None,
		}
	}
}

impl From<v6::ActorPhase> for v7::ActorPhase {
	fn from(value: v6::ActorPhase) -> Self {
		match value {
			v6::ActorPhase::Initializing(v) => Self::Initializing(v.into()),
			v6::ActorPhase::Starting(v) => Self::Starting(v.into()),
			v6::ActorPhase::Running(v) => Self::Running(v.into()),
			v6::ActorPhase::Stopping(v) => Self::Stopping(v.into()),
			v6::ActorPhase::Sleeping(v) => Self::Sleeping(v.into()),
			v6::ActorPhase::Destroying(v) => Self::Destroying(v.into()),
			v6::ActorPhase::Destroyed => Self::Destroyed,
			v6::ActorPhase::MigratingFromV2(v) => Self::MigratingFromV2(v.into()),
			v6::ActorPhase::DelegatedToV2 => Self::DelegatedToV2,
		}
	}
}

impl From<v6::ActorState> for v7::ActorState {
	fn from(value: v6::ActorState) -> Self {
		Self {
			phase: value.phase.into(),
			epoch: value.epoch,
			create_unix_ms: value.create_unix_ms,
			storage_origin: value.storage_origin.into(),
			command_index: value.command_index,
			index_bucket_count: value.index_bucket_count,
			pending_action: value.pending_action.into(),
			write_index: value.write_index.into(),
			start_waiters: value.start_waiters.into_iter().map(Into::into).collect(),
			fork_reset: value.fork_reset,
			database_origin: value.database_origin.into(),
			legacy_move: value.legacy_move.map(Into::into),
			legacy_namespace: value.legacy_namespace,
		}
	}
}

impl From<v6::ActorReadResponse> for v7::ActorReadResponse {
	fn from(value: v6::ActorReadResponse) -> Self {
		Self {
			data: value.data.map(Into::into),
			phase: value.phase.map(Into::into),
			epoch: value.epoch,
			database_origin: value.database_origin.map(Into::into),
			command_index: value.command_index,
			legacy_move: value.legacy_move.map(Into::into),
			legacy_write_index_phase: value.legacy_write_index_phase.map(Into::into),
		}
	}
}

impl From<v6::ActorMigrateFromV2BeginResponse> for v7::ActorMigrateFromV2BeginResponse {
	fn from(value: v6::ActorMigrateFromV2BeginResponse) -> Self {
		Self {
			phase: value.phase.into(),
			create_unix_ms: value.create_unix_ms,
			legacy_namespace: value.legacy_namespace,
		}
	}
}
