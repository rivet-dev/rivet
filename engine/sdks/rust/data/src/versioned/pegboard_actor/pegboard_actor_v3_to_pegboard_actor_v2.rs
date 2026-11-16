//! Downgrades every actor record to v2. Only the actor timer union changed, and its timers that the
//! engine arms at v3 or later have no v2 form.
// from: pegboard.actor.v3.bare, to: pegboard.actor.v2.bare

use anyhow::Result;

use crate::generated::{pegboard_actor_v2, pegboard_actor_v3};
use crate::versioned::Inexpressible;

pub fn convert_actor_data_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ActorData,
) -> Result<pegboard_actor_v2::ActorData> {
	Ok(pegboard_actor_v2::ActorData {
		actor_id: x.actor_id,
		namespace: x.namespace,
		pool_name: x.pool_name,
		name: x.name,
		key: x.key,
		input: x.input,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_actor_allocation_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ActorAllocation,
) -> Result<pegboard_actor_v2::ActorAllocation> {
	Ok(pegboard_actor_v2::ActorAllocation {
		envoy_key: x.envoy_key,
		worker_id: x.worker_id,
		connection_id: x.connection_id,
	})
}

pub fn convert_reserve_epoxy_phase_already_reserved_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ReserveEpoxyPhaseAlreadyReserved,
) -> Result<pegboard_actor_v2::ReserveEpoxyPhaseAlreadyReserved> {
	Ok(pegboard_actor_v2::ReserveEpoxyPhaseAlreadyReserved {
		reservation_id: x.reservation_id,
	})
}

pub fn convert_reserve_epoxy_phase_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ReserveEpoxyPhase,
) -> Result<pegboard_actor_v2::ReserveEpoxyPhase> {
	Ok(match x {
		pegboard_actor_v3::ReserveEpoxyPhase::ReserveEpoxyPhasePending => pegboard_actor_v2::ReserveEpoxyPhase::ReserveEpoxyPhasePending,
		pegboard_actor_v3::ReserveEpoxyPhase::ReserveEpoxyPhaseReserved => pegboard_actor_v2::ReserveEpoxyPhase::ReserveEpoxyPhaseReserved,
		pegboard_actor_v3::ReserveEpoxyPhase::ReserveEpoxyPhaseAlreadyReserved(v) => pegboard_actor_v2::ReserveEpoxyPhase::ReserveEpoxyPhaseAlreadyReserved(convert_reserve_epoxy_phase_already_reserved_pegboard_actor_v3_to_pegboard_actor_v2(v)?),
		pegboard_actor_v3::ReserveEpoxyPhase::ReserveEpoxyPhaseFailed => pegboard_actor_v2::ReserveEpoxyPhase::ReserveEpoxyPhaseFailed,
		pegboard_actor_v3::ReserveEpoxyPhase::ReserveEpoxyPhaseMaxAttemptsReached => pegboard_actor_v2::ReserveEpoxyPhase::ReserveEpoxyPhaseMaxAttemptsReached,
	})
}

pub fn convert_reserve_epoxy_state_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ReserveEpoxyState,
) -> Result<pegboard_actor_v2::ReserveEpoxyState> {
	Ok(pegboard_actor_v2::ReserveEpoxyState {
		phase: convert_reserve_epoxy_phase_pegboard_actor_v3_to_pegboard_actor_v2(x.phase)?,
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_reserve_epoxy_effect_data_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ReserveEpoxyEffectData,
) -> Result<pegboard_actor_v2::ReserveEpoxyEffectData> {
	Ok(pegboard_actor_v2::ReserveEpoxyEffectData {
		namespace: x.namespace,
		name: x.name,
		key: x.key,
		state: convert_reserve_epoxy_state_pegboard_actor_v3_to_pegboard_actor_v2(x.state)?,
	})
}

pub fn convert_pending_action_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::PendingAction,
) -> Result<pegboard_actor_v2::PendingAction> {
	Ok(match x {
		pegboard_actor_v3::PendingAction::PendingNone => {
			pegboard_actor_v2::PendingAction::PendingNone
		}
		pegboard_actor_v3::PendingAction::PendingSleep => {
			pegboard_actor_v2::PendingAction::PendingSleep
		}
		pegboard_actor_v3::PendingAction::PendingReallocate => {
			pegboard_actor_v2::PendingAction::PendingReallocate
		}
		pegboard_actor_v3::PendingAction::PendingDestroy => {
			pegboard_actor_v2::PendingAction::PendingDestroy
		}
	})
}

pub fn convert_actor_create_data_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ActorCreateData,
) -> Result<pegboard_actor_v2::ActorCreateData> {
	Ok(pegboard_actor_v2::ActorCreateData {
		actor_id: x.actor_id,
		namespace: x.namespace,
		pool_name: x.pool_name,
		name: x.name,
		key: x.key,
		input: x.input,
	})
}

pub fn convert_initializing_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::Initializing,
) -> Result<pegboard_actor_v2::Initializing> {
	Ok(pegboard_actor_v2::Initializing {
		reserve_epoxy: convert_reserve_epoxy_effect_data_pegboard_actor_v3_to_pegboard_actor_v2(
			x.reserve_epoxy,
		)?,
		allocation: x
			.allocation
			.map(|v| convert_actor_allocation_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.transpose()?,
	})
}

pub fn convert_starting_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::Starting,
) -> Result<pegboard_actor_v2::Starting> {
	Ok(pegboard_actor_v2::Starting {
		allocation: convert_actor_allocation_pegboard_actor_v3_to_pegboard_actor_v2(x.allocation)?,
	})
}

pub fn convert_running_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::Running,
) -> Result<pegboard_actor_v2::Running> {
	Ok(pegboard_actor_v2::Running {
		allocation: convert_actor_allocation_pegboard_actor_v3_to_pegboard_actor_v2(x.allocation)?,
	})
}

pub fn convert_sleeping_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::Sleeping,
) -> Result<pegboard_actor_v2::Sleeping> {
	Ok(pegboard_actor_v2::Sleeping {
		wake_retries: x.wake_retries,
	})
}

pub fn convert_stopping_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::Stopping,
) -> Result<pegboard_actor_v2::Stopping> {
	Ok(pegboard_actor_v2::Stopping {
		allocation: convert_actor_allocation_pegboard_actor_v3_to_pegboard_actor_v2(x.allocation)?,
	})
}

pub fn convert_destroy_phase_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::DestroyPhase,
) -> Result<pegboard_actor_v2::DestroyPhase> {
	Ok(match x {
		pegboard_actor_v3::DestroyPhase::DestroyPhasePending => {
			pegboard_actor_v2::DestroyPhase::DestroyPhasePending
		}
		pegboard_actor_v3::DestroyPhase::DestroyPhaseSuccess => {
			pegboard_actor_v2::DestroyPhase::DestroyPhaseSuccess
		}
		pegboard_actor_v3::DestroyPhase::DestroyPhaseFailed => {
			pegboard_actor_v2::DestroyPhase::DestroyPhaseFailed
		}
	})
}

pub fn convert_destroy_state_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::DestroyState,
) -> Result<pegboard_actor_v2::DestroyState> {
	Ok(pegboard_actor_v2::DestroyState {
		phase: convert_destroy_phase_pegboard_actor_v3_to_pegboard_actor_v2(x.phase)?,
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_destroy_effect_data_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::DestroyEffectData,
) -> Result<pegboard_actor_v2::DestroyEffectData> {
	Ok(pegboard_actor_v2::DestroyEffectData {
		create_unix_ms: x.create_unix_ms,
		state: convert_destroy_state_pegboard_actor_v3_to_pegboard_actor_v2(x.state)?,
	})
}

pub fn convert_destroying_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::Destroying,
) -> Result<pegboard_actor_v2::Destroying> {
	Ok(pegboard_actor_v2::Destroying {
		destroy: convert_destroy_effect_data_pegboard_actor_v3_to_pegboard_actor_v2(x.destroy)?,
		epoxy_already_reserved: x.epoxy_already_reserved,
	})
}

pub fn convert_migrating_from_v2_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::MigratingFromV2,
) -> Result<pegboard_actor_v2::MigratingFromV2> {
	Ok(pegboard_actor_v2::MigratingFromV2 {
		migration_id: x.migration_id,
		reserve_epoxy: x
			.reserve_epoxy
			.map(|v| convert_reserve_epoxy_effect_data_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.transpose()?,
		alarm_due: x.alarm_due,
	})
}

pub fn convert_actor_phase_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ActorPhase,
) -> Result<pegboard_actor_v2::ActorPhase> {
	Ok(match x {
		pegboard_actor_v3::ActorPhase::Initializing(v) => {
			pegboard_actor_v2::ActorPhase::Initializing(
				convert_initializing_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::ActorPhase::Starting(v) => pegboard_actor_v2::ActorPhase::Starting(
			convert_starting_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
		),
		pegboard_actor_v3::ActorPhase::Running(v) => pegboard_actor_v2::ActorPhase::Running(
			convert_running_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
		),
		pegboard_actor_v3::ActorPhase::Stopping(v) => pegboard_actor_v2::ActorPhase::Stopping(
			convert_stopping_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
		),
		pegboard_actor_v3::ActorPhase::Sleeping(v) => pegboard_actor_v2::ActorPhase::Sleeping(
			convert_sleeping_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
		),
		pegboard_actor_v3::ActorPhase::Destroying(v) => pegboard_actor_v2::ActorPhase::Destroying(
			convert_destroying_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
		),
		pegboard_actor_v3::ActorPhase::Destroyed => pegboard_actor_v2::ActorPhase::Destroyed,
		pegboard_actor_v3::ActorPhase::MigratingFromV2(v) => {
			pegboard_actor_v2::ActorPhase::MigratingFromV2(
				convert_migrating_from_v2_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
	})
}

pub fn convert_start_waiter_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::StartWaiter,
) -> Result<pegboard_actor_v2::StartWaiter> {
	Ok(pegboard_actor_v2::StartWaiter {
		worker_id: x.worker_id,
		wait_id: x.wait_id,
		expires_unix_ms: x.expires_unix_ms,
	})
}

pub fn convert_write_index_phase_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::WriteIndexPhase,
) -> Result<pegboard_actor_v2::WriteIndexPhase> {
	Ok(match x {
		pegboard_actor_v3::WriteIndexPhase::WriteIndexPhasePending => {
			pegboard_actor_v2::WriteIndexPhase::WriteIndexPhasePending
		}
		pegboard_actor_v3::WriteIndexPhase::WriteIndexPhaseSuccess => {
			pegboard_actor_v2::WriteIndexPhase::WriteIndexPhaseSuccess
		}
		pegboard_actor_v3::WriteIndexPhase::WriteIndexPhaseFailed => {
			pegboard_actor_v2::WriteIndexPhase::WriteIndexPhaseFailed
		}
	})
}

pub fn convert_write_index_state_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::WriteIndexState,
) -> Result<pegboard_actor_v2::WriteIndexState> {
	Ok(pegboard_actor_v2::WriteIndexState {
		phase: convert_write_index_phase_pegboard_actor_v3_to_pegboard_actor_v2(x.phase)?,
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_write_index_effect_data_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::WriteIndexEffectData,
) -> Result<pegboard_actor_v2::WriteIndexEffectData> {
	Ok(pegboard_actor_v2::WriteIndexEffectData {
		namespace: x.namespace,
		name: x.name,
		create_unix_ms: x.create_unix_ms,
		state: convert_write_index_state_pegboard_actor_v3_to_pegboard_actor_v2(x.state)?,
	})
}

pub fn convert_storage_origin_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::StorageOrigin,
) -> Result<pegboard_actor_v2::StorageOrigin> {
	Ok(match x {
		pegboard_actor_v3::StorageOrigin::Created => pegboard_actor_v2::StorageOrigin::Created,
		pegboard_actor_v3::StorageOrigin::Imported => pegboard_actor_v2::StorageOrigin::Imported,
	})
}

pub fn convert_actor_state_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ActorState,
) -> Result<pegboard_actor_v2::ActorState> {
	Ok(pegboard_actor_v2::ActorState {
		phase: convert_actor_phase_pegboard_actor_v3_to_pegboard_actor_v2(x.phase)?,
		epoch: x.epoch,
		create_unix_ms: x.create_unix_ms,
		storage_origin: convert_storage_origin_pegboard_actor_v3_to_pegboard_actor_v2(
			x.storage_origin,
		)?,
		command_index: x.command_index,
		index_bucket_count: x.index_bucket_count,
		pending_action: convert_pending_action_pegboard_actor_v3_to_pegboard_actor_v2(
			x.pending_action,
		)?,
		write_index: convert_write_index_effect_data_pegboard_actor_v3_to_pegboard_actor_v2(
			x.write_index,
		)?,
		start_waiters: x
			.start_waiters
			.into_iter()
			.map(|v| convert_start_waiter_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_reserve_epoxy_timeout_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ReserveEpoxyTimeout,
) -> Result<pegboard_actor_v2::ReserveEpoxyTimeout> {
	Ok(pegboard_actor_v2::ReserveEpoxyTimeout {
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_write_index_timeout_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::WriteIndexTimeout,
) -> Result<pegboard_actor_v2::WriteIndexTimeout> {
	Ok(pegboard_actor_v2::WriteIndexTimeout {
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_destroy_timeout_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::DestroyTimeout,
) -> Result<pegboard_actor_v2::DestroyTimeout> {
	Ok(pegboard_actor_v2::DestroyTimeout {
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_start_timeout_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::StartTimeout,
) -> Result<pegboard_actor_v2::StartTimeout> {
	Ok(pegboard_actor_v2::StartTimeout { epoch: x.epoch })
}

pub fn convert_stop_timeout_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::StopTimeout,
) -> Result<pegboard_actor_v2::StopTimeout> {
	Ok(pegboard_actor_v2::StopTimeout { epoch: x.epoch })
}

pub fn convert_envoy_check_timeout_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::EnvoyCheckTimeout,
) -> Result<pegboard_actor_v2::EnvoyCheckTimeout> {
	Ok(pegboard_actor_v2::EnvoyCheckTimeout { epoch: x.epoch })
}

pub fn convert_actor_timer_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ActorTimer,
) -> Result<pegboard_actor_v2::ActorTimer> {
	Ok(match x {
		pegboard_actor_v3::ActorTimer::ReserveEpoxyTimeout(v) => {
			pegboard_actor_v2::ActorTimer::ReserveEpoxyTimeout(
				convert_reserve_epoxy_timeout_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::ActorTimer::WriteIndexTimeout(v) => {
			pegboard_actor_v2::ActorTimer::WriteIndexTimeout(
				convert_write_index_timeout_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::ActorTimer::DestroyTimeout(v) => {
			pegboard_actor_v2::ActorTimer::DestroyTimeout(
				convert_destroy_timeout_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::ActorTimer::StartTimeout(v) => {
			pegboard_actor_v2::ActorTimer::StartTimeout(
				convert_start_timeout_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::ActorTimer::AlarmTimeout => pegboard_actor_v2::ActorTimer::AlarmTimeout,
		pegboard_actor_v3::ActorTimer::StopTimeout(v) => {
			pegboard_actor_v2::ActorTimer::StopTimeout(
				convert_stop_timeout_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::ActorTimer::EnvoyCheckTimeout(v) => {
			pegboard_actor_v2::ActorTimer::EnvoyCheckTimeout(
				convert_envoy_check_timeout_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		// V2 can decode this timer, which an early v2 build wrote, but the engine arms it only once
		// the fleet negotiates v3, so a v2 reader never has to handle a new one.
		pegboard_actor_v3::ActorTimer::StopThresholdTimeout(_) => {
			return Err(Inexpressible::lacks(
				2,
				"stop threshold timer armed, which only version 3 and later arm",
			));
		}
		pegboard_actor_v3::ActorTimer::AllocationRetry(_) => {
			return Err(Inexpressible::lacks(2, "allocation retry timer"));
		}
	})
}

pub fn convert_start_failed_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::StartFailed,
) -> Result<pegboard_actor_v2::StartFailed> {
	Ok(pegboard_actor_v2::StartFailed { epoch: x.epoch })
}

pub fn convert_envoy_gone_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::EnvoyGone,
) -> Result<pegboard_actor_v2::EnvoyGone> {
	Ok(pegboard_actor_v2::EnvoyGone { epoch: x.epoch })
}

pub fn convert_try_start_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::TryStart,
) -> Result<pegboard_actor_v2::TryStart> {
	Ok(pegboard_actor_v2::TryStart {
		waiter: x
			.waiter
			.map(|v| convert_start_waiter_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.transpose()?,
	})
}

pub fn convert_create_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::Create,
) -> Result<pegboard_actor_v2::Create> {
	Ok(pegboard_actor_v2::Create {
		init: convert_actor_create_data_pegboard_actor_v3_to_pegboard_actor_v2(x.init)?,
		waiter: x
			.waiter
			.map(|v| convert_start_waiter_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.transpose()?,
	})
}

pub fn convert_reallocate_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::Reallocate,
) -> Result<pegboard_actor_v2::Reallocate> {
	Ok(pegboard_actor_v2::Reallocate {
		epoch: x.epoch,
		lost: x.lost,
		waiter: x
			.waiter
			.map(|v| convert_start_waiter_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.transpose()?,
	})
}

pub fn convert_update_route_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::UpdateRoute,
) -> Result<pegboard_actor_v2::UpdateRoute> {
	Ok(pegboard_actor_v2::UpdateRoute {
		epoch: x.epoch,
		from: convert_actor_allocation_pegboard_actor_v3_to_pegboard_actor_v2(x.from)?,
		to: convert_actor_allocation_pegboard_actor_v3_to_pegboard_actor_v2(x.to)?,
	})
}

pub fn convert_to_envoy_event_kind_stopped_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ToEnvoyEventKindStopped,
) -> Result<pegboard_actor_v2::ToEnvoyEventKindStopped> {
	Ok(pegboard_actor_v2::ToEnvoyEventKindStopped {
		ok: x.ok,
		message: x.message,
	})
}

pub fn convert_to_envoy_event_kind_set_alarm_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ToEnvoyEventKindSetAlarm,
) -> Result<pegboard_actor_v2::ToEnvoyEventKindSetAlarm> {
	Ok(pegboard_actor_v2::ToEnvoyEventKindSetAlarm {
		alarm_unix_ms: x.alarm_unix_ms,
	})
}

pub fn convert_to_envoy_event_kind_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ToEnvoyEventKind,
) -> Result<pegboard_actor_v2::ToEnvoyEventKind> {
	Ok(match x {
		pegboard_actor_v3::ToEnvoyEventKind::ToEnvoyEventKindRunning => {
			pegboard_actor_v2::ToEnvoyEventKind::ToEnvoyEventKindRunning
		}
		pegboard_actor_v3::ToEnvoyEventKind::ToEnvoyEventKindStopped(v) => {
			pegboard_actor_v2::ToEnvoyEventKind::ToEnvoyEventKindStopped(
				convert_to_envoy_event_kind_stopped_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::ToEnvoyEventKind::ToEnvoyEventKindSleepIntent => {
			pegboard_actor_v2::ToEnvoyEventKind::ToEnvoyEventKindSleepIntent
		}
		pegboard_actor_v3::ToEnvoyEventKind::ToEnvoyEventKindStopIntent => {
			pegboard_actor_v2::ToEnvoyEventKind::ToEnvoyEventKindStopIntent
		}
		pegboard_actor_v3::ToEnvoyEventKind::ToEnvoyEventKindSetAlarm(v) => {
			pegboard_actor_v2::ToEnvoyEventKind::ToEnvoyEventKindSetAlarm(
				convert_to_envoy_event_kind_set_alarm_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
	})
}

pub fn convert_to_envoy_event_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ToEnvoyEvent,
) -> Result<pegboard_actor_v2::ToEnvoyEvent> {
	Ok(pegboard_actor_v2::ToEnvoyEvent {
		epoch: x.epoch,
		kind: convert_to_envoy_event_kind_pegboard_actor_v3_to_pegboard_actor_v2(x.kind)?,
	})
}

pub fn convert_migrate_from_v2_begin_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::MigrateFromV2Begin,
) -> Result<pegboard_actor_v2::MigrateFromV2Begin> {
	Ok(pegboard_actor_v2::MigrateFromV2Begin {
		migration_id: x.migration_id,
		init: convert_actor_create_data_pegboard_actor_v3_to_pegboard_actor_v2(x.init)?,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_migrate_from_v2_activate_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::MigrateFromV2Activate,
) -> Result<pegboard_actor_v2::MigrateFromV2Activate> {
	Ok(pegboard_actor_v2::MigrateFromV2Activate {
		migration_id: x.migration_id,
		alarm_unix_ms: x.alarm_unix_ms,
	})
}

pub fn convert_migrate_from_v2_discard_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::MigrateFromV2Discard,
) -> Result<pegboard_actor_v2::MigrateFromV2Discard> {
	Ok(pegboard_actor_v2::MigrateFromV2Discard {
		migration_id: x.migration_id,
	})
}

pub fn convert_migrate_from_v2_recall_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::MigrateFromV2Recall,
) -> Result<pegboard_actor_v2::MigrateFromV2Recall> {
	Ok(pegboard_actor_v2::MigrateFromV2Recall {
		migration_id: x.migration_id,
	})
}

pub fn convert_migrate_from_v2_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::MigrateFromV2,
) -> Result<pegboard_actor_v2::MigrateFromV2> {
	Ok(match x {
		pegboard_actor_v3::MigrateFromV2::MigrateFromV2Begin(v) => {
			pegboard_actor_v2::MigrateFromV2::MigrateFromV2Begin(
				convert_migrate_from_v2_begin_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::MigrateFromV2::MigrateFromV2Activate(v) => {
			pegboard_actor_v2::MigrateFromV2::MigrateFromV2Activate(
				convert_migrate_from_v2_activate_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::MigrateFromV2::MigrateFromV2Discard(v) => {
			pegboard_actor_v2::MigrateFromV2::MigrateFromV2Discard(
				convert_migrate_from_v2_discard_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::MigrateFromV2::MigrateFromV2Recall(v) => {
			pegboard_actor_v2::MigrateFromV2::MigrateFromV2Recall(
				convert_migrate_from_v2_recall_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
	})
}

pub fn convert_actor_event_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ActorEvent,
) -> Result<pegboard_actor_v2::ActorEvent> {
	Ok(match x {
		pegboard_actor_v3::ActorEvent::ReserveEpoxyState(v) => {
			pegboard_actor_v2::ActorEvent::ReserveEpoxyState(
				convert_reserve_epoxy_state_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::ActorEvent::WriteIndexState(v) => {
			pegboard_actor_v2::ActorEvent::WriteIndexState(
				convert_write_index_state_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::ActorEvent::StartFailed(v) => {
			pegboard_actor_v2::ActorEvent::StartFailed(
				convert_start_failed_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::ActorEvent::Sleep => pegboard_actor_v2::ActorEvent::Sleep,
		pegboard_actor_v3::ActorEvent::TryStart(v) => pegboard_actor_v2::ActorEvent::TryStart(
			convert_try_start_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
		),
		pegboard_actor_v3::ActorEvent::Create(v) => pegboard_actor_v2::ActorEvent::Create(
			convert_create_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
		),
		pegboard_actor_v3::ActorEvent::Reallocate(v) => pegboard_actor_v2::ActorEvent::Reallocate(
			convert_reallocate_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
		),
		pegboard_actor_v3::ActorEvent::UpdateRoute(v) => {
			pegboard_actor_v2::ActorEvent::UpdateRoute(
				convert_update_route_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::ActorEvent::Destroy => pegboard_actor_v2::ActorEvent::Destroy,
		pegboard_actor_v3::ActorEvent::DestroyState(v) => {
			pegboard_actor_v2::ActorEvent::DestroyState(
				convert_destroy_state_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::ActorEvent::ToEnvoyEvent(v) => {
			pegboard_actor_v2::ActorEvent::ToEnvoyEvent(
				convert_to_envoy_event_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::ActorEvent::EnvoyGone(v) => pegboard_actor_v2::ActorEvent::EnvoyGone(
			convert_envoy_gone_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
		),
		pegboard_actor_v3::ActorEvent::MigrateFromV2(v) => {
			pegboard_actor_v2::ActorEvent::MigrateFromV2(
				convert_migrate_from_v2_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
	})
}

pub fn convert_try_start_starting_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::TryStartStarting,
) -> Result<pegboard_actor_v2::TryStartStarting> {
	Ok(pegboard_actor_v2::TryStartStarting {
		epoch: x.epoch,
		allocation: convert_actor_allocation_pegboard_actor_v3_to_pegboard_actor_v2(x.allocation)?,
	})
}

pub fn convert_try_start_running_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::TryStartRunning,
) -> Result<pegboard_actor_v2::TryStartRunning> {
	Ok(pegboard_actor_v2::TryStartRunning {
		epoch: x.epoch,
		allocation: convert_actor_allocation_pegboard_actor_v3_to_pegboard_actor_v2(x.allocation)?,
	})
}

pub fn convert_try_start_epoxy_already_reserved_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::TryStartEpoxyAlreadyReserved,
) -> Result<pegboard_actor_v2::TryStartEpoxyAlreadyReserved> {
	Ok(pegboard_actor_v2::TryStartEpoxyAlreadyReserved {
		reservation_id: x.reservation_id,
	})
}

pub fn convert_try_start_outcome_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::TryStartOutcome,
) -> Result<pegboard_actor_v2::TryStartOutcome> {
	Ok(match x {
		pegboard_actor_v3::TryStartOutcome::TryStartWaiting => {
			pegboard_actor_v2::TryStartOutcome::TryStartWaiting
		}
		pegboard_actor_v3::TryStartOutcome::TryStartStarting(v) => {
			pegboard_actor_v2::TryStartOutcome::TryStartStarting(
				convert_try_start_starting_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::TryStartOutcome::TryStartRunning(v) => {
			pegboard_actor_v2::TryStartOutcome::TryStartRunning(
				convert_try_start_running_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::TryStartOutcome::TryStartDestroyed => {
			pegboard_actor_v2::TryStartOutcome::TryStartDestroyed
		}
		pegboard_actor_v3::TryStartOutcome::TryStartEpoxyAlreadyReserved(v) => {
			pegboard_actor_v2::TryStartOutcome::TryStartEpoxyAlreadyReserved(
				convert_try_start_epoxy_already_reserved_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::TryStartOutcome::TryStartFull => {
			pegboard_actor_v2::TryStartOutcome::TryStartFull
		}
		pegboard_actor_v3::TryStartOutcome::TryStartMigratingFromV2 => {
			pegboard_actor_v2::TryStartOutcome::TryStartMigratingFromV2
		}
	})
}

pub fn convert_actor_read_response_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ActorReadResponse,
) -> Result<pegboard_actor_v2::ActorReadResponse> {
	Ok(pegboard_actor_v2::ActorReadResponse {
		data: x
			.data
			.map(|v| convert_actor_data_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.transpose()?,
		phase: x
			.phase
			.map(|v| convert_actor_phase_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.transpose()?,
		epoch: x.epoch,
	})
}

pub fn convert_actor_event_response_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ActorEventResponse,
) -> Result<pegboard_actor_v2::ActorEventResponse> {
	Ok(pegboard_actor_v2::ActorEventResponse {
		outcome: x
			.outcome
			.map(|v| convert_try_start_outcome_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.transpose()?,
	})
}

pub fn convert_actor_migrate_from_v2_begin_response_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ActorMigrateFromV2BeginResponse,
) -> Result<pegboard_actor_v2::ActorMigrateFromV2BeginResponse> {
	Ok(pegboard_actor_v2::ActorMigrateFromV2BeginResponse {
		phase: convert_actor_phase_pegboard_actor_v3_to_pegboard_actor_v2(x.phase)?,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_actor_create_request_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ActorCreateRequest,
) -> Result<pegboard_actor_v2::ActorCreateRequest> {
	Ok(pegboard_actor_v2::ActorCreateRequest {
		init: convert_actor_create_data_pegboard_actor_v3_to_pegboard_actor_v2(x.init)?,
		waiter: x
			.waiter
			.map(|v| convert_start_waiter_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.transpose()?,
	})
}

pub fn convert_actor_create_response_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ActorCreateResponse,
) -> Result<pegboard_actor_v2::ActorCreateResponse> {
	Ok(pegboard_actor_v2::ActorCreateResponse {
		outcome: convert_try_start_outcome_pegboard_actor_v3_to_pegboard_actor_v2(x.outcome)?,
	})
}

pub fn convert_actor_start_notice_starting_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ActorStartNoticeStarting,
) -> Result<pegboard_actor_v2::ActorStartNoticeStarting> {
	Ok(pegboard_actor_v2::ActorStartNoticeStarting {
		epoch: x.epoch,
		allocation: convert_actor_allocation_pegboard_actor_v3_to_pegboard_actor_v2(x.allocation)?,
	})
}

pub fn convert_actor_start_notice_running_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ActorStartNoticeRunning,
) -> Result<pegboard_actor_v2::ActorStartNoticeRunning> {
	Ok(pegboard_actor_v2::ActorStartNoticeRunning {
		epoch: x.epoch,
		allocation: convert_actor_allocation_pegboard_actor_v3_to_pegboard_actor_v2(x.allocation)?,
	})
}

pub fn convert_actor_start_notice_epoxy_already_reserved_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ActorStartNoticeEpoxyAlreadyReserved,
) -> Result<pegboard_actor_v2::ActorStartNoticeEpoxyAlreadyReserved> {
	Ok(pegboard_actor_v2::ActorStartNoticeEpoxyAlreadyReserved {
		reservation_id: x.reservation_id,
	})
}

pub fn convert_actor_start_notice_kind_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ActorStartNoticeKind,
) -> Result<pegboard_actor_v2::ActorStartNoticeKind> {
	Ok(match x {
		pegboard_actor_v3::ActorStartNoticeKind::ActorStartNoticeStarting(v) => pegboard_actor_v2::ActorStartNoticeKind::ActorStartNoticeStarting(convert_actor_start_notice_starting_pegboard_actor_v3_to_pegboard_actor_v2(v)?),
		pegboard_actor_v3::ActorStartNoticeKind::ActorStartNoticeRunning(v) => pegboard_actor_v2::ActorStartNoticeKind::ActorStartNoticeRunning(convert_actor_start_notice_running_pegboard_actor_v3_to_pegboard_actor_v2(v)?),
		pegboard_actor_v3::ActorStartNoticeKind::ActorStartNoticeEpoxyAlreadyReserved(v) => pegboard_actor_v2::ActorStartNoticeKind::ActorStartNoticeEpoxyAlreadyReserved(convert_actor_start_notice_epoxy_already_reserved_pegboard_actor_v3_to_pegboard_actor_v2(v)?),
		pegboard_actor_v3::ActorStartNoticeKind::ActorStartNoticeDestroyed => pegboard_actor_v2::ActorStartNoticeKind::ActorStartNoticeDestroyed,
		pegboard_actor_v3::ActorStartNoticeKind::ActorStartNoticeMigratingFromV2 => pegboard_actor_v2::ActorStartNoticeKind::ActorStartNoticeMigratingFromV2,
	})
}

pub fn convert_actor_start_notice_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ActorStartNotice,
) -> Result<pegboard_actor_v2::ActorStartNotice> {
	Ok(pegboard_actor_v2::ActorStartNotice {
		actor_id: x.actor_id,
		kind: convert_actor_start_notice_kind_pegboard_actor_v3_to_pegboard_actor_v2(x.kind)?,
	})
}

pub fn convert_kv_value_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::KvValue,
) -> Result<pegboard_actor_v2::KvValue> {
	Ok(pegboard_actor_v2::KvValue {
		value: x.value,
		version: x.version,
		updated_unix_ms: x.updated_unix_ms,
	})
}

pub fn convert_index_insert_actor_request_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexInsertActorRequest,
) -> Result<pegboard_actor_v2::IndexInsertActorRequest> {
	Ok(pegboard_actor_v2::IndexInsertActorRequest {
		actor_id: x.actor_id,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_index_delete_actor_request_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexDeleteActorRequest,
) -> Result<pegboard_actor_v2::IndexDeleteActorRequest> {
	Ok(pegboard_actor_v2::IndexDeleteActorRequest {
		actor_id: x.actor_id,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_index_actor_cursor_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexActorCursor,
) -> Result<pegboard_actor_v2::IndexActorCursor> {
	Ok(pegboard_actor_v2::IndexActorCursor {
		create_unix_ms: x.create_unix_ms,
		actor_id: x.actor_id,
	})
}

pub fn convert_index_list_actors_request_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexListActorsRequest,
) -> Result<pegboard_actor_v2::IndexListActorsRequest> {
	Ok(pegboard_actor_v2::IndexListActorsRequest {
		after: x
			.after
			.map(|v| convert_index_actor_cursor_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.transpose()?,
		limit: x.limit,
		include_destroyed: x.include_destroyed,
	})
}

pub fn convert_index_destroyed_actor_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexDestroyedActor,
) -> Result<pegboard_actor_v2::IndexDestroyedActor> {
	Ok(pegboard_actor_v2::IndexDestroyedActor {
		create_unix_ms: x.create_unix_ms,
		actor_id: x.actor_id,
		destroy_unix_ms: x.destroy_unix_ms,
	})
}

pub fn convert_index_list_actors_response_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexListActorsResponse,
) -> Result<pegboard_actor_v2::IndexListActorsResponse> {
	Ok(pegboard_actor_v2::IndexListActorsResponse {
		actors: x
			.actors
			.into_iter()
			.map(|v| convert_index_actor_cursor_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.collect::<Result<Vec<_>>>()?,
		has_more: x.has_more,
		destroyed: x
			.destroyed
			.into_iter()
			.map(|v| convert_index_destroyed_actor_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_destroyed_actor_index_entry_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::DestroyedActorIndexEntry,
) -> Result<pegboard_actor_v2::DestroyedActorIndexEntry> {
	Ok(pegboard_actor_v2::DestroyedActorIndexEntry {
		destroy_unix_ms: x.destroy_unix_ms,
	})
}

pub fn convert_index_read_destroyed_actor_request_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexReadDestroyedActorRequest,
) -> Result<pegboard_actor_v2::IndexReadDestroyedActorRequest> {
	Ok(pegboard_actor_v2::IndexReadDestroyedActorRequest {
		create_unix_ms: x.create_unix_ms,
		actor_id: x.actor_id,
	})
}

pub fn convert_index_read_destroyed_actor_response_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexReadDestroyedActorResponse,
) -> Result<pegboard_actor_v2::IndexReadDestroyedActorResponse> {
	Ok(pegboard_actor_v2::IndexReadDestroyedActorResponse {
		destroy_unix_ms: x.destroy_unix_ms,
	})
}

pub fn convert_envoy_data_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::EnvoyData,
) -> Result<pegboard_actor_v2::EnvoyData> {
	Ok(pegboard_actor_v2::EnvoyData {
		key: x.key,
		worker_id: x.worker_id,
		connection_id: x.connection_id,
		connection_epoch: x.connection_epoch,
		version: x.version,
		names: x.names,
		create_unix_ms: x.create_unix_ms,
		heartbeat_unix_ms: x.heartbeat_unix_ms,
		stopping: x.stopping,
		connected: x.connected,
	})
}

pub fn convert_envoy_upsert_request_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::EnvoyUpsertRequest,
) -> Result<pegboard_actor_v2::EnvoyUpsertRequest> {
	Ok(pegboard_actor_v2::EnvoyUpsertRequest {
		data: convert_envoy_data_pegboard_actor_v3_to_pegboard_actor_v2(x.data)?,
		expected: x
			.expected
			.map(|v| convert_envoy_data_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.transpose()?,
	})
}

pub fn convert_envoy_upsert_response_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::EnvoyUpsertResponse,
) -> Result<pegboard_actor_v2::EnvoyUpsertResponse> {
	Ok(pegboard_actor_v2::EnvoyUpsertResponse {
		data: convert_envoy_data_pegboard_actor_v3_to_pegboard_actor_v2(x.data)?,
	})
}

pub fn convert_envoy_read_response_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::EnvoyReadResponse,
) -> Result<pegboard_actor_v2::EnvoyReadResponse> {
	Ok(pegboard_actor_v2::EnvoyReadResponse {
		data: x
			.data
			.map(|v| convert_envoy_data_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.transpose()?,
	})
}

pub fn convert_index_insert_envoy_request_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexInsertEnvoyRequest,
) -> Result<pegboard_actor_v2::IndexInsertEnvoyRequest> {
	Ok(pegboard_actor_v2::IndexInsertEnvoyRequest {
		create_unix_ms: x.create_unix_ms,
		envoy_key: x.envoy_key,
	})
}

pub fn convert_index_list_envoys_request_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexListEnvoysRequest,
) -> Result<pegboard_actor_v2::IndexListEnvoysRequest> {
	Ok(pegboard_actor_v2::IndexListEnvoysRequest { cursor: x.cursor })
}

pub fn convert_index_list_envoys_response_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexListEnvoysResponse,
) -> Result<pegboard_actor_v2::IndexListEnvoysResponse> {
	Ok(pegboard_actor_v2::IndexListEnvoysResponse {
		envoy_keys: x.envoy_keys,
		cursor: x.cursor,
	})
}

pub fn convert_active_envoy_index_entry_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ActiveEnvoyIndexEntry,
) -> Result<pegboard_actor_v2::ActiveEnvoyIndexEntry> {
	Ok(pegboard_actor_v2::ActiveEnvoyIndexEntry {
		worker_id: x.worker_id,
		connection_id: x.connection_id,
	})
}

pub fn convert_active_envoy_state_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::ActiveEnvoyState,
) -> Result<pegboard_actor_v2::ActiveEnvoyState> {
	Ok(pegboard_actor_v2::ActiveEnvoyState {
		envoy_key: x.envoy_key,
		connection_epoch: x.connection_epoch,
		version: x.version,
		heartbeat_unix_ms: x.heartbeat_unix_ms,
		names: x.names,
	})
}

pub fn convert_index_upsert_active_envoy_request_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexUpsertActiveEnvoyRequest,
) -> Result<pegboard_actor_v2::IndexUpsertActiveEnvoyRequest> {
	Ok(pegboard_actor_v2::IndexUpsertActiveEnvoyRequest {
		envoy_key: x.envoy_key,
		connection_epoch: x.connection_epoch,
		worker_id: x.worker_id,
		connection_id: x.connection_id,
		version: x.version,
		heartbeat_unix_ms: x.heartbeat_unix_ms,
		names: x.names,
	})
}

pub fn convert_index_upsert_active_envoy_response_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexUpsertActiveEnvoyResponse,
) -> Result<pegboard_actor_v2::IndexUpsertActiveEnvoyResponse> {
	Ok(pegboard_actor_v2::IndexUpsertActiveEnvoyResponse {
		superseded: x.superseded,
		newest_live_version: x.newest_live_version,
	})
}

pub fn convert_index_delete_active_envoy_request_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexDeleteActiveEnvoyRequest,
) -> Result<pegboard_actor_v2::IndexDeleteActiveEnvoyRequest> {
	Ok(pegboard_actor_v2::IndexDeleteActiveEnvoyRequest {
		envoy_key: x.envoy_key,
		connection_epoch: x.connection_epoch,
	})
}

pub fn convert_index_put_envoy_pool_request_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexPutEnvoyPoolRequest,
) -> Result<pegboard_actor_v2::IndexPutEnvoyPoolRequest> {
	Ok(pegboard_actor_v2::IndexPutEnvoyPoolRequest {
		pool_name: x.pool_name,
	})
}

pub fn convert_index_list_envoy_pools_response_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexListEnvoyPoolsResponse,
) -> Result<pegboard_actor_v2::IndexListEnvoyPoolsResponse> {
	Ok(pegboard_actor_v2::IndexListEnvoyPoolsResponse { pools: x.pools })
}

pub fn convert_index_list_active_envoys_request_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexListActiveEnvoysRequest,
) -> Result<pegboard_actor_v2::IndexListActiveEnvoysRequest> {
	Ok(pegboard_actor_v2::IndexListActiveEnvoysRequest {
		name: x.name,
		cursor: x.cursor,
	})
}

pub fn convert_index_list_active_envoys_response_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexListActiveEnvoysResponse,
) -> Result<pegboard_actor_v2::IndexListActiveEnvoysResponse> {
	Ok(pegboard_actor_v2::IndexListActiveEnvoysResponse {
		envoys: x
			.envoys
			.into_iter()
			.map(|v| convert_actor_allocation_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.collect::<Result<Vec<_>>>()?,
		cursor: x.cursor,
	})
}

pub fn convert_index_list_active_envoy_states_request_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexListActiveEnvoyStatesRequest,
) -> Result<pegboard_actor_v2::IndexListActiveEnvoyStatesRequest> {
	Ok(pegboard_actor_v2::IndexListActiveEnvoyStatesRequest { cursor: x.cursor })
}

pub fn convert_index_list_active_envoy_states_response_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::IndexListActiveEnvoyStatesResponse,
) -> Result<pegboard_actor_v2::IndexListActiveEnvoyStatesResponse> {
	Ok(pegboard_actor_v2::IndexListActiveEnvoyStatesResponse {
		envoys: x
			.envoys
			.into_iter()
			.map(|v| convert_active_envoy_state_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.collect::<Result<Vec<_>>>()?,
		cursor: x.cursor,
	})
}

pub fn convert_namespace_record_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::NamespaceRecord,
) -> Result<pegboard_actor_v2::NamespaceRecord> {
	Ok(pegboard_actor_v2::NamespaceRecord {
		create_unix_ms: x.create_unix_ms,
		create_operation_id: x.create_operation_id,
	})
}

pub fn convert_namespace_create_request_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::NamespaceCreateRequest,
) -> Result<pegboard_actor_v2::NamespaceCreateRequest> {
	Ok(pegboard_actor_v2::NamespaceCreateRequest {
		name: x.name,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_namespace_create_response_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::NamespaceCreateResponse,
) -> Result<pegboard_actor_v2::NamespaceCreateResponse> {
	Ok(pegboard_actor_v2::NamespaceCreateResponse {
		created: x.created,
		record: convert_namespace_record_pegboard_actor_v3_to_pegboard_actor_v2(x.record)?,
	})
}

pub fn convert_namespace_read_request_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::NamespaceReadRequest,
) -> Result<pegboard_actor_v2::NamespaceReadRequest> {
	Ok(pegboard_actor_v2::NamespaceReadRequest { name: x.name })
}

pub fn convert_namespace_read_response_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::NamespaceReadResponse,
) -> Result<pegboard_actor_v2::NamespaceReadResponse> {
	Ok(pegboard_actor_v2::NamespaceReadResponse {
		record: x
			.record
			.map(|v| convert_namespace_record_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.transpose()?,
	})
}

pub fn convert_namespace_entry_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::NamespaceEntry,
) -> Result<pegboard_actor_v2::NamespaceEntry> {
	Ok(pegboard_actor_v2::NamespaceEntry {
		name: x.name,
		record: convert_namespace_record_pegboard_actor_v3_to_pegboard_actor_v2(x.record)?,
	})
}

pub fn convert_namespace_list_request_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::NamespaceListRequest,
) -> Result<pegboard_actor_v2::NamespaceListRequest> {
	Ok(pegboard_actor_v2::NamespaceListRequest {
		after: x.after,
		limit: x.limit,
	})
}

pub fn convert_namespace_list_response_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::NamespaceListResponse,
) -> Result<pegboard_actor_v2::NamespaceListResponse> {
	Ok(pegboard_actor_v2::NamespaceListResponse {
		namespaces: x
			.namespaces
			.into_iter()
			.map(|v| convert_namespace_entry_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.collect::<Result<Vec<_>>>()?,
		cursor: x.cursor,
	})
}

pub fn convert_epoxy_actor_by_key_entry_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::EpoxyActorByKeyEntry,
) -> Result<pegboard_actor_v2::EpoxyActorByKeyEntry> {
	Ok(pegboard_actor_v2::EpoxyActorByKeyEntry {
		reservation_id: x.reservation_id,
	})
}

pub fn convert_node_pool_config_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::NodePoolConfig,
) -> Result<pegboard_actor_v2::NodePoolConfig> {
	Ok(pegboard_actor_v2::NodePoolConfig {
		drain_on_version_upgrade: x.drain_on_version_upgrade,
		actor_eviction_delay: x.actor_eviction_delay,
		actor_eviction_period: x.actor_eviction_period,
		actor_eviction_rate: x.actor_eviction_rate,
		metadata: x.metadata,
	})
}

pub fn convert_pool_connection_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::PoolConnection,
) -> Result<pegboard_actor_v2::PoolConnection> {
	Ok(pegboard_actor_v2::PoolConnection {
		connection_id: x.connection_id,
		heartbeat_unix_ms: x.heartbeat_unix_ms,
	})
}

pub fn convert_pool_put_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::PoolPut,
) -> Result<pegboard_actor_v2::PoolPut> {
	Ok(convert_node_pool_config_pegboard_actor_v3_to_pegboard_actor_v2(x)?)
}

pub fn convert_pool_connect_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::PoolConnect,
) -> Result<pegboard_actor_v2::PoolConnect> {
	Ok(convert_pool_connection_pegboard_actor_v3_to_pegboard_actor_v2(x)?)
}

pub fn convert_pool_renew_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::PoolRenew,
) -> Result<pegboard_actor_v2::PoolRenew> {
	Ok(convert_pool_connection_pegboard_actor_v3_to_pegboard_actor_v2(x)?)
}

pub fn convert_pool_operation_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::PoolOperation,
) -> Result<pegboard_actor_v2::PoolOperation> {
	Ok(match x {
		pegboard_actor_v3::PoolOperation::PoolRead => pegboard_actor_v2::PoolOperation::PoolRead,
		pegboard_actor_v3::PoolOperation::PoolPut(v) => pegboard_actor_v2::PoolOperation::PoolPut(
			convert_pool_put_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
		),
		pegboard_actor_v3::PoolOperation::PoolDelete => {
			pegboard_actor_v2::PoolOperation::PoolDelete
		}
		pegboard_actor_v3::PoolOperation::PoolConnect(v) => {
			pegboard_actor_v2::PoolOperation::PoolConnect(
				convert_pool_connect_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
		pegboard_actor_v3::PoolOperation::PoolDisconnect(v) => {
			pegboard_actor_v2::PoolOperation::PoolDisconnect(v)
		}
		pegboard_actor_v3::PoolOperation::PoolRenew(v) => {
			pegboard_actor_v2::PoolOperation::PoolRenew(
				convert_pool_renew_pegboard_actor_v3_to_pegboard_actor_v2(v)?,
			)
		}
	})
}

pub fn convert_pool_status_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::PoolStatus,
) -> Result<pegboard_actor_v2::PoolStatus> {
	Ok(match x {
		pegboard_actor_v3::PoolStatus::Ok => pegboard_actor_v2::PoolStatus::Ok,
		pegboard_actor_v3::PoolStatus::NotFound => pegboard_actor_v2::PoolStatus::NotFound,
		pegboard_actor_v3::PoolStatus::Connected => pegboard_actor_v2::PoolStatus::Connected,
	})
}

pub fn convert_pool_request_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::PoolRequest,
) -> Result<pegboard_actor_v2::PoolRequest> {
	Ok(pegboard_actor_v2::PoolRequest {
		pool_name: x.pool_name,
		operation: convert_pool_operation_pegboard_actor_v3_to_pegboard_actor_v2(x.operation)?,
	})
}

pub fn convert_pool_response_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::PoolResponse,
) -> Result<pegboard_actor_v2::PoolResponse> {
	Ok(pegboard_actor_v2::PoolResponse {
		status: convert_pool_status_pegboard_actor_v3_to_pegboard_actor_v2(x.status)?,
		config: x
			.config
			.map(|v| convert_node_pool_config_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.transpose()?,
	})
}

pub fn convert_pool_lease_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::PoolLease,
) -> Result<pegboard_actor_v2::PoolLease> {
	Ok(pegboard_actor_v2::PoolLease {
		heartbeat_unix_ms: x.heartbeat_unix_ms,
	})
}

pub fn convert_pool_entry_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::PoolEntry,
) -> Result<pegboard_actor_v2::PoolEntry> {
	Ok(pegboard_actor_v2::PoolEntry {
		name: x.name,
		config: convert_node_pool_config_pegboard_actor_v3_to_pegboard_actor_v2(x.config)?,
	})
}

pub fn convert_pool_list_request_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::PoolListRequest,
) -> Result<pegboard_actor_v2::PoolListRequest> {
	Ok(pegboard_actor_v2::PoolListRequest {
		after: x.after,
		limit: x.limit,
	})
}

pub fn convert_pool_list_response_pegboard_actor_v3_to_pegboard_actor_v2(
	x: pegboard_actor_v3::PoolListResponse,
) -> Result<pegboard_actor_v2::PoolListResponse> {
	Ok(pegboard_actor_v2::PoolListResponse {
		pools: x
			.pools
			.into_iter()
			.map(|v| convert_pool_entry_pegboard_actor_v3_to_pegboard_actor_v2(v))
			.collect::<Result<Vec<_>>>()?,
		cursor: x.cursor,
	})
}
