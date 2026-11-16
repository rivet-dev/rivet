//! Downgrades from version 6 to version 5 of the actor family. Version 5 names the namespace in an
//! actor's data, its create, its state and its effects, and has no fork reset, so it refuses a
//! record, create or state without a legacy namespace and a reservation in a phase read without a
//! legacy namespace to name. Version 5 also has no individual actor forks, so it refuses a forked
//! record, create or state and the fork's events and timer, and every backfill move, whose actors
//! are grouped. Types that are the same at both versions downgrade through the generated `From`.
use crate::{
	generated::{pegboard_actor_v5 as v5, pegboard_actor_v6 as v6},
	versioned::{Downgrade, Inexpressible, depot_fs_v2::downgrade_into},
};
use anyhow::Result;

const ACTOR_FORKS: &str = "actor forks";
const BACKFILL_MOVES: &str = "backfill moves, whose actors are grouped";
const GROUPED_ACTORS: &str = "grouped actors, since it names an actor's namespace";

impl Downgrade for v6::ActorData {
	type Older = v5::ActorData;

	fn downgrade(self) -> Result<Self::Older> {
		if self.forked_from.is_some() {
			return Err(Inexpressible::lacks(5, ACTOR_FORKS));
		}
		if self.legacy_moved_from.is_some() {
			return Err(Inexpressible::lacks(5, BACKFILL_MOVES));
		}
		let namespace = self
			.legacy_namespace
			.ok_or_else(|| Inexpressible::lacks(5, GROUPED_ACTORS))?;
		Ok(v5::ActorData {
			actor_id: self.actor_id,
			namespace,
			pool_name: self.pool_name,
			name: self.name,
			key: self.key,
			input: self.input,
			create_unix_ms: self.create_unix_ms,
		})
	}
}

impl Downgrade for v6::ActorCreateData {
	type Older = v5::ActorCreateData;

	fn downgrade(self) -> Result<Self::Older> {
		if self.fork_from.is_some() {
			return Err(Inexpressible::lacks(5, ACTOR_FORKS));
		}
		if self.legacy_move.is_some() {
			return Err(Inexpressible::lacks(5, BACKFILL_MOVES));
		}
		let namespace = self
			.legacy_namespace
			.ok_or_else(|| Inexpressible::lacks(5, GROUPED_ACTORS))?;
		Ok(v5::ActorCreateData {
			actor_id: self.actor_id,
			namespace,
			pool_name: self.pool_name,
			name: self.name,
			key: self.key,
			input: self.input,
		})
	}
}

/// Downgrades `phase` of an actor whose record names `legacy_namespace`. Version 5's reservations
/// name the actor's namespace, which version 6 keeps only on a legacy record; a grouped record has
/// none, and only binaries that read version 6 ever read one, so its reservation is refused. A
/// fork in progress is refused too: version 5 has no actor forks.
fn downgrade_phase(
	phase: v6::ActorPhase,
	legacy_namespace: Option<vbare::Str>,
) -> Result<v5::ActorPhase> {
	Ok(match phase {
		v6::ActorPhase::Initializing(v) => {
			if v.fork_database.is_some() {
				return Err(Inexpressible::lacks(5, ACTOR_FORKS));
			}
			if v.legacy_move.is_some() {
				return Err(Inexpressible::lacks(5, BACKFILL_MOVES));
			}
			let namespace =
				legacy_namespace.ok_or_else(|| Inexpressible::lacks(5, GROUPED_ACTORS))?;
			v5::ActorPhase::Initializing(v5::Initializing {
				reserve_epoxy: downgrade_reservation(v.reserve_epoxy, namespace),
				allocation: v.allocation.map(Into::into),
			})
		}
		v6::ActorPhase::Starting(v) => v5::ActorPhase::Starting(v.into()),
		v6::ActorPhase::Running(v) => v5::ActorPhase::Running(v.into()),
		v6::ActorPhase::Stopping(v) => v5::ActorPhase::Stopping(v.into()),
		v6::ActorPhase::Sleeping(v) => v5::ActorPhase::Sleeping(v.into()),
		v6::ActorPhase::Destroying(v) => v5::ActorPhase::Destroying(v.into()),
		v6::ActorPhase::Destroyed => v5::ActorPhase::Destroyed,
		v6::ActorPhase::MigratingFromV2(v) => {
			let reserve_epoxy = match v.reserve_epoxy {
				Some(reservation) => {
					let namespace =
						legacy_namespace.ok_or_else(|| Inexpressible::lacks(5, GROUPED_ACTORS))?;
					Some(downgrade_reservation(reservation, namespace))
				}
				None => None,
			};
			v5::ActorPhase::MigratingFromV2(v5::MigratingFromV2 {
				migration_id: v.migration_id,
				reserve_epoxy,
				alarm_due: v.alarm_due,
			})
		}
		v6::ActorPhase::DelegatedToV2 => v5::ActorPhase::DelegatedToV2,
	})
}

/// Version 5's reservation data, which names the actor's `namespace`.
fn downgrade_reservation(
	reservation: v6::ReserveEpoxyEffectData,
	namespace: vbare::Str,
) -> v5::ReserveEpoxyEffectData {
	v5::ReserveEpoxyEffectData {
		namespace,
		name: reservation.name,
		key: reservation.key,
		state: reservation.state.into(),
	}
}

impl Downgrade for v6::ActorState {
	type Older = v5::ActorState;

	/// Downgrades the state of an ungrouped actor, whose legacy namespace fills version 5's
	/// index effect and reservations. Refuses a grouped actor's state, which keeps no namespace,
	/// and a state version 5 cannot carry: one reset for a fork, one with a forked database or
	/// a failed fork, and one the backfill marked.
	fn downgrade(self) -> Result<Self::Older> {
		let namespace = self
			.legacy_namespace
			.ok_or_else(|| Inexpressible::lacks(5, GROUPED_ACTORS))?;
		if self.fork_reset.is_some() {
			return Err(Inexpressible::lacks(5, ACTOR_FORKS));
		}
		if !matches!(self.database_origin, v6::ActorDatabaseOrigin::DatabaseEmpty) {
			return Err(Inexpressible::lacks(5, ACTOR_FORKS));
		}
		if self.legacy_move.is_some() {
			return Err(Inexpressible::lacks(5, BACKFILL_MOVES));
		}

		Ok(v5::ActorState {
			phase: downgrade_phase(self.phase, Some(namespace.clone()))?,
			epoch: self.epoch,
			create_unix_ms: self.create_unix_ms,
			storage_origin: self.storage_origin.into(),
			command_index: self.command_index,
			index_bucket_count: self.index_bucket_count,
			pending_action: self.pending_action.into(),
			write_index: v5::WriteIndexEffectData {
				namespace,
				name: self.write_index.name,
				create_unix_ms: self.write_index.create_unix_ms,
				state: self.write_index.state.into(),
			},
			start_waiters: self.start_waiters.into_iter().map(Into::into).collect(),
		})
	}
}

impl Downgrade for v6::ActorMigrateFromV2BeginResponse {
	type Older = v5::ActorMigrateFromV2BeginResponse;

	/// Refuses only a grouped actor's begin, whose reservation version 5 cannot name.
	fn downgrade(self) -> Result<Self::Older> {
		Ok(v5::ActorMigrateFromV2BeginResponse {
			phase: downgrade_phase(self.phase, self.legacy_namespace)?,
			create_unix_ms: self.create_unix_ms,
		})
	}
}

impl Downgrade for v6::ActorCreateRequest {
	type Older = v5::ActorCreateRequest;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(v5::ActorCreateRequest {
			init: self.init.downgrade()?,
			waiter: self.waiter.map(Into::into),
			force_actor_v3: self.force_actor_v3,
		})
	}
}

impl Downgrade for v6::Create {
	type Older = v5::Create;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(v5::Create {
			init: self.init.downgrade()?,
			waiter: self.waiter.map(Into::into),
			force_actor_v3: self.force_actor_v3,
		})
	}
}

impl Downgrade for v6::MigrateFromV2Begin {
	type Older = v5::MigrateFromV2Begin;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(v5::MigrateFromV2Begin {
			migration_id: self.migration_id,
			init: self.init.downgrade()?,
			create_unix_ms: self.create_unix_ms,
		})
	}
}

impl Downgrade for v6::MigrateFromV2 {
	type Older = v5::MigrateFromV2;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::MigrateFromV2Begin(v) => v5::MigrateFromV2::MigrateFromV2Begin(v.downgrade()?),
			Self::MigrateFromV2Activate(v) => v5::MigrateFromV2::MigrateFromV2Activate(v.into()),
			Self::MigrateFromV2Discard(v) => v5::MigrateFromV2::MigrateFromV2Discard(v.into()),
			Self::MigrateFromV2Recall(v) => v5::MigrateFromV2::MigrateFromV2Recall(v.into()),
		})
	}
}

impl Downgrade for v6::ActorEvent {
	type Older = v5::ActorEvent;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::ReserveEpoxyState(v) => v5::ActorEvent::ReserveEpoxyState(v.into()),
			Self::WriteIndexState(v) => v5::ActorEvent::WriteIndexState(v.into()),
			Self::StartFailed(v) => v5::ActorEvent::StartFailed(v.into()),
			Self::Sleep => v5::ActorEvent::Sleep,
			Self::TryStart(v) => v5::ActorEvent::TryStart(v.into()),
			Self::Create(v) => v5::ActorEvent::Create(v.downgrade()?),
			Self::Reallocate(v) => v5::ActorEvent::Reallocate(v.into()),
			Self::UpdateRoute(v) => v5::ActorEvent::UpdateRoute(v.into()),
			Self::Destroy => v5::ActorEvent::Destroy,
			Self::DestroyState(v) => v5::ActorEvent::DestroyState(v.into()),
			Self::ToEnvoyEvent(v) => v5::ActorEvent::ToEnvoyEvent(v.into()),
			Self::EnvoyGone(v) => v5::ActorEvent::EnvoyGone(v.into()),
			Self::MigrateFromV2(v) => v5::ActorEvent::MigrateFromV2(v.downgrade()?),
			Self::ForkDatabaseState(_) | Self::ForkDatabaseRefused(_) => {
				return Err(Inexpressible::lacks(5, ACTOR_FORKS));
			}
			Self::LegacyMoveConfirm(_)
			| Self::LegacyMarkMoving(_)
			| Self::LegacyClearMoving(_)
			| Self::LegacyMoveDestroy(_)
			| Self::LegacyMoveAbandon(_) => {
				return Err(Inexpressible::lacks(5, BACKFILL_MOVES));
			}
		})
	}
}

impl Downgrade for v6::ActorTimer {
	type Older = v5::ActorTimer;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::ReserveEpoxyTimeout(v) => v5::ActorTimer::ReserveEpoxyTimeout(v.into()),
			Self::WriteIndexTimeout(v) => v5::ActorTimer::WriteIndexTimeout(v.into()),
			Self::DestroyTimeout(v) => v5::ActorTimer::DestroyTimeout(v.into()),
			Self::StartTimeout(v) => v5::ActorTimer::StartTimeout(v.into()),
			Self::AlarmTimeout => v5::ActorTimer::AlarmTimeout,
			Self::StopTimeout(v) => v5::ActorTimer::StopTimeout(v.into()),
			Self::EnvoyCheckTimeout(v) => v5::ActorTimer::EnvoyCheckTimeout(v.into()),
			Self::StopThresholdTimeout(v) => v5::ActorTimer::StopThresholdTimeout(v.into()),
			Self::AllocationRetry(v) => v5::ActorTimer::AllocationRetry(v.into()),
			Self::ForkDatabaseTimeout(_) | Self::ForkDatabaseRetry(_) => {
				return Err(Inexpressible::lacks(5, ACTOR_FORKS));
			}
			Self::LegacyMovingTimeout(_) => {
				return Err(Inexpressible::lacks(5, BACKFILL_MOVES));
			}
		})
	}
}

impl Downgrade for v6::ActorReadResponse {
	type Older = v5::ActorReadResponse;

	/// Drops the database origin and command index, which version 5 does not report, and the
	/// backfill's mark and index write phase: the backfill starts only once no version 5 binary
	/// runs, so no version 5 reader acts on a moving actor, and a version 5 migrator settles its
	/// activations without the phase, as it always did. A
	/// reservation in the phase takes its version 5 namespace from the record's legacy
	/// namespace; see [`downgrade_phase`].
	fn downgrade(self) -> Result<Self::Older> {
		let legacy_namespace = self
			.data
			.as_ref()
			.and_then(|data| data.legacy_namespace.clone());
		Ok(v5::ActorReadResponse {
			data: self.data.map(Downgrade::downgrade).transpose()?,
			phase: self
				.phase
				.map(|phase| downgrade_phase(phase, legacy_namespace))
				.transpose()?,
			epoch: self.epoch,
		})
	}
}

/// Downgrades a wake's outcome. Version 5 has no backfill moves, so the outcome of a wake of a
/// moving actor is refused rather than read as a wait. A caller of a previous release never sees
/// it, because the backfill starts only once every storage worker reads version 6; a caller of
/// this release that sealed its request at version 5 or older gets a retryable refusal and asks
/// again once the fleet's read floor rises.
fn downgrade_try_start_outcome(outcome: v6::TryStartOutcome) -> Result<v5::TryStartOutcome> {
	Ok(match outcome {
		v6::TryStartOutcome::TryStartLegacyMoving => {
			return Err(Inexpressible::lacks(5, BACKFILL_MOVES));
		}
		v6::TryStartOutcome::TryStartWaiting => v5::TryStartOutcome::TryStartWaiting,
		v6::TryStartOutcome::TryStartStarting(v) => v5::TryStartOutcome::TryStartStarting(v.into()),
		v6::TryStartOutcome::TryStartRunning(v) => v5::TryStartOutcome::TryStartRunning(v.into()),
		v6::TryStartOutcome::TryStartDestroyed => v5::TryStartOutcome::TryStartDestroyed,
		v6::TryStartOutcome::TryStartEpoxyAlreadyReserved(v) => {
			v5::TryStartOutcome::TryStartEpoxyAlreadyReserved(v.into())
		}
		v6::TryStartOutcome::TryStartFull => v5::TryStartOutcome::TryStartFull,
		v6::TryStartOutcome::TryStartMigratingFromV2 => {
			v5::TryStartOutcome::TryStartMigratingFromV2
		}
		v6::TryStartOutcome::TryStartDelegatedToV2 => v5::TryStartOutcome::TryStartDelegatedToV2,
	})
}

impl Downgrade for v6::ActorCreateResponse {
	type Older = v5::ActorCreateResponse;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(v5::ActorCreateResponse {
			outcome: downgrade_try_start_outcome(self.outcome)?,
		})
	}
}

impl Downgrade for v6::ActorEventResponse {
	type Older = v5::ActorEventResponse;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(v5::ActorEventResponse {
			outcome: self.outcome.map(downgrade_try_start_outcome).transpose()?,
		})
	}
}

/// Version 5 has no start notice of a finished database fork. One is never sealed for a reader
/// of version 5: only a fork create registers the waiter that needs one, and only a worker
/// reading version 6 runs that create's wait.
impl Downgrade for v6::ActorStartNoticeKind {
	type Older = v5::ActorStartNoticeKind;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(match self {
			Self::ActorStartNoticeStarting(starting) => {
				v5::ActorStartNoticeKind::ActorStartNoticeStarting(starting.into())
			}
			Self::ActorStartNoticeRunning(running) => {
				v5::ActorStartNoticeKind::ActorStartNoticeRunning(running.into())
			}
			Self::ActorStartNoticeEpoxyAlreadyReserved(reserved) => {
				v5::ActorStartNoticeKind::ActorStartNoticeEpoxyAlreadyReserved(reserved.into())
			}
			Self::ActorStartNoticeDestroyed => v5::ActorStartNoticeKind::ActorStartNoticeDestroyed,
			Self::ActorStartNoticeMigratingFromV2 => {
				v5::ActorStartNoticeKind::ActorStartNoticeMigratingFromV2
			}
			Self::ActorStartNoticeDelegatedToV2 => {
				v5::ActorStartNoticeKind::ActorStartNoticeDelegatedToV2
			}
			Self::ActorStartNoticeForked => {
				return Err(Inexpressible::lacks(
					3,
					"the start notice of a finished database fork",
				));
			}
		})
	}
}

impl Downgrade for v6::ActorStartNotice {
	type Older = v5::ActorStartNotice;

	fn downgrade(self) -> Result<Self::Older> {
		Ok(v5::ActorStartNotice {
			actor_id: self.actor_id,
			kind: self.kind.downgrade()?,
		})
	}
}

// Everything else is the same at both versions and downgrades through the generated `From`.
downgrade_into! {
	v6::NamespaceRecord => v5::NamespaceRecord,
	v6::NamespaceCreateRequest => v5::NamespaceCreateRequest,
	v6::NamespaceCreateResponse => v5::NamespaceCreateResponse,
	v6::NamespaceReadResponse => v5::NamespaceReadResponse,
	v6::NamespaceListResponse => v5::NamespaceListResponse,
	v6::IndexInsertActorRequest => v5::IndexInsertActorRequest,
	v6::IndexDeleteActorRequest => v5::IndexDeleteActorRequest,
	v6::IndexListActorsRequest => v5::IndexListActorsRequest,
	v6::IndexListActorsResponse => v5::IndexListActorsResponse,
	v6::DestroyedActorIndexEntry => v5::DestroyedActorIndexEntry,
	v6::IndexReadDestroyedActorRequest => v5::IndexReadDestroyedActorRequest,
	v6::IndexReadDestroyedActorResponse => v5::IndexReadDestroyedActorResponse,
	v6::IndexListEnvoysRequest => v5::IndexListEnvoysRequest,
	v6::IndexListEnvoysResponse => v5::IndexListEnvoysResponse,
	v6::EpoxyActorByKeyEntry => v5::EpoxyActorByKeyEntry,
	v6::KvValue => v5::KvValue,
	v6::EnvoyData => v5::EnvoyData,
	v6::EnvoyUpsertRequest => v5::EnvoyUpsertRequest,
	v6::EnvoyUpsertResponse => v5::EnvoyUpsertResponse,
	v6::EnvoyReadResponse => v5::EnvoyReadResponse,
	v6::IndexInsertEnvoyRequest => v5::IndexInsertEnvoyRequest,
	v6::IndexUpsertActiveEnvoyRequest => v5::IndexUpsertActiveEnvoyRequest,
	v6::IndexUpsertActiveEnvoyResponse => v5::IndexUpsertActiveEnvoyResponse,
	v6::IndexDeleteActiveEnvoyRequest => v5::IndexDeleteActiveEnvoyRequest,
	v6::IndexPutEnvoyPoolRequest => v5::IndexPutEnvoyPoolRequest,
	v6::IndexListEnvoyPoolsResponse => v5::IndexListEnvoyPoolsResponse,
	v6::IndexListActiveEnvoysRequest => v5::IndexListActiveEnvoysRequest,
	v6::IndexListActiveEnvoysResponse => v5::IndexListActiveEnvoysResponse,
	v6::IndexListActiveEnvoyStatesRequest => v5::IndexListActiveEnvoyStatesRequest,
	v6::IndexListActiveEnvoyStatesResponse => v5::IndexListActiveEnvoyStatesResponse,
	v6::ActiveEnvoyIndexEntry => v5::ActiveEnvoyIndexEntry,
	v6::ActiveEnvoyState => v5::ActiveEnvoyState,
	v6::NamespaceReadRequest => v5::NamespaceReadRequest,
	v6::NamespaceListRequest => v5::NamespaceListRequest,
	v6::NodePoolConfig => v5::NodePoolConfig,
	v6::PoolRequest => v5::PoolRequest,
	v6::PoolResponse => v5::PoolResponse,
	v6::PoolLease => v5::PoolLease,
	v6::PoolListRequest => v5::PoolListRequest,
	v6::PoolListResponse => v5::PoolListResponse,
}
