//! Upgrades actor records from v4, the layout 3.0.3 through 3.0.7 wrote under the v3 header, to v5,
//! the same layout under its own header.
// from: pegboard.actor.v4.bare, to: pegboard.actor.v5.bare

use anyhow::Result;

use crate::generated::{pegboard_actor_v4, pegboard_actor_v5};

pub fn convert_actor_data_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ActorData,
) -> Result<pegboard_actor_v5::ActorData> {
	Ok(pegboard_actor_v5::ActorData {
		actor_id: x.actor_id,
		namespace: x.namespace,
		pool_name: x.pool_name,
		name: x.name,
		key: x.key,
		input: x.input,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_actor_allocation_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ActorAllocation,
) -> Result<pegboard_actor_v5::ActorAllocation> {
	Ok(pegboard_actor_v5::ActorAllocation {
		envoy_key: x.envoy_key,
		worker_id: x.worker_id,
		connection_id: x.connection_id,
	})
}

pub fn convert_reserve_epoxy_phase_already_reserved_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ReserveEpoxyPhaseAlreadyReserved,
) -> Result<pegboard_actor_v5::ReserveEpoxyPhaseAlreadyReserved> {
	Ok(pegboard_actor_v5::ReserveEpoxyPhaseAlreadyReserved {
		reservation_id: x.reservation_id,
	})
}

pub fn convert_reserve_epoxy_phase_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ReserveEpoxyPhase,
) -> Result<pegboard_actor_v5::ReserveEpoxyPhase> {
	Ok(match x {
		pegboard_actor_v4::ReserveEpoxyPhase::ReserveEpoxyPhasePending => pegboard_actor_v5::ReserveEpoxyPhase::ReserveEpoxyPhasePending,
		pegboard_actor_v4::ReserveEpoxyPhase::ReserveEpoxyPhaseReserved => pegboard_actor_v5::ReserveEpoxyPhase::ReserveEpoxyPhaseReserved,
		pegboard_actor_v4::ReserveEpoxyPhase::ReserveEpoxyPhaseAlreadyReserved(v) => pegboard_actor_v5::ReserveEpoxyPhase::ReserveEpoxyPhaseAlreadyReserved(convert_reserve_epoxy_phase_already_reserved_pegboard_actor_v4_to_pegboard_actor_v5(v)?),
		pegboard_actor_v4::ReserveEpoxyPhase::ReserveEpoxyPhaseFailed => pegboard_actor_v5::ReserveEpoxyPhase::ReserveEpoxyPhaseFailed,
		pegboard_actor_v4::ReserveEpoxyPhase::ReserveEpoxyPhaseMaxAttemptsReached => pegboard_actor_v5::ReserveEpoxyPhase::ReserveEpoxyPhaseMaxAttemptsReached,
		pegboard_actor_v4::ReserveEpoxyPhase::ReserveEpoxyPhaseInV2 => pegboard_actor_v5::ReserveEpoxyPhase::ReserveEpoxyPhaseInV2,
	})
}

pub fn convert_reserve_epoxy_state_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ReserveEpoxyState,
) -> Result<pegboard_actor_v5::ReserveEpoxyState> {
	Ok(pegboard_actor_v5::ReserveEpoxyState {
		phase: convert_reserve_epoxy_phase_pegboard_actor_v4_to_pegboard_actor_v5(x.phase)?,
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_reserve_epoxy_effect_data_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ReserveEpoxyEffectData,
) -> Result<pegboard_actor_v5::ReserveEpoxyEffectData> {
	Ok(pegboard_actor_v5::ReserveEpoxyEffectData {
		namespace: x.namespace,
		name: x.name,
		key: x.key,
		state: convert_reserve_epoxy_state_pegboard_actor_v4_to_pegboard_actor_v5(x.state)?,
	})
}

pub fn convert_pending_action_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::PendingAction,
) -> Result<pegboard_actor_v5::PendingAction> {
	Ok(match x {
		pegboard_actor_v4::PendingAction::PendingNone => {
			pegboard_actor_v5::PendingAction::PendingNone
		}
		pegboard_actor_v4::PendingAction::PendingSleep => {
			pegboard_actor_v5::PendingAction::PendingSleep
		}
		pegboard_actor_v4::PendingAction::PendingReallocate => {
			pegboard_actor_v5::PendingAction::PendingReallocate
		}
		pegboard_actor_v4::PendingAction::PendingDestroy => {
			pegboard_actor_v5::PendingAction::PendingDestroy
		}
	})
}

pub fn convert_actor_create_data_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ActorCreateData,
) -> Result<pegboard_actor_v5::ActorCreateData> {
	Ok(pegboard_actor_v5::ActorCreateData {
		actor_id: x.actor_id,
		namespace: x.namespace,
		pool_name: x.pool_name,
		name: x.name,
		key: x.key,
		input: x.input,
	})
}

pub fn convert_initializing_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::Initializing,
) -> Result<pegboard_actor_v5::Initializing> {
	Ok(pegboard_actor_v5::Initializing {
		reserve_epoxy: convert_reserve_epoxy_effect_data_pegboard_actor_v4_to_pegboard_actor_v5(
			x.reserve_epoxy,
		)?,
		allocation: x
			.allocation
			.map(|v| convert_actor_allocation_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.transpose()?,
	})
}

pub fn convert_starting_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::Starting,
) -> Result<pegboard_actor_v5::Starting> {
	Ok(pegboard_actor_v5::Starting {
		allocation: convert_actor_allocation_pegboard_actor_v4_to_pegboard_actor_v5(x.allocation)?,
	})
}

pub fn convert_running_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::Running,
) -> Result<pegboard_actor_v5::Running> {
	Ok(pegboard_actor_v5::Running {
		allocation: convert_actor_allocation_pegboard_actor_v4_to_pegboard_actor_v5(x.allocation)?,
	})
}

pub fn convert_sleeping_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::Sleeping,
) -> Result<pegboard_actor_v5::Sleeping> {
	Ok(pegboard_actor_v5::Sleeping {
		wake_retries: x.wake_retries,
	})
}

pub fn convert_stopping_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::Stopping,
) -> Result<pegboard_actor_v5::Stopping> {
	Ok(pegboard_actor_v5::Stopping {
		allocation: convert_actor_allocation_pegboard_actor_v4_to_pegboard_actor_v5(x.allocation)?,
	})
}

pub fn convert_destroy_phase_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::DestroyPhase,
) -> Result<pegboard_actor_v5::DestroyPhase> {
	Ok(match x {
		pegboard_actor_v4::DestroyPhase::DestroyPhasePending => {
			pegboard_actor_v5::DestroyPhase::DestroyPhasePending
		}
		pegboard_actor_v4::DestroyPhase::DestroyPhaseSuccess => {
			pegboard_actor_v5::DestroyPhase::DestroyPhaseSuccess
		}
		pegboard_actor_v4::DestroyPhase::DestroyPhaseFailed => {
			pegboard_actor_v5::DestroyPhase::DestroyPhaseFailed
		}
	})
}

pub fn convert_destroy_state_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::DestroyState,
) -> Result<pegboard_actor_v5::DestroyState> {
	Ok(pegboard_actor_v5::DestroyState {
		phase: convert_destroy_phase_pegboard_actor_v4_to_pegboard_actor_v5(x.phase)?,
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_destroy_effect_data_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::DestroyEffectData,
) -> Result<pegboard_actor_v5::DestroyEffectData> {
	Ok(pegboard_actor_v5::DestroyEffectData {
		create_unix_ms: x.create_unix_ms,
		state: convert_destroy_state_pegboard_actor_v4_to_pegboard_actor_v5(x.state)?,
	})
}

pub fn convert_key_owner_datacenter_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::KeyOwnerDatacenter,
) -> Result<pegboard_actor_v5::KeyOwnerDatacenter> {
	Ok(pegboard_actor_v5::KeyOwnerDatacenter {
		reservation_id: x.reservation_id,
	})
}

pub fn convert_key_owner_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::KeyOwner,
) -> Result<pegboard_actor_v5::KeyOwner> {
	Ok(match x {
		pegboard_actor_v4::KeyOwner::KeyOwnerDatacenter(v) => {
			pegboard_actor_v5::KeyOwner::KeyOwnerDatacenter(
				convert_key_owner_datacenter_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::KeyOwner::KeyOwnerV2 => pegboard_actor_v5::KeyOwner::KeyOwnerV2,
	})
}

pub fn convert_destroying_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::Destroying,
) -> Result<pegboard_actor_v5::Destroying> {
	Ok(pegboard_actor_v5::Destroying {
		destroy: convert_destroy_effect_data_pegboard_actor_v4_to_pegboard_actor_v5(x.destroy)?,
		key_owner: x
			.key_owner
			.map(|v| convert_key_owner_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.transpose()?,
	})
}

pub fn convert_migrating_from_v2_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::MigratingFromV2,
) -> Result<pegboard_actor_v5::MigratingFromV2> {
	Ok(pegboard_actor_v5::MigratingFromV2 {
		migration_id: x.migration_id,
		reserve_epoxy: x
			.reserve_epoxy
			.map(|v| convert_reserve_epoxy_effect_data_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.transpose()?,
		alarm_due: x.alarm_due,
	})
}

pub fn convert_actor_phase_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ActorPhase,
) -> Result<pegboard_actor_v5::ActorPhase> {
	Ok(match x {
		pegboard_actor_v4::ActorPhase::Initializing(v) => {
			pegboard_actor_v5::ActorPhase::Initializing(
				convert_initializing_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::ActorPhase::Starting(v) => pegboard_actor_v5::ActorPhase::Starting(
			convert_starting_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
		),
		pegboard_actor_v4::ActorPhase::Running(v) => pegboard_actor_v5::ActorPhase::Running(
			convert_running_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
		),
		pegboard_actor_v4::ActorPhase::Stopping(v) => pegboard_actor_v5::ActorPhase::Stopping(
			convert_stopping_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
		),
		pegboard_actor_v4::ActorPhase::Sleeping(v) => pegboard_actor_v5::ActorPhase::Sleeping(
			convert_sleeping_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
		),
		pegboard_actor_v4::ActorPhase::Destroying(v) => pegboard_actor_v5::ActorPhase::Destroying(
			convert_destroying_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
		),
		pegboard_actor_v4::ActorPhase::Destroyed => pegboard_actor_v5::ActorPhase::Destroyed,
		pegboard_actor_v4::ActorPhase::MigratingFromV2(v) => {
			pegboard_actor_v5::ActorPhase::MigratingFromV2(
				convert_migrating_from_v2_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::ActorPhase::DelegatedToV2 => {
			pegboard_actor_v5::ActorPhase::DelegatedToV2
		}
	})
}

pub fn convert_start_waiter_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::StartWaiter,
) -> Result<pegboard_actor_v5::StartWaiter> {
	Ok(pegboard_actor_v5::StartWaiter {
		worker_id: x.worker_id,
		wait_id: x.wait_id,
		expires_unix_ms: x.expires_unix_ms,
	})
}

pub fn convert_write_index_phase_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::WriteIndexPhase,
) -> Result<pegboard_actor_v5::WriteIndexPhase> {
	Ok(match x {
		pegboard_actor_v4::WriteIndexPhase::WriteIndexPhasePending => {
			pegboard_actor_v5::WriteIndexPhase::WriteIndexPhasePending
		}
		pegboard_actor_v4::WriteIndexPhase::WriteIndexPhaseSuccess => {
			pegboard_actor_v5::WriteIndexPhase::WriteIndexPhaseSuccess
		}
		pegboard_actor_v4::WriteIndexPhase::WriteIndexPhaseFailed => {
			pegboard_actor_v5::WriteIndexPhase::WriteIndexPhaseFailed
		}
	})
}

pub fn convert_write_index_state_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::WriteIndexState,
) -> Result<pegboard_actor_v5::WriteIndexState> {
	Ok(pegboard_actor_v5::WriteIndexState {
		phase: convert_write_index_phase_pegboard_actor_v4_to_pegboard_actor_v5(x.phase)?,
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_write_index_effect_data_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::WriteIndexEffectData,
) -> Result<pegboard_actor_v5::WriteIndexEffectData> {
	Ok(pegboard_actor_v5::WriteIndexEffectData {
		namespace: x.namespace,
		name: x.name,
		create_unix_ms: x.create_unix_ms,
		state: convert_write_index_state_pegboard_actor_v4_to_pegboard_actor_v5(x.state)?,
	})
}

pub fn convert_storage_origin_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::StorageOrigin,
) -> Result<pegboard_actor_v5::StorageOrigin> {
	Ok(match x {
		pegboard_actor_v4::StorageOrigin::Created => pegboard_actor_v5::StorageOrigin::Created,
		pegboard_actor_v4::StorageOrigin::Imported => pegboard_actor_v5::StorageOrigin::Imported,
	})
}

pub fn convert_actor_state_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ActorState,
) -> Result<pegboard_actor_v5::ActorState> {
	Ok(pegboard_actor_v5::ActorState {
		phase: convert_actor_phase_pegboard_actor_v4_to_pegboard_actor_v5(x.phase)?,
		epoch: x.epoch,
		create_unix_ms: x.create_unix_ms,
		storage_origin: convert_storage_origin_pegboard_actor_v4_to_pegboard_actor_v5(
			x.storage_origin,
		)?,
		command_index: x.command_index,
		index_bucket_count: x.index_bucket_count,
		pending_action: convert_pending_action_pegboard_actor_v4_to_pegboard_actor_v5(
			x.pending_action,
		)?,
		write_index: convert_write_index_effect_data_pegboard_actor_v4_to_pegboard_actor_v5(
			x.write_index,
		)?,
		start_waiters: x
			.start_waiters
			.into_iter()
			.map(|v| convert_start_waiter_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_reserve_epoxy_timeout_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ReserveEpoxyTimeout,
) -> Result<pegboard_actor_v5::ReserveEpoxyTimeout> {
	Ok(pegboard_actor_v5::ReserveEpoxyTimeout {
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_write_index_timeout_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::WriteIndexTimeout,
) -> Result<pegboard_actor_v5::WriteIndexTimeout> {
	Ok(pegboard_actor_v5::WriteIndexTimeout {
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_destroy_timeout_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::DestroyTimeout,
) -> Result<pegboard_actor_v5::DestroyTimeout> {
	Ok(pegboard_actor_v5::DestroyTimeout {
		attempt: x.attempt,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_start_timeout_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::StartTimeout,
) -> Result<pegboard_actor_v5::StartTimeout> {
	Ok(pegboard_actor_v5::StartTimeout { epoch: x.epoch })
}

pub fn convert_stop_timeout_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::StopTimeout,
) -> Result<pegboard_actor_v5::StopTimeout> {
	Ok(pegboard_actor_v5::StopTimeout { epoch: x.epoch })
}

pub fn convert_envoy_check_timeout_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::EnvoyCheckTimeout,
) -> Result<pegboard_actor_v5::EnvoyCheckTimeout> {
	Ok(pegboard_actor_v5::EnvoyCheckTimeout { epoch: x.epoch })
}

pub fn convert_stop_threshold_timeout_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::StopThresholdTimeout,
) -> Result<pegboard_actor_v5::StopThresholdTimeout> {
	Ok(pegboard_actor_v5::StopThresholdTimeout {
		epoch: x.epoch,
		stop_unix_ms: x.stop_unix_ms,
	})
}

pub fn convert_allocation_retry_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::AllocationRetry,
) -> Result<pegboard_actor_v5::AllocationRetry> {
	Ok(pegboard_actor_v5::AllocationRetry {
		epoch: x.epoch,
		attempt: x.attempt,
	})
}

pub fn convert_actor_timer_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ActorTimer,
) -> Result<pegboard_actor_v5::ActorTimer> {
	Ok(match x {
		pegboard_actor_v4::ActorTimer::ReserveEpoxyTimeout(v) => {
			pegboard_actor_v5::ActorTimer::ReserveEpoxyTimeout(
				convert_reserve_epoxy_timeout_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::ActorTimer::WriteIndexTimeout(v) => {
			pegboard_actor_v5::ActorTimer::WriteIndexTimeout(
				convert_write_index_timeout_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::ActorTimer::DestroyTimeout(v) => {
			pegboard_actor_v5::ActorTimer::DestroyTimeout(
				convert_destroy_timeout_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::ActorTimer::StartTimeout(v) => {
			pegboard_actor_v5::ActorTimer::StartTimeout(
				convert_start_timeout_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::ActorTimer::AlarmTimeout => pegboard_actor_v5::ActorTimer::AlarmTimeout,
		pegboard_actor_v4::ActorTimer::StopTimeout(v) => {
			pegboard_actor_v5::ActorTimer::StopTimeout(
				convert_stop_timeout_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::ActorTimer::EnvoyCheckTimeout(v) => {
			pegboard_actor_v5::ActorTimer::EnvoyCheckTimeout(
				convert_envoy_check_timeout_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::ActorTimer::StopThresholdTimeout(v) => {
			pegboard_actor_v5::ActorTimer::StopThresholdTimeout(
				convert_stop_threshold_timeout_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::ActorTimer::AllocationRetry(v) => {
			pegboard_actor_v5::ActorTimer::AllocationRetry(
				convert_allocation_retry_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
	})
}

pub fn convert_start_failed_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::StartFailed,
) -> Result<pegboard_actor_v5::StartFailed> {
	Ok(pegboard_actor_v5::StartFailed { epoch: x.epoch })
}

pub fn convert_envoy_gone_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::EnvoyGone,
) -> Result<pegboard_actor_v5::EnvoyGone> {
	Ok(pegboard_actor_v5::EnvoyGone { epoch: x.epoch })
}

pub fn convert_try_start_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::TryStart,
) -> Result<pegboard_actor_v5::TryStart> {
	Ok(pegboard_actor_v5::TryStart {
		waiter: x
			.waiter
			.map(|v| convert_start_waiter_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.transpose()?,
	})
}

pub fn convert_create_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::Create,
) -> Result<pegboard_actor_v5::Create> {
	Ok(pegboard_actor_v5::Create {
		init: convert_actor_create_data_pegboard_actor_v4_to_pegboard_actor_v5(x.init)?,
		waiter: x
			.waiter
			.map(|v| convert_start_waiter_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.transpose()?,
		force_actor_v3: x.force_actor_v3,
	})
}

pub fn convert_reallocate_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::Reallocate,
) -> Result<pegboard_actor_v5::Reallocate> {
	Ok(pegboard_actor_v5::Reallocate {
		epoch: x.epoch,
		lost: x.lost,
		waiter: x
			.waiter
			.map(|v| convert_start_waiter_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.transpose()?,
	})
}

pub fn convert_update_route_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::UpdateRoute,
) -> Result<pegboard_actor_v5::UpdateRoute> {
	Ok(pegboard_actor_v5::UpdateRoute {
		epoch: x.epoch,
		from: convert_actor_allocation_pegboard_actor_v4_to_pegboard_actor_v5(x.from)?,
		to: convert_actor_allocation_pegboard_actor_v4_to_pegboard_actor_v5(x.to)?,
	})
}

pub fn convert_to_envoy_event_kind_stopped_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ToEnvoyEventKindStopped,
) -> Result<pegboard_actor_v5::ToEnvoyEventKindStopped> {
	Ok(pegboard_actor_v5::ToEnvoyEventKindStopped {
		ok: x.ok,
		message: x.message,
	})
}

pub fn convert_to_envoy_event_kind_set_alarm_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ToEnvoyEventKindSetAlarm,
) -> Result<pegboard_actor_v5::ToEnvoyEventKindSetAlarm> {
	Ok(pegboard_actor_v5::ToEnvoyEventKindSetAlarm {
		alarm_unix_ms: x.alarm_unix_ms,
	})
}

pub fn convert_to_envoy_event_kind_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ToEnvoyEventKind,
) -> Result<pegboard_actor_v5::ToEnvoyEventKind> {
	Ok(match x {
		pegboard_actor_v4::ToEnvoyEventKind::ToEnvoyEventKindRunning => {
			pegboard_actor_v5::ToEnvoyEventKind::ToEnvoyEventKindRunning
		}
		pegboard_actor_v4::ToEnvoyEventKind::ToEnvoyEventKindStopped(v) => {
			pegboard_actor_v5::ToEnvoyEventKind::ToEnvoyEventKindStopped(
				convert_to_envoy_event_kind_stopped_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::ToEnvoyEventKind::ToEnvoyEventKindSleepIntent => {
			pegboard_actor_v5::ToEnvoyEventKind::ToEnvoyEventKindSleepIntent
		}
		pegboard_actor_v4::ToEnvoyEventKind::ToEnvoyEventKindStopIntent => {
			pegboard_actor_v5::ToEnvoyEventKind::ToEnvoyEventKindStopIntent
		}
		pegboard_actor_v4::ToEnvoyEventKind::ToEnvoyEventKindSetAlarm(v) => {
			pegboard_actor_v5::ToEnvoyEventKind::ToEnvoyEventKindSetAlarm(
				convert_to_envoy_event_kind_set_alarm_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
	})
}

pub fn convert_to_envoy_event_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ToEnvoyEvent,
) -> Result<pegboard_actor_v5::ToEnvoyEvent> {
	Ok(pegboard_actor_v5::ToEnvoyEvent {
		epoch: x.epoch,
		kind: convert_to_envoy_event_kind_pegboard_actor_v4_to_pegboard_actor_v5(x.kind)?,
	})
}

pub fn convert_migrate_from_v2_begin_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::MigrateFromV2Begin,
) -> Result<pegboard_actor_v5::MigrateFromV2Begin> {
	Ok(pegboard_actor_v5::MigrateFromV2Begin {
		migration_id: x.migration_id,
		init: convert_actor_create_data_pegboard_actor_v4_to_pegboard_actor_v5(x.init)?,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_migrate_from_v2_activate_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::MigrateFromV2Activate,
) -> Result<pegboard_actor_v5::MigrateFromV2Activate> {
	Ok(pegboard_actor_v5::MigrateFromV2Activate {
		migration_id: x.migration_id,
		alarm_unix_ms: x.alarm_unix_ms,
	})
}

pub fn convert_migrate_from_v2_discard_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::MigrateFromV2Discard,
) -> Result<pegboard_actor_v5::MigrateFromV2Discard> {
	Ok(pegboard_actor_v5::MigrateFromV2Discard {
		migration_id: x.migration_id,
	})
}

pub fn convert_migrate_from_v2_recall_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::MigrateFromV2Recall,
) -> Result<pegboard_actor_v5::MigrateFromV2Recall> {
	Ok(pegboard_actor_v5::MigrateFromV2Recall {
		migration_id: x.migration_id,
	})
}

pub fn convert_migrate_from_v2_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::MigrateFromV2,
) -> Result<pegboard_actor_v5::MigrateFromV2> {
	Ok(match x {
		pegboard_actor_v4::MigrateFromV2::MigrateFromV2Begin(v) => {
			pegboard_actor_v5::MigrateFromV2::MigrateFromV2Begin(
				convert_migrate_from_v2_begin_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::MigrateFromV2::MigrateFromV2Activate(v) => {
			pegboard_actor_v5::MigrateFromV2::MigrateFromV2Activate(
				convert_migrate_from_v2_activate_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::MigrateFromV2::MigrateFromV2Discard(v) => {
			pegboard_actor_v5::MigrateFromV2::MigrateFromV2Discard(
				convert_migrate_from_v2_discard_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::MigrateFromV2::MigrateFromV2Recall(v) => {
			pegboard_actor_v5::MigrateFromV2::MigrateFromV2Recall(
				convert_migrate_from_v2_recall_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
	})
}

pub fn convert_actor_event_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ActorEvent,
) -> Result<pegboard_actor_v5::ActorEvent> {
	Ok(match x {
		pegboard_actor_v4::ActorEvent::ReserveEpoxyState(v) => {
			pegboard_actor_v5::ActorEvent::ReserveEpoxyState(
				convert_reserve_epoxy_state_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::ActorEvent::WriteIndexState(v) => {
			pegboard_actor_v5::ActorEvent::WriteIndexState(
				convert_write_index_state_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::ActorEvent::StartFailed(v) => {
			pegboard_actor_v5::ActorEvent::StartFailed(
				convert_start_failed_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::ActorEvent::Sleep => pegboard_actor_v5::ActorEvent::Sleep,
		pegboard_actor_v4::ActorEvent::TryStart(v) => pegboard_actor_v5::ActorEvent::TryStart(
			convert_try_start_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
		),
		pegboard_actor_v4::ActorEvent::Create(v) => pegboard_actor_v5::ActorEvent::Create(
			convert_create_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
		),
		pegboard_actor_v4::ActorEvent::Reallocate(v) => pegboard_actor_v5::ActorEvent::Reallocate(
			convert_reallocate_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
		),
		pegboard_actor_v4::ActorEvent::UpdateRoute(v) => {
			pegboard_actor_v5::ActorEvent::UpdateRoute(
				convert_update_route_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::ActorEvent::Destroy => pegboard_actor_v5::ActorEvent::Destroy,
		pegboard_actor_v4::ActorEvent::DestroyState(v) => {
			pegboard_actor_v5::ActorEvent::DestroyState(
				convert_destroy_state_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::ActorEvent::ToEnvoyEvent(v) => {
			pegboard_actor_v5::ActorEvent::ToEnvoyEvent(
				convert_to_envoy_event_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::ActorEvent::EnvoyGone(v) => pegboard_actor_v5::ActorEvent::EnvoyGone(
			convert_envoy_gone_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
		),
		pegboard_actor_v4::ActorEvent::MigrateFromV2(v) => {
			pegboard_actor_v5::ActorEvent::MigrateFromV2(
				convert_migrate_from_v2_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
	})
}

pub fn convert_try_start_starting_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::TryStartStarting,
) -> Result<pegboard_actor_v5::TryStartStarting> {
	Ok(pegboard_actor_v5::TryStartStarting {
		epoch: x.epoch,
		allocation: convert_actor_allocation_pegboard_actor_v4_to_pegboard_actor_v5(x.allocation)?,
	})
}

pub fn convert_try_start_running_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::TryStartRunning,
) -> Result<pegboard_actor_v5::TryStartRunning> {
	Ok(pegboard_actor_v5::TryStartRunning {
		epoch: x.epoch,
		allocation: convert_actor_allocation_pegboard_actor_v4_to_pegboard_actor_v5(x.allocation)?,
	})
}

pub fn convert_try_start_epoxy_already_reserved_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::TryStartEpoxyAlreadyReserved,
) -> Result<pegboard_actor_v5::TryStartEpoxyAlreadyReserved> {
	Ok(pegboard_actor_v5::TryStartEpoxyAlreadyReserved {
		reservation_id: x.reservation_id,
	})
}

pub fn convert_try_start_outcome_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::TryStartOutcome,
) -> Result<pegboard_actor_v5::TryStartOutcome> {
	Ok(match x {
		pegboard_actor_v4::TryStartOutcome::TryStartWaiting => {
			pegboard_actor_v5::TryStartOutcome::TryStartWaiting
		}
		pegboard_actor_v4::TryStartOutcome::TryStartStarting(v) => {
			pegboard_actor_v5::TryStartOutcome::TryStartStarting(
				convert_try_start_starting_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::TryStartOutcome::TryStartRunning(v) => {
			pegboard_actor_v5::TryStartOutcome::TryStartRunning(
				convert_try_start_running_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::TryStartOutcome::TryStartDestroyed => {
			pegboard_actor_v5::TryStartOutcome::TryStartDestroyed
		}
		pegboard_actor_v4::TryStartOutcome::TryStartEpoxyAlreadyReserved(v) => {
			pegboard_actor_v5::TryStartOutcome::TryStartEpoxyAlreadyReserved(
				convert_try_start_epoxy_already_reserved_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::TryStartOutcome::TryStartFull => {
			pegboard_actor_v5::TryStartOutcome::TryStartFull
		}
		pegboard_actor_v4::TryStartOutcome::TryStartMigratingFromV2 => {
			pegboard_actor_v5::TryStartOutcome::TryStartMigratingFromV2
		}
		pegboard_actor_v4::TryStartOutcome::TryStartDelegatedToV2 => {
			pegboard_actor_v5::TryStartOutcome::TryStartDelegatedToV2
		}
	})
}

pub fn convert_actor_read_response_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ActorReadResponse,
) -> Result<pegboard_actor_v5::ActorReadResponse> {
	Ok(pegboard_actor_v5::ActorReadResponse {
		data: x
			.data
			.map(|v| convert_actor_data_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.transpose()?,
		phase: x
			.phase
			.map(|v| convert_actor_phase_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.transpose()?,
		epoch: x.epoch,
	})
}

pub fn convert_actor_event_response_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ActorEventResponse,
) -> Result<pegboard_actor_v5::ActorEventResponse> {
	Ok(pegboard_actor_v5::ActorEventResponse {
		outcome: x
			.outcome
			.map(|v| convert_try_start_outcome_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.transpose()?,
	})
}

pub fn convert_actor_migrate_from_v2_begin_response_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ActorMigrateFromV2BeginResponse,
) -> Result<pegboard_actor_v5::ActorMigrateFromV2BeginResponse> {
	Ok(pegboard_actor_v5::ActorMigrateFromV2BeginResponse {
		phase: convert_actor_phase_pegboard_actor_v4_to_pegboard_actor_v5(x.phase)?,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_actor_create_request_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ActorCreateRequest,
) -> Result<pegboard_actor_v5::ActorCreateRequest> {
	Ok(pegboard_actor_v5::ActorCreateRequest {
		init: convert_actor_create_data_pegboard_actor_v4_to_pegboard_actor_v5(x.init)?,
		waiter: x
			.waiter
			.map(|v| convert_start_waiter_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.transpose()?,
		force_actor_v3: x.force_actor_v3,
	})
}

pub fn convert_actor_create_response_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ActorCreateResponse,
) -> Result<pegboard_actor_v5::ActorCreateResponse> {
	Ok(pegboard_actor_v5::ActorCreateResponse {
		outcome: convert_try_start_outcome_pegboard_actor_v4_to_pegboard_actor_v5(x.outcome)?,
	})
}

pub fn convert_actor_start_notice_starting_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ActorStartNoticeStarting,
) -> Result<pegboard_actor_v5::ActorStartNoticeStarting> {
	Ok(pegboard_actor_v5::ActorStartNoticeStarting {
		epoch: x.epoch,
		allocation: convert_actor_allocation_pegboard_actor_v4_to_pegboard_actor_v5(x.allocation)?,
	})
}

pub fn convert_actor_start_notice_running_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ActorStartNoticeRunning,
) -> Result<pegboard_actor_v5::ActorStartNoticeRunning> {
	Ok(pegboard_actor_v5::ActorStartNoticeRunning {
		epoch: x.epoch,
		allocation: convert_actor_allocation_pegboard_actor_v4_to_pegboard_actor_v5(x.allocation)?,
	})
}

pub fn convert_actor_start_notice_epoxy_already_reserved_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ActorStartNoticeEpoxyAlreadyReserved,
) -> Result<pegboard_actor_v5::ActorStartNoticeEpoxyAlreadyReserved> {
	Ok(pegboard_actor_v5::ActorStartNoticeEpoxyAlreadyReserved {
		reservation_id: x.reservation_id,
	})
}

pub fn convert_actor_start_notice_kind_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ActorStartNoticeKind,
) -> Result<pegboard_actor_v5::ActorStartNoticeKind> {
	Ok(match x {
		pegboard_actor_v4::ActorStartNoticeKind::ActorStartNoticeStarting(v) => pegboard_actor_v5::ActorStartNoticeKind::ActorStartNoticeStarting(convert_actor_start_notice_starting_pegboard_actor_v4_to_pegboard_actor_v5(v)?),
		pegboard_actor_v4::ActorStartNoticeKind::ActorStartNoticeRunning(v) => pegboard_actor_v5::ActorStartNoticeKind::ActorStartNoticeRunning(convert_actor_start_notice_running_pegboard_actor_v4_to_pegboard_actor_v5(v)?),
		pegboard_actor_v4::ActorStartNoticeKind::ActorStartNoticeEpoxyAlreadyReserved(v) => pegboard_actor_v5::ActorStartNoticeKind::ActorStartNoticeEpoxyAlreadyReserved(convert_actor_start_notice_epoxy_already_reserved_pegboard_actor_v4_to_pegboard_actor_v5(v)?),
		pegboard_actor_v4::ActorStartNoticeKind::ActorStartNoticeDestroyed => pegboard_actor_v5::ActorStartNoticeKind::ActorStartNoticeDestroyed,
		pegboard_actor_v4::ActorStartNoticeKind::ActorStartNoticeMigratingFromV2 => pegboard_actor_v5::ActorStartNoticeKind::ActorStartNoticeMigratingFromV2,
		pegboard_actor_v4::ActorStartNoticeKind::ActorStartNoticeDelegatedToV2 => pegboard_actor_v5::ActorStartNoticeKind::ActorStartNoticeDelegatedToV2,
	})
}

pub fn convert_actor_start_notice_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ActorStartNotice,
) -> Result<pegboard_actor_v5::ActorStartNotice> {
	Ok(pegboard_actor_v5::ActorStartNotice {
		actor_id: x.actor_id,
		kind: convert_actor_start_notice_kind_pegboard_actor_v4_to_pegboard_actor_v5(x.kind)?,
	})
}

pub fn convert_kv_value_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::KvValue,
) -> Result<pegboard_actor_v5::KvValue> {
	Ok(pegboard_actor_v5::KvValue {
		value: x.value,
		version: x.version,
		updated_unix_ms: x.updated_unix_ms,
	})
}

pub fn convert_index_insert_actor_request_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexInsertActorRequest,
) -> Result<pegboard_actor_v5::IndexInsertActorRequest> {
	Ok(pegboard_actor_v5::IndexInsertActorRequest {
		actor_id: x.actor_id,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_index_delete_actor_request_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexDeleteActorRequest,
) -> Result<pegboard_actor_v5::IndexDeleteActorRequest> {
	Ok(pegboard_actor_v5::IndexDeleteActorRequest {
		actor_id: x.actor_id,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_index_actor_cursor_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexActorCursor,
) -> Result<pegboard_actor_v5::IndexActorCursor> {
	Ok(pegboard_actor_v5::IndexActorCursor {
		create_unix_ms: x.create_unix_ms,
		actor_id: x.actor_id,
	})
}

pub fn convert_index_list_actors_request_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexListActorsRequest,
) -> Result<pegboard_actor_v5::IndexListActorsRequest> {
	Ok(pegboard_actor_v5::IndexListActorsRequest {
		after: x
			.after
			.map(|v| convert_index_actor_cursor_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.transpose()?,
		limit: x.limit,
		include_destroyed: x.include_destroyed,
	})
}

pub fn convert_index_destroyed_actor_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexDestroyedActor,
) -> Result<pegboard_actor_v5::IndexDestroyedActor> {
	Ok(pegboard_actor_v5::IndexDestroyedActor {
		create_unix_ms: x.create_unix_ms,
		actor_id: x.actor_id,
		destroy_unix_ms: x.destroy_unix_ms,
	})
}

pub fn convert_index_list_actors_response_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexListActorsResponse,
) -> Result<pegboard_actor_v5::IndexListActorsResponse> {
	Ok(pegboard_actor_v5::IndexListActorsResponse {
		actors: x
			.actors
			.into_iter()
			.map(|v| convert_index_actor_cursor_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.collect::<Result<Vec<_>>>()?,
		has_more: x.has_more,
		destroyed: x
			.destroyed
			.into_iter()
			.map(|v| convert_index_destroyed_actor_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.collect::<Result<Vec<_>>>()?,
	})
}

pub fn convert_destroyed_actor_index_entry_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::DestroyedActorIndexEntry,
) -> Result<pegboard_actor_v5::DestroyedActorIndexEntry> {
	Ok(pegboard_actor_v5::DestroyedActorIndexEntry {
		destroy_unix_ms: x.destroy_unix_ms,
	})
}

pub fn convert_index_read_destroyed_actor_request_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexReadDestroyedActorRequest,
) -> Result<pegboard_actor_v5::IndexReadDestroyedActorRequest> {
	Ok(pegboard_actor_v5::IndexReadDestroyedActorRequest {
		create_unix_ms: x.create_unix_ms,
		actor_id: x.actor_id,
	})
}

pub fn convert_index_read_destroyed_actor_response_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexReadDestroyedActorResponse,
) -> Result<pegboard_actor_v5::IndexReadDestroyedActorResponse> {
	Ok(pegboard_actor_v5::IndexReadDestroyedActorResponse {
		destroy_unix_ms: x.destroy_unix_ms,
	})
}

pub fn convert_envoy_data_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::EnvoyData,
) -> Result<pegboard_actor_v5::EnvoyData> {
	Ok(pegboard_actor_v5::EnvoyData {
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

pub fn convert_envoy_upsert_request_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::EnvoyUpsertRequest,
) -> Result<pegboard_actor_v5::EnvoyUpsertRequest> {
	Ok(pegboard_actor_v5::EnvoyUpsertRequest {
		data: convert_envoy_data_pegboard_actor_v4_to_pegboard_actor_v5(x.data)?,
		expected: x
			.expected
			.map(|v| convert_envoy_data_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.transpose()?,
	})
}

pub fn convert_envoy_upsert_response_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::EnvoyUpsertResponse,
) -> Result<pegboard_actor_v5::EnvoyUpsertResponse> {
	Ok(pegboard_actor_v5::EnvoyUpsertResponse {
		data: convert_envoy_data_pegboard_actor_v4_to_pegboard_actor_v5(x.data)?,
	})
}

pub fn convert_envoy_read_response_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::EnvoyReadResponse,
) -> Result<pegboard_actor_v5::EnvoyReadResponse> {
	Ok(pegboard_actor_v5::EnvoyReadResponse {
		data: x
			.data
			.map(|v| convert_envoy_data_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.transpose()?,
	})
}

pub fn convert_index_insert_envoy_request_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexInsertEnvoyRequest,
) -> Result<pegboard_actor_v5::IndexInsertEnvoyRequest> {
	Ok(pegboard_actor_v5::IndexInsertEnvoyRequest {
		create_unix_ms: x.create_unix_ms,
		envoy_key: x.envoy_key,
	})
}

pub fn convert_index_list_envoys_request_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexListEnvoysRequest,
) -> Result<pegboard_actor_v5::IndexListEnvoysRequest> {
	Ok(pegboard_actor_v5::IndexListEnvoysRequest { cursor: x.cursor })
}

pub fn convert_index_list_envoys_response_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexListEnvoysResponse,
) -> Result<pegboard_actor_v5::IndexListEnvoysResponse> {
	Ok(pegboard_actor_v5::IndexListEnvoysResponse {
		envoy_keys: x.envoy_keys,
		cursor: x.cursor,
	})
}

pub fn convert_active_envoy_index_entry_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ActiveEnvoyIndexEntry,
) -> Result<pegboard_actor_v5::ActiveEnvoyIndexEntry> {
	Ok(pegboard_actor_v5::ActiveEnvoyIndexEntry {
		worker_id: x.worker_id,
		connection_id: x.connection_id,
	})
}

pub fn convert_active_envoy_state_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::ActiveEnvoyState,
) -> Result<pegboard_actor_v5::ActiveEnvoyState> {
	Ok(pegboard_actor_v5::ActiveEnvoyState {
		envoy_key: x.envoy_key,
		connection_epoch: x.connection_epoch,
		version: x.version,
		heartbeat_unix_ms: x.heartbeat_unix_ms,
		names: x.names,
	})
}

pub fn convert_index_upsert_active_envoy_request_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexUpsertActiveEnvoyRequest,
) -> Result<pegboard_actor_v5::IndexUpsertActiveEnvoyRequest> {
	Ok(pegboard_actor_v5::IndexUpsertActiveEnvoyRequest {
		envoy_key: x.envoy_key,
		connection_epoch: x.connection_epoch,
		worker_id: x.worker_id,
		connection_id: x.connection_id,
		version: x.version,
		heartbeat_unix_ms: x.heartbeat_unix_ms,
		names: x.names,
	})
}

pub fn convert_index_upsert_active_envoy_response_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexUpsertActiveEnvoyResponse,
) -> Result<pegboard_actor_v5::IndexUpsertActiveEnvoyResponse> {
	Ok(pegboard_actor_v5::IndexUpsertActiveEnvoyResponse {
		superseded: x.superseded,
		newest_live_version: x.newest_live_version,
	})
}

pub fn convert_index_delete_active_envoy_request_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexDeleteActiveEnvoyRequest,
) -> Result<pegboard_actor_v5::IndexDeleteActiveEnvoyRequest> {
	Ok(pegboard_actor_v5::IndexDeleteActiveEnvoyRequest {
		envoy_key: x.envoy_key,
		connection_epoch: x.connection_epoch,
	})
}

pub fn convert_index_put_envoy_pool_request_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexPutEnvoyPoolRequest,
) -> Result<pegboard_actor_v5::IndexPutEnvoyPoolRequest> {
	Ok(pegboard_actor_v5::IndexPutEnvoyPoolRequest {
		pool_name: x.pool_name,
	})
}

pub fn convert_index_list_envoy_pools_response_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexListEnvoyPoolsResponse,
) -> Result<pegboard_actor_v5::IndexListEnvoyPoolsResponse> {
	Ok(pegboard_actor_v5::IndexListEnvoyPoolsResponse { pools: x.pools })
}

pub fn convert_index_list_active_envoys_request_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexListActiveEnvoysRequest,
) -> Result<pegboard_actor_v5::IndexListActiveEnvoysRequest> {
	Ok(pegboard_actor_v5::IndexListActiveEnvoysRequest {
		name: x.name,
		cursor: x.cursor,
	})
}

pub fn convert_index_list_active_envoys_response_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexListActiveEnvoysResponse,
) -> Result<pegboard_actor_v5::IndexListActiveEnvoysResponse> {
	Ok(pegboard_actor_v5::IndexListActiveEnvoysResponse {
		envoys: x
			.envoys
			.into_iter()
			.map(|v| convert_actor_allocation_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.collect::<Result<Vec<_>>>()?,
		cursor: x.cursor,
	})
}

pub fn convert_index_list_active_envoy_states_request_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexListActiveEnvoyStatesRequest,
) -> Result<pegboard_actor_v5::IndexListActiveEnvoyStatesRequest> {
	Ok(pegboard_actor_v5::IndexListActiveEnvoyStatesRequest { cursor: x.cursor })
}

pub fn convert_index_list_active_envoy_states_response_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::IndexListActiveEnvoyStatesResponse,
) -> Result<pegboard_actor_v5::IndexListActiveEnvoyStatesResponse> {
	Ok(pegboard_actor_v5::IndexListActiveEnvoyStatesResponse {
		envoys: x
			.envoys
			.into_iter()
			.map(|v| convert_active_envoy_state_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.collect::<Result<Vec<_>>>()?,
		cursor: x.cursor,
	})
}

pub fn convert_namespace_record_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::NamespaceRecord,
) -> Result<pegboard_actor_v5::NamespaceRecord> {
	Ok(pegboard_actor_v5::NamespaceRecord {
		create_unix_ms: x.create_unix_ms,
		create_operation_id: x.create_operation_id,
	})
}

pub fn convert_namespace_create_request_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::NamespaceCreateRequest,
) -> Result<pegboard_actor_v5::NamespaceCreateRequest> {
	Ok(pegboard_actor_v5::NamespaceCreateRequest {
		name: x.name,
		create_unix_ms: x.create_unix_ms,
	})
}

pub fn convert_namespace_create_response_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::NamespaceCreateResponse,
) -> Result<pegboard_actor_v5::NamespaceCreateResponse> {
	Ok(pegboard_actor_v5::NamespaceCreateResponse {
		created: x.created,
		record: convert_namespace_record_pegboard_actor_v4_to_pegboard_actor_v5(x.record)?,
	})
}

pub fn convert_namespace_read_request_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::NamespaceReadRequest,
) -> Result<pegboard_actor_v5::NamespaceReadRequest> {
	Ok(pegboard_actor_v5::NamespaceReadRequest { name: x.name })
}

pub fn convert_namespace_read_response_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::NamespaceReadResponse,
) -> Result<pegboard_actor_v5::NamespaceReadResponse> {
	Ok(pegboard_actor_v5::NamespaceReadResponse {
		record: x
			.record
			.map(|v| convert_namespace_record_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.transpose()?,
	})
}

pub fn convert_namespace_entry_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::NamespaceEntry,
) -> Result<pegboard_actor_v5::NamespaceEntry> {
	Ok(pegboard_actor_v5::NamespaceEntry {
		name: x.name,
		record: convert_namespace_record_pegboard_actor_v4_to_pegboard_actor_v5(x.record)?,
	})
}

pub fn convert_namespace_list_request_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::NamespaceListRequest,
) -> Result<pegboard_actor_v5::NamespaceListRequest> {
	Ok(pegboard_actor_v5::NamespaceListRequest {
		after: x.after,
		limit: x.limit,
	})
}

pub fn convert_namespace_list_response_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::NamespaceListResponse,
) -> Result<pegboard_actor_v5::NamespaceListResponse> {
	Ok(pegboard_actor_v5::NamespaceListResponse {
		namespaces: x
			.namespaces
			.into_iter()
			.map(|v| convert_namespace_entry_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.collect::<Result<Vec<_>>>()?,
		cursor: x.cursor,
	})
}

pub fn convert_epoxy_actor_by_key_entry_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::EpoxyActorByKeyEntry,
) -> Result<pegboard_actor_v5::EpoxyActorByKeyEntry> {
	Ok(pegboard_actor_v5::EpoxyActorByKeyEntry {
		reservation_id: x.reservation_id,
	})
}

pub fn convert_node_pool_config_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::NodePoolConfig,
) -> Result<pegboard_actor_v5::NodePoolConfig> {
	Ok(pegboard_actor_v5::NodePoolConfig {
		drain_on_version_upgrade: x.drain_on_version_upgrade,
		actor_eviction_delay: x.actor_eviction_delay,
		actor_eviction_period: x.actor_eviction_period,
		actor_eviction_rate: x.actor_eviction_rate,
		metadata: x.metadata,
	})
}

pub fn convert_pool_connection_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::PoolConnection,
) -> Result<pegboard_actor_v5::PoolConnection> {
	Ok(pegboard_actor_v5::PoolConnection {
		connection_id: x.connection_id,
		heartbeat_unix_ms: x.heartbeat_unix_ms,
	})
}

pub fn convert_pool_put_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::PoolPut,
) -> Result<pegboard_actor_v5::PoolPut> {
	Ok(convert_node_pool_config_pegboard_actor_v4_to_pegboard_actor_v5(x)?)
}

pub fn convert_pool_connect_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::PoolConnect,
) -> Result<pegboard_actor_v5::PoolConnect> {
	Ok(convert_pool_connection_pegboard_actor_v4_to_pegboard_actor_v5(x)?)
}

pub fn convert_pool_renew_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::PoolRenew,
) -> Result<pegboard_actor_v5::PoolRenew> {
	Ok(convert_pool_connection_pegboard_actor_v4_to_pegboard_actor_v5(x)?)
}

pub fn convert_pool_operation_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::PoolOperation,
) -> Result<pegboard_actor_v5::PoolOperation> {
	Ok(match x {
		pegboard_actor_v4::PoolOperation::PoolRead => pegboard_actor_v5::PoolOperation::PoolRead,
		pegboard_actor_v4::PoolOperation::PoolPut(v) => pegboard_actor_v5::PoolOperation::PoolPut(
			convert_pool_put_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
		),
		pegboard_actor_v4::PoolOperation::PoolDelete => {
			pegboard_actor_v5::PoolOperation::PoolDelete
		}
		pegboard_actor_v4::PoolOperation::PoolConnect(v) => {
			pegboard_actor_v5::PoolOperation::PoolConnect(
				convert_pool_connect_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
		pegboard_actor_v4::PoolOperation::PoolDisconnect(v) => {
			pegboard_actor_v5::PoolOperation::PoolDisconnect(v)
		}
		pegboard_actor_v4::PoolOperation::PoolRenew(v) => {
			pegboard_actor_v5::PoolOperation::PoolRenew(
				convert_pool_renew_pegboard_actor_v4_to_pegboard_actor_v5(v)?,
			)
		}
	})
}

pub fn convert_pool_status_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::PoolStatus,
) -> Result<pegboard_actor_v5::PoolStatus> {
	Ok(match x {
		pegboard_actor_v4::PoolStatus::Ok => pegboard_actor_v5::PoolStatus::Ok,
		pegboard_actor_v4::PoolStatus::NotFound => pegboard_actor_v5::PoolStatus::NotFound,
		pegboard_actor_v4::PoolStatus::Connected => pegboard_actor_v5::PoolStatus::Connected,
	})
}

pub fn convert_pool_request_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::PoolRequest,
) -> Result<pegboard_actor_v5::PoolRequest> {
	Ok(pegboard_actor_v5::PoolRequest {
		pool_name: x.pool_name,
		operation: convert_pool_operation_pegboard_actor_v4_to_pegboard_actor_v5(x.operation)?,
	})
}

pub fn convert_pool_response_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::PoolResponse,
) -> Result<pegboard_actor_v5::PoolResponse> {
	Ok(pegboard_actor_v5::PoolResponse {
		status: convert_pool_status_pegboard_actor_v4_to_pegboard_actor_v5(x.status)?,
		config: x
			.config
			.map(|v| convert_node_pool_config_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.transpose()?,
	})
}

pub fn convert_pool_lease_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::PoolLease,
) -> Result<pegboard_actor_v5::PoolLease> {
	Ok(pegboard_actor_v5::PoolLease {
		heartbeat_unix_ms: x.heartbeat_unix_ms,
	})
}

pub fn convert_pool_entry_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::PoolEntry,
) -> Result<pegboard_actor_v5::PoolEntry> {
	Ok(pegboard_actor_v5::PoolEntry {
		name: x.name,
		config: convert_node_pool_config_pegboard_actor_v4_to_pegboard_actor_v5(x.config)?,
	})
}

pub fn convert_pool_list_request_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::PoolListRequest,
) -> Result<pegboard_actor_v5::PoolListRequest> {
	Ok(pegboard_actor_v5::PoolListRequest {
		after: x.after,
		limit: x.limit,
	})
}

pub fn convert_pool_list_response_pegboard_actor_v4_to_pegboard_actor_v5(
	x: pegboard_actor_v4::PoolListResponse,
) -> Result<pegboard_actor_v5::PoolListResponse> {
	Ok(pegboard_actor_v5::PoolListResponse {
		pools: x
			.pools
			.into_iter()
			.map(|v| convert_pool_entry_pegboard_actor_v4_to_pegboard_actor_v5(v))
			.collect::<Result<Vec<_>>>()?,
		cursor: x.cursor,
	})
}
