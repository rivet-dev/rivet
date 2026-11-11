//! Namespaces: each namespace's own volume, holding its namespace state machine; the cluster
//! catalog, a listing index of names plus the legacy record rows not yet moved; and a
//! per-process cache of each namespace's standing, which keeps existence for good and re-reads
//! the phase in the background after a TTL.
use crate::{
	keys::{
		kv::{
			EnvoyPoolKey, EnvoyPoolPrefix, LegacyBackfillNameKey, LegacyBackfillNamePrefix,
			LegacyNamespaceKey, LegacyNamespacePrefix, NamespaceIndexKey, NamespaceIndexPrefix,
			PoolConnectionKey, PoolConnectionPrefix, PoolHeartbeatKey, PoolHeartbeatPrefix,
			at_write_version,
		},
		partition::{EnvoyPoolIndexVolume, NamespaceCatalogVolume, NamespaceVolume},
	},
	namespace_machine::{Event, NamespaceStateMachine, State, executor::Executor},
	wire,
};
use anyhow::{Context, Result, ensure};
use depot_protocol::command::{Command, CommandTag};
use depot_types::keys::partition::NamespaceGroupKey;
use depot_worker::command::{CommandRegistry, KvHandle};
use futures_util::StreamExt;
use rivet_data::families::VersionFamily;
use rivet_data::generated::pegboard_namespace_v1 as ns;
use rivet_engine_context::{CachedNamespace, EngineContext, NamespaceRefreshBegin};
use rivet_env::Clock;
use rivet_keys::keys::{
	KeyAddress, SubspaceRange,
	codec::{AtVersion, FormalKey, GroupId},
	tags::depot_command::*,
};
use rivet_metrics::GaugeGuardExt;
use rivet_state_machine_durable::{DurableStateMachine, Method, Runner};
use std::{collections::HashSet, ops::Bound, sync::Arc};

#[derive(Default)]
struct CatalogLock(tokio::sync::Mutex<()>);

/// What a namespace lookup answers: the name, the volume group its volumes live in when it has
/// one, and its phase as last read. Every request path that opens a namespace's volumes builds
/// its keys from this.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamespaceStanding {
	pub name: String,
	pub group: Option<ns::NamespaceGroup>,
	pub phase: ns::NamespacePhase,
	/// Legacy: whether the namespace's unprefixed volumes are copied into its group yet.
	/// Removed in 3.1 with the backfill.
	pub backfill: ns::NamespaceBackfill,
	/// Legacy: [`NamespaceStanding::legacy_fallback`], derived once from `group` and `backfill`
	/// when the standing is built, so request paths read it without rebuilding the skipped set.
	/// Private so every standing is built through [`NamespaceStanding::new`] and the two never
	/// disagree. Removed in 3.1 with the backfill.
	legacy_fallback: LegacyFallback,
}

/// Legacy: which actors of a namespace an open, a lookup or an event still looks for at their
/// unprefixed keys when the grouped volume does not exist, and the namespace name such a record
/// must carry. The legacy behavior keyed every volume unprefixed; new behavior keys every volume
/// under the namespace's group. Taken from [`NamespaceStanding::legacy_fallback`]. Removed in
/// 3.1 with the backfill.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LegacyFallback {
	/// No actor falls back: the namespace has no group, or its backfill is done and skipped
	/// nothing.
	Off,
	/// Every actor falls back: the namespace's backfill is not done.
	Every { namespace: String },
	/// Only the actors the finished backfill lists as skipped fall back, by id, so an operator
	/// can still reach them. Bounded by the list's cap, and shared by every clone of the
	/// standing it was derived from.
	Skipped {
		namespace: String,
		actors: Arc<HashSet<rivet_util::Id>>,
	},
}

impl LegacyFallback {
	/// The namespace a resolve of `actor_id` falls back under, or `None` when it does not fall
	/// back.
	pub fn namespace_for(&self, actor_id: rivet_util::Id) -> Option<&str> {
		match self {
			LegacyFallback::Off => None,
			LegacyFallback::Every { namespace } => Some(namespace),
			LegacyFallback::Skipped { namespace, actors } => match actors.contains(&actor_id) {
				true => Some(namespace),
				false => None,
			},
		}
	}

	/// Whether every actor falls back, so listings read the unprefixed buckets too.
	pub fn every(&self) -> bool {
		match self {
			LegacyFallback::Every { .. } => true,
			LegacyFallback::Off | LegacyFallback::Skipped { .. } => false,
		}
	}
}

/// Legacy: the fallback of a namespace with `group` and `backfill`; see
/// [`NamespaceStanding::legacy_fallback`]. Removed in 3.1 with the backfill.
fn derive_legacy_fallback(
	name: &str,
	group: Option<&ns::NamespaceGroup>,
	backfill: &ns::NamespaceBackfill,
) -> LegacyFallback {
	if group.is_none() {
		return LegacyFallback::Off;
	}
	match backfill {
		ns::NamespaceBackfill::NamespaceBackfillPending(_)
		| ns::NamespaceBackfill::NamespaceBackfillRunning(_) => LegacyFallback::Every {
			namespace: name.to_owned(),
		},
		ns::NamespaceBackfill::NamespaceBackfillDone(done) => {
			let actors: HashSet<rivet_util::Id> = done
				.skipped
				.iter()
				.filter_map(|skipped| rivet_util::Id::from_slice(&skipped.actor_id).ok())
				.collect();
			match actors.is_empty() {
				true => LegacyFallback::Off,
				false => LegacyFallback::Skipped {
					namespace: name.to_owned(),
					actors: Arc::new(actors),
				},
			}
		}
	}
}

impl NamespaceStanding {
	pub fn new(
		name: String,
		group: Option<ns::NamespaceGroup>,
		phase: ns::NamespacePhase,
		backfill: ns::NamespaceBackfill,
	) -> Self {
		let legacy_fallback = derive_legacy_fallback(&name, group.as_ref(), &backfill);
		Self {
			name,
			group,
			phase,
			backfill,
			legacy_fallback,
		}
	}

	pub fn from_state(name: &str, state: &ns::NamespaceState) -> Self {
		Self::new(
			name.to_owned(),
			state.group.clone(),
			state.phase.clone(),
			state.backfill.clone(),
		)
	}

	/// Whether the namespace has a volume group, so it can be forked.
	pub fn grouped(&self) -> bool {
		self.group.is_some()
	}

	/// The group prefix the namespace's volume keys carry. `None` only for a namespace served
	/// from its legacy catalog row while older binaries run, whose keys are the legacy
	/// unprefixed ones, since those are all the older binaries read.
	pub fn group_id(&self) -> Option<GroupId> {
		self.group.as_ref().map(|group| group.group)
	}

	/// Legacy: whether the namespace's backfill is done, so it has no unprefixed volumes left and
	/// its opens skip the legacy fallback, and it can be forked and archived. A namespace created
	/// through `NamespaceCreate` is done at birth; one moved from a legacy catalog row is not.
	/// Removed in 3.1 with the backfill.
	pub fn backfill_done(&self) -> bool {
		match self.backfill {
			ns::NamespaceBackfill::NamespaceBackfillDone(_) => true,
			ns::NamespaceBackfill::NamespaceBackfillPending(_)
			| ns::NamespaceBackfill::NamespaceBackfillRunning(_) => false,
		}
	}

	/// Legacy: which actors' opens fall back to the unprefixed volume when the grouped one does
	/// not exist: every actor of a grouped namespace whose backfill is not done, and after it is
	/// done only the actors it lists as skipped, so an operator can still read, export and
	/// destroy them. A namespace without a group keys everything unprefixed and falls back for
	/// nothing. Derived once when the standing is built. Removed in 3.1 with the backfill.
	pub fn legacy_fallback(&self) -> &LegacyFallback {
		&self.legacy_fallback
	}

	/// Legacy: how many actors the finished backfill gave up on and still counts, which forks and
	/// archives of the namespace wait for. Zero before the backfill is done. Removed in 3.1 with
	/// the backfill.
	pub fn legacy_skipped_count(&self) -> u32 {
		match &self.backfill {
			ns::NamespaceBackfill::NamespaceBackfillDone(done) => done.skipped_count,
			ns::NamespaceBackfill::NamespaceBackfillPending(_)
			| ns::NamespaceBackfill::NamespaceBackfillRunning(_) => 0,
		}
	}

	/// Legacy: the namespace an actor created in this namespace records as its legacy namespace.
	/// Volumes created before namespaces had groups are unprefixed and only their record says
	/// whose they are; new behavior keys every volume under its namespace's group and records no
	/// namespace. Taken only for a namespace without a group, which exists only while served from
	/// its legacy catalog row while older binaries run and still creates at unprefixed keys; `None`
	/// for every grouped namespace. Removed in 3.1 with the backfill.
	pub fn legacy_create_namespace(&self) -> Option<String> {
		match &self.group {
			Some(_) => None,
			None => Some(self.name.clone()),
		}
	}

	/// The index bucket count the namespace's listings and creates use: the lineage's, fixed at
	/// its root's creation, or `configured` for a pre-group namespace.
	pub fn index_bucket_count(&self, configured: u64) -> u64 {
		match &self.group {
			Some(group) => group.index_bucket_count,
			None => configured,
		}
	}

	/// Whether the namespace is live, so it admits new work.
	pub fn live(&self) -> bool {
		matches!(self.phase, ns::NamespacePhase::NamespaceLive)
	}

	/// Refuses the work the namespace's phase forbids: creates, wakes, listings and envoy
	/// connects while its group create or fork has not finished, while it is archiving or
	/// archived, or when its fork failed.
	pub fn admit(&self) -> Result<()> {
		match &self.phase {
			ns::NamespacePhase::NamespaceLive => Ok(()),
			ns::NamespacePhase::NamespaceCreating(_) => Err(namespace_creating()),
			ns::NamespacePhase::NamespaceForking(_) => {
				Err(crate::validation::NamespacePending.build())
			}
			ns::NamespacePhase::NamespaceArchiving(_)
			| ns::NamespacePhase::NamespaceArchived(_) => Err(crate::validation::NamespaceArchived.build()),
			ns::NamespacePhase::NamespaceFailed(_) => {
				Err(crate::validation::NamespaceFailed.build())
			}
		}
	}

	/// Whether an actor record read through this namespace's volumes is one of its actors; see
	/// [`actor_in_namespace`].
	pub fn holds_actor(&self, actor: &wire::ActorData) -> bool {
		actor_in_namespace(self.group_id(), &self.name, actor)
	}
}

/// Whether an actor record read through the volumes of `namespace`, whose volume key group is
/// `group`, is one of that namespace's actors. A grouped key already proves it, so the record is
/// not consulted; records name no namespace, since a fork carries them into another. An
/// unprefixed key (`group` is `None`) is in the key space every namespace used before groups, so
/// only the record's legacy namespace tells them apart; the legacy fallback reads pass `None` for
/// that reason, and a record without one belongs to no namespace there. Every path that reads an
/// actor by id from a request namespace refuses a record this rejects as not found, so a foreign
/// id answers like a missing one.
pub fn actor_in_namespace(
	group: Option<GroupId>,
	namespace: &str,
	actor: &wire::ActorData,
) -> bool {
	match group {
		Some(_) => true,
		None => actor.legacy_namespace.as_deref() == Some(namespace),
	}
}

/// Creates the namespace on its own volume, or answers the one there; see `create_handler`.
pub struct NamespaceCreate;
impl Command for NamespaceCreate {
	const TAG: CommandTag = CommandTag(PEGBOARD_NAMESPACE_STATE_CREATE);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::NamespaceCreateRequest;
	type Response = rivet_data::versioned::NamespaceCreateResponse;
}

/// Reads the namespace's state from its volume without touching its machine.
pub struct NamespaceRead;
impl Command for NamespaceRead {
	const TAG: CommandTag = CommandTag(PEGBOARD_NAMESPACE_STATE_READ);
	const READ_ONLY: bool = true;
	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::NamespaceReadRequest;
	type Response = rivet_data::versioned::NamespaceReadResponse;
}

/// Archives the namespace through its machine; see `archive_handler`.
pub struct NamespaceArchive;
impl Command for NamespaceArchive {
	const TAG: CommandTag = CommandTag(PEGBOARD_NAMESPACE_ARCHIVE);
	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::NamespaceArchiveRequest;
	type Response = rivet_data::versioned::NamespaceArchiveResponse;
}

/// The outcome of one of the namespace machine's effects, from its executor.
pub struct NamespaceStep;
impl Command for NamespaceStep {
	const TAG: CommandTag = CommandTag(PEGBOARD_NAMESPACE_STEP);
	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::NamespaceStepRequest;
	type Response = rivet_data::versioned::NamespaceStepResponse;
}

/// A due Depot alarm of a namespace volume: the namespace machine's timers fire.
pub struct NamespaceAlarm;
impl Command for NamespaceAlarm {
	const TAG: CommandTag = CommandTag(PEGBOARD_NAMESPACE_ALARM);
	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::NamespaceAlarmRequest;
	type Response = rivet_data::versioned::NamespaceAlarmResponse;
}

/// Writes a namespace's listing index entry on the catalog; see `index_put_handler`.
pub struct NamespaceIndexPut;
impl Command for NamespaceIndexPut {
	const TAG: CommandTag = CommandTag(PEGBOARD_NAMESPACE_INDEX_PUT);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::NamespaceIndexPutRequest;
	type Response = rivet_data::versioned::NamespaceIndexPutResponse;
}

/// Pages the catalog's names; see `index_list_handler`.
pub struct NamespaceIndexList;
impl Command for NamespaceIndexList {
	const TAG: CommandTag = CommandTag(PEGBOARD_NAMESPACE_INDEX_LIST);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::NamespaceIndexListRequest;
	type Response = rivet_data::versioned::NamespaceIndexListResponse;
}

/// Legacy: creates a record row in the catalog. New binaries send it only for a create while
/// the fleet cannot read namespace volumes; binaries from before namespace volumes send it for
/// every create.
pub struct LegacyNamespaceCreate;
impl Command for LegacyNamespaceCreate {
	const TAG: CommandTag = CommandTag(LEGACY_PEGBOARD_NAMESPACE_CREATE);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::LegacyNamespaceCreateRequest;
	type Response = rivet_data::versioned::LegacyNamespaceCreateResponse;
}

/// Legacy: reads one record row of the catalog. New binaries send it only for a name whose
/// namespace volume holds no state, or for every name while old binaries run; binaries from
/// before namespace volumes read every namespace this way. A snapshot read, so it never
/// waits behind the catalog's writers.
pub struct LegacyNamespaceRead;
impl Command for LegacyNamespaceRead {
	const TAG: CommandTag = CommandTag(LEGACY_PEGBOARD_NAMESPACE_READ);
	const READ_ONLY: bool = true;
	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::LegacyNamespaceReadRequest;
	type Response = rivet_data::versioned::LegacyNamespaceReadResponse;
}

/// Legacy: pages the catalog's record rows. Only binaries from before namespace volumes list
/// this way; new binaries page the listing index, which merges these rows.
pub struct LegacyNamespaceList;
impl Command for LegacyNamespaceList {
	const TAG: CommandTag = CommandTag(LEGACY_PEGBOARD_NAMESPACE_LIST);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::LegacyNamespaceListRequest;
	type Response = rivet_data::versioned::LegacyNamespaceListResponse;
}

/// Legacy: moves a namespace from its record row into its namespace volume; see
/// `legacy_move_namespace`.
pub struct LegacyNamespaceMove;
impl Command for LegacyNamespaceMove {
	const TAG: CommandTag = CommandTag(LEGACY_PEGBOARD_NAMESPACE_MOVE);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::LegacyNamespaceMoveRequest;
	type Response = rivet_data::versioned::LegacyNamespaceMoveResponse;
}

pub struct Pool;
impl Command for Pool {
	const TAG: CommandTag = CommandTag(PEGBOARD_POOL);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::PoolRequest;
	type Response = rivet_data::versioned::PoolResponse;
}

pub struct PoolList;
impl Command for PoolList {
	const TAG: CommandTag = CommandTag(PEGBOARD_POOL_LIST);
	const CREATE_IF_MISSING: bool = true;
	type Request = rivet_data::versioned::PoolListRequest;
	type Response = rivet_data::versioned::PoolListResponse;
}

/// Legacy: records an actor name for the backfill on a namespace volume; see
/// `legacy_backfill_name_record_handler`. Removed in 3.1 with the backfill.
pub struct LegacyBackfillNameRecord;
impl Command for LegacyBackfillNameRecord {
	const TAG: CommandTag = CommandTag(LEGACY_PEGBOARD_BACKFILL_NAME_RECORD);
	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::LegacyBackfillNameRecordRequest;
	type Response = rivet_data::versioned::LegacyBackfillNameRecordResponse;
}

/// Legacy: tells the namespace `namespace` that the V2 migrator finished in-flight entries at the
/// legacy placement, so a backfill waiting on them reads the migrator's state again. Best effort:
/// a namespace with no volume, or one whose backfill does not wait, ignores it, and a lost
/// notification is covered by the pending backfill's retry timer. Removed in 3.1 with the
/// backfill.
pub async fn legacy_notify_migrator_settled(ctx: &EngineContext, namespace: &str) -> Result<()> {
	let sent = ctx
		.depot()
		.command::<NamespaceStep>(
			&NamespaceVolume::new(namespace.to_owned()).volume_key(),
			ns::NamespaceStepRequest::NamespaceStepLegacyMigratorSettled,
		)
		.await;
	match sent {
		Ok(()) => Ok(()),
		Err(error) if crate::commands::is_volume_gone(&error) => Ok(()),
		Err(error) => Err(error.context("notify the namespace that V2 migrations settled")),
	}
}

/// Legacy: records `name` for the backfill of `namespace` and waits for the write, for a caller
/// that must know the backfill will walk the name, such as the V2 migrator once it activated an
/// actor at the legacy placement. Answers false when the namespace has no volume yet: it is still
/// a legacy catalog row, its backfill has not started, and nothing can be recorded for it. Removed
/// in 3.1 with the backfill.
pub async fn legacy_record_backfill_name(
	ctx: &EngineContext,
	namespace: &str,
	name: &str,
) -> Result<bool> {
	let written = ctx
		.depot()
		.command::<LegacyBackfillNameRecord>(
			&NamespaceVolume::new(namespace.to_owned()).volume_key(),
			ns::LegacyBackfillNameRecordRequest {
				name: name.to_owned().into(),
			},
		)
		.await;
	match written {
		Ok(_) => Ok(true),
		Err(error) if crate::commands::is_volume_gone(&error) => Ok(false),
		Err(error) => Err(error.context("record an actor name for the backfill")),
	}
}

/// Legacy: pages the actor names recorded for the backfill on a namespace volume. A snapshot
/// read. Removed in 3.1 with the backfill.
pub struct LegacyBackfillNameList;
impl Command for LegacyBackfillNameList {
	const TAG: CommandTag = CommandTag(LEGACY_PEGBOARD_BACKFILL_NAME_LIST);
	const READ_ONLY: bool = true;
	const CREATE_IF_MISSING: bool = false;
	type Request = rivet_data::versioned::LegacyBackfillNameListRequest;
	type Response = rivet_data::versioned::LegacyBackfillNameListResponse;
}

pub(crate) fn register(registry: &mut CommandRegistry, ctx: &EngineContext) -> Result<()> {
	// A registration's concurrency caps the command across every volume on the host, so the
	// namespace commands share the actor command caps. One namespace's commands are serialized
	// by its machine's state lock, not by these caps.
	let concurrency = &ctx.config().pegboard().command_concurrency;
	let ctx2 = ctx.clone();
	registry
		.register_with_concurrency::<NamespaceCreate>(concurrency.actor(), move |db, request| {
			create_handler(ctx2.clone(), db, request)
		})?;
	registry.register_snapshot_read::<NamespaceRead>(
		concurrency.actor_read(),
		None,
		read_handler,
	)?;
	let ctx2 = ctx.clone();
	registry.register_with_concurrency::<NamespaceArchive>(
		concurrency.actor(),
		move |db, request| archive_handler(ctx2.clone(), db, request),
	)?;
	let ctx2 = ctx.clone();
	registry.register_with_concurrency::<NamespaceStep>(
		concurrency.actor_event(),
		move |db, request| step_handler(ctx2.clone(), db, request),
	)?;
	let ctx2 = ctx.clone();
	registry.register_with_concurrency::<NamespaceAlarm>(
		concurrency.actor_event(),
		move |db, request| alarm_handler(ctx2.clone(), db, request),
	)?;
	let ctx2 = ctx.clone();
	registry.register_with_concurrency::<LegacyNamespaceMove>(
		concurrency.actor(),
		move |db, request| legacy_move_handler(ctx2.clone(), db, request),
	)?;

	// The catalog's writers share its lock, so the legacy create cannot claim one name twice.
	// The legacy read is a snapshot read outside the lock: it serves old binaries' every
	// resolution during a rollout.
	registry.register_with_concurrency::<LegacyBackfillNameRecord>(
		concurrency.actor(),
		legacy_backfill_name_record_handler,
	)?;
	registry.register_snapshot_read::<LegacyBackfillNameList>(
		concurrency.actor_read(),
		None,
		legacy_backfill_name_list_handler,
	)?;

	// The index volume is one volume for every namespace, so overlapping creates and listings
	// reach it at once. Its writes serialize on the catalog lock inside the handler; a cap of one
	// would only refuse the overlap as `CommandCapacity`, which the namespace's index retry then
	// waits out. The index command cap bounds them instead.
	registry
		.register_with_concurrency::<NamespaceIndexPut>(concurrency.index(), index_put_handler)?;
	registry
		.register_with_concurrency::<NamespaceIndexList>(concurrency.index(), index_list_handler)?;
	let clock = ctx.env().clock();
	let ctx2 = ctx.clone();
	registry.register_with_concurrency::<LegacyNamespaceCreate>(1, move |db, request| {
		legacy_create_handler(clock.clone(), legacy_old_binaries_run(&ctx2), db, request)
	})?;
	registry.register_snapshot_read::<LegacyNamespaceRead>(
		concurrency.actor_read(),
		None,
		legacy_read_handler,
	)?;
	let clock = ctx.env().clock();
	let ctx2 = ctx.clone();
	registry.register_with_concurrency::<LegacyNamespaceList>(1, move |db, request| {
		legacy_list_handler(clock.clone(), legacy_old_binaries_run(&ctx2), db, request)
	})?;

	let clock = ctx.env().clock();
	registry.register_with_concurrency::<Pool>(1, move |db, request| {
		pool_handler(clock.clone(), db, request)
	})?;
	registry.register_with_concurrency::<PoolList>(1, pool_list_handler)?;
	Ok(())
}

/// Legacy: whether a binary from before namespace volumes is known to run. The legacy behavior
/// seeded `default`'s catalog row for every binary; the new behavior seeds it only while this
/// is true, since only an old binary finds `default` there. Taken by the legacy create and
/// legacy list handlers, where an unknown view of the fleet seeds nothing, and by an envoy
/// event's namespace re-read at delivery, where an unknown view still reads.
pub(crate) fn legacy_old_binaries_run(ctx: &EngineContext) -> bool {
	matches!(namespace_volumes_open(ctx), Ok(false))
}

/// The namespace whose volume `db` is.
fn volume_name(db: &KvHandle) -> Result<String> {
	let volume = NamespaceVolume::from_volume_key(db.volume_key())?;
	crate::validation::namespace_name(&volume.name)?;
	Ok(volume.name)
}

/// The namespace's state as its volume stores it, without touching its machine.
async fn stored_state(db: &KvHandle) -> Result<Option<ns::NamespaceState>> {
	let key = rivet_state_machine_durable::keys::kv::MachineStateKey::new();
	let Some(bytes) = db.snapshot().await?.get(&key.pack()).await? else {
		return Ok(None);
	};
	Ok(NamespaceStateMachine::deserialize_state(&bytes)?.0)
}

/// Runs one event of the namespace's machine.
async fn run(
	ctx: &EngineContext,
	db: &KvHandle,
	method: Method<NamespaceStateMachine>,
	event: Event,
) -> Result<Option<ns::NamespaceState>> {
	let name = volume_name(db)?;
	let State(state) = Runner::<NamespaceStateMachine>::process_event(
		ctx,
		db.clone(),
		method,
		Arc::new(Executor::for_volume(ctx, name, db).await?),
		event,
	)
	.await?;
	Ok(state)
}

/// Creates the namespace, or answers the one already on its volume. `created` tells the caller
/// the namespace is its own: this command wrote it, a retry of this command finds it, or the
/// request forks the same parent as the fork in progress and joins it.
async fn create_handler(
	ctx: EngineContext,
	db: KvHandle,
	request: ns::NamespaceCreateRequest,
) -> Result<ns::NamespaceCreateResponse> {
	let name = volume_name(&db)?;
	let operation_id = db.operation_id();
	let state = run(
		&ctx,
		&db,
		Method::GetOrCreate { state: State(None) },
		Event::Create {
			name: name.clone(),
			create_unix_ms: request.create_unix_ms,
			operation_id,
			fork_from: request.fork_from.clone(),
			waiter: request.waiter.clone(),
		},
	)
	.await?
	.context("a namespace create leaves a state")?;
	let created = state.operation_id == operation_id
		|| crate::namespace_machine::joins_fork(&state, request.fork_from.as_ref());
	let starting = matches!(
		state.phase,
		ns::NamespacePhase::NamespaceCreating(_) | ns::NamespacePhase::NamespaceForking(_)
	);
	if let Some(waiter) = &request.waiter
		&& starting
		&& !crate::namespace_machine::has_waiter(&state, waiter)
	{
		tracing::warn!(
			namespace = %name,
			"a namespace's start waiter list is full, so a create reads the namespace at its deadline; raise pegboard.namespace_start_waiters_capacity if this persists"
		);
		crate::metrics::NAMESPACE_START_WAITERS_DROPPED.inc();
	}
	Ok(ns::NamespaceCreateResponse { created, state })
}

async fn read_handler(db: KvHandle, _request: ()) -> Result<ns::NamespaceReadResponse> {
	let name = volume_name(&db)?;
	let state = stored_state(&db).await?;
	// Legacy: reads are what usually open a finished namespace's volume, so the first one sets
	// its skipped list gauge for this opening. Removed in 3.1 with the backfill.
	let pages = db
		.opening_state::<crate::namespace_machine::executor::LegacyBackfillPages>()
		.await?;
	crate::namespace_machine::executor::legacy_observe_listed_at_open(
		&name,
		&pages,
		state.as_ref(),
	);
	Ok(ns::NamespaceReadResponse { state })
}

/// Archives the namespace: refused for `default`, for a namespace whose group create or fork
/// has not finished, or whose fork failed; a namespace already archiving or archived answers its state. The API refused the
/// archive while an envoy of the namespace was connected. The phase is read from a snapshot
/// outside the machine's state lock, so it only picks the early answers; the `Archive`
/// transition checks the phase again under the lock and decides.
async fn archive_handler(
	ctx: EngineContext,
	db: KvHandle,
	_request: (),
) -> Result<ns::NamespaceArchiveResponse> {
	let name = volume_name(&db)?;
	let Some(state) = stored_state(&db).await? else {
		return Err(crate::validation::NamespaceNotFound.build());
	};
	if name == crate::DEFAULT_NAMESPACE {
		return Err(crate::validation::NamespaceProtected.build());
	}
	// Legacy: the namespace's unprefixed volumes are in no group, so archiving the group would
	// not close them. Removed in 3.1 with the backfill.
	match &state.backfill {
		ns::NamespaceBackfill::NamespaceBackfillDone(done) if done.skipped_count > 0 => {
			return Err(crate::validation::NamespaceBackfillSkipped {
				skipped_count: done.skipped_count,
			}
			.build());
		}
		ns::NamespaceBackfill::NamespaceBackfillDone(_) => {}
		ns::NamespaceBackfill::NamespaceBackfillPending(_)
		| ns::NamespaceBackfill::NamespaceBackfillRunning(_) => {
			return Err(crate::validation::NamespaceBackfillPending.build());
		}
	}
	match &state.phase {
		ns::NamespacePhase::NamespaceLive => {}
		ns::NamespacePhase::NamespaceCreating(_) => return Err(namespace_creating()),
		ns::NamespacePhase::NamespaceForking(_) => {
			return Err(crate::validation::NamespacePending.build());
		}
		ns::NamespacePhase::NamespaceFailed(_) => {
			return Err(crate::validation::NamespaceFailed.build());
		}
		ns::NamespacePhase::NamespaceArchiving(_) | ns::NamespacePhase::NamespaceArchived(_) => {
			return Ok(ns::NamespaceArchiveResponse { state });
		}
	}
	let state = run(&ctx, &db, Method::Get, Event::Archive)
		.await?
		.context("an archived namespace keeps its state")?;
	Ok(ns::NamespaceArchiveResponse { state })
}

async fn step_handler(
	ctx: EngineContext,
	db: KvHandle,
	request: ns::NamespaceStepRequest,
) -> Result<()> {
	let event = match request {
		ns::NamespaceStepRequest::NamespaceStepCreate(step) => Event::CreateOutcome(step.outcome),
		ns::NamespaceStepRequest::NamespaceStepFork(step) => Event::ForkOutcome(step.outcome),
		ns::NamespaceStepRequest::NamespaceStepArchive(step) => Event::ArchiveOutcome(step.outcome),
		ns::NamespaceStepRequest::NamespaceStepIndex(step) => Event::IndexOutcome(step.outcome),
		ns::NamespaceStepRequest::NamespaceStepBackfill(step) => {
			Event::BackfillOutcome(step.outcome)
		}
		ns::NamespaceStepRequest::NamespaceStepLegacySkippedDestroyed(step) => {
			Event::LegacySkippedDestroyed {
				actor_id: step.actor_id.to_vec(),
			}
		}
		ns::NamespaceStepRequest::NamespaceStepBackfillPageFailed(step) => {
			Event::BackfillPageFailed(step.failed)
		}
		ns::NamespaceStepRequest::NamespaceStepLegacyMigratorHolds(step) => {
			Event::LegacyMigratorHolds { held: step.held }
		}
		ns::NamespaceStepRequest::NamespaceStepLegacyMigratorSettled => {
			Event::LegacyMigratorSettled
		}
	};
	run(&ctx, &db, Method::Get, event).await?;
	Ok(())
}

/// Fires the namespace machine's due timers for a delivered alarm. The alarm must still belong
/// to this volume opening and incarnation; a delivery that outlived either is refused, and
/// Depot retries it against the current owner.
async fn alarm_handler(
	ctx: EngineContext,
	db: KvHandle,
	request: ns::NamespaceAlarmRequest,
) -> Result<()> {
	ensure!(
		db.has_alarm_owner(&request.owner)
			&& db.volume_log_key().0.volume_incarnation == request.revision.volume_incarnation,
		"obsolete alarm owner or volume incarnation"
	);
	let name = volume_name(&db)?;
	Runner::<NamespaceStateMachine>::process_alarm(
		&ctx,
		db.clone(),
		Arc::new(Executor::for_volume(&ctx, name, &db).await?),
		request.revision,
	)
	.await?;
	Ok(())
}

/// Legacy: writes a namespace moved from its legacy catalog row into its volume, `Live`. A move
/// of a namespace already on its volume changes nothing and answers its state.
async fn legacy_move_handler(
	ctx: EngineContext,
	db: KvHandle,
	request: ns::LegacyNamespaceMoveRequest,
) -> Result<ns::LegacyNamespaceMoveResponse> {
	let name = volume_name(&db)?;
	let state = run(
		&ctx,
		&db,
		Method::GetOrCreate { state: State(None) },
		Event::LegacyMove {
			name,
			create_unix_ms: request.create_unix_ms,
			operation_id: db.operation_id(),
		},
	)
	.await?
	.context("a legacy namespace move leaves a state")?;
	Ok(ns::LegacyNamespaceMoveResponse { state })
}

/// Writes a namespace's listing index entry, and deletes its legacy record row in the same
/// transaction. Only a namespace whose volume holds its state is indexed, and every read
/// prefers the volume, so the row is dead once the entry lands; deleting it here is what
/// finishes a legacy move. Idempotent.
async fn index_put_handler(db: KvHandle, request: ns::NamespaceIndexPutRequest) -> Result<()> {
	NamespaceCatalogVolume::from_volume_key(db.volume_key())?;
	crate::validation::namespace_name(&request.name)?;
	let serial = db.opening_state::<CatalogLock>().await?;
	let _serial = serial.0.lock().await;
	let key = NamespaceIndexKey::new(request.name.to_string());
	let mut txn = db.begin().await?;
	let entry = ns::NamespaceIndexEntry {
		create_unix_ms: request.create_unix_ms,
	};
	let version = db.write_version(VersionFamily::PEGBOARD_NAMESPACE)?;
	txn.put(&key.pack(), &key.serialize(AtVersion::new(entry, version))?)?;
	txn.delete(&LegacyNamespaceKey::new(request.name.to_string()).pack())?;
	db.commit(txn).await?;
	Ok(())
}

/// Legacy: writes `request.name` as an actor name the namespace's backfill walks. The legacy
/// behavior found actor names only in envoy records; a name a legacy fallback read found an actor
/// under is recorded here too, so an idle actor whose name no envoy record keeps is still moved.
/// Taken only while the namespace's backfill is not done. Idempotent. Removed in 3.1 with the
/// backfill.
async fn legacy_backfill_name_record_handler(
	db: KvHandle,
	request: ns::LegacyBackfillNameRecordRequest,
) -> Result<()> {
	NamespaceVolume::from_volume_key(db.volume_key())?;
	let key = LegacyBackfillNameKey::new(request.name.to_string());
	let mut txn = db.begin().await?;
	txn.put(&key.pack(), &[])?;
	db.commit(txn).await?;
	Ok(())
}

/// Legacy: pages the actor names `legacy_backfill_name_record_handler` wrote, in order, after
/// `request.after`. Removed in 3.1 with the backfill.
async fn legacy_backfill_name_list_handler(
	db: KvHandle,
	request: ns::LegacyBackfillNameListRequest,
) -> Result<ns::LegacyBackfillNameListResponse> {
	NamespaceVolume::from_volume_key(db.volume_key())?;
	ensure!(
		request.limit > 0,
		"backfill name list limit must be positive"
	);
	let limit = (request.limit as usize).min(1_000);
	let range = LegacyBackfillNamePrefix::new().range();
	let start = request
		.after
		.as_ref()
		.map(|name| Bound::Excluded(LegacyBackfillNameKey::new(name.to_string()).pack().to_vec()))
		.unwrap_or(Bound::Included(range.start.clone()));
	let rows = db
		.snapshot()
		.await?
		.scan(
			(
				start.as_ref().map(Vec::as_slice),
				Bound::Excluded(range.end.as_slice()),
			),
			Some(limit + 1),
			false,
		)
		.await?;
	let mut names = Vec::with_capacity(rows.len().min(limit));
	for (key, _) in rows.iter().take(limit) {
		names.push(LegacyBackfillNameKey::unpack(key)?.name);
	}
	let cursor = match rows.len() > limit {
		true => names.last().cloned(),
		false => None,
	};
	Ok(ns::LegacyBackfillNameListResponse {
		names: names.into_iter().map(Into::into).collect(),
		cursor: cursor.map(Into::into),
	})
}

/// Pages the catalog's names in order: the index entries merged with the legacy record rows
/// not yet moved, each name once. With `legacy_index_only`, the index entries alone.
async fn index_list_handler(
	db: KvHandle,
	request: ns::NamespaceIndexListRequest,
) -> Result<ns::NamespaceIndexListResponse> {
	NamespaceCatalogVolume::from_volume_key(db.volume_key())?;
	ensure!(request.limit > 0, "namespace list limit must be positive");
	let limit = (request.limit as usize).min(100);
	let after = request.after.as_ref().map(|name| name.to_string());
	let mut snapshot = db.snapshot().await?;

	let index_range = NamespaceIndexPrefix::new().range();
	let index_start = after
		.clone()
		.map(|name| Bound::Excluded(NamespaceIndexKey::new(name).pack().to_vec()))
		.unwrap_or(Bound::Included(index_range.start.clone()));
	let indexed = snapshot
		.scan(
			(
				index_start.as_ref().map(Vec::as_slice),
				Bound::Excluded(index_range.end.as_slice()),
			),
			Some(limit + 1),
			false,
		)
		.await?;

	// Legacy: the record rows not yet moved. Removed in 3.1 with the backfill.
	let legacy = match request.legacy_index_only {
		true => Vec::new(),
		false => {
			let legacy_range = LegacyNamespacePrefix::new().range();
			let legacy_start = after
				.map(|name| Bound::Excluded(LegacyNamespaceKey::new(name).pack().to_vec()))
				.unwrap_or(Bound::Included(legacy_range.start.clone()));
			snapshot
				.scan(
					(
						legacy_start.as_ref().map(Vec::as_slice),
						Bound::Excluded(legacy_range.end.as_slice()),
					),
					Some(limit + 1),
					false,
				)
				.await?
		}
	};

	let mut names = std::collections::BTreeSet::new();
	for (key, _) in indexed {
		names.insert(NamespaceIndexKey::unpack(&key)?.name);
	}
	for (key, _) in legacy {
		names.insert(LegacyNamespaceKey::unpack(&key)?.name);
	}
	let has_more = names.len() > limit;
	let names = names.into_iter().take(limit).collect::<Vec<_>>();
	let cursor = if has_more {
		names.last().cloned()
	} else {
		None
	};
	Ok(ns::NamespaceIndexListResponse {
		names: names.into_iter().map(Into::into).collect(),
		cursor: cursor.map(Into::into),
	})
}

/// Legacy: seeds the `default` namespace's record row, as the catalog always did, when
/// `old_binaries` run, since a binary from before namespace volumes finds `default` only there.
/// Otherwise `default` is created on its own volume by `resolve` and no row is seeded.
async fn legacy_bootstrap(clock: &dyn Clock, old_binaries: bool, db: &KvHandle) -> Result<()> {
	NamespaceCatalogVolume::from_volume_key(db.volume_key())?;
	if !old_binaries {
		return Ok(());
	}
	let key = LegacyNamespaceKey::new(crate::DEFAULT_NAMESPACE.to_owned());
	let mut txn = db.begin().await?;
	if txn.get(&key.pack()).await?.is_none() {
		let record = wire::NamespaceRecord {
			create_unix_ms: clock.observe()?.unix_ms,
			create_operation_id: vec![0; 16].into(),
		};
		txn.put(&key.pack(), &key.serialize(at_write_version(db, record)?)?)?;
		db.commit(txn).await?;
	}
	Ok(())
}

/// Legacy: creates a namespace as a record row of the catalog, as binaries from before
/// namespace volumes do. It is a pre-group namespace for good once moved.
async fn legacy_create_handler(
	clock: Arc<dyn Clock>,
	old_binaries: bool,
	db: KvHandle,
	request: wire::NamespaceCreateRequest,
) -> Result<wire::NamespaceCreateResponse> {
	let serial = db.opening_state::<CatalogLock>().await?;
	let _serial = serial.0.lock().await;
	crate::validation::namespace_name(&request.name)?;
	legacy_bootstrap(clock.as_ref(), old_binaries, &db).await?;
	let key = LegacyNamespaceKey::new(request.name.to_string());
	let mut txn = db.begin().await?;
	if let Some(stored) = txn.get(&key.pack()).await? {
		let record = key.deserialize(stored.into())?;
		return Ok(wire::NamespaceCreateResponse {
			created: record.create_operation_id.as_ref() == db.operation_id(),
			record,
		});
	}
	// A name indexed by a newer binary is taken, though its record lives on its volume.
	if txn
		.get(&NamespaceIndexKey::new(request.name.to_string()).pack())
		.await?
		.is_some()
	{
		return Err(crate::validation::NamespaceNameNotUnique.build());
	}
	let record = wire::NamespaceRecord {
		create_unix_ms: request.create_unix_ms,
		create_operation_id: db.operation_id().to_vec().into(),
	};
	txn.put(
		&key.pack(),
		&key.serialize(at_write_version(&db, record.clone())?)?,
	)?;
	db.commit(txn).await?;
	Ok(wire::NamespaceCreateResponse {
		created: true,
		record,
	})
}

/// Legacy: reads one record row of the catalog from a snapshot.
async fn legacy_read_handler(
	db: KvHandle,
	request: wire::NamespaceReadRequest,
) -> Result<wire::NamespaceReadResponse> {
	NamespaceCatalogVolume::from_volume_key(db.volume_key())?;
	let key = LegacyNamespaceKey::new(request.name.to_string());
	let value = db.snapshot().await?.get(&key.pack()).await?;
	Ok(wire::NamespaceReadResponse {
		record: value
			.map(|value| key.deserialize(value.into()))
			.transpose()?,
	})
}

/// Legacy: pages the catalog's record rows, for binaries from before namespace volumes.
async fn legacy_list_handler(
	clock: Arc<dyn Clock>,
	old_binaries: bool,
	db: KvHandle,
	request: wire::NamespaceListRequest,
) -> Result<wire::NamespaceListResponse> {
	let serial = db.opening_state::<CatalogLock>().await?;
	let _serial = serial.0.lock().await;
	legacy_bootstrap(clock.as_ref(), old_binaries, &db).await?;
	ensure!(request.limit > 0, "namespace list limit must be positive");
	let range = LegacyNamespacePrefix::new().range();
	let start = request
		.after
		.map(|name| Bound::Excluded(LegacyNamespaceKey::new(name.to_string()).pack().to_vec()))
		.unwrap_or(Bound::Included(range.start));
	let limit = (request.limit as usize).min(100);
	let entries = db
		.snapshot()
		.await?
		.scan(
			(
				start.as_ref().map(Vec::as_slice),
				Bound::Excluded(range.end.as_slice()),
			),
			Some(limit + 1),
			false,
		)
		.await?;
	let has_more = entries.len() > limit;
	let mut namespaces = Vec::new();
	for (key, value) in entries.into_iter().take(limit) {
		let key = LegacyNamespaceKey::unpack(&key)?;
		namespaces.push(wire::NamespaceEntry {
			name: key.name.clone().into(),
			record: key.deserialize(value.into())?,
		});
	}
	let cursor = if has_more {
		namespaces.last().map(|entry| entry.name.clone())
	} else {
		None
	};
	Ok(wire::NamespaceListResponse { namespaces, cursor })
}

/// The namespace whose volume group is `group`, from the group record's key, resolved once
/// per process. A storage worker's executor needs it to pick an actor's envoys, which its data
/// record does not name for an inherited actor. Every namespace with a group created its record
/// before going `Live`, so a group without one is an error.
pub async fn group_namespace(ctx: &EngineContext, group: GroupId) -> Result<String> {
	if let Some(name) = ctx.cached_group_name(&group).await {
		return Ok(name);
	}
	let record = ctx
		.depot()
		.read_group(group)
		.await?
		.context("the volume's group has no record")?;
	let key = NamespaceGroupKey::from_volume_key(&depot_types::VolumeKey {
		partition_key: record.key.to_vec().into(),
		name: None,
	})
	.context("a group record's key is not a namespace group key")?;
	let name = key.namespace;
	ctx.remember_group_name(group, name.clone()).await;
	Ok(name)
}

/// Whether `key` is a namespace volume, whose alarms belong to its namespace machine.
pub fn is_namespace_volume(key: &depot_types::VolumeKey) -> bool {
	NamespaceVolume::from_volume_key(key).is_ok()
}

/// Delivers a due alarm of a namespace volume to its namespace machine.
pub async fn deliver_alarm(
	ctx: &EngineContext,
	key: depot_types::VolumeKey,
	alarm: depot_worker::alarms::AlarmContext,
) -> Result<()> {
	ctx.depot()
		.command::<NamespaceAlarm>(
			&key,
			ns::NamespaceAlarmRequest {
				revision: alarm.revision,
				owner: alarm.owner,
			},
		)
		.await?;
	Ok(())
}

async fn pool_handler(
	clock: Arc<dyn Clock>,
	db: KvHandle,
	request: wire::PoolRequest,
) -> Result<wire::PoolResponse> {
	let serial = db.opening_state::<CatalogLock>().await?;
	let _serial = serial.0.lock().await;
	EnvoyPoolIndexVolume::from_volume_key(db.volume_key())?;
	crate::validation::pool_name(&request.pool_name)?;
	let name = request.pool_name.to_string();
	let key = EnvoyPoolKey::new(name.clone());
	let mut txn = db.begin().await?;
	let mut config = txn
		.get(&key.pack())
		.await?
		.map(|value| key.deserialize(value.into()))
		.transpose()?;
	let mut status = if config.is_some() {
		wire::PoolStatus::Ok
	} else {
		wire::PoolStatus::NotFound
	};
	let oldest = clock
		.observe()?
		.unix_ms
		.saturating_sub(crate::ENVOY_HEARTBEAT_TIMEOUT_MS);
	match request.operation {
		wire::PoolOperation::PoolPut(value) => {
			txn.put(
				&key.pack(),
				&key.serialize(at_write_version(&db, value.clone())?)?,
			)?;
			config = Some(value);
			status = wire::PoolStatus::Ok;
		}
		wire::PoolOperation::PoolRead => {}
		wire::PoolOperation::PoolDelete => {
			// Newest lease first: one row proves whether any connected node remains.
			let range = PoolHeartbeatPrefix::new(name.clone()).range();
			let newest = txn
				.scan(
					(
						Bound::Included(range.start.as_slice()),
						Bound::Excluded(range.end.as_slice()),
					),
					Some(1),
					true,
				)
				.await?;
			let connected = newest
				.first()
				.map(|(raw, _)| PoolHeartbeatKey::unpack(raw))
				.transpose()?
				.is_some_and(|lease| lease.heartbeat_unix_ms > oldest);
			if connected {
				status = wire::PoolStatus::Connected;
			} else {
				txn.delete(&key.pack())?;
				txn.delete_range((
					Bound::Included(range.start.as_slice()),
					Bound::Excluded(range.end.as_slice()),
				))?;
				let range = PoolConnectionPrefix::new(name.clone()).range();
				txn.delete_range((
					Bound::Included(range.start.as_slice()),
					Bound::Excluded(range.end.as_slice()),
				))?;
			}
		}
		operation @ (wire::PoolOperation::PoolConnect(_) | wire::PoolOperation::PoolRenew(_)) => {
			let renew = matches!(operation, wire::PoolOperation::PoolRenew(_));
			let connection = match operation {
				wire::PoolOperation::PoolConnect(connection)
				| wire::PoolOperation::PoolRenew(connection) => connection,
				_ => unreachable!(),
			};
			let id = uuid::Uuid::from_slice(&connection.connection_id)
				.context("invalid connection id")?;
			let lease_key = PoolConnectionKey::new(name.clone(), id);
			let previous = txn
				.get(&lease_key.pack())
				.await?
				.map(|raw| lease_key.deserialize(raw.into()))
				.transpose()?;
			// Deletion and disconnect remove the admission under the same lock. A fresh
			// heartbeat can regain admission after expiry only while that record still exists.
			let can_admit = config.is_some() && (!renew || previous.is_some());
			let fresh = connection.heartbeat_unix_ms > oldest;
			if renew && can_admit && !fresh {
				// The heartbeat deadline passed. The next pong can retry on this connection.
				return Err(depot_client::RequestDeadlineExceeded.build());
			}
			if can_admit && fresh {
				let heartbeat_unix_ms = previous
					.as_ref()
					.map_or(connection.heartbeat_unix_ms, |lease| {
						lease.heartbeat_unix_ms.max(connection.heartbeat_unix_ms)
					});
				if let Some(previous) = previous {
					txn.delete(
						&PoolHeartbeatKey::new(name.clone(), previous.heartbeat_unix_ms, id).pack(),
					)?;
				}
				txn.put(
					&lease_key.pack(),
					&lease_key.serialize(at_write_version(
						&db,
						wire::PoolLease { heartbeat_unix_ms },
					)?)?,
				)?;
				txn.put(
					&PoolHeartbeatKey::new(name, heartbeat_unix_ms, id).pack(),
					&[],
				)?;
			} else {
				status = wire::PoolStatus::NotFound;
			}
		}
		wire::PoolOperation::PoolDisconnect(id) => {
			let id = uuid::Uuid::from_slice(&id).context("invalid connection id")?;
			let lease_key = PoolConnectionKey::new(name.clone(), id);
			if let Some(raw) = txn.get(&lease_key.pack()).await? {
				let lease = lease_key.deserialize(raw.into())?;
				txn.delete(&PoolHeartbeatKey::new(name, lease.heartbeat_unix_ms, id).pack())?;
				txn.delete(&lease_key.pack())?;
			}
		}
	}
	db.commit(txn).await?;
	Ok(wire::PoolResponse { status, config })
}

async fn pool_list_handler(
	db: KvHandle,
	request: wire::PoolListRequest,
) -> Result<wire::PoolListResponse> {
	EnvoyPoolIndexVolume::from_volume_key(db.volume_key())?;
	ensure!(request.limit > 0, "pool list limit must be positive");
	let range = EnvoyPoolPrefix::new().range();
	let start = request
		.after
		.map(|name| Bound::Excluded(EnvoyPoolKey::new(name.to_string()).pack().to_vec()))
		.unwrap_or(Bound::Included(range.start));
	let limit = (request.limit as usize).min(100);
	let entries = db
		.snapshot()
		.await?
		.scan(
			(
				start.as_ref().map(Vec::as_slice),
				Bound::Excluded(range.end.as_slice()),
			),
			Some(limit + 1),
			false,
		)
		.await?;
	let more = entries.len() > limit;
	let mut pools = Vec::new();
	for (key, value) in entries.into_iter().take(limit) {
		let key = EnvoyPoolKey::unpack(&key)?;
		pools.push(wire::PoolEntry {
			name: key.pool_name.clone().into(),
			config: key.deserialize(value.into())?,
		});
	}
	let cursor = if more {
		pools.last().map(|entry| entry.name.clone())
	} else {
		None
	};
	Ok(wire::PoolListResponse { pools, cursor })
}

/// Whether every live storage worker reads namespace volumes, so namespaces may be created on
/// them and legacy rows moved into them. `false` while a binary from before namespace volumes
/// runs, which would find a namespace only in the catalog. An error means the view of the fleet
/// is unknown right now: a create is refused and retried rather than written as a legacy row,
/// since a legacy row would leave the namespace pre-group for good.
pub fn namespace_volumes_open(ctx: &EngineContext) -> Result<bool> {
	let topology = ctx.topology();
	namespace_volumes_open_for(
		topology.version_for(VersionFamily::PEGBOARD_NAMESPACE),
		topology.config().lease.refresh_hint_interval_ms,
	)
}

/// [`namespace_volumes_open`] from a fleet read of the namespace family. An unknown fleet view
/// (`topology.version_unknown`: before this process's first storage listing, or while a listed
/// storage worker is unobserved) is the retryable refusal `depot.overloaded` with reason
/// [`VERSION_UNKNOWN_REASON`] and the prompt relist interval as its hint, which every edge
/// retries, rather than an internal error.
pub(crate) fn namespace_volumes_open_for(
	fleet_version: Result<u16>,
	retry_after_ms: u64,
) -> Result<bool> {
	let error = match fleet_version {
		Ok(_) => return Ok(true),
		Err(error) => error,
	};
	let structured = rivet_error::RivetError::extract(&error);
	match (structured.group(), structured.code()) {
		("topology", "version_unavailable") => Ok(false),
		("topology", "version_unknown") => Err(depot_client::Overloaded {
			reason: VERSION_UNKNOWN_REASON.to_owned(),
			retry_after_ms,
		}
		.build()),
		_ => Err(error),
	}
}

/// The `depot.overloaded` reason of a namespace request refused while this process does not
/// yet know which storage workers read namespace volumes.
pub const VERSION_UNKNOWN_REASON: &str = "version_unknown";

/// The namespace's state from its own volume, or `None` when the volume holds none. A host
/// from before namespace volumes does not know the command; no namespace volume holds a state
/// while one serves, so that reads as `None` too, unless `volumes_open` says every live
/// storage worker reads namespace volumes. Then the host is one that has not registered yet,
/// and the error surfaces instead of a transient not found.
pub async fn read_state(
	ctx: &EngineContext,
	name: &str,
	volumes_open: bool,
) -> Result<Option<ns::NamespaceState>> {
	crate::metrics::NAMESPACE_STATUS_READS.inc();
	match ctx
		.depot()
		.command::<NamespaceRead>(&NamespaceVolume::new(name.to_owned()).volume_key(), ())
		.await
	{
		Ok(response) => Ok(response.state),
		Err(error) if depot_client::groups::is_volume_not_found(&error) => Ok(None),
		Err(error) if !volumes_open && is_unknown_command(&error) => {
			tracing::debug!(
				namespace = name,
				"a host from before namespace volumes does not know the namespace read during the rollout; answering no namespace state"
			);
			Ok(None)
		}
		Err(error) => Err(error),
	}
}

/// Whether Depot answered that the host does not know the command, as a host from before the
/// command's binary does.
fn is_unknown_command(error: &anyhow::Error) -> bool {
	let structured = rivet_error::RivetError::extract(error);
	structured.group() == "depot" && structured.code() == "unknown_command"
}

/// Legacy: reads the namespace's record row from the catalog. The row is where every namespace
/// lived before namespace volumes; namespaces now live on their own volumes. Taken only for a
/// name whose volume holds no state: one created before namespace volumes, or while the fleet
/// could not read them, and not moved yet.
pub async fn legacy_read_namespace(
	ctx: &EngineContext,
	name: &str,
) -> Result<Option<wire::NamespaceRecord>> {
	let response = ctx
		.depot()
		.command::<LegacyNamespaceRead>(
			&NamespaceCatalogVolume::new().volume_key(),
			wire::NamespaceReadRequest {
				name: name.to_owned().into(),
			},
		)
		.await;
	match response {
		Ok(response) => Ok(response.record),
		// A catalog that was never written holds no rows.
		Err(error) if depot_client::groups::is_volume_not_found(&error) => Ok(None),
		Err(error) => Err(error),
	}
}

/// Legacy: one page of namespaces from the catalog's legacy record rows, each with the state
/// its row reads as, and the cursor of the next page. The legacy behavior listed every namespace
/// from these rows; the new one pages the listing index and reads each namespace's volume. Taken
/// only while a storage worker that cannot read namespace volumes lives: such a worker, from
/// before namespace volumes, does not know the index list command, and every namespace is a
/// legacy row until the fleet reads namespace volumes. Removed in 3.1.
///
/// The fleet can read as unable to open namespace volumes again after namespaces moved into
/// them, when a storage worker of the previous release is listed once more. The rows then miss
/// every moved and new namespace, so a listing whose catalog index holds any name is refused as
/// retryable instead of answered partial. The probe reads the index entries only: the legacy
/// rows, `default`'s seeded row among them, are what this listing answers. One probe suffices,
/// since a move writes the index entry and deletes the row in one catalog transaction, but the
/// probe and the row page are two snapshots, so a move committing between them can still be
/// missed by one listing. A catalog host of the previous release does not know the index list;
/// its catalog has no index unless it was granted the catalog after the move, which only that
/// same restart can cause.
pub async fn legacy_list_namespaces(
	ctx: &EngineContext,
	after: Option<String>,
	limit: u32,
) -> Result<(Vec<(String, ns::NamespaceState)>, Option<String>)> {
	let catalog = NamespaceCatalogVolume::new().volume_key();
	let depot = ctx.depot();
	let rows = depot.command::<LegacyNamespaceList>(
		&catalog,
		wire::NamespaceListRequest {
			after: after.map(Into::into),
			limit,
		},
	);
	let index = depot.command::<NamespaceIndexList>(
		&catalog,
		ns::NamespaceIndexListRequest {
			after: None,
			limit: 1,
			legacy_index_only: true,
		},
	);
	let (page, index) = futures_util::join!(rows, index);
	let indexed = match index {
		Ok(index) => !index.names.is_empty(),
		Err(error) if is_unknown_command(&error) => false,
		Err(error) => return Err(error),
	};
	if indexed {
		crate::metrics::LEGACY_NAMESPACE_LIST_REFUSALS.inc();
		tracing::warn!(
			"refused a legacy namespace listing because namespaces already moved into their volumes; a storage worker of the previous release is listed again after the roll"
		);
		return Err(depot_client::Overloaded {
			reason: LEGACY_LIST_PARTIAL_REASON.to_owned(),
			retry_after_ms: ctx.topology().config().lease.refresh_interval_ms,
		}
		.build());
	}
	let page = page?;
	let namespaces = page
		.namespaces
		.iter()
		.map(|entry| (entry.name.to_string(), legacy_state(&entry.record)))
		.collect();
	Ok((namespaces, page.cursor.map(|cursor| cursor.to_string())))
}

/// The `depot.overloaded` reason of a legacy listing refused because it would omit namespaces
/// that already moved into their volumes.
const LEGACY_LIST_PARTIAL_REASON: &str = "legacy_list_partial";

/// Legacy: the state a namespace served from its legacy row reads as, while the fleet cannot
/// read namespace volumes and the row cannot be moved. Every legacy row is a live namespace with
/// no group yet, so its volumes are the unprefixed ones the older binaries read, and its backfill
/// is pending. The move gives it its group once the fleet reads namespace volumes.
fn legacy_state(record: &wire::NamespaceRecord) -> ns::NamespaceState {
	let group = None;
	let mut operation_id = [0u8; 16];
	if let Ok(bytes) = <[u8; 16]>::try_from(record.create_operation_id.as_ref()) {
		operation_id = bytes;
	}
	ns::NamespaceState {
		create_unix_ms: record.create_unix_ms,
		operation_id,
		group,
		phase: ns::NamespacePhase::NamespaceLive,
		index: ns::NamespaceIndexState::NamespaceIndexWritten,
		backfill: ns::NamespaceBackfill::NamespaceBackfillPending(ns::NamespaceBackfillPending {
			fleet_ready_seen: false,
		}),
		start_waiters: Vec::new(),
	}
}

/// Legacy: moves a namespace from its legacy catalog row into its namespace volume. The legacy
/// behavior kept every record as a catalog row; the new one keeps it on the namespace's own
/// volume. Taken on the first resolution of a name that has only a legacy row, once the fleet
/// reads namespace volumes. The volume is written `Live` (concurrent moves converge on it) with
/// its index entry pending, and the machine's index write deletes the row in the same catalog
/// transaction. A crash before that write leaves both, every read prefers the volume, and the
/// machine's `IndexRetry` finishes it.
pub async fn legacy_move_namespace(
	ctx: &EngineContext,
	name: &str,
	record: &wire::NamespaceRecord,
) -> Result<ns::NamespaceState> {
	let moved = ctx
		.depot()
		.command::<LegacyNamespaceMove>(
			&NamespaceVolume::new(name.to_owned()).volume_key(),
			ns::LegacyNamespaceMoveRequest {
				create_unix_ms: record.create_unix_ms,
			},
		)
		.await?;
	crate::metrics::NAMESPACE_LEGACY_MOVES.inc();
	Ok(moved.state)
}

/// Legacy: one page of the catalog's legacy record rows after `after`, through the legacy list
/// command, for the legacy namespace row pass. The legacy behavior listed these rows for binaries
/// from before namespace volumes; the pass lists them to move every one. Taken only by the pass,
/// once every storage worker reads namespace volumes. Removed in 3.1 with the backfill.
pub(crate) async fn legacy_list_rows(
	ctx: &EngineContext,
	after: Option<String>,
	limit: u32,
) -> Result<crate::legacy_namespace_rows::LegacyRowPage> {
	let page = ctx
		.depot()
		.command::<LegacyNamespaceList>(
			&NamespaceCatalogVolume::new().volume_key(),
			wire::NamespaceListRequest {
				after: after.map(Into::into),
				limit,
			},
		)
		.await?;
	Ok(crate::legacy_namespace_rows::LegacyRowPage {
		rows: page
			.namespaces
			.into_iter()
			.map(|entry| (entry.name.to_string(), entry.record))
			.collect(),
		cursor: page.cursor.map(|cursor| cursor.to_string()),
	})
}

/// Legacy: moves `name` from its legacy catalog row into its namespace volume and finishes its
/// group create, as a resolution of the name does. The legacy behavior moved a row only when a
/// request read it; the legacy namespace row pass moves every row this way. Taken only by the
/// pass, once every storage worker reads namespace volumes. Removed in 3.1 with the backfill.
///
/// A row whose volume already holds an indexed state is deleted here, since no machine write
/// will delete it. The answer is not remembered in the process cache: the pass runs on one
/// storage worker, and a resolution on any worker reads the volume first anyway.
pub(crate) async fn legacy_move_row(
	ctx: &EngineContext,
	name: &str,
	record: &wire::NamespaceRecord,
) -> Result<()> {
	crate::validation::namespace_name(name)?;
	let moved = legacy_move_namespace(ctx, name, record).await?;
	let state = settle_group_create(ctx, name, moved).await?;
	legacy_delete_dead_row(name, &state, |request| async move {
		ctx.depot()
			.command::<NamespaceIndexPut>(&NamespaceCatalogVolume::new().volume_key(), request)
			.await?;
		Ok(())
	})
	.await
}

/// Legacy: sends through `put` the index write that deletes `name`'s legacy row after a move
/// answered `state`, when no machine write will. A move onto a volume that already holds a state
/// changes nothing, so a row written beside an indexed volume, by a 3.0.0 binary hosting the
/// catalog that accepted a create of a name already on its volume, outlives every move. The
/// volume wins every read, so the row is dead and the index write, which is idempotent, deletes
/// it. Nothing is sent while the index write is pending: the machine's `IndexRetry` writes it and
/// deletes the row. `put` is the catalog command in production and the catalog's handler in its
/// tests, which share this decision. Removed in 3.1 with the backfill.
pub(crate) async fn legacy_delete_dead_row<F, Fut>(
	name: &str,
	state: &ns::NamespaceState,
	put: F,
) -> Result<()>
where
	F: FnOnce(ns::NamespaceIndexPutRequest) -> Fut,
	Fut: std::future::Future<Output = Result<()>>,
{
	match state.index {
		ns::NamespaceIndexState::NamespaceIndexWritten => {
			put(ns::NamespaceIndexPutRequest {
				name: name.to_owned().into(),
				create_unix_ms: state.create_unix_ms,
			})
			.await
		}
		ns::NamespaceIndexState::NamespaceIndexPending(_) => Ok(()),
	}
}

/// Reads the namespace `name` wherever it lives: its volume, else its legacy row, moved on the
/// way when the fleet allows. `default` is created on first use: on its volume once the fleet
/// reads namespace volumes, and as its legacy row while old binaries run, since only that is
/// where they find it. While old binaries run no namespace volume holds a state, so the volume
/// read is skipped. Only the `default` seed writes; every other resolution reads.
///
/// The process cache keeps the catalog off the hot path: a miss costs one volume read plus, for
/// a name whose volume holds no state, one snapshot read of the catalog.
async fn resolve(ctx: &EngineContext, name: &str) -> Result<Option<ns::NamespaceState>> {
	let volumes_open = namespace_volumes_open(ctx);
	if !matches!(volumes_open, Ok(false))
		&& let Some(state) = read_state(ctx, name, matches!(volumes_open, Ok(true))).await?
	{
		return Ok(Some(state));
	}
	if let Some(record) = legacy_read_namespace(ctx, name).await? {
		// With old binaries running, or the fleet's view unknown, the row stays and serves.
		if matches!(volumes_open, Ok(true)) {
			let moved = legacy_move_namespace(ctx, name, &record).await?;
			return settle_group_create(ctx, name, moved).await.map(Some);
		}
		return Ok(Some(legacy_state(&record)));
	}
	let read_again = legacy_read_after_row_gone(&volumes_open, |volumes_open| {
		read_state(ctx, name, volumes_open)
	})
	.await?;
	if let Some(state) = read_again {
		return Ok(Some(state));
	}
	if name != crate::DEFAULT_NAMESPACE {
		return Ok(None);
	}
	if !volumes_open? {
		return legacy_seed_default(ctx).await.map(Some);
	}
	let created = ctx
		.depot()
		.command::<NamespaceCreate>(
			&NamespaceVolume::new(name.to_owned()).volume_key(),
			ns::NamespaceCreateRequest {
				create_unix_ms: ctx.env().clock().observe()?.unix_ms,
				fork_from: None,
				waiter: None,
			},
		)
		.await?;
	settle_group_create(ctx, name, created.state)
		.await
		.map(Some)
}

/// Legacy: reads the namespace's volume through `read` once more after its legacy row was found
/// gone. A row is deleted only after its namespace's volume holds the moved state, so a volume
/// found empty or skipped before and a row gone now means another process moved the namespace in
/// between; its volume answers. The read runs whatever this process's fleet view
/// `volumes_open` was, since that view is local and can lag the process that moved the
/// namespace: an unknown or unavailable view reads tolerantly, answering no state from a host
/// that does not know the read yet. A name that does not exist pays this second volume read on
/// every lookup, since a missing name is not cached; that is accepted for as long as legacy rows
/// exist. Removed in 3.1 with the legacy rows.
pub(crate) async fn legacy_read_after_row_gone<F, Fut>(
	volumes_open: &Result<bool>,
	read: F,
) -> Result<Option<ns::NamespaceState>>
where
	F: FnOnce(bool) -> Fut,
	Fut: std::future::Future<Output = Result<Option<ns::NamespaceState>>>,
{
	let fleet_reads_volumes = matches!(volumes_open, Ok(true));
	read(fleet_reads_volumes).await
}

/// Finishes the group create of a namespace a resolution creates (`default`) or moves from its
/// legacy row, on the request that wrote it, so the first request of a fresh or upgraded cluster
/// finds it live instead of answering `namespace.pending`. No start wait exists on this path, so it runs the
/// same group create the machine's effect runs, which is idempotent, then reads the state once.
/// An unknown outcome answers `Creating`, and the machine's `Retry` finishes it.
async fn settle_group_create(
	ctx: &EngineContext,
	name: &str,
	state: ns::NamespaceState,
) -> Result<ns::NamespaceState> {
	if !matches!(state.phase, ns::NamespacePhase::NamespaceCreating(_)) {
		return Ok(state);
	}
	let Some(group) = state.group.as_ref().map(|group| group.group) else {
		return Ok(state);
	};
	Executor::new(ctx, name.to_owned())
		.run_group_create(group)
		.await;
	let settled = read_state(ctx, name, true).await?;
	Ok(settled.unwrap_or(state))
}

/// Legacy: seeds `default`'s legacy catalog row while old binaries run and answers the state it
/// reads as. The legacy behavior seeded the row as each binary first touched the catalog; the
/// new behavior seeds it only while old binaries run, which the legacy create's bootstrap does.
/// Taken when a resolution of `default` finds no row while the fleet cannot read namespace
/// volumes; a row seeded by an earlier writer answers as it is.
async fn legacy_seed_default(ctx: &EngineContext) -> Result<ns::NamespaceState> {
	let response = ctx
		.depot()
		.command::<LegacyNamespaceCreate>(
			&NamespaceCatalogVolume::new().volume_key(),
			wire::NamespaceCreateRequest {
				name: crate::DEFAULT_NAMESPACE.to_owned().into(),
				create_unix_ms: ctx.env().clock().observe()?.unix_ms,
			},
		)
		.await?;
	Ok(legacy_state(&response.record))
}

/// The namespace's state, read exactly from wherever it lives, and remembered in the process
/// cache. `None` when no namespace has the name. A listing by name and the API's own writes
/// use it; request paths use the cached `namespace_standing`.
pub async fn lookup_namespace(
	ctx: &EngineContext,
	name: &str,
) -> Result<Option<ns::NamespaceState>> {
	crate::validation::namespace_name(name)?;
	let state = resolve(ctx, name).await?;
	if let Some(state) = &state {
		remember(ctx, name, state.clone()).await?;
	}
	Ok(state)
}

/// Legacy: before a create of `name`, moves a name that lives only as a legacy catalog row to
/// its namespace volume, where the create's get-or-create then finds it taken. The legacy
/// behavior resolved the name fully first, which also sent a read to the name's namespace
/// volume; that volume does not exist for a new name, so the read was refused as not found on
/// every create, and the create's own get-or-create of the volume already answers a name its
/// volume holds. Taken on every create while the fleet reads namespace volumes; `default` still
/// resolves fully, since its resolution seeds it. Removed in 3.1 with the legacy rows.
pub async fn legacy_move_before_create(ctx: &EngineContext, name: &str) -> Result<()> {
	crate::validation::namespace_name(name)?;
	if name == crate::DEFAULT_NAMESPACE {
		lookup_namespace(ctx, name).await?;
		return Ok(());
	}
	let Some(record) = legacy_read_namespace(ctx, name).await? else {
		return Ok(());
	};
	let moved = legacy_move_namespace(ctx, name, &record).await?;
	let state = settle_group_create(ctx, name, moved).await?;
	remember(ctx, name, state).await
}

/// Remembers `state` in the process cache as read now.
pub async fn remember(ctx: &EngineContext, name: &str, state: ns::NamespaceState) -> Result<()> {
	let now_boot_ms = ctx.env().clock().observe()?.boot_ms;
	ctx.remember_namespace(name, state, now_boot_ms).await;
	Ok(())
}

/// The namespace's state, from the process cache or the namespace's volume. Existence is
/// cached for good, because namespaces are never renamed or deleted, and a namespace this
/// process never saw is read on the request once. The phase is read again once the entry is
/// older than `pegboard.namespace_status_ttl_ms`, by one background read per namespace while
/// requests keep the cached state, so a slow namespace volume never holds up routing; only a
/// namespace nobody uses costs nothing. A cached `Creating` namespace is the exception: it is read
/// on the request after a much shorter TTL (see [`reads_on_request_when_stale`]). A re-read that
/// fails keeps the cached state, since the archived group at the tablet is the authoritative
/// fence, not this cache.
pub async fn namespace_state(ctx: &EngineContext, name: &str) -> Result<Arc<ns::NamespaceState>> {
	Ok(cached_namespace(ctx, name).await?.state().clone())
}

/// The process cache's entry for the namespace, read again as [`namespace_state`] describes.
async fn cached_namespace(ctx: &EngineContext, name: &str) -> Result<CachedNamespace> {
	cached_namespace_with(ctx, name, |ctx: EngineContext, name: String| async move {
		resolve(&ctx, &name).await
	})
	.await
}

/// [`cached_namespace`] with `read`, which reads the namespace's state uncached.
async fn cached_namespace_with<R, Fut>(
	ctx: &EngineContext,
	name: &str,
	read: R,
) -> Result<CachedNamespace>
where
	R: FnOnce(EngineContext, String) -> Fut + Send + 'static,
	Fut: std::future::Future<Output = Result<Option<ns::NamespaceState>>> + Send + 'static,
{
	crate::validation::namespace_name(name)?;
	let now_boot_ms = ctx.env().clock().observe()?.boot_ms;
	let ttl_ms = ctx.config().pegboard().namespace_status_ttl_ms();

	// A namespace this process never saw is read on the request, once.
	let Some(cached) = ctx.cached_namespace(name).await else {
		let state = read(ctx.clone(), name.to_owned()).await?;
		return remember_found(ctx, name, state, now_boot_ms).await;
	};
	if now_boot_ms.saturating_sub(cached.read_at_boot_ms()) < cached_ttl_ms(cached.state(), ttl_ms)
	{
		return Ok(cached);
	}

	if reads_on_request_when_stale(cached.state()) {
		return match read(ctx.clone(), name.to_owned()).await {
			Ok(state) => remember_found(ctx, name, state, now_boot_ms).await,
			Err(error) => {
				tracing::debug!(
					?error,
					namespace = name,
					"namespace status re-read failed; keeping the cached state"
				);
				Ok(cached)
			}
		};
	}
	refresh_in_background(ctx, name, &cached, now_boot_ms, read);
	Ok(cached)
}

/// Remembers `state`, read at `read_at_boot_ms`, and answers its entry, or refuses the request
/// as not found when no namespace has the name.
async fn remember_found(
	ctx: &EngineContext,
	name: &str,
	state: Option<ns::NamespaceState>,
	read_at_boot_ms: u64,
) -> Result<CachedNamespace> {
	let Some(state) = state else {
		return Err(crate::validation::NamespaceNotFound.build());
	};
	Ok(ctx.remember_namespace(name, state, read_at_boot_ms).await)
}

/// Starts the background re-read of `cached`, a stale entry of `name`, through `read`, unless
/// one already runs for the same entry or every one of the process's
/// `pegboard.namespace_status_refresh_max_concurrency` slots is taken. The read keeps the
/// Depot command's own deadline; its result replaces the entry. A missing state or a failure
/// keeps the cached one and holds off the next re-read for
/// [`NAMESPACE_REFRESH_FAILURE_HOLD_OFF_MS`], so a failing or missing namespace volume sees one
/// read per hold-off from each process instead of one always in flight.
fn refresh_in_background<R, Fut>(
	ctx: &EngineContext,
	name: &str,
	cached: &CachedNamespace,
	now_boot_ms: u64,
	read: R,
) where
	R: FnOnce(EngineContext, String) -> Fut + Send + 'static,
	Fut: std::future::Future<Output = Result<Option<ns::NamespaceState>>> + Send + 'static,
{
	let capacity = ctx
		.config()
		.pegboard()
		.namespace_status_refresh_max_concurrency();
	crate::metrics::NAMESPACE_STATUS_REFRESHES_CAPACITY
		.set(i64::try_from(capacity).unwrap_or(i64::MAX));
	let refresh = match ctx.begin_namespace_refresh(cached, capacity, now_boot_ms) {
		NamespaceRefreshBegin::Started(refresh) => refresh,
		NamespaceRefreshBegin::InFlight | NamespaceRefreshBegin::HeldOff => return,
		NamespaceRefreshBegin::Full => {
			crate::metrics::NAMESPACE_STATUS_REFRESHES_SKIPPED.inc();
			return;
		}
	};
	if refresh.reached_cap() {
		tracing::warn!(
			capacity,
			"background namespace status reads reached their cap; stale namespaces keep their cached status until a read ends, which points at slow namespace volumes"
		);
	}

	let refresh_ctx = ctx.clone();
	let name = name.to_owned();
	ctx.spawn_namespace_refresh(async move {
		let _in_flight = crate::metrics::NAMESPACE_STATUS_REFRESHES.inc_guard();
		let started_boot_ms = match refresh_ctx.env().clock().observe() {
			Ok(clocks) => clocks.boot_ms,
			Err(error) => {
				tracing::warn!(?error, namespace = %name, "clock failed; the namespace status is read again by a later request");
				return;
			}
		};
		let kept = match read(refresh_ctx.clone(), name.clone()).await {
			Ok(Some(state)) => {
				refresh_ctx
					.remember_namespace(&name, state, started_boot_ms)
					.await;
				false
			}
			Ok(None) => {
				tracing::debug!(
					namespace = %name,
					"namespace status re-read found no state; keeping the cached state"
				);
				true
			}
			Err(error) => {
				tracing::debug!(
					?error,
					namespace = %name,
					"namespace status re-read failed; keeping the cached state"
				);
				true
			}
		};
		if kept {
			match refresh_ctx.env().clock().observe() {
				Ok(clocks) => refresh.hold_off_until(
					clocks
						.boot_ms
						.saturating_add(NAMESPACE_REFRESH_FAILURE_HOLD_OFF_MS),
				),
				Err(error) => {
					tracing::debug!(?error, namespace = %name, "clock failed; the namespace status re-read that kept the cached state is not held off");
				}
			}
		}
		drop(refresh);
	});
}

/// How long a stale namespace waits for its next background re-read after one failed.
const NAMESPACE_REFRESH_FAILURE_HOLD_OFF_MS: u64 = 1_000;

/// Whether a stale cached `state` is read again on the request rather than in the background.
/// Only `Creating` is: a root leaves it within milliseconds of its create, so the request soon
/// after sees the namespace live instead of refused as pending, and only a namespace being created
/// pays the read.
fn reads_on_request_when_stale(state: &ns::NamespaceState) -> bool {
	match state.phase {
		ns::NamespacePhase::NamespaceCreating(_) => true,
		ns::NamespacePhase::NamespaceForking(_)
		| ns::NamespacePhase::NamespaceLive
		| ns::NamespacePhase::NamespaceArchiving(_)
		| ns::NamespacePhase::NamespaceArchived(_)
		| ns::NamespacePhase::NamespaceFailed(_) => false,
	}
}

/// The retry hint of a request refused while its namespace's group create settles. A create
/// usually settles within one request; one whose outcome was unknown waits for the namespace's
/// own retry, so the hint spaces the edge's retries out instead of matching the short re-read.
const NAMESPACE_CREATING_RETRY_AFTER_MS: u64 = 1_000;

/// The retryable refusal of work on a namespace whose group create has not settled.
fn namespace_creating() -> anyhow::Error {
	crate::validation::NamespaceCreating {
		retry_after_ms: NAMESPACE_CREATING_RETRY_AFTER_MS,
	}
	.build()
}

/// How long a cached `Creating` namespace answers before it is read again. A root leaves
/// `Creating` once its group record is written, which takes milliseconds, so a request soon
/// after the create sees it live; the bound keeps a root stuck in `Creating` from adding a read
/// to every request on the routing path.
const CREATING_STATUS_TTL_MS: u64 = 250;

/// How long a cached namespace state answers before it is read again: the short
/// `CREATING_STATUS_TTL_MS` while it is `Creating`, else `ttl_ms`, whichever is shorter.
fn cached_ttl_ms(state: &ns::NamespaceState, ttl_ms: u64) -> u64 {
	match state.phase {
		ns::NamespacePhase::NamespaceCreating(_) => CREATING_STATUS_TTL_MS.min(ttl_ms),
		ns::NamespacePhase::NamespaceForking(_)
		| ns::NamespacePhase::NamespaceLive
		| ns::NamespacePhase::NamespaceArchiving(_)
		| ns::NamespacePhase::NamespaceArchived(_)
		| ns::NamespacePhase::NamespaceFailed(_) => ttl_ms,
	}
}

/// The namespace's standing, from the state [`namespace_state`] answers. Derived once per cache
/// fill and shared by every lookup of that fill, so the request path builds no standing and no
/// legacy skipped set per request.
pub async fn namespace_standing(ctx: &EngineContext, name: &str) -> Result<Arc<NamespaceStanding>> {
	let cached = cached_namespace(ctx, name).await?;
	standing_of(name, &cached)
}

/// The namespace's standing from the process cache alone, with no command on the request, or
/// `None` when this process has not read the namespace yet. A stale entry is answered as it is
/// and starts its background re-read, as [`namespace_state`] describes. Taken by paths that
/// already hold a standing from the namespace's lookup and only want a newer one when the
/// process has it, such as an envoy connection's opens and deliveries.
pub async fn cached_namespace_standing(
	ctx: &EngineContext,
	name: &str,
) -> Result<Option<Arc<NamespaceStanding>>> {
	let Some(cached) = ctx.cached_namespace(name).await else {
		return Ok(None);
	};
	let now_boot_ms = ctx.env().clock().observe()?.boot_ms;
	let ttl_ms = ctx.config().pegboard().namespace_status_ttl_ms();
	if now_boot_ms.saturating_sub(cached.read_at_boot_ms()) >= cached_ttl_ms(cached.state(), ttl_ms)
	{
		refresh_in_background(
			ctx,
			name,
			&cached,
			now_boot_ms,
			|ctx: EngineContext, name: String| async move { resolve(&ctx, &name).await },
		);
	}
	standing_of(name, &cached).map(Some)
}

/// The standing derived from `cached`, deriving it into the entry on first use. Pegboard is the
/// only crate that fills the entry's derived slot, and only with a standing.
fn standing_of(name: &str, cached: &CachedNamespace) -> Result<Arc<NamespaceStanding>> {
	cached.derived_or_init(|state| NamespaceStanding::from_state(name, state))
}

/// Legacy: whether the backfill of `namespace` lists `actor_id` as skipped, so its destroy is
/// sent to its unprefixed volume and clears the entry: a finished backfill's list, whose actors
/// are still reached by id, or a running backfill's, where the clear makes room below the cap of
/// skipped actors. The running list is scanned, which only a destroy does and which its cap
/// bounds. Removed in 3.1 with the backfill.
pub fn legacy_lists_skipped(namespace: &NamespaceStanding, actor_id: rivet_util::Id) -> bool {
	match namespace.legacy_fallback() {
		LegacyFallback::Skipped { .. } => namespace
			.legacy_fallback()
			.namespace_for(actor_id)
			.is_some(),
		LegacyFallback::Every { .. } => match &namespace.backfill {
			ns::NamespaceBackfill::NamespaceBackfillRunning(running) => running
				.skipped
				.iter()
				.any(|skipped| skipped.actor_id.as_ref() == actor_id.as_bytes().as_slice()),
			ns::NamespaceBackfill::NamespaceBackfillPending(_)
			| ns::NamespaceBackfill::NamespaceBackfillDone(_) => false,
		},
		LegacyFallback::Off => false,
	}
}

/// Legacy: removes `actor_id`, destroyed at its unprefixed volume, from the skipped list of
/// `namespace`'s backfill through the namespace's step command: once done, so forks and archives
/// of the namespace stop waiting for it and its opens stop falling back; while running, so the
/// backfill has room to give up on another actor. The step is idempotent and ignored unless the
/// backfill lists the actor, so a caller may send it again after a lost reply. Removed in 3.1 with the backfill.
pub async fn legacy_clear_skipped(
	ctx: &EngineContext,
	namespace: &str,
	actor_id: rivet_util::Id,
) -> Result<()> {
	ctx.depot()
		.command::<NamespaceStep>(
			&NamespaceVolume::new(namespace.to_owned()).volume_key(),
			ns::NamespaceStepRequest::NamespaceStepLegacySkippedDestroyed(
				ns::NamespaceStepLegacySkippedDestroyed {
					actor_id: actor_id.as_bytes().into(),
				},
			),
		)
		.await
		.context("remove a destroyed actor from the namespace's skipped backfill list")?;
	Ok(())
}

/// Legacy: the standing to act on for `standing`, a possibly stale cached standing of a namespace
/// that had no group, given whether the fleet reads namespace volumes (`volumes_open`) and
/// `reread`, which reads the namespace uncached. A grouped standing is answered as it is, since a
/// namespace never leaves its group. An ungrouped one is read again, then admitted, unless older
/// binaries are known to run (`Ok(false)`): no namespace moves into its group then, so the read
/// is skipped. An unknown view of the fleet (`Err`) reads, since a namespace may have moved. The
/// legacy behavior trusted the cached standing; the new behavior acts on the group the moment
/// any process moved the namespace into it. Taken by keyed creates and envoy connects, which would
/// otherwise act on a group-less scope for up to the status TTL after a move. Removed in 3.1 with
/// the backfill.
pub async fn legacy_fresh_ungrouped_standing<F, Fut>(
	standing: &Arc<NamespaceStanding>,
	volumes_open: Result<bool>,
	reread: F,
) -> Result<Arc<NamespaceStanding>>
where
	F: FnOnce() -> Fut,
	Fut: std::future::Future<Output = Result<Option<ns::NamespaceState>>>,
{
	if standing.group_id().is_some() || matches!(volumes_open, Ok(false)) {
		return Ok(standing.clone());
	}
	let state = reread().await?;
	let fresh = crate::actor_key::legacy_reread_standing(standing, state)?;
	Ok(Arc::new(fresh))
}

/// Legacy: the group of the namespace `name` now, read uncached, for a caller whose view of the
/// namespace had no group. `None` without a read while older binaries are known to run, since
/// no namespace moves into its group then, and `None` for a namespace whose volume holds no state.
/// The legacy behavior trusted the group seen at connect; the new behavior closes a connection
/// whose namespace gained a group since. Taken only on an envoy connection without a group, when
/// an actor's event finds no unprefixed actor at delivery. A process whose view of the fleet
/// lags skips the read, which is safe there: an actor that never existed unprefixed reads as
/// missing, and delivery retries a missing actor until the connection closes unacknowledged.
/// Removed in 3.1 with the backfill.
pub async fn legacy_current_namespace_group(
	ctx: &EngineContext,
	name: &str,
) -> Result<Option<GroupId>> {
	if legacy_old_binaries_run(ctx) {
		return Ok(None);
	}
	legacy_read_namespace_group(ctx, name).await
}

/// Legacy: the group of the namespace `name` now, read uncached from its namespace volume, and
/// `None` for a namespace whose volume holds no state. The legacy behavior trusted the group
/// seen at connect; the new behavior reads it again. Taken by
/// [`legacy_current_namespace_group`], and on an envoy connection without a group before a
/// replayed event for an actor the connection does not own is acknowledged, where acknowledging
/// would drop the event for good. Removed in 3.1 with the backfill.
///
/// The read never consults the view of the fleet, the legacy catalog row or the `default` seed,
/// and never fills the process cache: a namespace gains its group only on its volume, and the
/// move that wrote it deleted the row, so a process whose view of the fleet lags would otherwise
/// read the row's absence as a missing namespace. An old host that does not know the volume read
/// answers `None`, which is right, since no namespace volume holds a state while one serves.
pub async fn legacy_read_namespace_group(
	ctx: &EngineContext,
	name: &str,
) -> Result<Option<GroupId>> {
	// `false` is deliberate: during a mixed roll an old host's unknown command answers "no
	// namespace state" instead of a hard error that would close the connection on every replay.
	legacy_namespace_group_from(name, || read_state(ctx, name, false)).await
}

/// Legacy: the group of the namespace `name` from `read`, an uncached read of its namespace
/// volume. Split from [`legacy_read_namespace_group`] so the read is the only input. Removed in
/// 3.1 with the backfill.
async fn legacy_namespace_group_from<F, Fut>(name: &str, read: F) -> Result<Option<GroupId>>
where
	F: FnOnce() -> Fut,
	Fut: std::future::Future<Output = Result<Option<ns::NamespaceState>>>,
{
	let state = read().await?;
	let group = state.and_then(|state| NamespaceStanding::from_state(name, &state).group_id());
	Ok(group)
}

/// Envoy records read at once while an archive looks for a connected envoy.
const CONNECTED_ENVOY_READ_CONCURRENCY: usize = 16;

/// Whether any envoy of the namespace is connected, which refuses its archive. Walks each pool's
/// active envoy index one page at a time and reads the records of a page with bounded
/// concurrency, stopping at the first connected one, so memory stays one page however many
/// envoys the namespace has. An envoy that connects after this answers is not stopped by the
/// archive: its namespace reads `Archiving` within the status TTL, and its actors' next open
/// answers `group.archived`.
pub async fn namespace_has_connected_envoys(ctx: &EngineContext, namespace: &str) -> Result<bool> {
	for pool in crate::index::list_envoy_pools(ctx, namespace).await? {
		let mut after: Option<String> = None;
		loop {
			let page = crate::index::list_active_envoy_states_page(
				ctx,
				namespace,
				&pool,
				after.as_deref(),
			)
			.await?;
			let pool = pool.as_str();
			let mut reads = futures_util::stream::iter(page.envoys)
				.map(|state| async move {
					crate::index::read_envoy(ctx, namespace, pool, &state.envoy_key).await
				})
				.buffer_unordered(CONNECTED_ENVOY_READ_CONCURRENCY);
			while let Some(record) = reads.next().await {
				if record?.is_some_and(|record| record.connected) {
					return Ok(true);
				}
			}

			match page.after {
				Some(next) => after = Some(next),
				None => break,
			}
		}
	}
	Ok(false)
}

/// Legacy: creates the namespace `name` as a record row of the catalog, as binaries from before
/// namespace volumes do. Taken only for a create while the fleet cannot read namespace volumes;
/// the namespace moves into its volume as a pre-group namespace on first use afterwards.
/// Answers its state as it will read until then, or `None` when the name is taken.
pub async fn legacy_create_namespace(
	ctx: &EngineContext,
	name: &str,
	create_unix_ms: i64,
) -> Result<Option<ns::NamespaceState>> {
	let response = ctx
		.depot()
		.command::<LegacyNamespaceCreate>(
			&NamespaceCatalogVolume::new().volume_key(),
			wire::NamespaceCreateRequest {
				name: name.to_owned().into(),
				create_unix_ms,
			},
		)
		.await?;
	if !response.created {
		return Ok(None);
	}
	Ok(Some(legacy_state(&response.record)))
}

/// The namespace's standing for new work: creates, wakes, listings and envoy connects, which
/// its phase may refuse. Reads and destroys of one actor use `namespace_standing`, since they
/// keep working while the namespace archives.
pub async fn ensure_namespace(ctx: &EngineContext, name: &str) -> Result<Arc<NamespaceStanding>> {
	let standing = namespace_standing(ctx, name).await?;
	standing.admit()?;
	Ok(standing)
}

/// [`ensure_namespace`] for an API request that waits out its namespace's group create instead
/// of being refused for it; see [`settled_namespace_standing`].
pub async fn ensure_settled_namespace(
	ctx: &EngineContext,
	name: &str,
	deadline_boot_ms: u64,
) -> Result<Arc<NamespaceStanding>> {
	let standing = settled_namespace_standing(ctx, name, deadline_boot_ms).await?;
	standing.admit()?;
	Ok(standing)
}

/// The namespace's standing, as [`namespace_standing`] answers it within `deadline_boot_ms`,
/// except that a namespace whose group create has not settled may be settled on this request
/// first, by the same deadline.
///
/// A namespace is `Creating` from its create, or from the move of its legacy catalog row, until
/// its group record is written. The request that creates or moves it settles it, but a request
/// that reads it meanwhile, such as a released node host's startup pool upsert while another
/// request moves the namespace, would be refused for it at once. So one request at a time per
/// process and cached state runs the same idempotent group create the mover runs and reads the
/// state once. Every other request that finds the namespace creating meanwhile, and every one
/// for a moment after a settle failed, is refused with the retryable `namespace.creating`
/// instead of driving the group create too: nothing was done, and the caller's edge retries
/// after its hint. So is the settling request when its attempt fails, passes the deadline, or
/// leaves the create's outcome unknown for the namespace's own retry to finish.
pub async fn settled_namespace_standing(
	ctx: &EngineContext,
	name: &str,
	deadline_boot_ms: u64,
) -> Result<Arc<NamespaceStanding>> {
	// The lookup itself is bounded by the deadline too: the first one after an upgrade moves the
	// namespace's legacy row and settles its group create on the request, and a stale creating
	// state is read again on it, either of which can outlast the caller's budget. One that does
	// is refused for now before anything is admitted: as creating when this process last read
	// the namespace creating, and otherwise as the generic overload, since it may be live or
	// missing.
	let clock = ctx.env().clock();
	let standing = match rivet_env::timeout_at(
		clock.as_ref(),
		deadline_boot_ms,
		namespace_standing(ctx, name),
	)
	.await
	{
		Ok(standing) => standing?,
		Err(rivet_env::TimeoutError::Elapsed) => {
			let creating = ctx.cached_namespace(name).await.is_some_and(|cached| {
				matches!(
					cached.state().phase,
					ns::NamespacePhase::NamespaceCreating(_)
				)
			});
			if creating {
				crate::metrics::NAMESPACE_CREATING_SETTLES
					.with_label_values(&["failed"])
					.inc();
				return Err(namespace_creating());
			}
			crate::metrics::NAMESPACE_CREATING_SETTLES
				.with_label_values(&["lookup_timed_out"])
				.inc();
			return Err(depot_client::Overloaded {
				reason: NAMESPACE_LOOKUP_TIMED_OUT_REASON.to_owned(),
				retry_after_ms: NAMESPACE_CREATING_RETRY_AFTER_MS,
			}
			.build());
		}
		Err(rivet_env::TimeoutError::Clock(error)) => return Err(error.into()),
	};
	if !matches!(standing.phase, ns::NamespacePhase::NamespaceCreating(_)) {
		return Ok(standing);
	}

	// One settle at a time per name in this process, and none for a moment after one failed.
	let now_boot_ms = clock.observe()?.boot_ms;
	if !ctx
		.begin_namespace_settle(name, now_boot_ms, deadline_boot_ms)
		.await
	{
		crate::metrics::NAMESPACE_CREATING_SETTLES
			.with_label_values(&["refused"])
			.inc();
		return Err(namespace_creating());
	}

	let settled = rivet_env::timeout_at(clock.as_ref(), deadline_boot_ms, async {
		let Some(state) = read_state(ctx, name, true).await? else {
			return Ok(None);
		};
		let state = settle_group_create(ctx, name, state).await?;
		remember(ctx, name, state).await?;
		namespace_standing(ctx, name).await.map(Some)
	})
	.await;
	let result = match settled {
		Ok(Ok(Some(standing))) => Ok(standing),
		Ok(Ok(None)) => Err(crate::validation::NamespaceNotFound.build()),
		Ok(Err(error)) => {
			tracing::debug!(
				?error,
				namespace = name,
				"settling a creating namespace failed; the request is refused as creating"
			);
			Err(namespace_creating())
		}
		Err(rivet_env::TimeoutError::Elapsed) => Err(namespace_creating()),
		Err(rivet_env::TimeoutError::Clock(error)) => Err(error.into()),
	};
	let settled = result
		.as_ref()
		.is_ok_and(|standing| !matches!(standing.phase, ns::NamespacePhase::NamespaceCreating(_)));
	// A failed settle holds the next one off for as long as the refusal's hint asks the edge to
	// wait.
	let held_off_until = if settled {
		None
	} else {
		let ended_boot_ms = clock.observe().map_or(now_boot_ms, |now| now.boot_ms);
		Some(ended_boot_ms.saturating_add(NAMESPACE_CREATING_RETRY_AFTER_MS))
	};
	ctx.end_namespace_settle(name, held_off_until).await;
	crate::metrics::NAMESPACE_CREATING_SETTLES
		.with_label_values(&[if settled { "settled" } else { "failed" }])
		.inc();
	result
}

/// The `depot.overloaded` reason of a request whose namespace lookup did not finish by its
/// deadline.
const NAMESPACE_LOOKUP_TIMED_OUT_REASON: &str = "namespace_lookup_timed_out";

/// Executes pool administration or connection registration in its namespace's catalog.
pub async fn pool(
	ctx: &EngineContext,
	namespace: &str,
	name: &str,
	operation: wire::PoolOperation,
) -> Result<wire::PoolResponse> {
	ensure_namespace(ctx, namespace).await?;
	crate::validation::pool_name(name)?;
	ctx.depot()
		.command::<Pool>(
			&EnvoyPoolIndexVolume::new(namespace.to_owned()).volume_key(),
			wire::PoolRequest {
				pool_name: name.to_owned().into(),
				operation,
			},
		)
		.await
}
