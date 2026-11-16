//! Upgrades from version 5 to version 6 of the actor family. Version 6 gives an actor's state the
//! fork it was last reset for, and drops the namespace from an actor's data, its create and its
//! effects, since a fork carries them into another namespace. A version 5 actor reads as one whose
//! legacy namespace is its own, and a version 5 state as one never reset. Version 6 also adds
//! individual actor forks: a version 5 actor never forked, and its database began empty unless its
//! storage origin says it was imported. Version 6 adds the backfill's moves, which a version 5
//! actor never took part in. Types that are the same at both versions upgrade through
//! the generated `From`.
use crate::generated::{pegboard_actor_v5 as v5, pegboard_actor_v6 as v6};

impl From<v5::ActorData> for v6::ActorData {
	fn from(value: v5::ActorData) -> Self {
		Self {
			actor_id: value.actor_id,
			pool_name: value.pool_name,
			name: value.name,
			key: value.key,
			input: value.input,
			create_unix_ms: value.create_unix_ms,
			legacy_namespace: Some(value.namespace),
			forked_from: None,
			legacy_moved_from: None,
		}
	}
}

impl From<v5::ActorCreateData> for v6::ActorCreateData {
	fn from(value: v5::ActorCreateData) -> Self {
		Self {
			actor_id: value.actor_id,
			pool_name: value.pool_name,
			name: value.name,
			key: value.key,
			input: value.input,
			legacy_namespace: Some(value.namespace),
			fork_from: None,
			legacy_move: None,
		}
	}
}

impl From<v5::ReserveEpoxyEffectData> for v6::ReserveEpoxyEffectData {
	fn from(value: v5::ReserveEpoxyEffectData) -> Self {
		Self {
			name: value.name,
			key: value.key,
			state: value.state.into(),
		}
	}
}

impl From<v5::WriteIndexEffectData> for v6::WriteIndexEffectData {
	fn from(value: v5::WriteIndexEffectData) -> Self {
		Self {
			name: value.name,
			create_unix_ms: value.create_unix_ms,
			state: value.state.into(),
		}
	}
}

impl From<v5::Initializing> for v6::Initializing {
	fn from(value: v5::Initializing) -> Self {
		Self {
			reserve_epoxy: value.reserve_epoxy.into(),
			allocation: value.allocation.map(Into::into),
			fork_database: None,
			legacy_move: None,
		}
	}
}

impl From<v5::MigratingFromV2> for v6::MigratingFromV2 {
	fn from(value: v5::MigratingFromV2) -> Self {
		Self {
			migration_id: value.migration_id,
			reserve_epoxy: value.reserve_epoxy.map(Into::into),
			alarm_due: value.alarm_due,
		}
	}
}

impl From<v5::ActorPhase> for v6::ActorPhase {
	fn from(value: v5::ActorPhase) -> Self {
		match value {
			v5::ActorPhase::Initializing(v) => Self::Initializing(v.into()),
			v5::ActorPhase::Starting(v) => Self::Starting(v.into()),
			v5::ActorPhase::Running(v) => Self::Running(v.into()),
			v5::ActorPhase::Stopping(v) => Self::Stopping(v.into()),
			v5::ActorPhase::Sleeping(v) => Self::Sleeping(v.into()),
			v5::ActorPhase::Destroying(v) => Self::Destroying(v.into()),
			v5::ActorPhase::Destroyed => Self::Destroyed,
			v5::ActorPhase::MigratingFromV2(v) => Self::MigratingFromV2(v.into()),
			v5::ActorPhase::DelegatedToV2 => Self::DelegatedToV2,
		}
	}
}

impl From<v5::ActorMigrateFromV2BeginResponse> for v6::ActorMigrateFromV2BeginResponse {
	/// A version 5 begin response carries no legacy namespace; its readers only need the phase.
	fn from(value: v5::ActorMigrateFromV2BeginResponse) -> Self {
		Self {
			phase: value.phase.into(),
			create_unix_ms: value.create_unix_ms,
			legacy_namespace: None,
		}
	}
}

impl From<v5::ActorState> for v6::ActorState {
	/// Version 5 states belong to ungrouped actors, which name their namespace in the index
	/// effect; version 6 keeps it as the state's legacy namespace.
	fn from(value: v5::ActorState) -> Self {
		let legacy_namespace = Some(value.write_index.namespace.clone());
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
			fork_reset: None,
			database_origin: v6::ActorDatabaseOrigin::DatabaseEmpty,
			legacy_move: None,
			legacy_namespace,
		}
	}
}

impl From<v5::ActorCreateRequest> for v6::ActorCreateRequest {
	fn from(value: v5::ActorCreateRequest) -> Self {
		Self {
			init: value.init.into(),
			waiter: value.waiter.map(Into::into),
			force_actor_v3: value.force_actor_v3,
		}
	}
}

impl From<v5::Create> for v6::Create {
	fn from(value: v5::Create) -> Self {
		Self {
			init: value.init.into(),
			waiter: value.waiter.map(Into::into),
			force_actor_v3: value.force_actor_v3,
		}
	}
}

impl From<v5::MigrateFromV2Begin> for v6::MigrateFromV2Begin {
	fn from(value: v5::MigrateFromV2Begin) -> Self {
		Self {
			migration_id: value.migration_id,
			init: value.init.into(),
			create_unix_ms: value.create_unix_ms,
		}
	}
}

impl From<v5::MigrateFromV2> for v6::MigrateFromV2 {
	fn from(value: v5::MigrateFromV2) -> Self {
		match value {
			v5::MigrateFromV2::MigrateFromV2Begin(v) => Self::MigrateFromV2Begin(v.into()),
			v5::MigrateFromV2::MigrateFromV2Activate(v) => Self::MigrateFromV2Activate(v.into()),
			v5::MigrateFromV2::MigrateFromV2Discard(v) => Self::MigrateFromV2Discard(v.into()),
			v5::MigrateFromV2::MigrateFromV2Recall(v) => Self::MigrateFromV2Recall(v.into()),
		}
	}
}

impl From<v5::ActorEvent> for v6::ActorEvent {
	fn from(value: v5::ActorEvent) -> Self {
		match value {
			v5::ActorEvent::ReserveEpoxyState(v) => Self::ReserveEpoxyState(v.into()),
			v5::ActorEvent::WriteIndexState(v) => Self::WriteIndexState(v.into()),
			v5::ActorEvent::StartFailed(v) => Self::StartFailed(v.into()),
			v5::ActorEvent::Sleep => Self::Sleep,
			v5::ActorEvent::TryStart(v) => Self::TryStart(v.into()),
			v5::ActorEvent::Create(v) => Self::Create(v.into()),
			v5::ActorEvent::Reallocate(v) => Self::Reallocate(v.into()),
			v5::ActorEvent::UpdateRoute(v) => Self::UpdateRoute(v.into()),
			v5::ActorEvent::Destroy => Self::Destroy,
			v5::ActorEvent::DestroyState(v) => Self::DestroyState(v.into()),
			v5::ActorEvent::ToEnvoyEvent(v) => Self::ToEnvoyEvent(v.into()),
			v5::ActorEvent::EnvoyGone(v) => Self::EnvoyGone(v.into()),
			v5::ActorEvent::MigrateFromV2(v) => Self::MigrateFromV2(v.into()),
		}
	}
}

impl From<v5::ActorTimer> for v6::ActorTimer {
	fn from(value: v5::ActorTimer) -> Self {
		match value {
			v5::ActorTimer::ReserveEpoxyTimeout(v) => Self::ReserveEpoxyTimeout(v.into()),
			v5::ActorTimer::WriteIndexTimeout(v) => Self::WriteIndexTimeout(v.into()),
			v5::ActorTimer::DestroyTimeout(v) => Self::DestroyTimeout(v.into()),
			v5::ActorTimer::StartTimeout(v) => Self::StartTimeout(v.into()),
			v5::ActorTimer::AlarmTimeout => Self::AlarmTimeout,
			v5::ActorTimer::StopTimeout(v) => Self::StopTimeout(v.into()),
			v5::ActorTimer::EnvoyCheckTimeout(v) => Self::EnvoyCheckTimeout(v.into()),
			v5::ActorTimer::StopThresholdTimeout(v) => Self::StopThresholdTimeout(v.into()),
			v5::ActorTimer::AllocationRetry(v) => Self::AllocationRetry(v.into()),
		}
	}
}

impl From<v5::ActorReadResponse> for v6::ActorReadResponse {
	fn from(value: v5::ActorReadResponse) -> Self {
		Self {
			data: value.data.map(Into::into),
			phase: value.phase.map(Into::into),
			epoch: value.epoch,
			database_origin: None,
			command_index: None,
			legacy_move: None,
			legacy_write_index_phase: None,
		}
	}
}

/// Upgrades a wake's outcome from version 5, which has every outcome but the backfill's.
fn upgrade_try_start_outcome(outcome: v5::TryStartOutcome) -> v6::TryStartOutcome {
	match outcome {
		v5::TryStartOutcome::TryStartWaiting => v6::TryStartOutcome::TryStartWaiting,
		v5::TryStartOutcome::TryStartStarting(v) => v6::TryStartOutcome::TryStartStarting(v.into()),
		v5::TryStartOutcome::TryStartRunning(v) => v6::TryStartOutcome::TryStartRunning(v.into()),
		v5::TryStartOutcome::TryStartDestroyed => v6::TryStartOutcome::TryStartDestroyed,
		v5::TryStartOutcome::TryStartEpoxyAlreadyReserved(v) => {
			v6::TryStartOutcome::TryStartEpoxyAlreadyReserved(v.into())
		}
		v5::TryStartOutcome::TryStartFull => v6::TryStartOutcome::TryStartFull,
		v5::TryStartOutcome::TryStartMigratingFromV2 => {
			v6::TryStartOutcome::TryStartMigratingFromV2
		}
		v5::TryStartOutcome::TryStartDelegatedToV2 => v6::TryStartOutcome::TryStartDelegatedToV2,
	}
}

impl From<v5::ActorCreateResponse> for v6::ActorCreateResponse {
	fn from(value: v5::ActorCreateResponse) -> Self {
		Self {
			outcome: upgrade_try_start_outcome(value.outcome),
		}
	}
}

impl From<v5::ActorEventResponse> for v6::ActorEventResponse {
	fn from(value: v5::ActorEventResponse) -> Self {
		Self {
			outcome: value.outcome.map(upgrade_try_start_outcome),
		}
	}
}

impl From<v5::ActorStartNoticeKind> for v6::ActorStartNoticeKind {
	fn from(value: v5::ActorStartNoticeKind) -> Self {
		match value {
			v5::ActorStartNoticeKind::ActorStartNoticeStarting(starting) => {
				Self::ActorStartNoticeStarting(starting.into())
			}
			v5::ActorStartNoticeKind::ActorStartNoticeRunning(running) => {
				Self::ActorStartNoticeRunning(running.into())
			}
			v5::ActorStartNoticeKind::ActorStartNoticeEpoxyAlreadyReserved(reserved) => {
				Self::ActorStartNoticeEpoxyAlreadyReserved(reserved.into())
			}
			v5::ActorStartNoticeKind::ActorStartNoticeDestroyed => Self::ActorStartNoticeDestroyed,
			v5::ActorStartNoticeKind::ActorStartNoticeMigratingFromV2 => {
				Self::ActorStartNoticeMigratingFromV2
			}
			v5::ActorStartNoticeKind::ActorStartNoticeDelegatedToV2 => {
				Self::ActorStartNoticeDelegatedToV2
			}
		}
	}
}

impl From<v5::ActorStartNotice> for v6::ActorStartNotice {
	fn from(value: v5::ActorStartNotice) -> Self {
		Self {
			actor_id: value.actor_id,
			kind: value.kind.into(),
		}
	}
}
