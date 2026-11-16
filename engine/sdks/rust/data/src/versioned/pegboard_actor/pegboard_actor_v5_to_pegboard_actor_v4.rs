//! Downgrades actor records from v5 to v4 for readers on 3.0.3 through 3.0.7, which read v4's
//! layout under the v3 header.
// from: pegboard.actor.v5.bare, to: pegboard.actor.v4.bare

use anyhow::Result;

use crate::generated::{pegboard_actor_v4, pegboard_actor_v5};

pub fn convert_actor_data_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ActorData,
) -> Result<pegboard_actor_v4::ActorData> {
	Ok(pegboard_actor_v4::ActorData {
		actor_id: x.actor_id,
		namespace: x.namespace,
		pool_name: x.pool_name,
		name: x.name,
		key: x.key,
		input: x.input,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_actor_allocation_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ActorAllocation,
) -> Result<pegboard_actor_v4::ActorAllocation> {
	Ok(pegboard_actor_v4::ActorAllocation {
		envoy_key: x.envoy_key,
		worker_id: x.worker_id,
		connection_id: x.connection_id,
	})
}

pub fn convert_reserve_epoxy_phase_already_reserved_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ReserveEpoxyPhaseAlreadyReserved,
) -> Result<pegboard_actor_v4::ReserveEpoxyPhaseAlreadyReserved> {
	Ok(pegboard_actor_v4::ReserveEpoxyPhaseAlreadyReserved {
		reservation_id: x.reservation_id,
	})
}

pub fn convert_reserve_epoxy_phase_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ReserveEpoxyPhase,
) -> Result<pegboard_actor_v4::ReserveEpoxyPhase> {
	Ok(match x {
		pegboard_actor_v5::ReserveEpoxyPhase::ReserveEpoxyPhasePending => pegboard_actor_v4::ReserveEpoxyPhase::ReserveEpoxyPhasePending,
		pegboard_actor_v5::ReserveEpoxyPhase::ReserveEpoxyPhaseReserved => pegboard_actor_v4::ReserveEpoxyPhase::ReserveEpoxyPhaseReserved,
		pegboard_actor_v5::ReserveEpoxyPhase::ReserveEpoxyPhaseAlreadyReserved(v) => pegboard_actor_v4::ReserveEpoxyPhase::ReserveEpoxyPhaseAlreadyReserved(convert_reserve_epoxy_phase_already_reserved_pegboard_actor_v5_to_pegboard_actor_v4(v)?),
		pegboard_actor_v5::ReserveEpoxyPhase::ReserveEpoxyPhaseFailed => pegboard_actor_v4::ReserveEpoxyPhase::ReserveEpoxyPhaseFailed,
		pegboard_actor_v5::ReserveEpoxyPhase::ReserveEpoxyPhaseMaxAttemptsReached => pegboard_actor_v4::ReserveEpoxyPhase::ReserveEpoxyPhaseMaxAttemptsReached,
		pegboard_actor_v5::ReserveEpoxyPhase::ReserveEpoxyPhaseInV2 => pegboard_actor_v4::ReserveEpoxyPhase::ReserveEpoxyPhaseInV2,
	})
}

pub fn convert_reserve_epoxy_state_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ReserveEpoxyState,
) -> Result<pegboard_actor_v4::ReserveEpoxyState> {
	Ok(pegboard_actor_v4::ReserveEpoxyState {
		phase: convert_reserve_epoxy_phase_pegboard_actor_v5_to_pegboard_actor_v4(x.phase)?,
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_reserve_epoxy_effect_data_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ReserveEpoxyEffectData,
) -> Result<pegboard_actor_v4::ReserveEpoxyEffectData> {
	Ok(pegboard_actor_v4::ReserveEpoxyEffectData {
		namespace: x.namespace,
		name: x.name,
		key: x.key,
		state: convert_reserve_epoxy_state_pegboard_actor_v5_to_pegboard_actor_v4(x.state)?,
	})
}

pub fn convert_pending_action_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::PendingAction,
) -> Result<pegboard_actor_v4::PendingAction> {
	Ok(match x {
		pegboard_actor_v5::PendingAction::PendingNone => {
			pegboard_actor_v4::PendingAction::PendingNone
		}
		pegboard_actor_v5::PendingAction::PendingSleep => {
			pegboard_actor_v4::PendingAction::PendingSleep
		}
		pegboard_actor_v5::PendingAction::PendingReallocate => {
			pegboard_actor_v4::PendingAction::PendingReallocate
		}
		pegboard_actor_v5::PendingAction::PendingDestroy => {
			pegboard_actor_v4::PendingAction::PendingDestroy
		}
	})
}

pub fn convert_actor_create_data_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ActorCreateData,
) -> Result<pegboard_actor_v4::ActorCreateData> {
	Ok(pegboard_actor_v4::ActorCreateData {
		actor_id: x.actor_id,
		namespace: x.namespace,
		pool_name: x.pool_name,
		name: x.name,
		key: x.key,
		input: x.input,
	})
}

pub fn convert_initializing_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::Initializing,
) -> Result<pegboard_actor_v4::Initializing> {
	Ok(pegboard_actor_v4::Initializing {
		reserve_epoxy: convert_reserve_epoxy_effect_data_pegboard_actor_v5_to_pegboard_actor_v4(
			x.reserve_epoxy,
		)?,
		allocation: x
			.allocation
			.map(|v| convert_actor_allocation_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.transpose()?,
	})
}

pub fn convert_starting_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::Starting,
) -> Result<pegboard_actor_v4::Starting> {
	Ok(pegboard_actor_v4::Starting {
		allocation: convert_actor_allocation_pegboard_actor_v5_to_pegboard_actor_v4(x.allocation)?,
	})
}

pub fn convert_running_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::Running,
) -> Result<pegboard_actor_v4::Running> {
	Ok(pegboard_actor_v4::Running {
		allocation: convert_actor_allocation_pegboard_actor_v5_to_pegboard_actor_v4(x.allocation)?,
	})
}

pub fn convert_sleeping_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::Sleeping,
) -> Result<pegboard_actor_v4::Sleeping> {
	Ok(pegboard_actor_v4::Sleeping {
		wake_retries: x.wake_retries,
	})
}

pub fn convert_stopping_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::Stopping,
) -> Result<pegboard_actor_v4::Stopping> {
	Ok(pegboard_actor_v4::Stopping {
		allocation: convert_actor_allocation_pegboard_actor_v5_to_pegboard_actor_v4(x.allocation)?,
	})
}

pub fn convert_destroy_phase_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::DestroyPhase,
) -> Result<pegboard_actor_v4::DestroyPhase> {
	Ok(match x {
		pegboard_actor_v5::DestroyPhase::DestroyPhasePending => {
			pegboard_actor_v4::DestroyPhase::DestroyPhasePending
		}
		pegboard_actor_v5::DestroyPhase::DestroyPhaseSuccess => {
			pegboard_actor_v4::DestroyPhase::DestroyPhaseSuccess
		}
		pegboard_actor_v5::DestroyPhase::DestroyPhaseFailed => {
			pegboard_actor_v4::DestroyPhase::DestroyPhaseFailed
		}
	})
}

pub fn convert_destroy_state_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::DestroyState,
) -> Result<pegboard_actor_v4::DestroyState> {
	Ok(pegboard_actor_v4::DestroyState {
		phase: convert_destroy_phase_pegboard_actor_v5_to_pegboard_actor_v4(x.phase)?,
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_destroy_effect_data_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::DestroyEffectData,
) -> Result<pegboard_actor_v4::DestroyEffectData> {
	Ok(pegboard_actor_v4::DestroyEffectData {
		create_unix_ms: x.create_unix_ms,
		state: convert_destroy_state_pegboard_actor_v5_to_pegboard_actor_v4(x.state)?,
	})
}

pub fn convert_key_owner_datacenter_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::KeyOwnerDatacenter,
) -> Result<pegboard_actor_v4::KeyOwnerDatacenter> {
	Ok(pegboard_actor_v4::KeyOwnerDatacenter {
		reservation_id: x.reservation_id,
	})
}

pub fn convert_key_owner_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::KeyOwner,
) -> Result<pegboard_actor_v4::KeyOwner> {
	Ok(match x {
		pegboard_actor_v5::KeyOwner::KeyOwnerDatacenter(v) => {
			pegboard_actor_v4::KeyOwner::KeyOwnerDatacenter(
				convert_key_owner_datacenter_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::KeyOwner::KeyOwnerV2 => pegboard_actor_v4::KeyOwner::KeyOwnerV2,
	})
}

pub fn convert_destroying_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::Destroying,
) -> Result<pegboard_actor_v4::Destroying> {
	Ok(pegboard_actor_v4::Destroying {
		destroy: convert_destroy_effect_data_pegboard_actor_v5_to_pegboard_actor_v4(x.destroy)?,
		key_owner: x
			.key_owner
			.map(|v| convert_key_owner_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.transpose()?,
	})
}

pub fn convert_migrating_from_v2_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::MigratingFromV2,
) -> Result<pegboard_actor_v4::MigratingFromV2> {
	Ok(pegboard_actor_v4::MigratingFromV2 {
		migration_id: x.migration_id,
		reserve_epoxy: x
			.reserve_epoxy
			.map(|v| convert_reserve_epoxy_effect_data_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.transpose()?,
		alarm_due: x.alarm_due,
	})
}

pub fn convert_actor_phase_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ActorPhase,
) -> Result<pegboard_actor_v4::ActorPhase> {
	Ok(match x {
		pegboard_actor_v5::ActorPhase::Initializing(v) => {
			pegboard_actor_v4::ActorPhase::Initializing(
				convert_initializing_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::ActorPhase::Starting(v) => pegboard_actor_v4::ActorPhase::Starting(
			convert_starting_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
		),
		pegboard_actor_v5::ActorPhase::Running(v) => pegboard_actor_v4::ActorPhase::Running(
			convert_running_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
		),
		pegboard_actor_v5::ActorPhase::Stopping(v) => pegboard_actor_v4::ActorPhase::Stopping(
			convert_stopping_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
		),
		pegboard_actor_v5::ActorPhase::Sleeping(v) => pegboard_actor_v4::ActorPhase::Sleeping(
			convert_sleeping_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
		),
		pegboard_actor_v5::ActorPhase::Destroying(v) => pegboard_actor_v4::ActorPhase::Destroying(
			convert_destroying_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
		),
		pegboard_actor_v5::ActorPhase::Destroyed => pegboard_actor_v4::ActorPhase::Destroyed,
		pegboard_actor_v5::ActorPhase::MigratingFromV2(v) => {
			pegboard_actor_v4::ActorPhase::MigratingFromV2(
				convert_migrating_from_v2_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::ActorPhase::DelegatedToV2 => {
			pegboard_actor_v4::ActorPhase::DelegatedToV2
		}
	})
}

pub fn convert_start_waiter_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::StartWaiter,
) -> Result<pegboard_actor_v4::StartWaiter> {
	Ok(pegboard_actor_v4::StartWaiter {
		worker_id: x.worker_id,
		wait_id: x.wait_id,
		expires_unix_ms: x.expires_unix_ms,
	})
}

pub fn convert_write_index_phase_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::WriteIndexPhase,
) -> Result<pegboard_actor_v4::WriteIndexPhase> {
	Ok(match x {
		pegboard_actor_v5::WriteIndexPhase::WriteIndexPhasePending => {
			pegboard_actor_v4::WriteIndexPhase::WriteIndexPhasePending
		}
		pegboard_actor_v5::WriteIndexPhase::WriteIndexPhaseSuccess => {
			pegboard_actor_v4::WriteIndexPhase::WriteIndexPhaseSuccess
		}
		pegboard_actor_v5::WriteIndexPhase::WriteIndexPhaseFailed => {
			pegboard_actor_v4::WriteIndexPhase::WriteIndexPhaseFailed
		}
	})
}

pub fn convert_write_index_state_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::WriteIndexState,
) -> Result<pegboard_actor_v4::WriteIndexState> {
	Ok(pegboard_actor_v4::WriteIndexState {
		phase: convert_write_index_phase_pegboard_actor_v5_to_pegboard_actor_v4(x.phase)?,
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_write_index_effect_data_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::WriteIndexEffectData,
) -> Result<pegboard_actor_v4::WriteIndexEffectData> {
	Ok(pegboard_actor_v4::WriteIndexEffectData {
		namespace: x.namespace,
		name: x.name,
		create_unix_ms: x.create_unix_ms,
		state: convert_write_index_state_pegboard_actor_v5_to_pegboard_actor_v4(x.state)?,
	})
}

pub fn convert_storage_origin_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::StorageOrigin,
) -> Result<pegboard_actor_v4::StorageOrigin> {
	Ok(match x {
		pegboard_actor_v5::StorageOrigin::Created => pegboard_actor_v4::StorageOrigin::Created,
		pegboard_actor_v5::StorageOrigin::Imported => pegboard_actor_v4::StorageOrigin::Imported,
	})
}

pub fn convert_actor_state_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ActorState,
) -> Result<pegboard_actor_v4::ActorState> {
	Ok(pegboard_actor_v4::ActorState {
		phase: convert_actor_phase_pegboard_actor_v5_to_pegboard_actor_v4(x.phase)?,
		epoch: x.epoch,
		create_unix_ms: x.create_unix_ms,
		storage_origin: convert_storage_origin_pegboard_actor_v5_to_pegboard_actor_v4(
			x.storage_origin,
		)?,
		command_index: x.command_index,
		index_bucket_count: x.index_bucket_count,
		pending_action: convert_pending_action_pegboard_actor_v5_to_pegboard_actor_v4(
			x.pending_action,
		)?,
		write_index: convert_write_index_effect_data_pegboard_actor_v5_to_pegboard_actor_v4(
			x.write_index,
		)?,
		start_waiters: x
			.start_waiters
			.into_iter()
			.map(|v| convert_start_waiter_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_reserve_epoxy_timeout_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ReserveEpoxyTimeout,
) -> Result<pegboard_actor_v4::ReserveEpoxyTimeout> {
	Ok(pegboard_actor_v4::ReserveEpoxyTimeout {
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_write_index_timeout_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::WriteIndexTimeout,
) -> Result<pegboard_actor_v4::WriteIndexTimeout> {
	Ok(pegboard_actor_v4::WriteIndexTimeout {
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_destroy_timeout_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::DestroyTimeout,
) -> Result<pegboard_actor_v4::DestroyTimeout> {
	Ok(pegboard_actor_v4::DestroyTimeout {
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_start_timeout_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::StartTimeout,
) -> Result<pegboard_actor_v4::StartTimeout> {
	Ok(pegboard_actor_v4::StartTimeout { epoch: x.epoch })
}

pub fn convert_stop_timeout_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::StopTimeout,
) -> Result<pegboard_actor_v4::StopTimeout> {
	Ok(pegboard_actor_v4::StopTimeout { epoch: x.epoch })
}

pub fn convert_envoy_check_timeout_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::EnvoyCheckTimeout,
) -> Result<pegboard_actor_v4::EnvoyCheckTimeout> {
	Ok(pegboard_actor_v4::EnvoyCheckTimeout { epoch: x.epoch })
}

pub fn convert_stop_threshold_timeout_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::StopThresholdTimeout,
) -> Result<pegboard_actor_v4::StopThresholdTimeout> {
	Ok(pegboard_actor_v4::StopThresholdTimeout {
		epoch: x.epoch,
		stop_unix_ms: x.stop_unix_ms,
	})
}

pub fn convert_allocation_retry_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::AllocationRetry,
) -> Result<pegboard_actor_v4::AllocationRetry> {
	Ok(pegboard_actor_v4::AllocationRetry {
		epoch: x.epoch,
		attempt: x.attempt,
	})
}

pub fn convert_actor_timer_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ActorTimer,
) -> Result<pegboard_actor_v4::ActorTimer> {
	Ok(match x {
		pegboard_actor_v5::ActorTimer::ReserveEpoxyTimeout(v) => {
			pegboard_actor_v4::ActorTimer::ReserveEpoxyTimeout(
				convert_reserve_epoxy_timeout_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::ActorTimer::WriteIndexTimeout(v) => {
			pegboard_actor_v4::ActorTimer::WriteIndexTimeout(
				convert_write_index_timeout_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::ActorTimer::DestroyTimeout(v) => {
			pegboard_actor_v4::ActorTimer::DestroyTimeout(
				convert_destroy_timeout_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::ActorTimer::StartTimeout(v) => {
			pegboard_actor_v4::ActorTimer::StartTimeout(
				convert_start_timeout_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::ActorTimer::AlarmTimeout => pegboard_actor_v4::ActorTimer::AlarmTimeout,
		pegboard_actor_v5::ActorTimer::StopTimeout(v) => {
			pegboard_actor_v4::ActorTimer::StopTimeout(
				convert_stop_timeout_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::ActorTimer::EnvoyCheckTimeout(v) => {
			pegboard_actor_v4::ActorTimer::EnvoyCheckTimeout(
				convert_envoy_check_timeout_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::ActorTimer::StopThresholdTimeout(v) => {
			pegboard_actor_v4::ActorTimer::StopThresholdTimeout(
				convert_stop_threshold_timeout_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::ActorTimer::AllocationRetry(v) => {
			pegboard_actor_v4::ActorTimer::AllocationRetry(
				convert_allocation_retry_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
	})
}

pub fn convert_start_failed_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::StartFailed,
) -> Result<pegboard_actor_v4::StartFailed> {
	Ok(pegboard_actor_v4::StartFailed { epoch: x.epoch })
}

pub fn convert_envoy_gone_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::EnvoyGone,
) -> Result<pegboard_actor_v4::EnvoyGone> {
	Ok(pegboard_actor_v4::EnvoyGone { epoch: x.epoch })
}

pub fn convert_try_start_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::TryStart,
) -> Result<pegboard_actor_v4::TryStart> {
	Ok(pegboard_actor_v4::TryStart {
		waiter: x
			.waiter
			.map(|v| convert_start_waiter_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.transpose()?,
	})
}

pub fn convert_create_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::Create,
) -> Result<pegboard_actor_v4::Create> {
	Ok(pegboard_actor_v4::Create {
		init: convert_actor_create_data_pegboard_actor_v5_to_pegboard_actor_v4(x.init)?,
		waiter: x
			.waiter
			.map(|v| convert_start_waiter_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.transpose()?,
		force_actor_v3: x.force_actor_v3,
	})
}

pub fn convert_reallocate_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::Reallocate,
) -> Result<pegboard_actor_v4::Reallocate> {
	Ok(pegboard_actor_v4::Reallocate {
		epoch: x.epoch,
		lost: x.lost,
		waiter: x
			.waiter
			.map(|v| convert_start_waiter_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.transpose()?,
	})
}

pub fn convert_update_route_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::UpdateRoute,
) -> Result<pegboard_actor_v4::UpdateRoute> {
	Ok(pegboard_actor_v4::UpdateRoute {
		epoch: x.epoch,
		from: convert_actor_allocation_pegboard_actor_v5_to_pegboard_actor_v4(x.from)?,
		to: convert_actor_allocation_pegboard_actor_v5_to_pegboard_actor_v4(x.to)?,
	})
}

pub fn convert_to_envoy_event_kind_stopped_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ToEnvoyEventKindStopped,
) -> Result<pegboard_actor_v4::ToEnvoyEventKindStopped> {
	Ok(pegboard_actor_v4::ToEnvoyEventKindStopped {
		ok: x.ok,
		message: x.message,
	})
}

pub fn convert_to_envoy_event_kind_set_alarm_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ToEnvoyEventKindSetAlarm,
) -> Result<pegboard_actor_v4::ToEnvoyEventKindSetAlarm> {
	Ok(pegboard_actor_v4::ToEnvoyEventKindSetAlarm {
		alarm_unix_ms: x.alarm_unix_ms,
	})
}

pub fn convert_to_envoy_event_kind_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ToEnvoyEventKind,
) -> Result<pegboard_actor_v4::ToEnvoyEventKind> {
	Ok(match x {
		pegboard_actor_v5::ToEnvoyEventKind::ToEnvoyEventKindRunning => {
			pegboard_actor_v4::ToEnvoyEventKind::ToEnvoyEventKindRunning
		}
		pegboard_actor_v5::ToEnvoyEventKind::ToEnvoyEventKindStopped(v) => {
			pegboard_actor_v4::ToEnvoyEventKind::ToEnvoyEventKindStopped(
				convert_to_envoy_event_kind_stopped_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::ToEnvoyEventKind::ToEnvoyEventKindSleepIntent => {
			pegboard_actor_v4::ToEnvoyEventKind::ToEnvoyEventKindSleepIntent
		}
		pegboard_actor_v5::ToEnvoyEventKind::ToEnvoyEventKindStopIntent => {
			pegboard_actor_v4::ToEnvoyEventKind::ToEnvoyEventKindStopIntent
		}
		pegboard_actor_v5::ToEnvoyEventKind::ToEnvoyEventKindSetAlarm(v) => {
			pegboard_actor_v4::ToEnvoyEventKind::ToEnvoyEventKindSetAlarm(
				convert_to_envoy_event_kind_set_alarm_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
	})
}

pub fn convert_to_envoy_event_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ToEnvoyEvent,
) -> Result<pegboard_actor_v4::ToEnvoyEvent> {
	Ok(pegboard_actor_v4::ToEnvoyEvent {
		epoch: x.epoch,
		kind: convert_to_envoy_event_kind_pegboard_actor_v5_to_pegboard_actor_v4(x.kind)?,
	})
}

pub fn convert_migrate_from_v2_begin_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::MigrateFromV2Begin,
) -> Result<pegboard_actor_v4::MigrateFromV2Begin> {
	Ok(pegboard_actor_v4::MigrateFromV2Begin {
		migration_id: x.migration_id,
		init: convert_actor_create_data_pegboard_actor_v5_to_pegboard_actor_v4(x.init)?,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_migrate_from_v2_activate_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::MigrateFromV2Activate,
) -> Result<pegboard_actor_v4::MigrateFromV2Activate> {
	Ok(pegboard_actor_v4::MigrateFromV2Activate {
		migration_id: x.migration_id,
		alarm_unix_ms: x.alarm_unix_ms,
	})
}

pub fn convert_migrate_from_v2_discard_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::MigrateFromV2Discard,
) -> Result<pegboard_actor_v4::MigrateFromV2Discard> {
	Ok(pegboard_actor_v4::MigrateFromV2Discard {
		migration_id: x.migration_id,
	})
}

pub fn convert_migrate_from_v2_recall_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::MigrateFromV2Recall,
) -> Result<pegboard_actor_v4::MigrateFromV2Recall> {
	Ok(pegboard_actor_v4::MigrateFromV2Recall {
		migration_id: x.migration_id,
	})
}

pub fn convert_migrate_from_v2_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::MigrateFromV2,
) -> Result<pegboard_actor_v4::MigrateFromV2> {
	Ok(match x {
		pegboard_actor_v5::MigrateFromV2::MigrateFromV2Begin(v) => {
			pegboard_actor_v4::MigrateFromV2::MigrateFromV2Begin(
				convert_migrate_from_v2_begin_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::MigrateFromV2::MigrateFromV2Activate(v) => {
			pegboard_actor_v4::MigrateFromV2::MigrateFromV2Activate(
				convert_migrate_from_v2_activate_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::MigrateFromV2::MigrateFromV2Discard(v) => {
			pegboard_actor_v4::MigrateFromV2::MigrateFromV2Discard(
				convert_migrate_from_v2_discard_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::MigrateFromV2::MigrateFromV2Recall(v) => {
			pegboard_actor_v4::MigrateFromV2::MigrateFromV2Recall(
				convert_migrate_from_v2_recall_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
	})
}

pub fn convert_actor_event_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ActorEvent,
) -> Result<pegboard_actor_v4::ActorEvent> {
	Ok(match x {
		pegboard_actor_v5::ActorEvent::ReserveEpoxyState(v) => {
			pegboard_actor_v4::ActorEvent::ReserveEpoxyState(
				convert_reserve_epoxy_state_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::ActorEvent::WriteIndexState(v) => {
			pegboard_actor_v4::ActorEvent::WriteIndexState(
				convert_write_index_state_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::ActorEvent::StartFailed(v) => {
			pegboard_actor_v4::ActorEvent::StartFailed(
				convert_start_failed_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::ActorEvent::Sleep => pegboard_actor_v4::ActorEvent::Sleep,
		pegboard_actor_v5::ActorEvent::TryStart(v) => pegboard_actor_v4::ActorEvent::TryStart(
			convert_try_start_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
		),
		pegboard_actor_v5::ActorEvent::Create(v) => pegboard_actor_v4::ActorEvent::Create(
			convert_create_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
		),
		pegboard_actor_v5::ActorEvent::Reallocate(v) => pegboard_actor_v4::ActorEvent::Reallocate(
			convert_reallocate_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
		),
		pegboard_actor_v5::ActorEvent::UpdateRoute(v) => {
			pegboard_actor_v4::ActorEvent::UpdateRoute(
				convert_update_route_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::ActorEvent::Destroy => pegboard_actor_v4::ActorEvent::Destroy,
		pegboard_actor_v5::ActorEvent::DestroyState(v) => {
			pegboard_actor_v4::ActorEvent::DestroyState(
				convert_destroy_state_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::ActorEvent::ToEnvoyEvent(v) => {
			pegboard_actor_v4::ActorEvent::ToEnvoyEvent(
				convert_to_envoy_event_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::ActorEvent::EnvoyGone(v) => pegboard_actor_v4::ActorEvent::EnvoyGone(
			convert_envoy_gone_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
		),
		pegboard_actor_v5::ActorEvent::MigrateFromV2(v) => {
			pegboard_actor_v4::ActorEvent::MigrateFromV2(
				convert_migrate_from_v2_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
	})
}

pub fn convert_try_start_starting_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::TryStartStarting,
) -> Result<pegboard_actor_v4::TryStartStarting> {
	Ok(pegboard_actor_v4::TryStartStarting {
		epoch: x.epoch,
		allocation: convert_actor_allocation_pegboard_actor_v5_to_pegboard_actor_v4(x.allocation)?,
	})
}

pub fn convert_try_start_running_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::TryStartRunning,
) -> Result<pegboard_actor_v4::TryStartRunning> {
	Ok(pegboard_actor_v4::TryStartRunning {
		epoch: x.epoch,
		allocation: convert_actor_allocation_pegboard_actor_v5_to_pegboard_actor_v4(x.allocation)?,
	})
}

pub fn convert_try_start_epoxy_already_reserved_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::TryStartEpoxyAlreadyReserved,
) -> Result<pegboard_actor_v4::TryStartEpoxyAlreadyReserved> {
	Ok(pegboard_actor_v4::TryStartEpoxyAlreadyReserved {
		reservation_id: x.reservation_id,
	})
}

pub fn convert_try_start_outcome_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::TryStartOutcome,
) -> Result<pegboard_actor_v4::TryStartOutcome> {
	Ok(match x {
		pegboard_actor_v5::TryStartOutcome::TryStartWaiting => {
			pegboard_actor_v4::TryStartOutcome::TryStartWaiting
		}
		pegboard_actor_v5::TryStartOutcome::TryStartStarting(v) => {
			pegboard_actor_v4::TryStartOutcome::TryStartStarting(
				convert_try_start_starting_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::TryStartOutcome::TryStartRunning(v) => {
			pegboard_actor_v4::TryStartOutcome::TryStartRunning(
				convert_try_start_running_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::TryStartOutcome::TryStartDestroyed => {
			pegboard_actor_v4::TryStartOutcome::TryStartDestroyed
		}
		pegboard_actor_v5::TryStartOutcome::TryStartEpoxyAlreadyReserved(v) => {
			pegboard_actor_v4::TryStartOutcome::TryStartEpoxyAlreadyReserved(
				convert_try_start_epoxy_already_reserved_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::TryStartOutcome::TryStartFull => {
			pegboard_actor_v4::TryStartOutcome::TryStartFull
		}
		pegboard_actor_v5::TryStartOutcome::TryStartMigratingFromV2 => {
			pegboard_actor_v4::TryStartOutcome::TryStartMigratingFromV2
		}
		pegboard_actor_v5::TryStartOutcome::TryStartDelegatedToV2 => {
			pegboard_actor_v4::TryStartOutcome::TryStartDelegatedToV2
		}
	})
}

pub fn convert_actor_read_response_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ActorReadResponse,
) -> Result<pegboard_actor_v4::ActorReadResponse> {
	Ok(pegboard_actor_v4::ActorReadResponse {
		data: x
			.data
			.map(|v| convert_actor_data_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.transpose()?,
		phase: x
			.phase
			.map(|v| convert_actor_phase_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.transpose()?,
		epoch: x.epoch,
	})
}

pub fn convert_actor_event_response_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ActorEventResponse,
) -> Result<pegboard_actor_v4::ActorEventResponse> {
	Ok(pegboard_actor_v4::ActorEventResponse {
		outcome: x
			.outcome
			.map(|v| convert_try_start_outcome_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.transpose()?,
	})
}

pub fn convert_actor_migrate_from_v2_begin_response_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ActorMigrateFromV2BeginResponse,
) -> Result<pegboard_actor_v4::ActorMigrateFromV2BeginResponse> {
	Ok(pegboard_actor_v4::ActorMigrateFromV2BeginResponse {
		phase: convert_actor_phase_pegboard_actor_v5_to_pegboard_actor_v4(x.phase)?,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_actor_create_request_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ActorCreateRequest,
) -> Result<pegboard_actor_v4::ActorCreateRequest> {
	Ok(pegboard_actor_v4::ActorCreateRequest {
		init: convert_actor_create_data_pegboard_actor_v5_to_pegboard_actor_v4(x.init)?,
		waiter: x
			.waiter
			.map(|v| convert_start_waiter_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.transpose()?,
		force_actor_v3: x.force_actor_v3,
	})
}

pub fn convert_actor_create_response_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ActorCreateResponse,
) -> Result<pegboard_actor_v4::ActorCreateResponse> {
	Ok(pegboard_actor_v4::ActorCreateResponse {
		outcome: convert_try_start_outcome_pegboard_actor_v5_to_pegboard_actor_v4(x.outcome)?,
	})
}

pub fn convert_actor_start_notice_starting_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ActorStartNoticeStarting,
) -> Result<pegboard_actor_v4::ActorStartNoticeStarting> {
	Ok(pegboard_actor_v4::ActorStartNoticeStarting {
		epoch: x.epoch,
		allocation: convert_actor_allocation_pegboard_actor_v5_to_pegboard_actor_v4(x.allocation)?,
	})
}

pub fn convert_actor_start_notice_running_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ActorStartNoticeRunning,
) -> Result<pegboard_actor_v4::ActorStartNoticeRunning> {
	Ok(pegboard_actor_v4::ActorStartNoticeRunning {
		epoch: x.epoch,
		allocation: convert_actor_allocation_pegboard_actor_v5_to_pegboard_actor_v4(x.allocation)?,
	})
}

pub fn convert_actor_start_notice_epoxy_already_reserved_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ActorStartNoticeEpoxyAlreadyReserved,
) -> Result<pegboard_actor_v4::ActorStartNoticeEpoxyAlreadyReserved> {
	Ok(pegboard_actor_v4::ActorStartNoticeEpoxyAlreadyReserved {
		reservation_id: x.reservation_id,
	})
}

pub fn convert_actor_start_notice_kind_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ActorStartNoticeKind,
) -> Result<pegboard_actor_v4::ActorStartNoticeKind> {
	Ok(match x {
		pegboard_actor_v5::ActorStartNoticeKind::ActorStartNoticeStarting(v) => pegboard_actor_v4::ActorStartNoticeKind::ActorStartNoticeStarting(convert_actor_start_notice_starting_pegboard_actor_v5_to_pegboard_actor_v4(v)?),
		pegboard_actor_v5::ActorStartNoticeKind::ActorStartNoticeRunning(v) => pegboard_actor_v4::ActorStartNoticeKind::ActorStartNoticeRunning(convert_actor_start_notice_running_pegboard_actor_v5_to_pegboard_actor_v4(v)?),
		pegboard_actor_v5::ActorStartNoticeKind::ActorStartNoticeEpoxyAlreadyReserved(v) => pegboard_actor_v4::ActorStartNoticeKind::ActorStartNoticeEpoxyAlreadyReserved(convert_actor_start_notice_epoxy_already_reserved_pegboard_actor_v5_to_pegboard_actor_v4(v)?),
		pegboard_actor_v5::ActorStartNoticeKind::ActorStartNoticeDestroyed => pegboard_actor_v4::ActorStartNoticeKind::ActorStartNoticeDestroyed,
		pegboard_actor_v5::ActorStartNoticeKind::ActorStartNoticeMigratingFromV2 => pegboard_actor_v4::ActorStartNoticeKind::ActorStartNoticeMigratingFromV2,
		pegboard_actor_v5::ActorStartNoticeKind::ActorStartNoticeDelegatedToV2 => pegboard_actor_v4::ActorStartNoticeKind::ActorStartNoticeDelegatedToV2,
	})
}

pub fn convert_actor_start_notice_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ActorStartNotice,
) -> Result<pegboard_actor_v4::ActorStartNotice> {
	Ok(pegboard_actor_v4::ActorStartNotice {
		actor_id: x.actor_id,
		kind: convert_actor_start_notice_kind_pegboard_actor_v5_to_pegboard_actor_v4(x.kind)?,
	})
}

pub fn convert_kv_value_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::KvValue,
) -> Result<pegboard_actor_v4::KvValue> {
	Ok(pegboard_actor_v4::KvValue {
		value: x.value,
		version: x.version,
		updated_unix_ms: x.updated_unix_ms,
	})
}

pub fn convert_index_insert_actor_request_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexInsertActorRequest,
) -> Result<pegboard_actor_v4::IndexInsertActorRequest> {
	Ok(pegboard_actor_v4::IndexInsertActorRequest {
		actor_id: x.actor_id,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_index_delete_actor_request_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexDeleteActorRequest,
) -> Result<pegboard_actor_v4::IndexDeleteActorRequest> {
	Ok(pegboard_actor_v4::IndexDeleteActorRequest {
		actor_id: x.actor_id,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_index_actor_cursor_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexActorCursor,
) -> Result<pegboard_actor_v4::IndexActorCursor> {
	Ok(pegboard_actor_v4::IndexActorCursor {
		create_unix_ms: x.create_unix_ms,
		actor_id: x.actor_id,
	})
}

pub fn convert_index_list_actors_request_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexListActorsRequest,
) -> Result<pegboard_actor_v4::IndexListActorsRequest> {
	Ok(pegboard_actor_v4::IndexListActorsRequest {
		after: x
			.after
			.map(|v| convert_index_actor_cursor_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.transpose()?,
		limit: x.limit,
		include_destroyed: x.include_destroyed,
	})
}

pub fn convert_index_destroyed_actor_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexDestroyedActor,
) -> Result<pegboard_actor_v4::IndexDestroyedActor> {
	Ok(pegboard_actor_v4::IndexDestroyedActor {
		create_unix_ms: x.create_unix_ms,
		actor_id: x.actor_id,
		destroy_unix_ms: x.destroy_unix_ms,
	})
}

pub fn convert_index_list_actors_response_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexListActorsResponse,
) -> Result<pegboard_actor_v4::IndexListActorsResponse> {
	Ok(pegboard_actor_v4::IndexListActorsResponse {
		actors: x
			.actors
			.into_iter()
			.map(|v| convert_index_actor_cursor_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.collect::<Result<Vec<_>>>()?,
		has_more: x.has_more,
		destroyed: x
			.destroyed
			.into_iter()
			.map(|v| convert_index_destroyed_actor_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_destroyed_actor_index_entry_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::DestroyedActorIndexEntry,
) -> Result<pegboard_actor_v4::DestroyedActorIndexEntry> {
	Ok(pegboard_actor_v4::DestroyedActorIndexEntry {
		destroy_unix_ms: x.destroy_unix_ms,
	})
}

pub fn convert_index_read_destroyed_actor_request_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexReadDestroyedActorRequest,
) -> Result<pegboard_actor_v4::IndexReadDestroyedActorRequest> {
	Ok(pegboard_actor_v4::IndexReadDestroyedActorRequest {
		create_unix_ms: x.create_unix_ms,
		actor_id: x.actor_id,
	})
}

pub fn convert_index_read_destroyed_actor_response_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexReadDestroyedActorResponse,
) -> Result<pegboard_actor_v4::IndexReadDestroyedActorResponse> {
	Ok(pegboard_actor_v4::IndexReadDestroyedActorResponse {
		destroy_unix_ms: x.destroy_unix_ms,
	})
}

pub fn convert_envoy_data_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::EnvoyData,
) -> Result<pegboard_actor_v4::EnvoyData> {
	Ok(pegboard_actor_v4::EnvoyData {
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

pub fn convert_envoy_upsert_request_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::EnvoyUpsertRequest,
) -> Result<pegboard_actor_v4::EnvoyUpsertRequest> {
	Ok(pegboard_actor_v4::EnvoyUpsertRequest {
		data: convert_envoy_data_pegboard_actor_v5_to_pegboard_actor_v4(x.data)?,
		expected: x
			.expected
			.map(|v| convert_envoy_data_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.transpose()?,
	})
}

pub fn convert_envoy_upsert_response_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::EnvoyUpsertResponse,
) -> Result<pegboard_actor_v4::EnvoyUpsertResponse> {
	Ok(pegboard_actor_v4::EnvoyUpsertResponse {
		data: convert_envoy_data_pegboard_actor_v5_to_pegboard_actor_v4(x.data)?,
	})
}

pub fn convert_envoy_read_response_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::EnvoyReadResponse,
) -> Result<pegboard_actor_v4::EnvoyReadResponse> {
	Ok(pegboard_actor_v4::EnvoyReadResponse {
		data: x
			.data
			.map(|v| convert_envoy_data_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.transpose()?,
	})
}

pub fn convert_index_insert_envoy_request_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexInsertEnvoyRequest,
) -> Result<pegboard_actor_v4::IndexInsertEnvoyRequest> {
	Ok(pegboard_actor_v4::IndexInsertEnvoyRequest {
		create_unix_ms: x.create_unix_ms,
		envoy_key: x.envoy_key,
	})
}

pub fn convert_index_list_envoys_request_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexListEnvoysRequest,
) -> Result<pegboard_actor_v4::IndexListEnvoysRequest> {
	Ok(pegboard_actor_v4::IndexListEnvoysRequest { cursor: x.cursor })
}

pub fn convert_index_list_envoys_response_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexListEnvoysResponse,
) -> Result<pegboard_actor_v4::IndexListEnvoysResponse> {
	Ok(pegboard_actor_v4::IndexListEnvoysResponse {
		envoy_keys: x.envoy_keys,
		cursor: x.cursor,
	})
}

pub fn convert_active_envoy_index_entry_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ActiveEnvoyIndexEntry,
) -> Result<pegboard_actor_v4::ActiveEnvoyIndexEntry> {
	Ok(pegboard_actor_v4::ActiveEnvoyIndexEntry {
		worker_id: x.worker_id,
		connection_id: x.connection_id,
	})
}

pub fn convert_active_envoy_state_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::ActiveEnvoyState,
) -> Result<pegboard_actor_v4::ActiveEnvoyState> {
	Ok(pegboard_actor_v4::ActiveEnvoyState {
		envoy_key: x.envoy_key,
		connection_epoch: x.connection_epoch,
		version: x.version,
		heartbeat_unix_ms: x.heartbeat_unix_ms,
		names: x.names,
	})
}

pub fn convert_index_upsert_active_envoy_request_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexUpsertActiveEnvoyRequest,
) -> Result<pegboard_actor_v4::IndexUpsertActiveEnvoyRequest> {
	Ok(pegboard_actor_v4::IndexUpsertActiveEnvoyRequest {
		envoy_key: x.envoy_key,
		connection_epoch: x.connection_epoch,
		worker_id: x.worker_id,
		connection_id: x.connection_id,
		version: x.version,
		heartbeat_unix_ms: x.heartbeat_unix_ms,
		names: x.names,
	})
}

pub fn convert_index_upsert_active_envoy_response_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexUpsertActiveEnvoyResponse,
) -> Result<pegboard_actor_v4::IndexUpsertActiveEnvoyResponse> {
	Ok(pegboard_actor_v4::IndexUpsertActiveEnvoyResponse {
		superseded: x.superseded,
		newest_live_version: x.newest_live_version,
	})
}

pub fn convert_index_delete_active_envoy_request_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexDeleteActiveEnvoyRequest,
) -> Result<pegboard_actor_v4::IndexDeleteActiveEnvoyRequest> {
	Ok(pegboard_actor_v4::IndexDeleteActiveEnvoyRequest {
		envoy_key: x.envoy_key,
		connection_epoch: x.connection_epoch,
	})
}

pub fn convert_index_put_envoy_pool_request_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexPutEnvoyPoolRequest,
) -> Result<pegboard_actor_v4::IndexPutEnvoyPoolRequest> {
	Ok(pegboard_actor_v4::IndexPutEnvoyPoolRequest {
		pool_name: x.pool_name,
	})
}

pub fn convert_index_list_envoy_pools_response_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexListEnvoyPoolsResponse,
) -> Result<pegboard_actor_v4::IndexListEnvoyPoolsResponse> {
	Ok(pegboard_actor_v4::IndexListEnvoyPoolsResponse { pools: x.pools })
}

pub fn convert_index_list_active_envoys_request_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexListActiveEnvoysRequest,
) -> Result<pegboard_actor_v4::IndexListActiveEnvoysRequest> {
	Ok(pegboard_actor_v4::IndexListActiveEnvoysRequest {
		name: x.name,
		cursor: x.cursor,
	})
}

pub fn convert_index_list_active_envoys_response_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexListActiveEnvoysResponse,
) -> Result<pegboard_actor_v4::IndexListActiveEnvoysResponse> {
	Ok(pegboard_actor_v4::IndexListActiveEnvoysResponse {
		envoys: x
			.envoys
			.into_iter()
			.map(|v| convert_actor_allocation_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.collect::<Result<Vec<_>>>()?,
		cursor: x.cursor,
	})
}

pub fn convert_index_list_active_envoy_states_request_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexListActiveEnvoyStatesRequest,
) -> Result<pegboard_actor_v4::IndexListActiveEnvoyStatesRequest> {
	Ok(pegboard_actor_v4::IndexListActiveEnvoyStatesRequest { cursor: x.cursor })
}

pub fn convert_index_list_active_envoy_states_response_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::IndexListActiveEnvoyStatesResponse,
) -> Result<pegboard_actor_v4::IndexListActiveEnvoyStatesResponse> {
	Ok(pegboard_actor_v4::IndexListActiveEnvoyStatesResponse {
		envoys: x
			.envoys
			.into_iter()
			.map(|v| convert_active_envoy_state_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.collect::<Result<Vec<_>>>()?,
		cursor: x.cursor,
	})
}

pub fn convert_namespace_record_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::NamespaceRecord,
) -> Result<pegboard_actor_v4::NamespaceRecord> {
	Ok(pegboard_actor_v4::NamespaceRecord {
		create_unix_ms: x.create_unix_ms,
		create_operation_id: x.create_operation_id,
	})
}

pub fn convert_namespace_create_request_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::NamespaceCreateRequest,
) -> Result<pegboard_actor_v4::NamespaceCreateRequest> {
	Ok(pegboard_actor_v4::NamespaceCreateRequest {
		name: x.name,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_namespace_create_response_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::NamespaceCreateResponse,
) -> Result<pegboard_actor_v4::NamespaceCreateResponse> {
	Ok(pegboard_actor_v4::NamespaceCreateResponse {
		created: x.created,
		record: convert_namespace_record_pegboard_actor_v5_to_pegboard_actor_v4(x.record)?,
	})
}

pub fn convert_namespace_read_request_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::NamespaceReadRequest,
) -> Result<pegboard_actor_v4::NamespaceReadRequest> {
	Ok(pegboard_actor_v4::NamespaceReadRequest { name: x.name })
}

pub fn convert_namespace_read_response_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::NamespaceReadResponse,
) -> Result<pegboard_actor_v4::NamespaceReadResponse> {
	Ok(pegboard_actor_v4::NamespaceReadResponse {
		record: x
			.record
			.map(|v| convert_namespace_record_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.transpose()?,
	})
}

pub fn convert_namespace_entry_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::NamespaceEntry,
) -> Result<pegboard_actor_v4::NamespaceEntry> {
	Ok(pegboard_actor_v4::NamespaceEntry {
		name: x.name,
		record: convert_namespace_record_pegboard_actor_v5_to_pegboard_actor_v4(x.record)?,
	})
}

pub fn convert_namespace_list_request_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::NamespaceListRequest,
) -> Result<pegboard_actor_v4::NamespaceListRequest> {
	Ok(pegboard_actor_v4::NamespaceListRequest {
		after: x.after,
		limit: x.limit,
	})
}

pub fn convert_namespace_list_response_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::NamespaceListResponse,
) -> Result<pegboard_actor_v4::NamespaceListResponse> {
	Ok(pegboard_actor_v4::NamespaceListResponse {
		namespaces: x
			.namespaces
			.into_iter()
			.map(|v| convert_namespace_entry_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.collect::<Result<Vec<_>>>()?,
		cursor: x.cursor,
	})
}

pub fn convert_epoxy_actor_by_key_entry_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::EpoxyActorByKeyEntry,
) -> Result<pegboard_actor_v4::EpoxyActorByKeyEntry> {
	Ok(pegboard_actor_v4::EpoxyActorByKeyEntry {
		reservation_id: x.reservation_id,
	})
}

pub fn convert_node_pool_config_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::NodePoolConfig,
) -> Result<pegboard_actor_v4::NodePoolConfig> {
	Ok(pegboard_actor_v4::NodePoolConfig {
		drain_on_version_upgrade: x.drain_on_version_upgrade,
		actor_eviction_delay: x.actor_eviction_delay,
		actor_eviction_period: x.actor_eviction_period,
		actor_eviction_rate: x.actor_eviction_rate,
		metadata: x.metadata,
	})
}

pub fn convert_pool_connection_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::PoolConnection,
) -> Result<pegboard_actor_v4::PoolConnection> {
	Ok(pegboard_actor_v4::PoolConnection {
		connection_id: x.connection_id,
		heartbeat_unix_ms: x.heartbeat_unix_ms,
	})
}

pub fn convert_pool_put_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::PoolPut,
) -> Result<pegboard_actor_v4::PoolPut> {
	Ok(convert_node_pool_config_pegboard_actor_v5_to_pegboard_actor_v4(x)?)
}

pub fn convert_pool_connect_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::PoolConnect,
) -> Result<pegboard_actor_v4::PoolConnect> {
	Ok(convert_pool_connection_pegboard_actor_v5_to_pegboard_actor_v4(x)?)
}

pub fn convert_pool_renew_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::PoolRenew,
) -> Result<pegboard_actor_v4::PoolRenew> {
	Ok(convert_pool_connection_pegboard_actor_v5_to_pegboard_actor_v4(x)?)
}

pub fn convert_pool_operation_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::PoolOperation,
) -> Result<pegboard_actor_v4::PoolOperation> {
	Ok(match x {
		pegboard_actor_v5::PoolOperation::PoolRead => pegboard_actor_v4::PoolOperation::PoolRead,
		pegboard_actor_v5::PoolOperation::PoolPut(v) => pegboard_actor_v4::PoolOperation::PoolPut(
			convert_pool_put_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
		),
		pegboard_actor_v5::PoolOperation::PoolDelete => {
			pegboard_actor_v4::PoolOperation::PoolDelete
		}
		pegboard_actor_v5::PoolOperation::PoolConnect(v) => {
			pegboard_actor_v4::PoolOperation::PoolConnect(
				convert_pool_connect_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
		pegboard_actor_v5::PoolOperation::PoolDisconnect(v) => {
			pegboard_actor_v4::PoolOperation::PoolDisconnect(v)
		}
		pegboard_actor_v5::PoolOperation::PoolRenew(v) => {
			pegboard_actor_v4::PoolOperation::PoolRenew(
				convert_pool_renew_pegboard_actor_v5_to_pegboard_actor_v4(v)?,
			)
		}
	})
}

pub fn convert_pool_status_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::PoolStatus,
) -> Result<pegboard_actor_v4::PoolStatus> {
	Ok(match x {
		pegboard_actor_v5::PoolStatus::Ok => pegboard_actor_v4::PoolStatus::Ok,
		pegboard_actor_v5::PoolStatus::NotFound => pegboard_actor_v4::PoolStatus::NotFound,
		pegboard_actor_v5::PoolStatus::Connected => pegboard_actor_v4::PoolStatus::Connected,
	})
}

pub fn convert_pool_request_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::PoolRequest,
) -> Result<pegboard_actor_v4::PoolRequest> {
	Ok(pegboard_actor_v4::PoolRequest {
		pool_name: x.pool_name,
		operation: convert_pool_operation_pegboard_actor_v5_to_pegboard_actor_v4(x.operation)?,
	})
}

pub fn convert_pool_response_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::PoolResponse,
) -> Result<pegboard_actor_v4::PoolResponse> {
	Ok(pegboard_actor_v4::PoolResponse {
		status: convert_pool_status_pegboard_actor_v5_to_pegboard_actor_v4(x.status)?,
		config: x
			.config
			.map(|v| convert_node_pool_config_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.transpose()?,
	})
}

pub fn convert_pool_lease_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::PoolLease,
) -> Result<pegboard_actor_v4::PoolLease> {
	Ok(pegboard_actor_v4::PoolLease {
		heartbeat_unix_ms: x.heartbeat_unix_ms,
	})
}

pub fn convert_pool_entry_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::PoolEntry,
) -> Result<pegboard_actor_v4::PoolEntry> {
	Ok(pegboard_actor_v4::PoolEntry {
		name: x.name,
		config: convert_node_pool_config_pegboard_actor_v5_to_pegboard_actor_v4(x.config)?,
	})
}

pub fn convert_pool_list_request_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::PoolListRequest,
) -> Result<pegboard_actor_v4::PoolListRequest> {
	Ok(pegboard_actor_v4::PoolListRequest {
		after: x.after,
		limit: x.limit,
	})
}

pub fn convert_pool_list_response_pegboard_actor_v5_to_pegboard_actor_v4(
	x: pegboard_actor_v5::PoolListResponse,
) -> Result<pegboard_actor_v4::PoolListResponse> {
	Ok(pegboard_actor_v4::PoolListResponse {
		pools: x
			.pools
			.into_iter()
			.map(|v| convert_pool_entry_pegboard_actor_v5_to_pegboard_actor_v4(v))
			.collect::<Result<Vec<_>>>()?,
		cursor: x.cursor,
	})
}
