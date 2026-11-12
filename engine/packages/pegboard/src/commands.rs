//! Routed commands execute under a shared partition mutex and acknowledge durability.
use crate::actor_machine::{ActorStateMachine, Event, State, executor::Executor};
use crate::errors::MigrateFromV2Unsupported;
use crate::keys;
use crate::keys::kv::at_write_version;
use crate::wire;
use anyhow::{Context, Result, bail, ensure};
use depot_protocol::command::{Command, CommandTag};
use depot_worker::command::{KvHandle, Transaction};
use rivet_data::families::VersionFamily;
use rivet_engine_context::EngineContext;
use rivet_keys::keys::{
	KeyAddress, SubspaceRange,
	codec::FormalKey,
	tags::depot_command::{
		LEGACY_PEGBOARD_ACTOR_ALARM_READ, PEGBOARD_ACTOR_ALARM, PEGBOARD_ACTOR_CREATE,
		PEGBOARD_ACTOR_EVENT, PEGBOARD_ACTOR_MIGRATE_FROM_V2_BEGIN, PEGBOARD_ACTOR_READ,
		PEGBOARD_ENVOY_READ, PEGBOARD_ENVOY_UPSERT,
	},
};
use rivet_state_machine_durable::DurableStateMachine;
use rivet_state_machine_durable::Method;
use rivet_util::Id;
use std::sync::Arc;

pub struct ActorCreate;

impl Command for ActorCreate {
	const TAG: CommandTag = CommandTag(PEGBOARD_ACTOR_CREATE);
	/// A create of an existing actor applies as a wake that keeps its own init, and
	/// `has_start_waiter` deduplicates the waiter.
	const IDEMPOTENT_ANSWER: bool = true;

	const CREATE_IF_MISSING: bool = true;
	const SPECULATIVE_CREATE: bool = true;
	type Request = rivet_data::versioned::ActorCreateRequest;
	type Response = rivet_data::versioned::ActorCreateResponse;
}

pub struct ActorRead;

impl Command for ActorRead {
	const TAG: CommandTag = CommandTag(PEGBOARD_ACTOR_READ);
	const READ_ONLY: bool = true;
	const IDEMPOTENT_ANSWER: bool = true;

	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::ActorReadRequest;
	type Response = rivet_data::versioned::ActorReadResponse;
}

pub struct ActorEvent;

impl Command for ActorEvent {
	const TAG: CommandTag = CommandTag(PEGBOARD_ACTOR_EVENT);
	/// Only wakes (`TryStart`, `Create`), which store no second waiter, and `Reallocate`, which
	/// its epoch fences, answer an outcome version 1 cannot express.
	const IDEMPOTENT_ANSWER: bool = true;

	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::ActorEvent;
	type Response = rivet_data::versioned::ActorEventResponse;
}

/// Begins a V2 migration of the actor, creating its state volume if it has none, and answers
/// the phase and incarnation the begin left. Only the V2 migrator sends it; its later events go
/// through `ActorEvent`. A repeated begin finds the actor already held under its own id.
pub struct ActorMigrateFromV2Begin;

impl Command for ActorMigrateFromV2Begin {
	const TAG: CommandTag = CommandTag(PEGBOARD_ACTOR_MIGRATE_FROM_V2_BEGIN);

	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::MigrateFromV2Begin;
	type Response = rivet_data::versioned::ActorMigrateFromV2BeginResponse;
}

/// The first `pegboard.actor` version that can hold an actor for the V2 migrator.
const MIGRATE_FROM_V2_ACTOR_VERSION: u16 = 2;

/// Turns a create that released `pegboard.actor` v3 cannot hold, such as one with
/// `forceActorV3` while readers on 3.0.2 are live, into the retryable
/// [`MigrateFromV2Unsupported`] a host on this binary returns for the same request. Every other
/// error passes through. Senders of [`ActorCreate`] apply it to the command's result.
///
/// Exists only for 3.0.2 readers; see `rivet_data::versioned::legacy_v3`. Without it the sender
/// reports an unstructured conversion error. Delete it with that module, once no reader of
/// released v3 can be live.
pub fn legacy_v3_create_refusal(error: anyhow::Error) -> anyhow::Error {
	let unrepresentable = error.chain().any(|cause| {
		cause
			.downcast_ref::<rivet_data::versioned::legacy_v3::LegacyV3Unrepresentable>()
			.is_some()
	});
	if !unrepresentable {
		return error;
	}
	MigrateFromV2Unsupported {
		write_version: 3,
		required_version: rivet_data::versioned::V2_DELEGATION_ACTOR_VERSION,
	}
	.build()
}

/// Legacy: reads when the actor's scheduled alarm fires; see `legacy_alarm_read_handler`.
/// Removed in 3.1 with the backfill.
pub struct LegacyActorAlarmRead;

impl Command for LegacyActorAlarmRead {
	const TAG: CommandTag = CommandTag(LEGACY_PEGBOARD_ACTOR_ALARM_READ);
	const READ_ONLY: bool = true;

	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::LegacyActorAlarmReadRequest;
	type Response = rivet_data::versioned::LegacyActorAlarmReadResponse;
}

pub fn is_volume_gone(error: &anyhow::Error) -> bool {
	let structured = rivet_error::RivetError::extract(error);
	structured.group() == "depot"
		&& matches!(structured.code(), "volume_not_found" | "volume_deleted")
}

/// Delivers a due depot alarm to the actor's state machine, which fires the timers it covers and
/// acknowledges the alarm revision in the same commit. Depot retries an alarm until a commit
/// acknowledges it, so a lost or failed delivery is safe.
pub struct ActorAlarm;

impl Command for ActorAlarm {
	const TAG: CommandTag = CommandTag(PEGBOARD_ACTOR_ALARM);

	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::ActorAlarmRequestData;
	type Response = rivet_data::versioned::ActorEventResponse;
}

pub struct EnvoyUpsert;

impl Command for EnvoyUpsert {
	const TAG: CommandTag = CommandTag(PEGBOARD_ENVOY_UPSERT);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::EnvoyUpsertRequest;
	type Response = rivet_data::versioned::EnvoyUpsertResponse;
}

pub struct EnvoyRead;

impl Command for EnvoyRead {
	const TAG: CommandTag = CommandTag(PEGBOARD_ENVOY_READ);
	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::EnvoyReadRequest;
	type Response = rivet_data::versioned::EnvoyReadResponse;
}

pub fn register(
	registry: &mut depot_worker::command::CommandRegistry,
	ctx: &EngineContext,
) -> Result<()> {
	let concurrency = &ctx.config().pegboard().command_concurrency;
	let ctx2 = ctx.clone();
	registry
		.register_with_concurrency::<ActorCreate>(concurrency.actor(), move |db, request| {
			actor_create_handler(ctx2.clone(), db, request)
		})?;
	let ctx2 = ctx.clone();
	registry.register_snapshot_read::<ActorRead>(
		concurrency.actor_read(),
		None,
		move |db, request| actor_read_handler(ctx2.clone(), db, request),
	)?;
	let ctx2 = ctx.clone();
	registry.register_with_concurrency::<ActorEvent>(
		concurrency.actor_event(),
		move |db, request| actor_event_handler(ctx2.clone(), db, request),
	)?;

	let ctx2 = ctx.clone();
	registry.register_with_concurrency::<ActorMigrateFromV2Begin>(
		concurrency.actor_event(),
		move |db, request| actor_migrate_from_v2_begin_handler(ctx2.clone(), db, request),
	)?;

	let ctx2 = ctx.clone();
	registry.register_with_concurrency::<ActorAlarm>(
		concurrency.actor_event(),
		move |db, request| actor_alarm_handler(ctx2.clone(), db, request),
	)?;

	registry.register_snapshot_read::<LegacyActorAlarmRead>(
		concurrency.actor_read(),
		None,
		|db, request| legacy_alarm_read_handler(db, request),
	)?;

	registry.register::<EnvoyUpsert>(|db, request| envoy_upsert_handler(db, request))?;
	registry.register::<EnvoyRead>(|db, request| envoy_read_handler(db, request))?;

	Ok(())
}

/// Creates the actor if its state volume is new and applies a `Create` either way: a new actor
/// or a destroyed keyed actor starts its next incarnation from `init`, and an existing actor
/// is woken as by `TryStart`, keeping its own `init` including its input.
async fn actor_create_handler(
	ctx: EngineContext,
	db: KvHandle,
	request: wire::ActorCreateRequest,
) -> Result<wire::ActorCreateResponse> {
	let (group, actor_id) = keys::partition::ActorVolume::parse(db.volume_key())?;
	// A keyed create in a namespace migrating in from V2 may delegate its key to V2, which
	// only the version that stores a delegated actor can write.
	if request.init.key.is_some() {
		let dynamic = ctx.config().dynamic();
		if let Some(config) = dynamic.migrate_from_v2() {
			let namespace = match group {
				Some(group) => crate::catalog::group_namespace(&ctx, group).await?,
				None => request
					.init
					.legacy_namespace
					.as_ref()
					.map(|namespace| namespace.to_string())
					.context("an unprefixed actor's create has no legacy namespace")?,
			};
			if crate::migrate_from_v2::applies(Some(config), &namespace).is_some() {
				ensure_actor_version(&db, rivet_data::versioned::V2_DELEGATION_ACTOR_VERSION)?;
			}
		}
	}

	let State(state) = rivet_state_machine_durable::Runner::process_event(
		&ctx,
		db.clone(),
		Method::GetOrCreate {
			state: default_state(&ctx, &request.init),
		},
		Arc::new(Executor::new(&ctx, db, group, actor_id)),
		Event(wire::ActorEvent::Create(wire::Create {
			init: request.init,
			waiter: request.waiter.clone(),
			force_actor_v3: request.force_actor_v3,
		})),
	)
	.await?;

	Ok(wire::ActorCreateResponse {
		outcome: try_start_outcome(&state, request.waiter.as_ref()),
	})
}

/// The state of an actor volume that holds none yet: a destroyed actor that has never run, so a
/// brand-new actor and a recreated one take the same path. The event that creates it commits
/// with it, so nothing observes it.
fn default_state(ctx: &EngineContext, init: &wire::ActorCreateData) -> State {
	let pegboard_config = ctx.config().pegboard();
	let placeholder_write_index = wire::WriteIndexEffectData {
		name: init.name.clone(),
		create_unix_ms: 0,
		state: wire::WriteIndexState {
			phase: wire::WriteIndexPhase::WriteIndexPhasePending,
			attempt: 0,
			create_unix_ms: 0,
		},
	};

	State(wire::ActorState {
		phase: wire::ActorPhase::Destroyed,
		epoch: 0,
		create_unix_ms: 0,
		storage_origin: wire::StorageOrigin::Created,
		command_index: 0,
		index_bucket_count: pegboard_config.actor_index_buckets_per_name(),
		pending_action: wire::PendingAction::PendingNone,
		write_index: placeholder_write_index,
		start_waiters: Vec::new(),
		fork_reset: None,
		database_origin: wire::ActorDatabaseOrigin::DatabaseEmpty,
		legacy_move: None,
		legacy_namespace: init.legacy_namespace.clone(),
	})
}

/// Holds the actor for the V2 migrator, creating its state volume if it has none. The begin is
/// refused before anything is written while the cluster cannot store a held actor.
async fn actor_migrate_from_v2_begin_handler(
	ctx: EngineContext,
	db: KvHandle,
	request: wire::MigrateFromV2Begin,
) -> Result<wire::ActorMigrateFromV2BeginResponse> {
	let (group, actor_id) = keys::partition::ActorVolume::parse(db.volume_key())?;
	ensure_actor_version(&db, MIGRATE_FROM_V2_ACTOR_VERSION)?;

	let State(state) = rivet_state_machine_durable::Runner::process_event(
		&ctx,
		db.clone(),
		Method::GetOrCreate {
			state: default_state(&ctx, &request.init),
		},
		Arc::new(Executor::new(&ctx, db, group, actor_id)),
		Event(wire::ActorEvent::MigrateFromV2(
			wire::MigrateFromV2::MigrateFromV2Begin(request),
		)),
	)
	.await?;

	Ok(wire::ActorMigrateFromV2BeginResponse {
		phase: state.phase,
		create_unix_ms: state.create_unix_ms,
		legacy_namespace: state.legacy_namespace,
	})
}

/// Refuses a V2 migration event while some live storage worker reads only actor state that
/// cannot represent what the event may write: a held actor for the migrator, or a delegated one
/// for a create in a migrating namespace. The caller retries on its own deadline.
fn ensure_actor_version(db: &KvHandle, required_version: u16) -> Result<()> {
	let write_version = db.write_version(VersionFamily::PEGBOARD_ACTOR)?;
	if write_version < required_version {
		return Err(MigrateFromV2Unsupported {
			write_version,
			required_version,
		}
		.build());
	}

	Ok(())
}

fn try_start_outcome(
	state: &wire::ActorState,
	waiter: Option<&wire::StartWaiter>,
) -> wire::TryStartOutcome {
	// Legacy: a wake of an actor the backfill is moving started nothing; a running one still
	// serves until the backfill's sleep stops it. Removed in 3.1 with the backfill.
	let starts_nothing = match &state.phase {
		wire::ActorPhase::Initializing(_)
		| wire::ActorPhase::Starting(_)
		| wire::ActorPhase::Stopping(_)
		| wire::ActorPhase::Sleeping(_) => true,
		// A held actor is V2's to serve, which its own outcome below says.
		wire::ActorPhase::Running(_)
		| wire::ActorPhase::Destroying(_)
		| wire::ActorPhase::Destroyed
		| wire::ActorPhase::MigratingFromV2(_)
		| wire::ActorPhase::DelegatedToV2 => false,
	};
	if crate::actor_machine::legacy_moving(state) && starts_nothing {
		return wire::TryStartOutcome::TryStartLegacyMoving;
	}
	match &state.phase {
		wire::ActorPhase::Starting(starting) => {
			if waiter.is_some_and(|waiter| !crate::actor_machine::has_start_waiter(state, waiter)) {
				return wire::TryStartOutcome::TryStartFull;
			}
			wire::TryStartOutcome::TryStartStarting(wire::TryStartStarting {
				epoch: state.epoch,
				allocation: starting.allocation.clone(),
			})
		}
		wire::ActorPhase::Running(running) => {
			wire::TryStartOutcome::TryStartRunning(wire::TryStartRunning {
				epoch: state.epoch,
				allocation: running.allocation.clone(),
			})
		}
		wire::ActorPhase::Destroying(destroying) => match &destroying.key_owner {
			Some(wire::KeyOwner::KeyOwnerDatacenter(owner)) => {
				wire::TryStartOutcome::TryStartEpoxyAlreadyReserved(
					wire::TryStartEpoxyAlreadyReserved {
						reservation_id: owner.reservation_id.clone(),
					},
				)
			}
			Some(wire::KeyOwner::KeyOwnerV2) => wire::TryStartOutcome::TryStartDelegatedToV2,
			None => wire::TryStartOutcome::TryStartDestroyed,
		},
		wire::ActorPhase::Destroyed => wire::TryStartOutcome::TryStartDestroyed,
		wire::ActorPhase::MigratingFromV2(_) => wire::TryStartOutcome::TryStartMigratingFromV2,
		wire::ActorPhase::DelegatedToV2 => wire::TryStartOutcome::TryStartDelegatedToV2,
		wire::ActorPhase::Initializing(_)
		| wire::ActorPhase::Stopping(_)
		| wire::ActorPhase::Sleeping(_) => {
			if waiter.is_some_and(|waiter| !crate::actor_machine::has_start_waiter(state, waiter)) {
				return wire::TryStartOutcome::TryStartFull;
			}
			wire::TryStartOutcome::TryStartWaiting
		}
	}
}

async fn actor_read_handler(
	_ctx: EngineContext,
	db: KvHandle,
	_request: (),
) -> Result<wire::ActorReadResponse> {
	keys::partition::ActorVolume::parse(db.volume_key())?;

	let data_key = keys::kv::ActorDataKey::new();
	let state_key = rivet_state_machine_durable::keys::kv::MachineStateKey::new();

	let mut snapshot = db.snapshot().await?;
	let data = match snapshot.get(&data_key.pack()).await? {
		Some(data) => Some(data_key.deserialize(data.into())?),
		None => None,
	};
	// An actor delegated to V2 has state but no data record.
	let Some(state) = snapshot.get(&state_key.pack()).await? else {
		ensure!(data.is_none(), "should have machine state if data exists");
		return Ok(wire::ActorReadResponse {
			data: None,
			phase: None,
			epoch: 0,
			database_origin: None,
			command_index: None,
			legacy_move: None,
			legacy_write_index_phase: None,
		});
	};
	let state = ActorStateMachine::deserialize_state(&state)?.0;
	// An inherited state that was never reset for this volume reads as the reset would leave
	// it, so a reader never routes to the parent's envoy. Nothing is persisted.
	let state = match db.volume_record().pending_fork_reset(state.fork_reset) {
		Some(incarnation) => crate::actor_machine::reset_view(&state, incarnation),
		None => state,
	};

	Ok(wire::ActorReadResponse {
		data,
		phase: Some(state.phase),
		epoch: state.epoch,
		database_origin: Some(state.database_origin),
		command_index: Some(state.command_index),
		legacy_move: state.legacy_move,
		legacy_write_index_phase: Some(state.write_index.state.phase),
	})
}

/// Legacy: when the alarm the backfill carries into the actor's moved copy fires, or `None` with
/// none: the earlier of the alarm armed in the machine's timer row and an alarm that came due
/// while the actor was marked moving, which its mark keeps instead of arming
/// ([`crate::actor_machine::legacy_carried_alarm`]). Both are read from one snapshot, so an
/// alarm that fires between two reads, moving from the timer row into the mark, is never missed.
/// The legacy behavior kept an actor's alarm only in its own state volume; the backfill reads it
/// because the move creates a new state volume. Taken only by the backfill of a namespace from
/// before volume groups. Removed in 3.1 with the backfill.
async fn legacy_alarm_read_handler(
	db: KvHandle,
	_request: (),
) -> Result<wire::LegacyActorAlarmReadResponse> {
	keys::partition::ActorVolume::parse(db.volume_key())?;
	let timer_id = ActorStateMachine::timer_id(&wire::ActorTimer::AlarmTimeout);
	let timer_key =
		rivet_state_machine_durable::keys::kv::MachineTimerByIdKey::new(timer_id.into());
	let state_key = rivet_state_machine_durable::keys::kv::MachineStateKey::new();

	let mut snapshot = db.snapshot().await?;
	let armed_unix_ms = match snapshot.get(&timer_key.pack()).await? {
		Some(entry) => Some(timer_key.deserialize(entry.into())?.deadline_unix_ms),
		None => legacy_row_alarm_unix_ms(&mut snapshot).await?,
	};
	let alarm_unix_ms = match snapshot.get(&state_key.pack()).await? {
		Some(state) => {
			let state = ActorStateMachine::deserialize_state(&state)?.0;
			crate::actor_machine::legacy_carried_alarm(&state, armed_unix_ms)
		}
		None => armed_unix_ms,
	};
	Ok(wire::LegacyActorAlarmReadResponse { alarm_unix_ms })
}

/// Legacy: the alarm armed in a timer row from before timers had stable ids, which stays there
/// until the actor's next commit moves it. An actor holds a few timers at most. Removed in 3.1
/// with the backfill.
async fn legacy_row_alarm_unix_ms(snapshot: &mut Transaction) -> Result<Option<i64>> {
	let range = rivet_state_machine_durable::keys::kv::LegacyMachineTimerPrefix::new().range();
	let rows = snapshot
		.scan(
			(
				std::ops::Bound::Included(range.start.as_slice()),
				std::ops::Bound::Excluded(range.end.as_slice()),
			),
			None,
			false,
		)
		.await?;
	for (row, _) in rows {
		let row = rivet_state_machine_durable::keys::kv::LegacyMachineTimerKey::unpack(&row)?;
		let timer = ActorStateMachine::deserialize_timer(&row.timer)?;
		if matches!(timer, wire::ActorTimer::AlarmTimeout) {
			return Ok(Some(row.deadline_unix_ms.0));
		}
	}
	Ok(None)
}

async fn actor_event_handler(
	ctx: EngineContext,
	db: KvHandle,
	event: wire::ActorEvent,
) -> Result<wire::ActorEventResponse> {
	let (group, actor_id) = keys::partition::ActorVolume::parse(db.volume_key())?;
	if let wire::ActorEvent::MigrateFromV2(_) = &event {
		ensure_actor_version(&db, MIGRATE_FROM_V2_ACTOR_VERSION)?;
	}

	let waiter = match &event {
		wire::ActorEvent::TryStart(try_start) => Some(try_start.waiter.clone()),
		wire::ActorEvent::Create(create) => Some(create.waiter.clone()),
		wire::ActorEvent::Reallocate(reallocate) => reallocate.waiter.clone().map(Some),
		wire::ActorEvent::ReserveEpoxyState(_)
		| wire::ActorEvent::WriteIndexState(_)
		| wire::ActorEvent::StartFailed(_)
		| wire::ActorEvent::Sleep
		| wire::ActorEvent::UpdateRoute(_)
		| wire::ActorEvent::Destroy
		| wire::ActorEvent::DestroyState(_)
		| wire::ActorEvent::ToEnvoyEvent(_)
		| wire::ActorEvent::EnvoyGone(_)
		| wire::ActorEvent::MigrateFromV2(_)
		| wire::ActorEvent::ForkDatabaseState(_)
		| wire::ActorEvent::ForkDatabaseRefused(_)
		| wire::ActorEvent::LegacyMoveConfirm(_)
		| wire::ActorEvent::LegacyMarkMoving(_)
		| wire::ActorEvent::LegacyClearMoving(_)
		| wire::ActorEvent::LegacyMoveDestroy(_)
		| wire::ActorEvent::LegacyMoveAbandon(_) => None,
	};

	let envoy_epoch = match &event {
		wire::ActorEvent::ToEnvoyEvent(to_envoy) => Some(to_envoy.epoch),
		wire::ActorEvent::ReserveEpoxyState(_)
		| wire::ActorEvent::WriteIndexState(_)
		| wire::ActorEvent::StartFailed(_)
		| wire::ActorEvent::Sleep
		| wire::ActorEvent::TryStart(_)
		| wire::ActorEvent::Create(_)
		| wire::ActorEvent::Reallocate(_)
		| wire::ActorEvent::UpdateRoute(_)
		| wire::ActorEvent::Destroy
		| wire::ActorEvent::DestroyState(_)
		| wire::ActorEvent::EnvoyGone(_)
		| wire::ActorEvent::ForkDatabaseState(_)
		| wire::ActorEvent::ForkDatabaseRefused(_)
		| wire::ActorEvent::LegacyMoveConfirm(_)
		| wire::ActorEvent::LegacyMarkMoving(_)
		| wire::ActorEvent::LegacyClearMoving(_)
		| wire::ActorEvent::LegacyMoveDestroy(_)
		| wire::ActorEvent::LegacyMoveAbandon(_)
		| wire::ActorEvent::MigrateFromV2(_) => None,
	};

	let State(state) = rivet_state_machine_durable::Runner::process_event(
		&ctx,
		db.clone(),
		Method::Get,
		Arc::new(Executor::new(&ctx, db.clone(), group, actor_id)),
		Event(event),
	)
	.await?;

	if let Some(event_epoch) = envoy_epoch
		&& group.is_some()
		&& legacy_refuses_envoy_event(&db, &state, event_epoch, actor_id).await?
	{
		return Err(crate::validation::LegacyActorMovedAway.build());
	}

	Ok(wire::ActorEventResponse {
		outcome: waiter.map(|waiter| try_start_outcome(&state, waiter.as_ref())),
	})
}

/// Legacy: whether the grouped actor in `state`, a copy the backfill made of the legacy actor
/// with the same id `actor_id`, refuses an Envoy event at `event_epoch` that belongs to the legacy
/// actor ([`crate::legacy_volumes::legacy_copy_refuses_envoy_event`]), so delivery sends it to the
/// legacy actor. The state is the one after the event, which the actor ignored, since nothing at
/// another epoch reaches it. An event at the epoch the event itself moved past, such as a stop
/// report whose handling restarts the actor, is refused after it was applied, and the legacy actor
/// it is then sent to ignores it too. The record is read only once the state alone would refuse,
/// so an event at a live actor's current epoch costs nothing more. The legacy behavior accepted
/// every such event. Removed in 3.1 with the backfill.
async fn legacy_refuses_envoy_event(
	db: &KvHandle,
	state: &wire::ActorState,
	event_epoch: u32,
	actor_id: Id,
) -> Result<bool> {
	if !crate::legacy_volumes::legacy_copy_refuses_envoy_event_state(state, event_epoch) {
		return Ok(false);
	}
	let data_key = keys::kv::ActorDataKey::new();
	let mut snapshot = db.snapshot().await?;
	let Some(data) = snapshot.get(&data_key.pack()).await? else {
		return Ok(false);
	};
	let data = data_key.deserialize(data.into())?;
	Ok(crate::legacy_volumes::legacy_copy_refuses_envoy_event(
		state,
		event_epoch,
		&data,
		actor_id,
	))
}

/// Fires the actor's due timers for a delivered alarm. The alarm must still belong to this
/// volume opening and incarnation: a delivery that outlived either is refused, and depot
/// retries it against the current owner.
async fn actor_alarm_handler(
	ctx: EngineContext,
	db: KvHandle,
	request: rivet_data::generated::pegboard_actor_alarm_v1::ActorAlarmRequest,
) -> Result<wire::ActorEventResponse> {
	ensure!(
		db.has_alarm_owner(&request.owner)
			&& db.volume_log_key().0.volume_incarnation == request.revision.volume_incarnation,
		"obsolete alarm owner or volume incarnation"
	);
	let (group, actor_id) = keys::partition::ActorVolume::parse(db.volume_key())?;

	rivet_state_machine_durable::Runner::<ActorStateMachine>::process_alarm(
		&ctx,
		db.clone(),
		Arc::new(Executor::new(&ctx, db, group, actor_id)),
		request.revision,
	)
	.await?;

	Ok(wire::ActorEventResponse { outcome: None })
}

/// The depot alarm driver for actor volumes, given to the storage worker at startup. A due
/// alarm of an actor volume is delivered back to that volume as an `ActorAlarm` command;
/// alarms of other volumes are not pegboard's and are left unacknowledged.
pub struct ActorAlarmDriver(pub EngineContext);

impl depot_worker::alarms::DepotDriver for ActorAlarmDriver {
	fn on_alarm(
		&self,
		key: depot_types::VolumeKey,
		alarm: depot_worker::alarms::AlarmContext,
	) -> futures_util::future::BoxFuture<'_, Result<()>> {
		Box::pin(async move {
			keys::partition::ActorVolume::parse(&key)
				.context("alarm for a volume that is not an actor volume")?;
			self.0
				.depot()
				.command::<ActorAlarm>(
					&key,
					rivet_data::generated::pegboard_actor_alarm_v1::ActorAlarmRequest {
						revision: alarm.revision,
						owner: alarm.owner,
					},
				)
				.await?;
			Ok(())
		})
	}
}

// CAS Upserter
async fn envoy_upsert_handler(
	db: KvHandle,
	request: wire::EnvoyUpsertRequest,
) -> Result<wire::EnvoyUpsertResponse> {
	keys::partition::EnvoyVolume::from_volume_key(db.volume_key())?;

	let key = keys::kv::EnvoyDataKey::new();
	let packed = key.pack();

	// The partition lock serializes upserts, so the read below cannot go stale before commit
	let mut end = packed.to_vec();
	end.push(0);
	let _guard = db.lock_partition(packed.clone()..end.into()).await?;

	let mut txn = db.begin().await?;
	let stored = match txn.get(&packed).await? {
		Some(value) => Some(key.deserialize(value.into())?),
		None => None,
	};

	if stored != request.expected {
		let Some(stored) = stored else {
			bail!("envoy data expected to exist but is missing");
		};

		return Ok(wire::EnvoyUpsertResponse { data: stored });
	}

	txn.put(
		&packed,
		&key.serialize(at_write_version(&db, request.data.clone())?)?,
	)?;
	db.commit(txn).await?;

	Ok(wire::EnvoyUpsertResponse { data: request.data })
}

async fn envoy_read_handler(db: KvHandle, _request: ()) -> Result<wire::EnvoyReadResponse> {
	let volume = keys::partition::EnvoyVolume::from_volume_key(db.volume_key())?;
	let key = keys::kv::EnvoyDataKey::new();

	let data = match db.snapshot().await?.get(&key.pack()).await? {
		Some(value) => Some(key.deserialize(value.into())?),
		None => None,
	};

	ensure!(
		data.as_ref()
			.is_none_or(|data| data.key == volume.envoy_key),
		"envoy record does not match routed volume"
	);
	Ok(wire::EnvoyReadResponse { data })
}
