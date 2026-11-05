//! Routed commands execute under a shared partition mutex and acknowledge durability.
use crate::actor_machine::{ActorStateMachine, Event, State, executor::Executor};
use crate::errors::MigrateFromV2Unsupported;
use crate::keys;
use crate::keys::kv::at_write_version;
use crate::wire;
use anyhow::{Context, Result, bail, ensure};
use depot_protocol::command::{Command, CommandTag};
use depot_worker::command::KvHandle;
use rivet_data::families::VersionFamily;
use rivet_engine_context::EngineContext;
use rivet_keys::keys::{
	KeyAddress,
	codec::FormalKey,
	tags::depot_command::{
		PEGBOARD_ACTOR_ALARM, PEGBOARD_ACTOR_CREATE, PEGBOARD_ACTOR_EVENT,
		PEGBOARD_ACTOR_MIGRATE_FROM_V2_BEGIN, PEGBOARD_ACTOR_READ, PEGBOARD_ENVOY_READ,
		PEGBOARD_ENVOY_UPSERT,
	},
};
use rivet_state_machine_durable::DurableStateMachine;
use rivet_state_machine_durable::Method;
use std::sync::Arc;

pub struct ActorCreate;

impl Command for ActorCreate {
	const TAG: CommandTag = CommandTag(PEGBOARD_ACTOR_CREATE);

	const CREATE_IF_MISSING: bool = true;
	const SPECULATIVE_CREATE: bool = true;
	type Request = rivet_data::versioned::ActorCreateRequest;
	type Response = rivet_data::versioned::ActorCreateResponse;
}

pub struct ActorRead;

impl Command for ActorRead {
	const TAG: CommandTag = CommandTag(PEGBOARD_ACTOR_READ);
	const READ_ONLY: bool = true;

	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::ActorReadRequest;
	type Response = rivet_data::versioned::ActorReadResponse;
}

pub struct ActorEvent;

impl Command for ActorEvent {
	const TAG: CommandTag = CommandTag(PEGBOARD_ACTOR_EVENT);

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
/// The first `pegboard.actor` version that can store an actor delegated to V2.
const DELEGATE_TO_V2_ACTOR_VERSION: u16 = 3;

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
	let volume = keys::partition::ActorVolume::from_volume_key(db.volume_key())?;
	// A keyed create in a namespace migrating in from V2 may delegate its key to V2, which
	// only the version that stores a delegated actor can write.
	if request.init.key.is_some() {
		let dynamic = ctx.config().dynamic_ref();
		if crate::migrate_from_v2::applies(dynamic.migrate_from_v2(), &request.init.namespace)
			.is_some()
		{
			ensure_actor_version(&db, DELEGATE_TO_V2_ACTOR_VERSION)?;
		}
	}

	let State(state) = rivet_state_machine_durable::Runner::process_event(
		&ctx,
		db.clone(),
		Method::GetOrCreate {
			state: default_state(&ctx, &request.init),
		},
		Arc::new(Executor::new(&ctx, db, volume.actor_id)),
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
		namespace: init.namespace.clone(),
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
	})
}

/// Holds the actor for the V2 migrator, creating its state volume if it has none. The begin is
/// refused before anything is written while the cluster cannot store a held actor.
async fn actor_migrate_from_v2_begin_handler(
	ctx: EngineContext,
	db: KvHandle,
	request: wire::MigrateFromV2Begin,
) -> Result<wire::ActorMigrateFromV2BeginResponse> {
	let volume = keys::partition::ActorVolume::from_volume_key(db.volume_key())?;
	ensure_actor_version(&db, MIGRATE_FROM_V2_ACTOR_VERSION)?;

	let State(state) = rivet_state_machine_durable::Runner::process_event(
		&ctx,
		db.clone(),
		Method::GetOrCreate {
			state: default_state(&ctx, &request.init),
		},
		Arc::new(Executor::new(&ctx, db, volume.actor_id)),
		Event(wire::ActorEvent::MigrateFromV2(
			wire::MigrateFromV2::MigrateFromV2Begin(request),
		)),
	)
	.await?;

	Ok(wire::ActorMigrateFromV2BeginResponse {
		phase: state.phase,
		create_unix_ms: state.create_unix_ms,
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
	keys::partition::ActorVolume::from_volume_key(db.volume_key())?;

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
		});
	};
	let state = ActorStateMachine::deserialize_state(&state)?.0;

	Ok(wire::ActorReadResponse {
		data,
		phase: Some(state.phase),
		epoch: state.epoch,
	})
}

async fn actor_event_handler(
	ctx: EngineContext,
	db: KvHandle,
	event: wire::ActorEvent,
) -> Result<wire::ActorEventResponse> {
	let volume = keys::partition::ActorVolume::from_volume_key(db.volume_key())?;
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
		| wire::ActorEvent::MigrateFromV2(_) => None,
	};

	let State(state) = rivet_state_machine_durable::Runner::process_event(
		&ctx,
		db.clone(),
		Method::Get,
		Arc::new(Executor::new(&ctx, db, volume.actor_id)),
		Event(event),
	)
	.await?;

	Ok(wire::ActorEventResponse {
		outcome: waiter.map(|waiter| try_start_outcome(&state, waiter.as_ref())),
	})
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
	let volume = keys::partition::ActorVolume::from_volume_key(db.volume_key())?;

	rivet_state_machine_durable::Runner::<ActorStateMachine>::process_alarm(
		&ctx,
		db.clone(),
		Arc::new(Executor::new(&ctx, db, volume.actor_id)),
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
			keys::partition::ActorVolume::from_volume_key(&key)
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
