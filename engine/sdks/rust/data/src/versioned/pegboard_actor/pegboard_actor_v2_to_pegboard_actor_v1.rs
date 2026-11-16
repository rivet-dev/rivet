//! Field-by-field conversion of every `pegboard.actor` type from version 2 to version 1. A value that only version 2 can represent fails to convert.

use anyhow::Result;

use crate::generated::{pegboard_actor_v1, pegboard_actor_v2};
use crate::versioned::Inexpressible;

pub fn convert_actor_data_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ActorData,
) -> Result<pegboard_actor_v1::ActorData> {
	Ok(pegboard_actor_v1::ActorData {
		actor_id: x.actor_id,
		namespace: x.namespace,
		pool_name: x.pool_name,
		name: x.name,
		key: x.key,
		input: x.input,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_actor_allocation_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ActorAllocation,
) -> Result<pegboard_actor_v1::ActorAllocation> {
	Ok(pegboard_actor_v1::ActorAllocation {
		envoy_key: x.envoy_key,
		worker_id: x.worker_id,
		connection_id: x.connection_id,
	})
}

pub fn convert_reserve_epoxy_phase_already_reserved_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ReserveEpoxyPhaseAlreadyReserved,
) -> Result<pegboard_actor_v1::ReserveEpoxyPhaseAlreadyReserved> {
	Ok(pegboard_actor_v1::ReserveEpoxyPhaseAlreadyReserved {
		reservation_id: x.reservation_id,
	})
}

pub fn convert_reserve_epoxy_phase_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ReserveEpoxyPhase,
) -> Result<pegboard_actor_v1::ReserveEpoxyPhase> {
	Ok(match x {
		pegboard_actor_v2::ReserveEpoxyPhase::ReserveEpoxyPhasePending => pegboard_actor_v1::ReserveEpoxyPhase::ReserveEpoxyPhasePending,
		pegboard_actor_v2::ReserveEpoxyPhase::ReserveEpoxyPhaseReserved => pegboard_actor_v1::ReserveEpoxyPhase::ReserveEpoxyPhaseReserved,
		pegboard_actor_v2::ReserveEpoxyPhase::ReserveEpoxyPhaseAlreadyReserved(v) => pegboard_actor_v1::ReserveEpoxyPhase::ReserveEpoxyPhaseAlreadyReserved(convert_reserve_epoxy_phase_already_reserved_pegboard_actor_v2_to_pegboard_actor_v1(v)?),
		pegboard_actor_v2::ReserveEpoxyPhase::ReserveEpoxyPhaseFailed => pegboard_actor_v1::ReserveEpoxyPhase::ReserveEpoxyPhaseFailed,
		pegboard_actor_v2::ReserveEpoxyPhase::ReserveEpoxyPhaseMaxAttemptsReached => pegboard_actor_v1::ReserveEpoxyPhase::ReserveEpoxyPhaseMaxAttemptsReached,
	})
}

pub fn convert_reserve_epoxy_state_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ReserveEpoxyState,
) -> Result<pegboard_actor_v1::ReserveEpoxyState> {
	Ok(pegboard_actor_v1::ReserveEpoxyState {
		phase: convert_reserve_epoxy_phase_pegboard_actor_v2_to_pegboard_actor_v1(x.phase)?,
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_reserve_epoxy_effect_data_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ReserveEpoxyEffectData,
) -> Result<pegboard_actor_v1::ReserveEpoxyEffectData> {
	Ok(pegboard_actor_v1::ReserveEpoxyEffectData {
		namespace: x.namespace,
		name: x.name,
		key: x.key,
		state: convert_reserve_epoxy_state_pegboard_actor_v2_to_pegboard_actor_v1(x.state)?,
	})
}

pub fn convert_pending_action_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::PendingAction,
) -> Result<pegboard_actor_v1::PendingAction> {
	Ok(match x {
		pegboard_actor_v2::PendingAction::PendingNone => {
			pegboard_actor_v1::PendingAction::PendingNone
		}
		pegboard_actor_v2::PendingAction::PendingSleep => {
			pegboard_actor_v1::PendingAction::PendingSleep
		}
		pegboard_actor_v2::PendingAction::PendingReallocate => {
			pegboard_actor_v1::PendingAction::PendingReallocate
		}
		pegboard_actor_v2::PendingAction::PendingDestroy => {
			pegboard_actor_v1::PendingAction::PendingDestroy
		}
	})
}

pub fn convert_actor_create_data_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ActorCreateData,
) -> Result<pegboard_actor_v1::ActorCreateData> {
	Ok(pegboard_actor_v1::ActorCreateData {
		actor_id: x.actor_id,
		namespace: x.namespace,
		pool_name: x.pool_name,
		name: x.name,
		key: x.key,
		input: x.input,
	})
}

pub fn convert_initializing_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::Initializing,
) -> Result<pegboard_actor_v1::Initializing> {
	Ok(pegboard_actor_v1::Initializing {
		reserve_epoxy: convert_reserve_epoxy_effect_data_pegboard_actor_v2_to_pegboard_actor_v1(
			x.reserve_epoxy,
		)?,
		allocation: x
			.allocation
			.map(|v| convert_actor_allocation_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.transpose()?,
	})
}

pub fn convert_starting_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::Starting,
) -> Result<pegboard_actor_v1::Starting> {
	Ok(pegboard_actor_v1::Starting {
		allocation: convert_actor_allocation_pegboard_actor_v2_to_pegboard_actor_v1(x.allocation)?,
	})
}

pub fn convert_running_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::Running,
) -> Result<pegboard_actor_v1::Running> {
	Ok(pegboard_actor_v1::Running {
		allocation: convert_actor_allocation_pegboard_actor_v2_to_pegboard_actor_v1(x.allocation)?,
	})
}

pub fn convert_stopping_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::Stopping,
) -> Result<pegboard_actor_v1::Stopping> {
	Ok(pegboard_actor_v1::Stopping {
		allocation: convert_actor_allocation_pegboard_actor_v2_to_pegboard_actor_v1(x.allocation)?,
	})
}

pub fn convert_destroy_phase_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::DestroyPhase,
) -> Result<pegboard_actor_v1::DestroyPhase> {
	Ok(match x {
		pegboard_actor_v2::DestroyPhase::DestroyPhasePending => {
			pegboard_actor_v1::DestroyPhase::DestroyPhasePending
		}
		pegboard_actor_v2::DestroyPhase::DestroyPhaseSuccess => {
			pegboard_actor_v1::DestroyPhase::DestroyPhaseSuccess
		}
		pegboard_actor_v2::DestroyPhase::DestroyPhaseFailed => {
			pegboard_actor_v1::DestroyPhase::DestroyPhaseFailed
		}
	})
}

pub fn convert_destroy_state_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::DestroyState,
) -> Result<pegboard_actor_v1::DestroyState> {
	Ok(pegboard_actor_v1::DestroyState {
		phase: convert_destroy_phase_pegboard_actor_v2_to_pegboard_actor_v1(x.phase)?,
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_destroy_effect_data_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::DestroyEffectData,
) -> Result<pegboard_actor_v1::DestroyEffectData> {
	Ok(pegboard_actor_v1::DestroyEffectData {
		create_unix_ms: x.create_unix_ms,
		state: convert_destroy_state_pegboard_actor_v2_to_pegboard_actor_v1(x.state)?,
	})
}

pub fn convert_destroying_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::Destroying,
) -> Result<pegboard_actor_v1::Destroying> {
	Ok(pegboard_actor_v1::Destroying {
		destroy: convert_destroy_effect_data_pegboard_actor_v2_to_pegboard_actor_v1(x.destroy)?,
		epoxy_already_reserved: x.epoxy_already_reserved,
	})
}

pub fn convert_actor_phase_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ActorPhase,
) -> Result<pegboard_actor_v1::ActorPhase> {
	Ok(match x {
		pegboard_actor_v2::ActorPhase::Initializing(v) => {
			pegboard_actor_v1::ActorPhase::Initializing(
				convert_initializing_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
			)
		}
		pegboard_actor_v2::ActorPhase::Starting(v) => pegboard_actor_v1::ActorPhase::Starting(
			convert_starting_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
		),
		pegboard_actor_v2::ActorPhase::Running(v) => pegboard_actor_v1::ActorPhase::Running(
			convert_running_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
		),
		pegboard_actor_v2::ActorPhase::Stopping(v) => pegboard_actor_v1::ActorPhase::Stopping(
			convert_stopping_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
		),
		// Version 1 keeps no alarm retry count; a downgraded actor starts its backoff over.
		pegboard_actor_v2::ActorPhase::Sleeping(_) => pegboard_actor_v1::ActorPhase::Sleeping,
		pegboard_actor_v2::ActorPhase::Destroying(v) => pegboard_actor_v1::ActorPhase::Destroying(
			convert_destroying_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
		),
		pegboard_actor_v2::ActorPhase::Destroyed => pegboard_actor_v1::ActorPhase::Destroyed,
		pegboard_actor_v2::ActorPhase::MigratingFromV2(_) => {
			return Err(Inexpressible::lacks(1, "actor held for the V2 migrator"));
		}
	})
}

pub fn convert_start_waiter_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::StartWaiter,
) -> Result<pegboard_actor_v1::StartWaiter> {
	Ok(pegboard_actor_v1::StartWaiter {
		worker_id: x.worker_id,
		wait_id: x.wait_id,
		expires_unix_ms: x.expires_unix_ms,
	})
}

pub fn convert_write_index_phase_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::WriteIndexPhase,
) -> Result<pegboard_actor_v1::WriteIndexPhase> {
	Ok(match x {
		pegboard_actor_v2::WriteIndexPhase::WriteIndexPhasePending => {
			pegboard_actor_v1::WriteIndexPhase::WriteIndexPhasePending
		}
		pegboard_actor_v2::WriteIndexPhase::WriteIndexPhaseSuccess => {
			pegboard_actor_v1::WriteIndexPhase::WriteIndexPhaseSuccess
		}
		pegboard_actor_v2::WriteIndexPhase::WriteIndexPhaseFailed => {
			pegboard_actor_v1::WriteIndexPhase::WriteIndexPhaseFailed
		}
	})
}

pub fn convert_write_index_state_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::WriteIndexState,
) -> Result<pegboard_actor_v1::WriteIndexState> {
	Ok(pegboard_actor_v1::WriteIndexState {
		phase: convert_write_index_phase_pegboard_actor_v2_to_pegboard_actor_v1(x.phase)?,
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_write_index_effect_data_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::WriteIndexEffectData,
) -> Result<pegboard_actor_v1::WriteIndexEffectData> {
	Ok(pegboard_actor_v1::WriteIndexEffectData {
		namespace: x.namespace,
		name: x.name,
		create_unix_ms: x.create_unix_ms,
		state: convert_write_index_state_pegboard_actor_v2_to_pegboard_actor_v1(x.state)?,
	})
}

pub fn convert_actor_state_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ActorState,
) -> Result<pegboard_actor_v1::ActorState> {
	// Version 1 treats an incarnation's first start as new storage, so an imported incarnation
	// that has not started yet would start on an empty keyspace over its imported data.
	if matches!(x.storage_origin, pegboard_actor_v2::StorageOrigin::Imported)
		&& x.command_index == 0
	{
		return Err(Inexpressible::lacks(
			1,
			"imported actor before its first start",
		));
	}

	Ok(pegboard_actor_v1::ActorState {
		phase: convert_actor_phase_pegboard_actor_v2_to_pegboard_actor_v1(x.phase)?,
		epoch: x.epoch,
		create_unix_ms: x.create_unix_ms,
		command_index: x.command_index,
		index_bucket_count: x.index_bucket_count,
		pending_action: convert_pending_action_pegboard_actor_v2_to_pegboard_actor_v1(
			x.pending_action,
		)?,
		write_index: convert_write_index_effect_data_pegboard_actor_v2_to_pegboard_actor_v1(
			x.write_index,
		)?,
		start_waiters: x
			.start_waiters
			.into_iter()
			.map(|v| convert_start_waiter_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.collect::<Result<Vec<_>>>()?,
		// Version 2 reads the cap from `pegboard.actor_start_waiters_capacity` on every event
		// instead of keeping it in the record, so a downgraded record gets that setting's default.
		start_waiters_capacity: 64,
	})
}

pub fn convert_reserve_epoxy_timeout_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ReserveEpoxyTimeout,
) -> Result<pegboard_actor_v1::ReserveEpoxyTimeout> {
	Ok(pegboard_actor_v1::ReserveEpoxyTimeout {
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_write_index_timeout_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::WriteIndexTimeout,
) -> Result<pegboard_actor_v1::WriteIndexTimeout> {
	Ok(pegboard_actor_v1::WriteIndexTimeout {
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_destroy_timeout_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::DestroyTimeout,
) -> Result<pegboard_actor_v1::DestroyTimeout> {
	Ok(pegboard_actor_v1::DestroyTimeout {
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_start_timeout_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::StartTimeout,
) -> Result<pegboard_actor_v1::StartTimeout> {
	Ok(pegboard_actor_v1::StartTimeout { epoch: x.epoch })
}

pub fn convert_stop_timeout_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::StopTimeout,
) -> Result<pegboard_actor_v1::StopTimeout> {
	Ok(pegboard_actor_v1::StopTimeout { epoch: x.epoch })
}

pub fn convert_actor_timer_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ActorTimer,
) -> Result<pegboard_actor_v1::ActorTimer> {
	Ok(match x {
		pegboard_actor_v2::ActorTimer::ReserveEpoxyTimeout(v) => {
			pegboard_actor_v1::ActorTimer::ReserveEpoxyTimeout(
				convert_reserve_epoxy_timeout_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
			)
		}
		pegboard_actor_v2::ActorTimer::WriteIndexTimeout(v) => {
			pegboard_actor_v1::ActorTimer::WriteIndexTimeout(
				convert_write_index_timeout_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
			)
		}
		pegboard_actor_v2::ActorTimer::DestroyTimeout(v) => {
			pegboard_actor_v1::ActorTimer::DestroyTimeout(
				convert_destroy_timeout_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
			)
		}
		pegboard_actor_v2::ActorTimer::StartTimeout(v) => {
			pegboard_actor_v1::ActorTimer::StartTimeout(
				convert_start_timeout_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
			)
		}
		pegboard_actor_v2::ActorTimer::AlarmTimeout => pegboard_actor_v1::ActorTimer::AlarmTimeout,
		pegboard_actor_v2::ActorTimer::StopTimeout(v) => {
			pegboard_actor_v1::ActorTimer::StopTimeout(
				convert_stop_timeout_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
			)
		}
		pegboard_actor_v2::ActorTimer::EnvoyCheckTimeout(_) => {
			return Err(Inexpressible::lacks(1, "envoy check timeout"));
		}
		pegboard_actor_v2::ActorTimer::StopThresholdTimeout(_) => {
			return Err(Inexpressible::lacks(1, "stop threshold timeout"));
		}
	})
}

pub fn convert_start_failed_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::StartFailed,
) -> Result<pegboard_actor_v1::StartFailed> {
	Ok(pegboard_actor_v1::StartFailed { epoch: x.epoch })
}

pub fn convert_envoy_gone_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::EnvoyGone,
) -> Result<pegboard_actor_v1::EnvoyGone> {
	Ok(pegboard_actor_v1::EnvoyGone { epoch: x.epoch })
}

pub fn convert_try_start_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::TryStart,
) -> Result<pegboard_actor_v1::TryStart> {
	Ok(pegboard_actor_v1::TryStart {
		waiter: x
			.waiter
			.map(|v| convert_start_waiter_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.transpose()?,
	})
}

pub fn convert_create_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::Create,
) -> Result<pegboard_actor_v1::Create> {
	Ok(pegboard_actor_v1::Create {
		init: convert_actor_create_data_pegboard_actor_v2_to_pegboard_actor_v1(x.init)?,
		waiter: x
			.waiter
			.map(|v| convert_start_waiter_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.transpose()?,
	})
}

pub fn convert_reallocate_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::Reallocate,
) -> Result<pegboard_actor_v1::Reallocate> {
	Ok(pegboard_actor_v1::Reallocate {
		epoch: x.epoch,
		lost: x.lost,
		waiter: x
			.waiter
			.map(|v| convert_start_waiter_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.transpose()?,
	})
}

pub fn convert_update_route_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::UpdateRoute,
) -> Result<pegboard_actor_v1::UpdateRoute> {
	Ok(pegboard_actor_v1::UpdateRoute {
		epoch: x.epoch,
		from: convert_actor_allocation_pegboard_actor_v2_to_pegboard_actor_v1(x.from)?,
		to: convert_actor_allocation_pegboard_actor_v2_to_pegboard_actor_v1(x.to)?,
	})
}

pub fn convert_to_envoy_event_kind_stopped_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ToEnvoyEventKindStopped,
) -> Result<pegboard_actor_v1::ToEnvoyEventKindStopped> {
	Ok(pegboard_actor_v1::ToEnvoyEventKindStopped {
		ok: x.ok,
		message: x.message,
	})
}

pub fn convert_to_envoy_event_kind_set_alarm_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ToEnvoyEventKindSetAlarm,
) -> Result<pegboard_actor_v1::ToEnvoyEventKindSetAlarm> {
	Ok(pegboard_actor_v1::ToEnvoyEventKindSetAlarm {
		alarm_unix_ms: x.alarm_unix_ms,
	})
}

pub fn convert_to_envoy_event_kind_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ToEnvoyEventKind,
) -> Result<pegboard_actor_v1::ToEnvoyEventKind> {
	Ok(match x {
		pegboard_actor_v2::ToEnvoyEventKind::ToEnvoyEventKindRunning => {
			pegboard_actor_v1::ToEnvoyEventKind::ToEnvoyEventKindRunning
		}
		pegboard_actor_v2::ToEnvoyEventKind::ToEnvoyEventKindStopped(v) => {
			pegboard_actor_v1::ToEnvoyEventKind::ToEnvoyEventKindStopped(
				convert_to_envoy_event_kind_stopped_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
			)
		}
		pegboard_actor_v2::ToEnvoyEventKind::ToEnvoyEventKindSleepIntent => {
			pegboard_actor_v1::ToEnvoyEventKind::ToEnvoyEventKindSleepIntent
		}
		pegboard_actor_v2::ToEnvoyEventKind::ToEnvoyEventKindStopIntent => {
			pegboard_actor_v1::ToEnvoyEventKind::ToEnvoyEventKindStopIntent
		}
		pegboard_actor_v2::ToEnvoyEventKind::ToEnvoyEventKindSetAlarm(v) => {
			pegboard_actor_v1::ToEnvoyEventKind::ToEnvoyEventKindSetAlarm(
				convert_to_envoy_event_kind_set_alarm_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
			)
		}
	})
}

pub fn convert_to_envoy_event_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ToEnvoyEvent,
) -> Result<pegboard_actor_v1::ToEnvoyEvent> {
	Ok(pegboard_actor_v1::ToEnvoyEvent {
		epoch: x.epoch,
		kind: convert_to_envoy_event_kind_pegboard_actor_v2_to_pegboard_actor_v1(x.kind)?,
	})
}

pub fn convert_actor_event_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ActorEvent,
) -> Result<pegboard_actor_v1::ActorEvent> {
	Ok(match x {
		pegboard_actor_v2::ActorEvent::ReserveEpoxyState(v) => {
			pegboard_actor_v1::ActorEvent::ReserveEpoxyState(
				convert_reserve_epoxy_state_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
			)
		}
		pegboard_actor_v2::ActorEvent::WriteIndexState(v) => {
			pegboard_actor_v1::ActorEvent::WriteIndexState(
				convert_write_index_state_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
			)
		}
		pegboard_actor_v2::ActorEvent::StartFailed(v) => {
			pegboard_actor_v1::ActorEvent::StartFailed(
				convert_start_failed_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
			)
		}
		pegboard_actor_v2::ActorEvent::Sleep => pegboard_actor_v1::ActorEvent::Sleep,
		pegboard_actor_v2::ActorEvent::TryStart(v) => pegboard_actor_v1::ActorEvent::TryStart(
			convert_try_start_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
		),
		pegboard_actor_v2::ActorEvent::Create(v) => pegboard_actor_v1::ActorEvent::Create(
			convert_create_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
		),
		pegboard_actor_v2::ActorEvent::Reallocate(v) => pegboard_actor_v1::ActorEvent::Reallocate(
			convert_reallocate_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
		),
		pegboard_actor_v2::ActorEvent::UpdateRoute(v) => {
			pegboard_actor_v1::ActorEvent::UpdateRoute(
				convert_update_route_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
			)
		}
		pegboard_actor_v2::ActorEvent::Destroy => pegboard_actor_v1::ActorEvent::Destroy,
		pegboard_actor_v2::ActorEvent::DestroyState(v) => {
			pegboard_actor_v1::ActorEvent::DestroyState(
				convert_destroy_state_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
			)
		}
		pegboard_actor_v2::ActorEvent::ToEnvoyEvent(v) => {
			pegboard_actor_v1::ActorEvent::ToEnvoyEvent(
				convert_to_envoy_event_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
			)
		}
		pegboard_actor_v2::ActorEvent::EnvoyGone(v) => pegboard_actor_v1::ActorEvent::EnvoyGone(
			convert_envoy_gone_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
		),
		pegboard_actor_v2::ActorEvent::MigrateFromV2(_) => {
			return Err(Inexpressible::lacks(1, "V2 migration events"));
		}
	})
}

pub fn convert_try_start_starting_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::TryStartStarting,
) -> Result<pegboard_actor_v1::TryStartStarting> {
	Ok(pegboard_actor_v1::TryStartStarting {
		epoch: x.epoch,
		allocation: convert_actor_allocation_pegboard_actor_v2_to_pegboard_actor_v1(x.allocation)?,
	})
}

pub fn convert_try_start_running_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::TryStartRunning,
) -> Result<pegboard_actor_v1::TryStartRunning> {
	Ok(pegboard_actor_v1::TryStartRunning {
		epoch: x.epoch,
		allocation: convert_actor_allocation_pegboard_actor_v2_to_pegboard_actor_v1(x.allocation)?,
	})
}

pub fn convert_try_start_epoxy_already_reserved_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::TryStartEpoxyAlreadyReserved,
) -> Result<pegboard_actor_v1::TryStartEpoxyAlreadyReserved> {
	Ok(pegboard_actor_v1::TryStartEpoxyAlreadyReserved {
		reservation_id: x.reservation_id,
	})
}

pub fn convert_try_start_outcome_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::TryStartOutcome,
) -> Result<pegboard_actor_v1::TryStartOutcome> {
	Ok(match x {
		pegboard_actor_v2::TryStartOutcome::TryStartWaiting => {
			pegboard_actor_v1::TryStartOutcome::TryStartWaiting
		}
		pegboard_actor_v2::TryStartOutcome::TryStartStarting(v) => {
			pegboard_actor_v1::TryStartOutcome::TryStartStarting(
				convert_try_start_starting_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
			)
		}
		pegboard_actor_v2::TryStartOutcome::TryStartRunning(v) => {
			pegboard_actor_v1::TryStartOutcome::TryStartRunning(
				convert_try_start_running_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
			)
		}
		pegboard_actor_v2::TryStartOutcome::TryStartDestroyed => {
			pegboard_actor_v1::TryStartOutcome::TryStartDestroyed
		}
		pegboard_actor_v2::TryStartOutcome::TryStartEpoxyAlreadyReserved(v) => {
			pegboard_actor_v1::TryStartOutcome::TryStartEpoxyAlreadyReserved(
				convert_try_start_epoxy_already_reserved_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
			)
		}
		pegboard_actor_v2::TryStartOutcome::TryStartFull => {
			pegboard_actor_v1::TryStartOutcome::TryStartFull
		}
		pegboard_actor_v2::TryStartOutcome::TryStartMigratingFromV2 => {
			return Err(Inexpressible::lacks(
				1,
				"start outcome for an actor held for the V2 migrator",
			));
		}
	})
}

pub fn convert_actor_read_response_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ActorReadResponse,
) -> Result<pegboard_actor_v1::ActorReadResponse> {
	Ok(pegboard_actor_v1::ActorReadResponse {
		data: x
			.data
			.map(|v| convert_actor_data_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.transpose()?,
		phase: x
			.phase
			.map(|v| convert_actor_phase_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.transpose()?,
		epoch: x.epoch,
	})
}

pub fn convert_actor_event_response_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ActorEventResponse,
) -> Result<pegboard_actor_v1::ActorEventResponse> {
	Ok(pegboard_actor_v1::ActorEventResponse {
		outcome: x
			.outcome
			.map(|v| convert_try_start_outcome_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.transpose()?,
	})
}

pub fn convert_actor_create_request_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ActorCreateRequest,
) -> Result<pegboard_actor_v1::ActorCreateRequest> {
	Ok(pegboard_actor_v1::ActorCreateRequest {
		init: convert_actor_create_data_pegboard_actor_v2_to_pegboard_actor_v1(x.init)?,
		waiter: x
			.waiter
			.map(|v| convert_start_waiter_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.transpose()?,
	})
}

pub fn convert_actor_create_response_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ActorCreateResponse,
) -> Result<pegboard_actor_v1::ActorCreateResponse> {
	Ok(pegboard_actor_v1::ActorCreateResponse {
		outcome: convert_try_start_outcome_pegboard_actor_v2_to_pegboard_actor_v1(x.outcome)?,
	})
}

pub fn convert_actor_start_notice_starting_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ActorStartNoticeStarting,
) -> Result<pegboard_actor_v1::ActorStartNoticeStarting> {
	Ok(pegboard_actor_v1::ActorStartNoticeStarting {
		epoch: x.epoch,
		allocation: convert_actor_allocation_pegboard_actor_v2_to_pegboard_actor_v1(x.allocation)?,
	})
}

pub fn convert_actor_start_notice_running_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ActorStartNoticeRunning,
) -> Result<pegboard_actor_v1::ActorStartNoticeRunning> {
	Ok(pegboard_actor_v1::ActorStartNoticeRunning {
		epoch: x.epoch,
		allocation: convert_actor_allocation_pegboard_actor_v2_to_pegboard_actor_v1(x.allocation)?,
	})
}

pub fn convert_actor_start_notice_epoxy_already_reserved_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ActorStartNoticeEpoxyAlreadyReserved,
) -> Result<pegboard_actor_v1::ActorStartNoticeEpoxyAlreadyReserved> {
	Ok(pegboard_actor_v1::ActorStartNoticeEpoxyAlreadyReserved {
		reservation_id: x.reservation_id,
	})
}

pub fn convert_actor_start_notice_kind_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ActorStartNoticeKind,
) -> Result<pegboard_actor_v1::ActorStartNoticeKind> {
	Ok(match x {
		pegboard_actor_v2::ActorStartNoticeKind::ActorStartNoticeStarting(v) => pegboard_actor_v1::ActorStartNoticeKind::ActorStartNoticeStarting(convert_actor_start_notice_starting_pegboard_actor_v2_to_pegboard_actor_v1(v)?),
		pegboard_actor_v2::ActorStartNoticeKind::ActorStartNoticeRunning(v) => pegboard_actor_v1::ActorStartNoticeKind::ActorStartNoticeRunning(convert_actor_start_notice_running_pegboard_actor_v2_to_pegboard_actor_v1(v)?),
		pegboard_actor_v2::ActorStartNoticeKind::ActorStartNoticeEpoxyAlreadyReserved(v) => pegboard_actor_v1::ActorStartNoticeKind::ActorStartNoticeEpoxyAlreadyReserved(convert_actor_start_notice_epoxy_already_reserved_pegboard_actor_v2_to_pegboard_actor_v1(v)?),
		pegboard_actor_v2::ActorStartNoticeKind::ActorStartNoticeDestroyed => pegboard_actor_v1::ActorStartNoticeKind::ActorStartNoticeDestroyed,
		pegboard_actor_v2::ActorStartNoticeKind::ActorStartNoticeMigratingFromV2 => {
			return Err(Inexpressible::lacks(
				1,
				"start notice for an actor held for the V2 migrator",
			))
		}
	})
}

pub fn convert_actor_start_notice_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ActorStartNotice,
) -> Result<pegboard_actor_v1::ActorStartNotice> {
	Ok(pegboard_actor_v1::ActorStartNotice {
		actor_id: x.actor_id,
		kind: convert_actor_start_notice_kind_pegboard_actor_v2_to_pegboard_actor_v1(x.kind)?,
	})
}

pub fn convert_kv_value_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::KvValue,
) -> Result<pegboard_actor_v1::KvValue> {
	Ok(pegboard_actor_v1::KvValue {
		value: x.value,
		version: x.version,
		updated_unix_ms: x.updated_unix_ms,
	})
}

pub fn convert_index_insert_actor_request_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexInsertActorRequest,
) -> Result<pegboard_actor_v1::IndexInsertActorRequest> {
	Ok(pegboard_actor_v1::IndexInsertActorRequest {
		actor_id: x.actor_id,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_index_delete_actor_request_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexDeleteActorRequest,
) -> Result<pegboard_actor_v1::IndexDeleteActorRequest> {
	Ok(pegboard_actor_v1::IndexDeleteActorRequest {
		actor_id: x.actor_id,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_index_actor_cursor_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexActorCursor,
) -> Result<pegboard_actor_v1::IndexActorCursor> {
	Ok(pegboard_actor_v1::IndexActorCursor {
		create_unix_ms: x.create_unix_ms,
		actor_id: x.actor_id,
	})
}

pub fn convert_index_list_actors_request_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexListActorsRequest,
) -> Result<pegboard_actor_v1::IndexListActorsRequest> {
	Ok(pegboard_actor_v1::IndexListActorsRequest {
		after: x
			.after
			.map(|v| convert_index_actor_cursor_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.transpose()?,
		limit: x.limit,
		include_destroyed: x.include_destroyed,
	})
}

pub fn convert_index_destroyed_actor_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexDestroyedActor,
) -> Result<pegboard_actor_v1::IndexDestroyedActor> {
	Ok(pegboard_actor_v1::IndexDestroyedActor {
		create_unix_ms: x.create_unix_ms,
		actor_id: x.actor_id,
		destroy_unix_ms: x.destroy_unix_ms,
	})
}

pub fn convert_index_list_actors_response_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexListActorsResponse,
) -> Result<pegboard_actor_v1::IndexListActorsResponse> {
	Ok(pegboard_actor_v1::IndexListActorsResponse {
		actors: x
			.actors
			.into_iter()
			.map(|v| convert_index_actor_cursor_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.collect::<Result<Vec<_>>>()?,
		has_more: x.has_more,
		destroyed: x
			.destroyed
			.into_iter()
			.map(|v| convert_index_destroyed_actor_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_destroyed_actor_index_entry_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::DestroyedActorIndexEntry,
) -> Result<pegboard_actor_v1::DestroyedActorIndexEntry> {
	Ok(pegboard_actor_v1::DestroyedActorIndexEntry {
		destroy_unix_ms: x.destroy_unix_ms,
	})
}

pub fn convert_index_read_destroyed_actor_request_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexReadDestroyedActorRequest,
) -> Result<pegboard_actor_v1::IndexReadDestroyedActorRequest> {
	Ok(pegboard_actor_v1::IndexReadDestroyedActorRequest {
		create_unix_ms: x.create_unix_ms,
		actor_id: x.actor_id,
	})
}

pub fn convert_index_read_destroyed_actor_response_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexReadDestroyedActorResponse,
) -> Result<pegboard_actor_v1::IndexReadDestroyedActorResponse> {
	Ok(pegboard_actor_v1::IndexReadDestroyedActorResponse {
		destroy_unix_ms: x.destroy_unix_ms,
	})
}

pub fn convert_envoy_data_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::EnvoyData,
) -> Result<pegboard_actor_v1::EnvoyData> {
	Ok(pegboard_actor_v1::EnvoyData {
		key: x.key,
		worker_id: x.worker_id,
		connection_id: x.connection_id,
		connection_epoch: x.connection_epoch,
		names: x.names,
		create_unix_ms: x.create_unix_ms,
		heartbeat_unix_ms: x.heartbeat_unix_ms,
		stopping: x.stopping,
		connected: x.connected,
	})
}

pub fn convert_envoy_upsert_request_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::EnvoyUpsertRequest,
) -> Result<pegboard_actor_v1::EnvoyUpsertRequest> {
	Ok(pegboard_actor_v1::EnvoyUpsertRequest {
		data: convert_envoy_data_pegboard_actor_v2_to_pegboard_actor_v1(x.data)?,
		expected: x
			.expected
			.map(|v| convert_envoy_data_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.transpose()?,
	})
}

pub fn convert_envoy_upsert_response_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::EnvoyUpsertResponse,
) -> Result<pegboard_actor_v1::EnvoyUpsertResponse> {
	Ok(pegboard_actor_v1::EnvoyUpsertResponse {
		data: convert_envoy_data_pegboard_actor_v2_to_pegboard_actor_v1(x.data)?,
	})
}

pub fn convert_envoy_read_response_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::EnvoyReadResponse,
) -> Result<pegboard_actor_v1::EnvoyReadResponse> {
	Ok(pegboard_actor_v1::EnvoyReadResponse {
		data: x
			.data
			.map(|v| convert_envoy_data_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.transpose()?,
	})
}

pub fn convert_index_insert_envoy_request_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexInsertEnvoyRequest,
) -> Result<pegboard_actor_v1::IndexInsertEnvoyRequest> {
	Ok(pegboard_actor_v1::IndexInsertEnvoyRequest {
		create_unix_ms: x.create_unix_ms,
		envoy_key: x.envoy_key,
	})
}

pub fn convert_index_list_envoys_request_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexListEnvoysRequest,
) -> Result<pegboard_actor_v1::IndexListEnvoysRequest> {
	Ok(pegboard_actor_v1::IndexListEnvoysRequest { cursor: x.cursor })
}

pub fn convert_index_list_envoys_response_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexListEnvoysResponse,
) -> Result<pegboard_actor_v1::IndexListEnvoysResponse> {
	Ok(pegboard_actor_v1::IndexListEnvoysResponse {
		envoy_keys: x.envoy_keys,
		cursor: x.cursor,
	})
}

pub fn convert_active_envoy_index_entry_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ActiveEnvoyIndexEntry,
) -> Result<pegboard_actor_v1::ActiveEnvoyIndexEntry> {
	Ok(pegboard_actor_v1::ActiveEnvoyIndexEntry {
		worker_id: x.worker_id,
		connection_id: x.connection_id,
	})
}

pub fn convert_active_envoy_state_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::ActiveEnvoyState,
) -> Result<pegboard_actor_v1::ActiveEnvoyState> {
	Ok(pegboard_actor_v1::ActiveEnvoyState {
		envoy_key: x.envoy_key,
		connection_epoch: x.connection_epoch,
		heartbeat_unix_ms: x.heartbeat_unix_ms,
		names: x.names,
	})
}

pub fn convert_index_upsert_active_envoy_request_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexUpsertActiveEnvoyRequest,
) -> Result<pegboard_actor_v1::IndexUpsertActiveEnvoyRequest> {
	Ok(pegboard_actor_v1::IndexUpsertActiveEnvoyRequest {
		envoy_key: x.envoy_key,
		connection_epoch: x.connection_epoch,
		worker_id: x.worker_id,
		connection_id: x.connection_id,
		heartbeat_unix_ms: x.heartbeat_unix_ms,
		names: x.names,
	})
}

pub fn convert_index_upsert_active_envoy_response_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexUpsertActiveEnvoyResponse,
) -> Result<pegboard_actor_v1::IndexUpsertActiveEnvoyResponse> {
	Ok(pegboard_actor_v1::IndexUpsertActiveEnvoyResponse {
		superseded: x.superseded,
	})
}

pub fn convert_index_delete_active_envoy_request_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexDeleteActiveEnvoyRequest,
) -> Result<pegboard_actor_v1::IndexDeleteActiveEnvoyRequest> {
	Ok(pegboard_actor_v1::IndexDeleteActiveEnvoyRequest {
		envoy_key: x.envoy_key,
		connection_epoch: x.connection_epoch,
	})
}

pub fn convert_index_put_envoy_pool_request_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexPutEnvoyPoolRequest,
) -> Result<pegboard_actor_v1::IndexPutEnvoyPoolRequest> {
	Ok(pegboard_actor_v1::IndexPutEnvoyPoolRequest {
		pool_name: x.pool_name,
	})
}

pub fn convert_index_list_envoy_pools_response_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexListEnvoyPoolsResponse,
) -> Result<pegboard_actor_v1::IndexListEnvoyPoolsResponse> {
	Ok(pegboard_actor_v1::IndexListEnvoyPoolsResponse { pools: x.pools })
}

pub fn convert_index_list_active_envoys_request_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexListActiveEnvoysRequest,
) -> Result<pegboard_actor_v1::IndexListActiveEnvoysRequest> {
	Ok(pegboard_actor_v1::IndexListActiveEnvoysRequest {
		name: x.name,
		cursor: x.cursor,
	})
}

pub fn convert_index_list_active_envoys_response_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexListActiveEnvoysResponse,
) -> Result<pegboard_actor_v1::IndexListActiveEnvoysResponse> {
	Ok(pegboard_actor_v1::IndexListActiveEnvoysResponse {
		envoys: x
			.envoys
			.into_iter()
			.map(|v| convert_actor_allocation_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.collect::<Result<Vec<_>>>()?,
		cursor: x.cursor,
	})
}

pub fn convert_index_list_active_envoy_states_request_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexListActiveEnvoyStatesRequest,
) -> Result<pegboard_actor_v1::IndexListActiveEnvoyStatesRequest> {
	Ok(pegboard_actor_v1::IndexListActiveEnvoyStatesRequest { cursor: x.cursor })
}

pub fn convert_index_list_active_envoy_states_response_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::IndexListActiveEnvoyStatesResponse,
) -> Result<pegboard_actor_v1::IndexListActiveEnvoyStatesResponse> {
	Ok(pegboard_actor_v1::IndexListActiveEnvoyStatesResponse {
		envoys: x
			.envoys
			.into_iter()
			.map(|v| convert_active_envoy_state_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.collect::<Result<Vec<_>>>()?,
		cursor: x.cursor,
	})
}

pub fn convert_namespace_record_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::NamespaceRecord,
) -> Result<pegboard_actor_v1::NamespaceRecord> {
	Ok(pegboard_actor_v1::NamespaceRecord {
		create_unix_ms: x.create_unix_ms,
		create_operation_id: x.create_operation_id,
	})
}

pub fn convert_namespace_create_request_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::NamespaceCreateRequest,
) -> Result<pegboard_actor_v1::NamespaceCreateRequest> {
	Ok(pegboard_actor_v1::NamespaceCreateRequest {
		name: x.name,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_namespace_create_response_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::NamespaceCreateResponse,
) -> Result<pegboard_actor_v1::NamespaceCreateResponse> {
	Ok(pegboard_actor_v1::NamespaceCreateResponse {
		created: x.created,
		record: convert_namespace_record_pegboard_actor_v2_to_pegboard_actor_v1(x.record)?,
	})
}

pub fn convert_namespace_read_request_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::NamespaceReadRequest,
) -> Result<pegboard_actor_v1::NamespaceReadRequest> {
	Ok(pegboard_actor_v1::NamespaceReadRequest { name: x.name })
}

pub fn convert_namespace_read_response_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::NamespaceReadResponse,
) -> Result<pegboard_actor_v1::NamespaceReadResponse> {
	Ok(pegboard_actor_v1::NamespaceReadResponse {
		record: x
			.record
			.map(|v| convert_namespace_record_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.transpose()?,
	})
}

pub fn convert_namespace_entry_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::NamespaceEntry,
) -> Result<pegboard_actor_v1::NamespaceEntry> {
	Ok(pegboard_actor_v1::NamespaceEntry {
		name: x.name,
		record: convert_namespace_record_pegboard_actor_v2_to_pegboard_actor_v1(x.record)?,
	})
}

pub fn convert_namespace_list_request_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::NamespaceListRequest,
) -> Result<pegboard_actor_v1::NamespaceListRequest> {
	Ok(pegboard_actor_v1::NamespaceListRequest {
		after: x.after,
		limit: x.limit,
	})
}

pub fn convert_namespace_list_response_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::NamespaceListResponse,
) -> Result<pegboard_actor_v1::NamespaceListResponse> {
	Ok(pegboard_actor_v1::NamespaceListResponse {
		namespaces: x
			.namespaces
			.into_iter()
			.map(|v| convert_namespace_entry_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.collect::<Result<Vec<_>>>()?,
		cursor: x.cursor,
	})
}

pub fn convert_epoxy_actor_by_key_entry_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::EpoxyActorByKeyEntry,
) -> Result<pegboard_actor_v1::EpoxyActorByKeyEntry> {
	Ok(pegboard_actor_v1::EpoxyActorByKeyEntry {
		reservation_id: x.reservation_id,
	})
}

pub fn convert_node_pool_config_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::NodePoolConfig,
) -> Result<pegboard_actor_v1::NodePoolConfig> {
	Ok(pegboard_actor_v1::NodePoolConfig {
		drain_on_version_upgrade: x.drain_on_version_upgrade,
		actor_eviction_delay: x.actor_eviction_delay,
		actor_eviction_period: x.actor_eviction_period,
		actor_eviction_rate: x.actor_eviction_rate,
		metadata: x.metadata,
	})
}

pub fn convert_pool_connection_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::PoolConnection,
) -> Result<pegboard_actor_v1::PoolConnection> {
	Ok(pegboard_actor_v1::PoolConnection {
		connection_id: x.connection_id,
		heartbeat_unix_ms: x.heartbeat_unix_ms,
	})
}

pub fn convert_pool_put_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::PoolPut,
) -> Result<pegboard_actor_v1::PoolPut> {
	Ok(convert_node_pool_config_pegboard_actor_v2_to_pegboard_actor_v1(x)?)
}

pub fn convert_pool_connect_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::PoolConnect,
) -> Result<pegboard_actor_v1::PoolConnect> {
	Ok(convert_pool_connection_pegboard_actor_v2_to_pegboard_actor_v1(x)?)
}

pub fn convert_pool_renew_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::PoolRenew,
) -> Result<pegboard_actor_v1::PoolRenew> {
	Ok(convert_pool_connection_pegboard_actor_v2_to_pegboard_actor_v1(x)?)
}

pub fn convert_pool_operation_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::PoolOperation,
) -> Result<pegboard_actor_v1::PoolOperation> {
	Ok(match x {
		pegboard_actor_v2::PoolOperation::PoolRead => pegboard_actor_v1::PoolOperation::PoolRead,
		pegboard_actor_v2::PoolOperation::PoolPut(v) => pegboard_actor_v1::PoolOperation::PoolPut(
			convert_pool_put_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
		),
		pegboard_actor_v2::PoolOperation::PoolDelete => {
			pegboard_actor_v1::PoolOperation::PoolDelete
		}
		pegboard_actor_v2::PoolOperation::PoolConnect(v) => {
			pegboard_actor_v1::PoolOperation::PoolConnect(
				convert_pool_connect_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
			)
		}
		pegboard_actor_v2::PoolOperation::PoolDisconnect(v) => {
			pegboard_actor_v1::PoolOperation::PoolDisconnect(v)
		}
		pegboard_actor_v2::PoolOperation::PoolRenew(v) => {
			pegboard_actor_v1::PoolOperation::PoolRenew(
				convert_pool_renew_pegboard_actor_v2_to_pegboard_actor_v1(v)?,
			)
		}
	})
}

pub fn convert_pool_status_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::PoolStatus,
) -> Result<pegboard_actor_v1::PoolStatus> {
	Ok(match x {
		pegboard_actor_v2::PoolStatus::Ok => pegboard_actor_v1::PoolStatus::Ok,
		pegboard_actor_v2::PoolStatus::NotFound => pegboard_actor_v1::PoolStatus::NotFound,
		pegboard_actor_v2::PoolStatus::Connected => pegboard_actor_v1::PoolStatus::Connected,
	})
}

pub fn convert_pool_request_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::PoolRequest,
) -> Result<pegboard_actor_v1::PoolRequest> {
	Ok(pegboard_actor_v1::PoolRequest {
		pool_name: x.pool_name,
		operation: convert_pool_operation_pegboard_actor_v2_to_pegboard_actor_v1(x.operation)?,
	})
}

pub fn convert_pool_response_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::PoolResponse,
) -> Result<pegboard_actor_v1::PoolResponse> {
	Ok(pegboard_actor_v1::PoolResponse {
		status: convert_pool_status_pegboard_actor_v2_to_pegboard_actor_v1(x.status)?,
		config: x
			.config
			.map(|v| convert_node_pool_config_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.transpose()?,
	})
}

pub fn convert_pool_lease_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::PoolLease,
) -> Result<pegboard_actor_v1::PoolLease> {
	Ok(pegboard_actor_v1::PoolLease {
		heartbeat_unix_ms: x.heartbeat_unix_ms,
	})
}

pub fn convert_pool_entry_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::PoolEntry,
) -> Result<pegboard_actor_v1::PoolEntry> {
	Ok(pegboard_actor_v1::PoolEntry {
		name: x.name,
		config: convert_node_pool_config_pegboard_actor_v2_to_pegboard_actor_v1(x.config)?,
	})
}

pub fn convert_pool_list_request_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::PoolListRequest,
) -> Result<pegboard_actor_v1::PoolListRequest> {
	Ok(pegboard_actor_v1::PoolListRequest {
		after: x.after,
		limit: x.limit,
	})
}

pub fn convert_pool_list_response_pegboard_actor_v2_to_pegboard_actor_v1(
	x: pegboard_actor_v2::PoolListResponse,
) -> Result<pegboard_actor_v1::PoolListResponse> {
	Ok(pegboard_actor_v1::PoolListResponse {
		pools: x
			.pools
			.into_iter()
			.map(|v| convert_pool_entry_pegboard_actor_v2_to_pegboard_actor_v1(v))
			.collect::<Result<Vec<_>>>()?,
		cursor: x.cursor,
	})
}
