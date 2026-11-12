use crate::actor_machine::{
	ActorStateMachine, CreatePlacement, ENVOY_GONE_MARGIN_MS, Effect, Event, KeyCheck, Preparation,
	State, WriteEffect, reset_view,
};
use crate::commands::EnvoyRead;
use crate::envoy_check::{
	EnvoyCheck, Route, envoy_check, no_responders, repair_route, report_reallocate,
	retryable_ups_error, unreachable_worker,
};
use crate::index;
use crate::keys::{self, ups::ActorStartNoticeSubject};
use crate::migrate_from_v2;
use crate::wire;
use anyhow::{Context, Result, ensure};
use bytes::Bytes;
use depot_worker::command::{KvHandle, Transaction};
use rivet_data::families::VersionFamily;
use rivet_data::generated::{
	depot_client_protocol_v2::VolumeRejectResponse,
	shared_depot_fs_v2::{FsForkError, FsForkSelector},
};
use rivet_engine_context::EngineContext;
use rivet_envoy_protocol as p;
use rivet_keys::keys::{
	KeyAddress,
	codec::{AtVersion, FormalKey},
};
use rivet_state_machine::EffectContext;
use rivet_state_machine_durable::RuntimeEvent;
use rivet_util::Id;
use rivetkit_shared_types::actor_key;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use vbare::Versioned;
use xxhash_rust::xxh3::Xxh3;

/// The namespace an actor's unprefixed index bucket names: the record's legacy namespace for an
/// unprefixed volume, and empty for a grouped one, whose bucket key carries no namespace.
fn legacy_index_namespace(
	group: Option<rivet_keys::keys::codec::GroupId>,
	data: &wire::ActorData,
) -> Result<String> {
	match group {
		Some(_) => Ok(String::new()),
		None => data
			.legacy_namespace
			.as_ref()
			.map(ToString::to_string)
			.context("an unprefixed actor's record has no legacy namespace"),
	}
}

/// The acknowledgement an envoy connection sends after queueing routed commands on its socket.
pub const COMMANDS_ACK: &[u8] = b"ok";

/// The answer of an envoy connection that refused routed commands because they name another
/// volume group than its namespace's: the allocation addressed an envoy of another namespace,
/// whose actor may have the same id. Nothing was queued, so the send failed.
pub const COMMANDS_REFUSED_FOREIGN_GROUP: &[u8] = b"foreign_group";

/// Legacy: the answer of an envoy connection that opened before its namespace moved into the
/// volume group its routed commands name. Nothing was queued, so the send failed; the connection
/// closes and leaves the active envoy index, and the envoy reconnects with the group, so the
/// actor's next wake reaches it. The legacy behavior refused such commands as foreign, which left
/// every grouped actor hashed to that envoy unstarted until it happened to reconnect. Removed in
/// 3.1 with the backfill.
pub const LEGACY_COMMANDS_REFUSED_NAMESPACE_MOVED: &[u8] = b"namespace_moved";

/// The acknowledgement an envoy connection sends once the actor database volume it holds has a
/// committed entry of its own claim at its tail, so the volume's members prove the tail.
pub const TAIL_PROVEN_ACK: &[u8] = b"tail proven";

/// The first envoy protocol version that carries [`p::ToEnvoySqliteProveTail`].
const PROVE_TAIL_MIN_PROTOCOL_VERSION: u16 = 9;

/// The part of the `Prepared` step's attempt timer kept back from a tail proof and the resend's
/// `resume_fork` request together, for the step's answer to reach the state machine.
const PROVE_TAIL_STEP_MARGIN_MS: u64 = 2_500;

/// The budget of proving a running fork source's tail before the `Prepared` step resends the
/// fork: the read of the source and the request to its holder share one deadline. It is what
/// the step's attempt timer leaves after the resend's `resume_fork`, which a depot volume
/// request bounds by `resume_timeout_ms`, and a margin, so the step answers before its timer
/// starts the next attempt. With the default 5 s volume request timeout that is 7.5 s. The
/// same deadline reaches pegboard-envoy with the holder's stream, which bounds the proof by it.
pub(super) fn prove_tail_budget_ms(resume_timeout_ms: u64) -> u64 {
	super::EFFECT_TIMEOUT_MS
		.saturating_sub(resume_timeout_ms)
		.saturating_sub(PROVE_TAIL_STEP_MARGIN_MS)
}

/// The smallest share of the proof budget that leaves a source read and a holder's quorum
/// append a realistic chance.
pub(super) const PROVE_TAIL_MIN_BUDGET_MS: u64 = 1_000;

/// Whether the `refusals`-th `TailUnproven` refusal in a row of one fork step warns that the
/// proof budget `budget_ms` can never prove a source's tail: it is below a source read
/// ([`PROVE_TAIL_MIN_BUDGET_MS`]) plus the open of a sleeping source, which starts only while
/// `volume_request_timeout_ms` of the budget remains. A `depot.volume_request_timeout_ms`
/// above [`prove_tail_max_volume_request_timeout_ms`] leaves no such budget under the actor
/// effect timeout, so every fork of a sleeping source would be refused without end with only
/// debug logs. It warns on the first
/// refusal and then every [`FORK_REFUSALS_WARN_EVERY`] refusals, so a stuck fork keeps saying
/// why without flooding the log.
pub(super) fn prove_tail_budget_warns(
	budget_ms: u64,
	volume_request_timeout_ms: u64,
	refusals: u32,
) -> bool {
	let needed_ms = volume_request_timeout_ms.saturating_add(PROVE_TAIL_MIN_BUDGET_MS);
	if budget_ms >= needed_ms || refusals == 0 {
		return false;
	}
	(refusals - 1) % FORK_REFUSALS_WARN_EVERY == 0
}

/// The largest `depot.volume_request_timeout_ms` whose proof budget still fits a source read
/// and a sleeping source's open: 5.75 s with the 15 s effect timeout.
pub(super) fn prove_tail_max_volume_request_timeout_ms() -> u64 {
	super::EFFECT_TIMEOUT_MS
		.saturating_sub(PROVE_TAIL_STEP_MARGIN_MS)
		.saturating_sub(PROVE_TAIL_MIN_BUDGET_MS)
		/ 2
}

/// The wait before an effect's first retry.
const EFFECT_BACKOFF_BASE_MS: u64 = 250;
/// The longest wait between two attempts of one effect.
const EFFECT_BACKOFF_MAX_MS: u64 = 30_000;
/// How many refusals in a row of one report or effect pass before the first warning, and how
/// many pass between warnings after it. A refusal is load shedding the state machine recovers
/// from on its own, so only a long run of them is worth a warning.
const REFUSALS_WARN_AFTER: u32 = 3;
const REFUSALS_WARN_EVERY: u32 = 10;
/// The longest one refusal's hint holds back a report's redelivery, so a large hint never
/// spends the report's whole deadline on one wait.
const REPORT_REFUSAL_HINT_MAX_MS: u64 = 1_000;

/// How long envoy selection may take before the event fails. Every event of the actor waits
/// behind it, so it is short; the caller retries.
const SELECT_TIMEOUT: Duration = Duration::from_secs(5);
/// How far past its lost threshold a disconnected envoy must be before it is judged gone. It

/// A forked start was not sent, because topology's view knows the envoy's host worker reads an
/// envoy protocol version without forks.
#[derive(Debug)]
struct HostWithoutForks {
	worker_id: universalpubsub::WorkerId,
}

impl std::fmt::Display for HostWithoutForks {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"the envoy's host worker {} reads an envoy protocol without forks",
			self.worker_id
		)
	}
}

impl std::error::Error for HostWithoutForks {}

/// The start command the current opening of an actor's state volume last dispatched, as its
/// epoch and command index, shared by every command of that opening. A wake of a starting actor
/// reads it to skip a resend this opening already sent. A new opening, as after a revocation,
/// starts empty, so the resend that covers a dispatch the revoked opening lost still goes out.
#[derive(Default)]
pub struct DispatchedStarts {
	last: parking_lot::Mutex<Option<(u32, i64)>>,
}

impl DispatchedStarts {
	/// Records the start of `epoch` and `command_index` as dispatched. Called from the sync
	/// effect dispatch, so the lock is a short `parking_lot` section with no await.
	fn record(&self, epoch: u32, command_index: i64) {
		*self.last.lock() = Some((epoch, command_index));
	}

	/// Whether this opening dispatched the start of `epoch` and `command_index`.
	fn dispatched(&self, epoch: u32, command_index: i64) -> bool {
		*self.last.lock() == Some((epoch, command_index))
	}
}

#[derive(Clone)]
pub struct Executor {
	ctx: EngineContext,
	db: KvHandle,
	actor_id: Id,
	/// The volume group of the actor's namespace, from the state volume's key: every volume
	/// the effects address is in it.
	group: Option<rivet_keys::keys::codec::GroupId>,
	/// The opening's dispatched starts, loaded by `prepare`, which runs before every
	/// transition this executor dispatches effects for.
	starts: std::sync::OnceLock<std::sync::Arc<DispatchedStarts>>,
}

impl Executor {
	pub fn new(
		ctx: &EngineContext,
		db: KvHandle,
		group: Option<rivet_keys::keys::codec::GroupId>,
		actor_id: Id,
	) -> Self {
		Executor {
			ctx: ctx.clone(),
			db,
			actor_id,
			group,
			starts: std::sync::OnceLock::new(),
		}
	}

	/// The actor's state volume, in its group.
	fn state_volume(&self) -> rivet_keys::keys::VolumeKey {
		keys::partition::ActorVolume::in_namespace(self.group, self.actor_id)
	}

	/// Reports an effect's result to the state machine as `event`, named `name` in metrics and
	/// logs, sending it again while depot refuses it at admission (see [`redeliver_refused`]).
	async fn report(&self, name: &'static str, event: wire::ActorEvent) -> Result<()> {
		let clock = self.ctx.env().clock();
		let mut rng = self.ctx.env().rng("pegboard.actor.report_refusal");
		redeliver_refused(clock.as_ref(), &mut rng, name, || {
			let event = event.clone();
			async move {
				self.ctx
					.depot()
					.command::<crate::commands::ActorEvent>(&self.state_volume(), event)
					.await?;
				Ok(())
			}
		})
		.await
	}
}

impl Executor {
	/// The actor's data record, present once its first data write succeeded.
	async fn read_data(&self) -> Result<wire::ActorData> {
		let data_key = keys::kv::ActorDataKey::new();
		let data = self
			.db
			.snapshot()
			.await?
			.get(&data_key.pack())
			.await?
			.context("actor has no data")?;
		data_key.deserialize(data.into())
	}

	async fn check_envoy(&self, epoch: u32, allocation: &wire::ActorAllocation) -> Result<()> {
		let route = Route::of_allocation(allocation)?;
		let route_refused = match self
			.ctx
			.ups()?
			.has_subscriber(&route.worker, route.subject())
			.await
		{
			Ok(true) => return Ok(()),
			Ok(false) => true,
			Err(error) => {
				tracing::debug!(
					?error,
					"could not ask the route's worker for the envoy connection; judging the envoy by its record"
				);
				false
			}
		};

		// A disconnected envoy inside its lost threshold may still come back with its actors,
		// so the record cannot decide yet; the check's timer runs it again once it can.
		let data = self.read_data().await?;
		let namespace = self
			.envoy_namespace(
				data.legacy_namespace
					.as_ref()
					.map(|namespace| namespace.to_string()),
			)
			.await?;
		match envoy_check(&self.ctx, &namespace, &data, allocation, route_refused).await? {
			EnvoyCheck::Serving => Ok(()),
			EnvoyCheck::Newer(newer) => {
				repair_route(
					&self.ctx,
					self.group,
					self.actor_id,
					epoch,
					allocation,
					newer,
				);
				Ok(())
			}
			EnvoyCheck::Lost => self.report_check(epoch, true).await,
			EnvoyCheck::Draining => self.report_check(epoch, false).await,
		}
	}

	async fn report_check(&self, epoch: u32, lost: bool) -> Result<()> {
		tracing::info!(
			epoch,
			lost,
			"envoy check found the running epoch's envoy gone or draining; reporting it for reallocation"
		);
		report_reallocate(&self.ctx, self.group, self.actor_id, epoch, lost).await
	}

	/// Where `event` places a keyed create under the current migration config, as
	/// `Preparation::create_placement` describes. The namespace is looked up only while a
	/// migration from V2 is configured.
	async fn create_placement(
		&self,
		event: &RuntimeEvent<Event, wire::ActorTimer>,
	) -> Result<CreatePlacement> {
		let RuntimeEvent::Input(Event(wire::ActorEvent::Create(create))) = event else {
			return Ok(CreatePlacement::Standard);
		};
		let dynamic = self.ctx.config().dynamic();
		let Some(config) = dynamic.migrate_from_v2() else {
			return Ok(CreatePlacement::Standard);
		};
		let legacy_namespace = create
			.init
			.legacy_namespace
			.as_ref()
			.map(|namespace| namespace.to_string());
		let namespace = self.envoy_namespace(legacy_namespace).await?;
		let Some(config) = migrate_from_v2::applies(Some(config), &namespace) else {
			return Ok(CreatePlacement::Standard);
		};
		let Some(key) = &create.init.key else {
			return Ok(CreatePlacement::MigrationV3);
		};
		if create.force_actor_v3 {
			return Ok(CreatePlacement::MigrationV3);
		}
		let share = migrate_from_v2::key_share(&namespace, &create.init.name, key);
		if share < config.create_ratio {
			Ok(CreatePlacement::MigrationV3)
		} else {
			Ok(CreatePlacement::MigrationV2)
		}
	}

	/// The namespace whose envoys host the actor: the one whose group the volume is in, from the
	/// group's record and cached per process. An unprefixed volume does not say whose it is, so
	/// only there the record's legacy namespace, `legacy_namespace`, does.
	async fn envoy_namespace(&self, legacy_namespace: Option<String>) -> Result<String> {
		match self.group {
			Some(group) => crate::catalog::group_namespace(&self.ctx, group).await,
			None => {
				legacy_namespace.context("an unprefixed actor's record has no legacy namespace")
			}
		}
	}

	/// The namespace the unprefixed index and Epoxy keys of the actor name: `None` for a grouped
	/// volume, whose keys name none, and the record's legacy namespace for an unprefixed one.
	async fn legacy_namespace(&self) -> Result<Option<String>> {
		match self.group {
			Some(_) => Ok(None),
			None => {
				let data = self.read_data().await?;
				let namespace = data
					.legacy_namespace
					.context("an unprefixed actor's record has no legacy namespace")?;
				Ok(Some(namespace.to_string()))
			}
		}
	}
}

/// Settles who owns a key: Epoxy reserves it for this datacenter or names the datacenter that
/// already holds it, and for `KeyCheck::EpoxyAndV2` while the namespace migrates in from V2, V2
/// is asked at the same time and wins when it holds the key. A key V2 holds is delegated to V2,
/// so its Epoxy reservation here stays with the delegated record.
async fn reserve_key(
	executor: &Executor,
	carried_legacy_namespace: Option<String>,
	name: String,
	key: String,
	key_check: KeyCheck,
) -> Result<wire::ReserveEpoxyPhase> {
	let ctx = &executor.ctx;
	// Legacy: only a retry reads the record, which commits with the create. Removed in 3.1 with
	// the backfill.
	let legacy_namespace = match carried_legacy_namespace {
		Some(namespace) => Some(namespace),
		None => executor.legacy_namespace().await?,
	};
	let migration = match key_check {
		KeyCheck::Epoxy => None,
		KeyCheck::EpoxyAndV2 => {
			let namespace = executor.envoy_namespace(legacy_namespace.clone()).await?;
			let dynamic = ctx.config().dynamic();
			migrate_from_v2::applies(dynamic.migrate_from_v2(), &namespace).cloned()
		}
	};
	let placement = keys::partition::actor_placement(executor.actor_id);
	let Some(migration) = migration else {
		return reserve_epoxy(ctx, executor.group, legacy_namespace, placement, name, key).await;
	};

	// V2's actor API reads the slash form of the key.
	let slash_key = actor_key::to_slash(&key);
	let (in_v2, reserved) = futures_util::future::join(
		migrate_from_v2::key_exists(ctx, &migration, &name, &slash_key),
		reserve_epoxy(
			ctx,
			executor.group,
			legacy_namespace,
			placement,
			name.clone(),
			key,
		),
	)
	.await;
	if in_v2.context("ask V2 whether it holds the key")? {
		return Ok(wire::ReserveEpoxyPhase::ReserveEpoxyPhaseInV2);
	}
	reserved
}

async fn reserve_epoxy(
	ctx: &EngineContext,
	group: Option<rivet_keys::keys::codec::GroupId>,
	legacy_namespace: Option<String>,
	placement: rivet_keys::keys::codec::Placement,
	name: String,
	key: String,
) -> Result<wire::ReserveEpoxyPhase> {
	match crate::actor_key::reserve(ctx, group, legacy_namespace, placement, name, key).await? {
		crate::actor_key::ReserveResult::Reserved { .. } => {
			Ok(wire::ReserveEpoxyPhase::ReserveEpoxyPhaseReserved)
		}
		crate::actor_key::ReserveResult::AlreadyReserved { reservation_id } => {
			Ok(wire::ReserveEpoxyPhase::ReserveEpoxyPhaseAlreadyReserved(
				wire::ReserveEpoxyPhaseAlreadyReserved {
					reservation_id: reservation_id.as_bytes().into(),
				},
			))
		}
	}
}

fn retryable_allocation_error(error: &anyhow::Error) -> bool {
	error
		.chain()
		.filter_map(|cause| cause.downcast_ref::<rivet_error::RivetError>())
		.any(|error| {
			error.group() == "depot"
				&& matches!(
					error.code(),
					"overloaded" | "unavailable" | "request_deadline_exceeded" | "read_interrupted"
				)
		})
}

/// All timers in one alarm batch share this absolute cutoff, leaving a second for the
/// owning actor command to persist its retry even when nested reads use their whole budget.
fn allocation_deadline(
	now_boot_ms: u64,
	command_deadline_boot_ms: u64,
	durable_retry: bool,
) -> u64 {
	let limit = now_boot_ms.saturating_add(SELECT_TIMEOUT.as_millis() as u64);
	if durable_retry {
		limit.min(command_deadline_boot_ms.saturating_sub(1_000))
	} else {
		limit
	}
}

fn allocation_result(
	selection: std::result::Result<Result<Option<wire::ActorAllocation>>, rivet_env::TimeoutError>,
	durable_retry: bool,
) -> Result<Option<wire::ActorAllocation>> {
	match selection {
		Ok(Ok(allocation)) => Ok(allocation),
		Err(rivet_env::TimeoutError::Elapsed) if durable_retry => Ok(None),
		Ok(Err(error)) if durable_retry && retryable_allocation_error(&error) => Ok(None),
		Err(error) => Err(error).context("envoy selection failed"),
		Ok(Err(error)) => Err(error),
	}
}

impl Executor {
	/// Runs the current step of the database fork in `data` and returns the event that reports
	/// it, or `None` when the outcome is unknown or nothing was sent, which the fork timer
	/// covers. The destination is this incarnation's database volume, in the actor's group.
	async fn fork_database_step(
		&self,
		data: &wire::ForkDatabaseEffectData,
	) -> Option<wire::ActorEvent> {
		let attempt = data.state.attempt;
		let create_unix_ms = data.state.create_unix_ms;
		let state = |phase| {
			Some(wire::ActorEvent::ForkDatabaseState(
				wire::ForkDatabaseState {
					phase,
					attempt,
					create_unix_ms,
				},
			))
		};
		let depot = self.ctx.depot();

		let result = match &data.state.phase {
			wire::ForkDatabasePhase::ForkDatabasePhasePreparing => {
				let destination = keys::partition::ActorDatabaseVolume::in_namespace(
					self.group,
					self.actor_id,
					create_unix_ms,
				);
				let region = match depot.local_region() {
					Ok(region) => region,
					Err(err) => {
						tracing::warn!(?err, "no local region for an actor database fork");
						return None;
					}
				};
				// No open of the source is needed for the cut: with no minimum, the source's
				// maintainer reads every member of its log and applies through what the members
				// prove committed before the job copies. A tail above that, which a claim that
				// settled with no claim entry leaves after a maintainer restart, is waited for
				// until the holder's next write or a recovery proves it, so the copy never
				// misses a commit the source acknowledged. Depot refuses such a wait as
				// `TailUnproven`, and the `Prepared` step then proves it: it opens a sleeping
				// source, and asks the engine worker holding a running source's database.
				let prepared = depot
					.prepare_fork(
						data.source.database_volume.clone(),
						FsForkSelector::FsForkLatest,
						destination,
						region,
					)
					.await;
				match prepared {
					Ok(spec) => match depot_client::forks::encode_spec(&spec) {
						Ok(spec) => {
							return state(wire::ForkDatabasePhase::ForkDatabasePhasePrepared(
								wire::ForkDatabasePhasePrepared { spec: spec.into() },
							));
						}
						Err(err) => {
							tracing::error!(
								?err,
								"could not encode a prepared actor database fork"
							);
							return None;
						}
					},
					Err(error) => error,
				}
			}
			wire::ForkDatabasePhase::ForkDatabasePhasePrepared(prepared) => {
				let spec = match depot_client::forks::decode_spec(&prepared.spec) {
					Ok(spec) => spec,
					Err(error) => return undecodable_body_event(self.actor_id, data, &error),
				};
				let destination = spec.destination_volume_key.clone();
				if data.tail_unproven_refusals > 0 {
					self.prove_source_tail(&data.source, data.tail_unproven_refusals)
						.await;
				}
				match depot.resume_fork(spec).await {
					Ok(_) => {
						tracing::info!(attempt, "actor database fork published");
						return state(wire::ForkDatabasePhase::ForkDatabasePhaseForked);
					}
					Err(error) if reopens_source(&error) => {
						return self
							.prepare_after_opening_source(data, destination, state)
							.await;
					}
					Err(error) => error,
				}
			}
			wire::ForkDatabasePhase::ForkDatabasePhaseForked
			| wire::ForkDatabasePhase::ForkDatabasePhaseFailed(_) => return None,
		};

		fork_error_event(self.actor_id, data, result)
	}
}

/// Whether a fork job's failure means its source has no record in this group yet, so the
/// executor opens the source and prepares the body again. Only an inherited source never
/// opened in this group answers that way, and the job reads it as not found.
///
/// `VolumeRecordMissing` is not one: Depot answers it at admission for a quarantined
/// incarnation whose record was lost, and refuses the reopen the same way until an operator
/// deletes the name, so the fork fails with it instead of reopening forever.
pub(super) fn reopens_source(error: &depot_client::forks::Error) -> bool {
	match error {
		depot_client::forks::Error::Rejected(FsForkError::VolumeNotFound)
		| depot_client::forks::Error::LifecycleRejected(VolumeRejectResponse::VolumeNotFound) => true,
		depot_client::forks::Error::Rejected(_)
		| depot_client::forks::Error::LifecycleRejected(_)
		| depot_client::forks::Error::Unknown { .. }
		| depot_client::forks::Error::Refused { .. }
		| depot_client::forks::Error::Preparation(_) => false,
	}
}

/// How many refusals in a row of one fork step pass before they are logged as a warning.
const FORK_REFUSALS_WARN_AFTER: u32 = 16;

/// After the first warning, how many more refusals in a row pass between two warnings. At the
/// refusal backoff's cap this is about one warning every ten minutes per stuck fork.
const FORK_REFUSALS_WARN_EVERY: u32 = 64;

/// How many attempts of a fork whose body is from a newer schema version pass between two
/// error logs. At the attempt backoff's cap this is about one log every twelve minutes.
const FORK_UNDECODABLE_LOG_EVERY: u32 = 16;

/// Whether the `attempt`-th send of a body from a newer schema version logs an error rather
/// than at debug level: the first send of a body prepared on the first try, then every
/// `FORK_UNDECODABLE_LOG_EVERY`-th attempt.
pub(super) fn undecodable_body_logs_error(attempt: u32) -> bool {
	attempt <= 2 || attempt % FORK_UNDECODABLE_LOG_EVERY == 0
}

/// The event that reports a persisted fork body this binary cannot decode.
///
/// A body from a newer schema version may come from a newer binary whose job can still publish,
/// so the fork never fails over it: this returns `None` and the timer resends it until a binary
/// that reads it runs the step. That assumes no rollback: after one, the fork and any destroy
/// waiting on it stay pending until a binary that reads the body runs. Any other decode error
/// means no binary reads the body, so the fork fails as `Rejected` rather than keep a
/// destroy waiting on it forever.
pub(super) fn undecodable_body_event(
	actor_id: Id,
	data: &wire::ForkDatabaseEffectData,
	error: &depot_client::forks::DecodeSpecError,
) -> Option<wire::ActorEvent> {
	let attempt = data.state.attempt;
	match error {
		depot_client::forks::DecodeSpecError::NewerVersion { .. } => {
			if undecodable_body_logs_error(attempt) {
				tracing::error!(
					?error,
					%actor_id,
					attempt,
					"persisted actor database fork body is from a newer binary; the fork waits for a binary that reads it"
				);
			} else {
				tracing::debug!(
					?error,
					%actor_id,
					attempt,
					"persisted actor database fork body is from a newer binary"
				);
			}
			None
		}
		depot_client::forks::DecodeSpecError::Corrupt(_) => {
			tracing::error!(
				?error,
				%actor_id,
				attempt,
				"persisted actor database fork body is corrupt; the fork fails"
			);
			Some(wire::ActorEvent::ForkDatabaseState(
				wire::ForkDatabaseState {
					phase: wire::ForkDatabasePhase::ForkDatabasePhaseFailed(
						wire::ForkDatabasePhaseFailed {
							failure: wire::ForkDatabaseFailure::Rejected,
						},
					),
					attempt,
					create_unix_ms: data.state.create_unix_ms,
				},
			))
		}
	}
}

/// The event that reports a failed fork step `error` of `data` to the state machine: `Failed`
/// for a permanent answer, `ForkDatabaseRefused` for a refusal, and `None` when the outcome is
/// unknown or nothing was sent, which the fork timer covers.
///
/// A refusal has no bound by design, since the job may still run, so a long run of refusals is
/// logged as a rate-limited warning naming the actor and the reason.
pub(super) fn fork_error_event(
	actor_id: Id,
	data: &wire::ForkDatabaseEffectData,
	error: depot_client::forks::Error,
) -> Option<wire::ActorEvent> {
	let attempt = data.state.attempt;
	let create_unix_ms = data.state.create_unix_ms;
	let tail_unproven = error.tail_unproven();
	let failed = |failure| {
		Some(wire::ActorEvent::ForkDatabaseState(
			wire::ForkDatabaseState {
				phase: wire::ForkDatabasePhase::ForkDatabasePhaseFailed(
					wire::ForkDatabasePhaseFailed { failure },
				),
				attempt,
				create_unix_ms,
			},
		))
	};

	match error {
		depot_client::forks::Error::Rejected(error) => {
			tracing::info!(?error, "depot refused an actor database fork for good");
			failed(fork_failure(&error))
		}
		depot_client::forks::Error::LifecycleRejected(reject) => {
			tracing::info!(?reject, "depot refused an actor database fork for good");
			failed(lifecycle_failure(&reject))
		}
		depot_client::forks::Error::Refused {
			retry_after_ms,
			reason,
			..
		} => {
			let refusals = data.refusals.saturating_add(1);
			if refusals >= FORK_REFUSALS_WARN_AFTER
				&& (refusals - FORK_REFUSALS_WARN_AFTER) % FORK_REFUSALS_WARN_EVERY == 0
			{
				tracing::warn!(
					%actor_id,
					?reason,
					refusals,
					attempt,
					"actor database fork keeps being refused; the source or destination volume may be under repair or overloaded"
				);
			} else {
				tracing::debug!(
					?reason,
					retry_after_ms,
					"actor database fork refused for now"
				);
			}
			Some(wire::ActorEvent::ForkDatabaseRefused(
				wire::ForkDatabaseRefused {
					attempt,
					retry_after_ms: u32::try_from(retry_after_ms).unwrap_or(u32::MAX),
					create_unix_ms,
					tail_unproven,
				},
			))
		}
		depot_client::forks::Error::Unknown { .. } => {
			tracing::debug!(
				attempt,
				"actor database fork outcome unknown; the timer resends it"
			);
			None
		}
		depot_client::forks::Error::Preparation(err) => {
			tracing::warn!(
				?err,
				attempt,
				"actor database fork preparation failed; the timer retries it"
			);
			None
		}
	}
}

/// How the `Prepared` step proves a fork source's tail that Depot refused the fork for, decided
/// from pegboard's record of the source actor.
///
/// The claim on a database names the process that held it, not the actor, so Depot cannot tell
/// a sleeping actor from a live writer that is quiet; pegboard can. After a tablet maintainer
/// restart, a claim that settled with no claim entry leaves the source's last tail unproven
/// while that process lives, and nothing else proves it until the actor writes again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum SourceTailProof {
	/// The source actor sleeps, so no process serves its database. Pegboard opens and closes it
	/// under the actor's authority, and the open's recovery commits a claim entry above every
	/// committed entry.
	OpenSleeping,
	/// The source actor runs on the envoy `allocation` names, whose pegboard-envoy connection
	/// holds its database for `epoch`. Pegboard asks that connection to commit an entry of its
	/// own claim, which proves the tail without stopping or fencing the actor.
	AskHolder {
		allocation: wire::ActorAllocation,
		epoch: u32,
	},
	/// The source is starting, stopping, destroyed, held by the V2 migrator or has no record. Its own open, close or
	/// the next resend settles the tail, so nothing is done now.
	Skip,
}

/// The way to prove the tail of the fork source pegboard read as `read`.
pub(super) fn source_tail_proof(read: &wire::ActorReadResponse) -> SourceTailProof {
	match &read.phase {
		Some(wire::ActorPhase::Sleeping(_)) => SourceTailProof::OpenSleeping,
		Some(wire::ActorPhase::Running(running)) => SourceTailProof::AskHolder {
			allocation: running.allocation.clone(),
			epoch: read.epoch,
		},
		Some(
			wire::ActorPhase::MigratingFromV2(_)
			| wire::ActorPhase::Initializing(_)
			| wire::ActorPhase::Starting(_)
			| wire::ActorPhase::Stopping(_)
			| wire::ActorPhase::Destroying(_)
			| wire::ActorPhase::Destroyed
			| wire::ActorPhase::DelegatedToV2,
		)
		| None => SourceTailProof::Skip,
	}
}

/// Proves the tail of `ancestor_database`, an ancestor group's copy of inherited actor
/// `actor_id`'s database, whose unproven tail held back the lazy fork of the actor's database
/// in a forked namespace: the child's open was refused as `VolumeTailUnproven` naming it. The
/// actor has the same id along its lineage, so its record in the ancestor's group says whether
/// the ancestor's actor sleeps or which pegboard-envoy connection holds it, as for an actor
/// fork's source. Pegboard-envoy calls it once per such refusal of the start's open, under the
/// child actor's start; the next open of the database, which the start's retry sends, then
/// finds the ancestor pinned. A parent whose tail is proven is never refused this way, so it
/// pays nothing.
pub async fn prove_inherited_database_tail(
	ctx: &EngineContext,
	actor_id: Id,
	ancestor_database: &depot_types::VolumeKey,
) {
	let Some(group) =
		depot_types::keys::partition::forkable_group(&ancestor_database.partition_key)
	else {
		tracing::warn!(
			%actor_id,
			"a lazy fork's unproven ancestor is not a grouped volume; its tail is left to its holder"
		);
		return;
	};
	// Each call follows one refusal and nothing here counts a start's refusals in a row, so
	// every call counts as the first: a proof budget too small to work warns on each one.
	prove_actor_tail(ctx, Some(group), actor_id, ancestor_database, 1).await
}

/// Proves the tail Depot refused a fork of `actor_id`'s database `database_volume` for, the
/// way [`source_tail_proof`] picks from pegboard's record of the actor in `group`: before the
/// `Prepared` step resends an actor fork, or after an inherited actor's lazy fork of that
/// database was refused (see [`prove_inherited_database_tail`]). Runs once per `TailUnproven`
/// refusal, so the refusal backoff bounds it; failures are logged and leave the next attempt
/// to be refused again.
///
/// The read and a running source's request share the deadline [`prove_tail_budget_ms`]
/// gives. A budget too small for a read and a sleeping source's open (see
/// [`prove_tail_budget_warns`]) is warned about, rate-limited by `refusals`, the step's
/// `TailUnproven` refusals in a row; an empty budget skips the proof.
///
/// A sleeping source's open and close keep their own depot request deadlines instead:
/// cancelling an open between its claim and its recovery would leave the claim without the
/// entry that proves the tail. The open starts only while at least one volume request
/// timeout of the budget remains, and otherwise waits for the next round. An open that
/// recovers and the close after it can still run past the step's attempt timer; the next
/// attempt then opens the source again, which fences this open and writes one more claim
/// entry. The tail is proven either way and the resend is idempotent, so that overrun is
/// accepted.
///
/// Each forking copy proves its source on its own. N concurrent forks of one source each
/// read it and prove its tail once per backoff round: for a sleeping source their opens
/// fence each other and write up to N claim entries, and for a running one its holder
/// commits up to N empty entries. That is bounded by the refusal backoff and rare, since it
/// needs a maintainer restart within the source's cold window, so it is accepted rather than
/// coordinated across copies.
async fn prove_actor_tail(
	ctx: &EngineContext,
	group: Option<rivet_keys::keys::codec::GroupId>,
	actor_id: Id,
	database_volume: &depot_types::VolumeKey,
	refusals: u32,
) {
	// The read and the request to a running source's holder share one deadline, so the
	// resend's own request still fits in the step's attempt timer.
	let volume_request_timeout_ms = ctx.config().depot().volume_request_timeout_ms();
	let budget_ms = prove_tail_budget_ms(volume_request_timeout_ms);
	if prove_tail_budget_warns(budget_ms, volume_request_timeout_ms, refusals) {
		tracing::warn!(
			source_actor_id = %actor_id,
			budget_ms,
			volume_request_timeout_ms,
			max_volume_request_timeout_ms = prove_tail_max_volume_request_timeout_ms(),
			effect_timeout_ms = super::EFFECT_TIMEOUT_MS,
			refusals,
			"a fork source's tail proof budget cannot fit a source read and a sleeping source's open; lower depot.volume_request_timeout_ms to max_volume_request_timeout_ms or below, or forks of sleeping sources whose tail is unproven stay refused"
		);
	}
	if budget_ms == 0 {
		return;
	}
	let clock = ctx.env().clock();
	let deadline_boot_ms =
		match rivet_env::deadline_after(clock.as_ref(), Duration::from_millis(budget_ms)) {
			Ok(deadline_boot_ms) => deadline_boot_ms,
			Err(err) => {
				tracing::warn!(
					?err,
					"could not read the clock to prove a fork source's tail"
				);
				return;
			}
		};

	let state_volume = keys::partition::ActorVolume::in_namespace(group, actor_id);
	let read = rivet_env::timeout_at(
		clock.as_ref(),
		deadline_boot_ms,
		ctx.depot()
			.command::<crate::commands::ActorRead>(&state_volume, ()),
	)
	.await;
	let read = match read {
		Ok(Ok(read)) => read,
		Ok(Err(err)) => {
			tracing::debug!(
				?err,
				"could not read a fork source actor whose tail is unproven"
			);
			return;
		}
		Err(err) => {
			tracing::debug!(
				?err,
				"reading a fork source actor whose tail is unproven used up the proof's budget"
			);
			return;
		}
	};

	match source_tail_proof(&read) {
		SourceTailProof::OpenSleeping => {
			// The open bounds itself by the volume request timeout, so it starts only while
			// that much of the budget remains and the step stays inside its timer.
			let remaining_ms = match clock.observe() {
				Ok(now) => deadline_boot_ms.saturating_sub(now.boot_ms),
				Err(err) => {
					tracing::warn!(
						?err,
						"could not read the clock to prove a fork source's tail"
					);
					return;
				}
			};
			if remaining_ms < volume_request_timeout_ms {
				tracing::debug!(
					source_actor_id = %actor_id,
					remaining_ms,
					volume_request_timeout_ms,
					"too little of the proof's budget remains to open a sleeping fork source; the next round opens it"
				);
				return;
			}
			open_sleeping_source(ctx, actor_id, database_volume).await
		}
		SourceTailProof::AskHolder { allocation, epoch } => {
			ask_holder_to_prove_tail(ctx, actor_id, &allocation, epoch, deadline_boot_ms).await
		}
		SourceTailProof::Skip => {
			tracing::debug!(
				source_actor_id = %actor_id,
				"fork source with an unproven tail is neither asleep nor running; the next resend proves it"
			);
		}
	}
}

/// Opens and closes a sleeping fork source's database volume `database_volume`, so the open's recovery
/// proves the tail. The open claims the source under the actor's authority, as an open of
/// a source with no record does, and its recovery commits a claim entry above every
/// committed entry.
///
/// A wake that lands between pegboard's read of the source and this open has its first
/// writer fenced once by this open, and its supervisor restarts it. That needs a maintainer
/// restart within the source's cold window and a wake at this instant, so it is accepted.
async fn open_sleeping_source(
	ctx: &EngineContext,
	actor_id: Id,
	database_volume: &depot_types::VolumeKey,
) {
	let depot = ctx.depot();
	let opened = match depot.open_filesystem(database_volume.clone()).await {
		Ok(opened) => opened,
		Err(err) => {
			tracing::warn!(
				?err,
				source_actor_id = %actor_id,
				"could not open a sleeping fork source to prove its tail; the fork step retries it"
			);
			return;
		}
	};
	if let Err(err) = opened.shutdown().await {
		tracing::warn!(
			?err,
			source_actor_id = %actor_id,
			"could not close a fork source after proving its tail"
		);
	}
	tracing::info!(
		source_actor_id = %actor_id,
		"opened a sleeping fork source to prove its tail"
	);
}

/// Asks the pegboard-envoy connection that `allocation` names, which holds a running fork
/// source's database for `epoch`, to prove its tail: the connection's volume client
/// commits one entry with no operations at its own claim and acknowledges once that is
/// committed. The actor keeps running and is never fenced. The request ends at
/// `deadline_boot_ms`, the proof's deadline from [`prove_tail_budget_ms`], and carries it to
/// the holder.
///
/// A receiving engine worker from before envoy protocol v9 cannot take the request, so it is
/// not sent, and the fork stays refused until the source writes, sleeps or that worker is
/// upgraded. A connection that moved, an epoch that changed or a volume not open there
/// answers no acknowledgement; the next resend reads the source again.
async fn ask_holder_to_prove_tail(
	ctx: &EngineContext,
	actor_id: Id,
	allocation: &wire::ActorAllocation,
	epoch: u32,
	deadline_boot_ms: u64,
) {
	let route = match Route::of_allocation(allocation) {
		Ok(route) => route,
		Err(err) => {
			tracing::warn!(
				?err,
				source_actor_id = %actor_id,
				"a running fork source's allocation names an invalid route"
			);
			return;
		}
	};
	let version = crate::envoy_protocol::version_for_worker(
		ctx,
		&route.worker,
		crate::envoy_protocol::InternalMessage::ProveTail,
	);
	if version < PROVE_TAIL_MIN_PROTOCOL_VERSION {
		tracing::debug!(
			source_actor_id = %actor_id,
			version,
			"the engine worker holding a running fork source reads an envoy protocol without tail proofs; the fork waits for the source to write or sleep"
		);
		return;
	}

	let request = p::ToEnvoy::ToEnvoySqliteProveTail(p::ToEnvoySqliteProveTail {
		actor_id: actor_id.to_string().into(),
		generation: epoch,
	});
	let payload = match p::versioned::ToEnvoy::serialize_with_embedded_version(&request, version) {
		Ok(payload) => Bytes::from(payload),
		Err(err) => {
			tracing::warn!(?err, "could not encode a SQLite tail proof request");
			return;
		}
	};
	let result = async {
		let now_boot_ms = ctx.env().clock().observe()?.boot_ms;
		let remaining_ms = deadline_boot_ms.saturating_sub(now_boot_ms);
		ensure!(
			remaining_ms > 0,
			"the proof's budget ended before its request"
		);
		let ack = ctx
			.ups()?
			.request_with_timeout(
				universalpubsub::RequestClass::EnvoyCommand,
				&route.worker,
				route.subject(),
				&payload,
				Duration::from_millis(remaining_ms),
			)
			.await?;
		ensure!(ack.as_ref() == TAIL_PROVEN_ACK, "unexpected tail proof ack");
		anyhow::Ok(())
	}
	.await;
	match result {
		Ok(()) => tracing::info!(
			source_actor_id = %actor_id,
			epoch,
			"the engine worker holding a running fork source proved its tail"
		),
		Err(err) => tracing::debug!(
			?err,
			source_actor_id = %actor_id,
			epoch,
			"a running fork source's holder did not prove its tail; the fork step retries it"
		),
	}
}

impl Executor {
	/// Proves the tail of the fork source `source`, see [`prove_actor_tail`].
	async fn prove_source_tail(&self, source: &wire::ActorForkSource, refusals: u32) {
		let actor_id = match Id::from_slice(&source.actor_id) {
			Ok(actor_id) => actor_id,
			Err(err) => {
				tracing::warn!(?err, "a fork source names an invalid actor id");
				return;
			}
		};
		prove_actor_tail(
			&self.ctx,
			self.group,
			actor_id,
			&source.database_volume,
			refusals,
		)
		.await
	}

	/// Makes the record of a fork source that has none in this group and prepares the fork
	/// again, returning the new body as `Prepared` for the state machine to persist and send.
	///
	/// A source inherited from a parent namespace and never opened in this one has no record,
	/// so the job of a body naming it fails for good as not found. The lazy-fork open makes the
	/// record from the parent group's copy, and closing it right away leaves the source idle
	/// again. Nothing
	/// holds the source while it has no record, since any open would have made one; an actor
	/// that wakes it at this same instant has its first writer fenced by this open once, and its
	/// supervisor restarts it. A source that turns out to be gone fails the fork.
	async fn prepare_after_opening_source(
		&self,
		data: &wire::ForkDatabaseEffectData,
		destination: depot_types::VolumeKey,
		state: impl Fn(wire::ForkDatabasePhase) -> Option<wire::ActorEvent>,
	) -> Option<wire::ActorEvent> {
		let depot = self.ctx.depot();
		let source = &data.source.database_volume;

		let opened = match depot.open_filesystem(source.clone()).await {
			Ok(opened) => opened,
			Err(err) if source_gone(&err) => {
				return state(wire::ForkDatabasePhase::ForkDatabasePhaseFailed(
					wire::ForkDatabasePhaseFailed {
						failure: wire::ForkDatabaseFailure::SourceGone,
					},
				));
			}
			Err(err) => {
				tracing::warn!(
					?err,
					"could not open a fork source with no record; the fork timer retries it"
				);
				return None;
			}
		};
		if let Err(err) = opened.shutdown().await {
			tracing::warn!(
				?err,
				"could not close a fork source after making its record"
			);
		}

		let region = match depot.local_region() {
			Ok(region) => region,
			Err(err) => {
				tracing::warn!(?err, "no local region for an actor database fork");
				return None;
			}
		};
		let prepared = depot
			.prepare_fork(
				source.clone(),
				FsForkSelector::FsForkLatest,
				destination,
				region,
			)
			.await;
		let spec = match prepared {
			Ok(spec) => spec,
			Err(err) => {
				tracing::warn!(
					?err,
					"could not prepare an actor database fork again; the fork timer retries it"
				);
				return None;
			}
		};
		match depot_client::forks::encode_spec(&spec) {
			Ok(spec) => state(wire::ForkDatabasePhase::ForkDatabasePhasePrepared(
				wire::ForkDatabasePhasePrepared { spec: spec.into() },
			)),
			Err(err) => {
				tracing::error!(?err, "could not encode a prepared actor database fork");
				None
			}
		}
	}

	/// The namespace label of the actor's fork metrics: its group's namespace, or empty when
	/// the group's record cannot be read now, which never fails the fork.
	async fn fork_metrics_namespace(&self) -> String {
		let Some(group) = self.group else {
			return String::new();
		};
		match crate::catalog::group_namespace(&self.ctx, group).await {
			Ok(namespace) => namespace,
			Err(err) => {
				tracing::debug!(?err, "no namespace for an actor fork metric");
				String::new()
			}
		}
	}
}

/// Whether an open of a fork source failed because its database volume does not exist or was
/// deleted.
pub(crate) fn source_gone(error: &anyhow::Error) -> bool {
	let lifecycle = error.chain().any(|cause| {
		cause
			.downcast_ref::<depot_client::volumes::Error>()
			.is_some_and(depot_client::volumes::Error::is_volume_gone)
	});
	let structured = rivet_error::RivetError::extract(error);
	lifecycle
		|| (structured.group() == "depot"
			&& matches!(structured.code(), "volume_not_found" | "volume_deleted"))
}

/// The permanent failure a Depot fork refusal stands for.
fn fork_failure(error: &FsForkError) -> wire::ForkDatabaseFailure {
	match error {
		FsForkError::ForkChainTooDeep => wire::ForkDatabaseFailure::ChainTooDeep,
		FsForkError::VolumeNotFound
		| FsForkError::VolumeDeleted
		| FsForkError::ForkDeleted
		| FsForkError::IncarnationChanged => wire::ForkDatabaseFailure::SourceGone,
		// Depot reports `BaseUnavailable` as a refusal, never as a rejection; it is listed so a
		// new answer is a compile error here.
		FsForkError::BaseUnavailable
		| FsForkError::Invalid
		| FsForkError::Busy
		| FsForkError::DestinationExists
		| FsForkError::OperationConflict
		| FsForkError::Lagging
		| FsForkError::CheckpointNotFound
		| FsForkError::CheckpointNotReachable
		| FsForkError::CheckpointAmbiguous
		| FsForkError::ForkOutOfRetention
		| FsForkError::RegionUnavailable
		| FsForkError::TailUnproven => wire::ForkDatabaseFailure::Rejected,
	}
}

/// The permanent failure a lifecycle rejection of a fork stands for. A missing or deleted volume
/// is the source's, since the destination is this incarnation's own new volume; an existing
/// destination, an archived group and a missing record are refused for another reason.
pub(super) fn lifecycle_failure(reject: &VolumeRejectResponse) -> wire::ForkDatabaseFailure {
	match reject {
		VolumeRejectResponse::VolumeNotFound | VolumeRejectResponse::VolumeDeleted => {
			wire::ForkDatabaseFailure::SourceGone
		}
		// Depot reports the transient answers as refusals, never as rejections; they are listed
		// so a new answer is a compile error here.
		VolumeRejectResponse::VolumeExists
		| VolumeRejectResponse::VolumeGroupArchived
		| VolumeRejectResponse::VolumeRecordMissing(_)
		| VolumeRejectResponse::NotMaintainer(_)
		| VolumeRejectResponse::TabletSplitting
		| VolumeRejectResponse::Overloaded(_)
		| VolumeRejectResponse::VolumeUnderRepair(_)
		| VolumeRejectResponse::VolumeTailUnproven(_) => wire::ForkDatabaseFailure::Rejected,
	}
}

impl rivet_state_machine_durable::Executor<ActorStateMachine> for Executor {
	/// Decides whether the state is an inherited one the transition resets first, and selects
	/// the envoy for a start `event` causes, as `needs_allocation` names them on the state
	/// the transition will use. The pool comes from the create's own data or the actor's
	/// data record; the envoys are those of the namespace whose group the volume is in.
	fn prepare<'a>(
		&'a self,
		state: &'a State,
		event: &'a RuntimeEvent<Event, wire::ActorTimer>,
	) -> Pin<Box<dyn Future<Output = Result<Preparation>> + Send + 'a>> {
		Box::pin(async move {
			let pegboard_config = self.ctx.config().pegboard();
			let start_threshold_ms = pegboard_config.actor_start_threshold();
			let stop_threshold_ms = pegboard_config.actor_stop_threshold();
			let start_waiters_capacity = pegboard_config.actor_start_waiters_capacity();
			let envoy_lost_threshold_ms = pegboard_config.envoy_lost_threshold();
			let write_version = self.db.write_version(VersionFamily::PEGBOARD_ACTOR)?;
			let create_placement = self.create_placement(event).await?;
			let fork_reset = self
				.db
				.volume_record()
				.pending_fork_reset(state.0.fork_reset);
			let effective;
			let decided = match fork_reset {
				Some(incarnation) => {
					effective = reset_view(&state.0, incarnation);
					&effective
				}
				None => &state.0,
			};

			let starts = match self.starts.get() {
				Some(starts) => starts.clone(),
				None => {
					let starts = self.db.opening_state::<DispatchedStarts>().await?;
					self.starts.get_or_init(|| starts).clone()
				}
			};
			let start_dispatched = matches!(decided.phase, wire::ActorPhase::Starting(_))
				&& starts.dispatched(decided.epoch, decided.command_index);
			// A create that leaves or puts the key with V2 starts nothing, so it selects no
			// envoy.
			let starts_nothing = match decided.phase {
				wire::ActorPhase::Destroyed => create_placement == CreatePlacement::MigrationV2,
				wire::ActorPhase::DelegatedToV2 => create_placement != CreatePlacement::Standard,
				wire::ActorPhase::Initializing(_)
				| wire::ActorPhase::Starting(_)
				| wire::ActorPhase::Running(_)
				| wire::ActorPhase::Stopping(_)
				| wire::ActorPhase::Sleeping(_)
				| wire::ActorPhase::Destroying(_)
				| wire::ActorPhase::MigratingFromV2(_) => false,
			};
			if starts_nothing
				|| !crate::actor_machine::needs_allocation_at_version(decided, event, write_version)
			{
				return Ok(Preparation {
					allocation: None,
					start_threshold_ms,
					stop_threshold_ms,
					start_waiters_capacity,
					envoy_lost_threshold_ms,
					write_version,
					create_placement,
					fork_reset,
					start_dispatched,
				});
			}

			let durable_retry = matches!(
				event,
				RuntimeEvent::Timer(
					wire::ActorTimer::AllocationRetry(_) | wire::ActorTimer::AlarmTimeout
				)
			);
			let clock = self.ctx.env().clock();
			let now = clock.observe()?.boot_ms;
			let deadline = allocation_deadline(now, self.db.deadline_boot_ms(), durable_retry);
			let selection = rivet_env::timeout_at(&*clock, deadline, async {
				let (actor_id, legacy_namespace, pool_name, name) = match event {
					RuntimeEvent::Input(Event(wire::ActorEvent::Create(create))) => (
						create.init.actor_id.clone(),
						create.init.legacy_namespace.clone(),
						create.init.pool_name.clone(),
						create.init.name.clone(),
					),
					_ => {
						let data = self.read_data().await?;
						(
							data.actor_id,
							data.legacy_namespace,
							data.pool_name,
							data.name,
						)
					}
				};
				let namespace = self
					.envoy_namespace(legacy_namespace.map(|namespace| namespace.to_string()))
					.await?;

				select_allocation(&self.ctx, &actor_id, &namespace, &pool_name, &name).await
			})
			.await;
			let allocation = allocation_result(selection, durable_retry)?;

			Ok(Preparation {
				allocation,
				start_threshold_ms,
				stop_threshold_ms,
				start_waiters_capacity,
				envoy_lost_threshold_ms,
				write_version,
				create_placement,
				fork_reset,
				start_dispatched,
			})
		})
	}

	/// Puts the actor's data record where every later effect reads it, in the create's own
	/// transaction.
	fn write<'a>(
		&'a self,
		effect: WriteEffect,
		tx: &'a mut Transaction,
		version: u16,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
		Box::pin(async move {
			match effect {
				WriteEffect::WriteData { data } => {
					let data_key = keys::kv::ActorDataKey::new();
					tx.put(
						&data_key.pack(),
						&data_key.serialize(AtVersion::new(data, version))?,
					)?;
					Ok(())
				}
			}
		})
	}

	fn execute(&self, effect: Effect, ctx: EffectContext<()>) {
		match effect {
			Effect::ReserveEpoxy {
				name,
				key,
				attempt,
				create_unix_ms,
				key_check,
				legacy_namespace,
			} => {
				let self2 = self.clone();
				ctx.spawn(Box::pin(async move {
					let res = async {
						self2
							.ctx
							.env()
							.clock()
							.sleep_until(
								self2.ctx.env().clock().observe()?.boot_ms
									+ effect_backoff(attempt),
							)
							.await?;

						reserve_key(&self2, legacy_namespace, name, key, key_check).await
					}
					.await;

					let res = async {
						let phase = match res {
							Ok(phase) => phase,
							Err(err) => {
								log_effect_failure("reserve_epoxy", attempt, &err);
								wire::ReserveEpoxyPhase::ReserveEpoxyPhaseFailed
							}
						};

						self2
							.report(
								"reserve_epoxy_state",
								wire::ActorEvent::ReserveEpoxyState(wire::ReserveEpoxyState {
									phase,
									attempt,
									create_unix_ms,
								}),
							)
							.await
					}
					.await;

					match res {
						Ok(()) => {}
						// The reservation starts before the commit that creates the machine. When
						// that commit failed, its outcome has no machine to update; a retried
						// create of the same key adopts the reservation.
						Err(err) if rivet_state_machine_durable::is_machine_not_found(&err) => {
							tracing::debug!(
								?err,
								actor_id = %self2.actor_id,
								"reservation outcome for an actor whose create never committed"
							);
						}
						Err(err) => {
							tracing::warn!(
								?err,
								"could not report the epoxy reservation; its effect timer runs it again"
							);
						}
					}
				}));
			}
			Effect::WriteIndex {
				name,
				index_bucket_count,
				create_unix_ms,
				attempt,
			} => {
				let self2 = self.clone();
				ctx.spawn(Box::pin(async move {
					let res = async {
						self2
							.ctx
							.env()
							.clock()
							.sleep_until(
								self2.ctx.env().clock().observe()?.boot_ms
									+ effect_backoff(attempt),
							)
							.await?;

						// The name of a grouped bucket carries no namespace, so it is empty.
						let legacy_namespace = self2.legacy_namespace().await?.unwrap_or_default();
						self2
							.ctx
							.depot()
							.command::<crate::index::IndexInsertActor>(
								&keys::partition::ActorIndexVolume::for_actor_in_namespace(
									self2.group,
									legacy_namespace,
									name,
									self2.actor_id,
									index_bucket_count,
								),
								wire::IndexInsertActorRequest {
									actor_id: self2.actor_id.as_bytes().into(),
									create_unix_ms,
								},
							)
							.await?;

						anyhow::Ok(())
					}
					.await;

					let res = async {
						let phase = match res {
							Ok(()) => wire::WriteIndexPhase::WriteIndexPhaseSuccess,
							Err(err) => {
								log_effect_failure("write_index", attempt, &err);
								wire::WriteIndexPhase::WriteIndexPhaseFailed
							}
						};

						self2
							.report(
								"write_index_state",
								wire::ActorEvent::WriteIndexState(wire::WriteIndexState {
									phase,
									attempt,
									create_unix_ms,
								}),
							)
							.await
					}
					.await;

					if let Err(err) = res {
						tracing::warn!(
							?err,
							"could not report the index write; its effect timer runs it again"
						);
					}
				}));
			}
			Effect::SendStopCommand {
				epoch,
				command_index,
				allocation,
				pending_action,
			} => {
				let self2 = self.clone();
				ctx.spawn(Box::pin(async move {
					let res = async {
						let reason = match pending_action {
							wire::PendingAction::PendingNone
							| wire::PendingAction::PendingSleep => p::StopActorReason::SleepIntent,
							wire::PendingAction::PendingReallocate => p::StopActorReason::GoingAway,
							wire::PendingAction::PendingDestroy => p::StopActorReason::Destroy,
						};

						let commands = p::ToEnvoy::ToEnvoyCommands(vec![p::CommandWrapper {
							checkpoint: p::ActorCheckpoint {
								actor_id: self2.actor_id.to_string().into(),
								generation: epoch,
								index: command_index,
							},
							inner: p::Command::CommandStopActor(p::CommandStopActor { reason }),
							group: self2.group.map(|group| group.to_vec().into()),
						}]);
						// Each route is encoded at the version its worker reads.
						let encode = |worker_id: &universalpubsub::WorkerId| -> Result<Bytes> {
							let version = crate::envoy_protocol::version_for_worker(
								&self2.ctx,
								worker_id,
								crate::envoy_protocol::InternalMessage::Stop,
							);
							Ok(Bytes::from(
								p::versioned::ToEnvoy::serialize_with_embedded_version(
									&commands, version,
								)?,
							))
						};

						// The allocation's route is the common case and costs no read. Only a
						// transport failure there is worth the envoy record: the envoy may have
						// reconnected on a newer connection, possibly to another worker.
						let route = Route::of_allocation(&allocation)?;
						let first =
							send_commands(&self2.ctx, &route, &encode(&route.worker)?).await;
						let Err(error) = first else {
							return anyhow::Ok(());
						};
						if !retryable_ups_error(&error) {
							return Err(error);
						}
						tracing::debug!(
							?error,
							"stop command failed on the allocation's route; reading the envoy record"
						);

						let data_key = keys::kv::ActorDataKey::new();
						let data = self2
							.db
							.snapshot()
							.await?
							.get(&data_key.pack())
							.await?
							.context("actor has no data")?;
						let data = data_key.deserialize(data.into())?;

						let namespace = self2
							.envoy_namespace(
								data.legacy_namespace.as_ref().map(ToString::to_string),
							)
							.await?;
						let envoy_volume = keys::partition::EnvoyVolume::new(
							namespace,
							data.pool_name.to_string(),
							allocation.envoy_key.to_string(),
						)
						.volume_key();
						// The clock is read before the record, so a reconnect the record does not
						// show yet happened after this time.
						let read_unix_ms = self2.ctx.env().clock().observe()?.unix_ms;
						let envoy = match self2
							.ctx
							.depot()
							.command::<EnvoyRead>(&envoy_volume, ())
							.await
						{
							Ok(response) => response.data,
							Err(error) if crate::commands::is_volume_gone(&error) => None,
							Err(error) => return Err(error),
						};

						// A newer connection may host the epoch now. The allocation's own
						// connection already refused the stop, and a disconnected record names
						// no connection worth trying.
						let mut error = error;
						if let Some(envoy) = &envoy
							&& envoy.connected && envoy.connection_id != allocation.connection_id
						{
							let route = Route::of_envoy(envoy)?;
							let second =
								send_commands(&self2.ctx, &route, &encode(&route.worker)?).await;
							let Err(second) = second else {
								return anyhow::Ok(());
							};
							error = second;
						}

						// The stop reached no connection. An envoy disconnected past its lost
						// threshold has stopped every actor it held, so the epoch can end
						// without its stopped report; otherwise the stop timer sends it again.
						if !envoy_gone(
							envoy.as_ref(),
							read_unix_ms,
							self2.ctx.config().pegboard().envoy_lost_threshold(),
						) {
							if let Some(envoy) = &envoy
								&& envoy.connected && connection_ended(&self2.ctx, envoy, &error)
								.await?
							{
								mark_disconnected(&self2.ctx, &envoy_volume, envoy).await;
							}
							return Err(error);
						}

						tracing::info!(
							?error,
							envoy_key = %allocation.envoy_key,
							epoch,
							"envoy is gone past its lost threshold; ending the epoch without its stopped report"
						);
						self2
							.report(
								"envoy_gone",
								wire::ActorEvent::EnvoyGone(wire::EnvoyGone { epoch }),
							)
							.await
					}
					.await;

					// A failed stop is sent again by the state machine's stop timer.
					if let Err(err) = res {
						tracing::warn!(
							?err,
							"failed to send stop command; the stop timer resends it"
						);
					}
				}));
			}
			Effect::NotifyStartWaiters { waiters, kind } => {
				let self2 = self.clone();
				ctx.spawn(Box::pin(async move {
					let sends = waiters.into_iter().map(|waiter| {
						let ctx = self2.ctx.clone();
						let notice = wire::ActorStartNotice {
							actor_id: self2.actor_id.as_bytes().into(),
							kind: kind.clone(),
						};

						async move {
							if let Err(err) =
								send_start_notice(&ctx, notice, &waiter.worker_id, waiter.wait_id)
									.await
							{
								tracing::debug!(?err, "actor start notice send failed");
							}
						}
					});
					futures_util::future::join_all(sends).await;
				}));
			}
			Effect::Start {
				epoch,
				command_index,
				allocation,
				new_storage,
			} => {
				if let Some(starts) = self.starts.get() {
					starts.record(epoch, command_index);
				}
				let self2 = self.clone();
				ctx.spawn(Box::pin(async move {
					let preloaded_kv = if new_storage {
						Some(p::preloaded_kv::empty_keyspace())
					} else {
						None
					};
					let sent = async {
						let data = self2.read_data().await?;
						let route = Route::of_allocation(&allocation)?;
						let fork = data
							.forked_from
							.as_ref()
							.map(|source| start_fork(source, data.create_unix_ms))
							.transpose()?;
						// A forked actor's start always carries its fork, so it is encoded at the
						// latest version. Forks are refused at admission until every listed worker
						// reads the latest envoy protocol, so an older engine worker hosts the
						// envoy only when it is stateless or the view lapsed since. When topology's
						// view knows the host reads an older version, the start fails here without
						// sending; when it does not, the old host drops the frame and the request
						// fails. Either way the send error below reports the start failed, the
						// actor sleeps, and its next wake allocates again, which beats starting the
						// actor without its fork hook. Every other start is encoded at the version
						// the envoy's host worker reads.
						let version = if fork.is_some() {
							if crate::envoy_protocol::known_older(&self2.ctx, &route.worker) {
								return Err(HostWithoutForks {
									worker_id: route.worker.clone(),
								}
								.into());
							}
							p::PROTOCOL_VERSION
						} else {
							crate::envoy_protocol::version_for_worker(
								&self2.ctx,
								&route.worker,
								crate::envoy_protocol::InternalMessage::Start,
							)
						};

						let payload =
							Bytes::from(p::versioned::ToEnvoy::serialize_with_embedded_version(
								&p::ToEnvoy::ToEnvoyCommands(vec![p::CommandWrapper {
									checkpoint: p::ActorCheckpoint {
										actor_id: self2.actor_id.to_string().into(),
										generation: epoch,
										index: command_index,
									},
									inner: p::Command::CommandStartActor(p::CommandStartActor {
										config: p::ActorConfig {
											name: data.name.to_string().into(),
											key: data.key.map(Into::into),
											create_ts: data.create_unix_ms,
											input: data.input.map(Into::into),
										},
										hibernating_requests: Vec::new(),
										preloaded_kv,
										fork,
									}),
									group: self2.group.map(|group| group.to_vec().into()),
								}]),
								version,
							)?);

						send_commands(&self2.ctx, &route, &payload).await
					}
					.await;

					// A failed send may still have been delivered, so the state machine stops
					// the allocation before it moves on. A failed report is left to the start
					// timer, which takes the same path.
					let Err(err) = sent else {
						return;
					};
					// Skipping a host known to be old is expected on every wake of a forked
					// actor while a deploy rolls, so it is not a warning.
					if let Some(skipped) = err.downcast_ref::<HostWithoutForks>() {
						tracing::debug!(
							actor_id = %self2.actor_id,
							worker_id = %skipped.worker_id,
							"forked start not sent to an envoy host that reads no forks; reporting the start failed"
						);
					} else {
						tracing::warn!(
							?err,
							"start command send failed; reporting the start failed"
						);
					}
					let reported = self2
						.report(
							"start_failed",
							wire::ActorEvent::StartFailed(wire::StartFailed { epoch }),
						)
						.await;
					if let Err(err) = reported {
						tracing::warn!(
							?err,
							"could not report a failed start; the start timer covers it"
						);
					}
				}));
			}
			Effect::CheckEnvoy { epoch, allocation } => {
				let self2 = self.clone();
				ctx.spawn(Box::pin(async move {
					if let Err(error) = self2.check_envoy(epoch, &allocation).await {
						tracing::warn!(
							?error,
							epoch,
							"envoy check failed; the next alarm or request judges the envoy again"
						);
					}
				}));
			}
			Effect::ForkDatabase { data, backoff_ms } => {
				let self2 = self.clone();
				ctx.spawn(Box::pin(async move {
					let attempt = data.state.attempt;
					let create_unix_ms = data.state.create_unix_ms;
					let outcome = async {
						self2
							.ctx
							.env()
							.clock()
							.sleep_until(self2.ctx.env().clock().observe()?.boot_ms + backoff_ms)
							.await?;
						anyhow::Ok(self2.fork_database_step(&data).await)
					}
					.await;
					let event = match outcome {
						Ok(Some(event)) => event,
						Ok(None) => return,
						Err(err) => {
							tracing::warn!(
								?err,
								attempt,
								"actor database fork step failed before it ran; the fork timer sends it again"
							);
							return;
						}
					};
					let reported = self2.report("fork_database_step", event).await;
					if let Err(err) = reported {
						tracing::warn!(
							?err,
							attempt,
							create_unix_ms,
							"could not report an actor database fork step; the fork timer sends it again"
						);
					}
				}));
			}
			Effect::ObserveForkStep { result, elapsed_ms } => {
				let self2 = self.clone();
				ctx.spawn(Box::pin(async move {
					let namespace = self2.fork_metrics_namespace().await;
					crate::metrics::ACTOR_FORK_STEPS
						.with_label_values(&[namespace.as_str(), result.label()])
						.inc();
					if let Some(elapsed_ms) = elapsed_ms {
						crate::metrics::ACTOR_FORK_SECONDS
							.with_label_values(&[namespace.as_str()])
							.observe(elapsed_ms as f64 / 1000.0);
					}
				}));
			}
			Effect::Destroy {
				attempt,
				index_bucket_count,
				create_unix_ms,
				legacy_keep_index_entry,
			} => {
				let self2 = self.clone();
				ctx.spawn(Box::pin(async move {
					let res = async {
						self2
							.ctx
							.env()
							.clock()
							.sleep_until(
								self2.ctx.env().clock().observe()?.boot_ms
									+ effect_backoff(attempt),
							)
							.await?;

						// The record may already belong to the next incarnation; only its
						// legacy namespace and name are read, and those never change for a key. The
						// index entry and the database volume are addressed by this
						// incarnation's `create_unix_ms`, so a late destroy cannot touch the
						// next incarnation's.
						let data_key = keys::kv::ActorDataKey::new();
						let data = self2
							.db
							.snapshot()
							.await?
							.get(&data_key.pack())
							.await?
							.context("destroyed actor has no data")?;
						let data = data_key.deserialize(data.into())?;

						// Legacy: a moved actor's entry stays until the backfill confirms its
						// copy; the page deletes it then. Removed in 3.1 with the backfill.
						if !legacy_keep_index_entry {
							self2
								.ctx
								.depot()
								.command::<crate::index::IndexDeleteActor>(
									&keys::partition::ActorIndexVolume::for_actor_in_namespace(
										self2.group,
										legacy_index_namespace(self2.group, &data)?,
										data.name.to_string(),
										self2.actor_id,
										index_bucket_count,
									),
									wire::IndexDeleteActorRequest {
										actor_id: self2.actor_id.as_bytes().into(),
										create_unix_ms,
									},
								)
								.await?;
						}

						self2
							.ctx
							.depot()
							.delete_current(keys::partition::ActorDatabaseVolume::in_namespace(
								self2.group,
								self2.actor_id,
								create_unix_ms,
							))
							.await?;

						// Legacy: an unprefixed actor the finished backfill skipped leaves the
						// skipped list however it was destroyed, its own destroy through the
						// envoy included. Only a listed actor sends the step, and a failed step
						// fails this attempt, so the retry sends the idempotent step again.
						// Removed in 3.1 with the backfill.
						if self2.group.is_none() {
							let namespace = legacy_index_namespace(None, &data)?;
							let standing =
								crate::catalog::namespace_standing(&self2.ctx, &namespace).await;
							if legacy_destroy_clears_skipped(
								&namespace,
								standing.as_deref(),
								self2.actor_id,
							) {
								crate::catalog::legacy_clear_skipped(
									&self2.ctx,
									&namespace,
									self2.actor_id,
								)
								.await?;
							}
						}

						anyhow::Ok(())
					}
					.await;

					let res = async {
						let phase = match res {
							Ok(()) => wire::DestroyPhase::DestroyPhaseSuccess,
							Err(err) => {
								log_effect_failure("destroy", attempt, &err);
								wire::DestroyPhase::DestroyPhaseFailed
							}
						};

						self2
							.report(
								"destroy_state",
								wire::ActorEvent::DestroyState(wire::DestroyState {
									phase,
									attempt,
									create_unix_ms,
								}),
							)
							.await
					}
					.await;

					if let Err(err) = res {
						tracing::warn!(
							?err,
							"could not report the destroy step; its effect timer runs it again"
						);
					}
				}));
			}
		}
	}
}

/// The fork a forked actor's start command carries, sent on every start of the incarnation so
/// the runtime can tell its first start of this fork from a later one. The fork time is the
/// incarnation's create time, and the fork input travels as the actor's input. The source's
/// chain comes from the record, so the runtime keeps the root of a fork of a fork even when the
/// copied database holds no chain.
pub(super) fn start_fork(
	source: &wire::ActorForkSource,
	create_unix_ms: i64,
) -> Result<p::ActorFork> {
	let chain = source
		.chain
		.iter()
		.map(|link| {
			Ok(p::ActorForkLink {
				actor_id: Id::from_slice(&link.actor_id)?.to_string().into(),
				key: link.key.as_ref().map(|key| key.to_string().into()),
				forked_at: link.forked_at,
			})
		})
		.collect::<Result<Vec<_>>>()?;
	Ok(p::ActorFork {
		fork_id: source.fork_id.to_vec().into(),
		actor_id: Id::from_slice(&source.actor_id)?.to_string().into(),
		key: source.key.as_ref().map(|key| key.to_string().into()),
		forked_at: create_unix_ms,
		chain,
	})
}

/// Sends one notice to the gateway wait `(worker_id, wait_id)` on the wait's own subject.
async fn send_start_notice(
	ctx: &EngineContext,
	notice: wire::ActorStartNotice,
	worker_id: &[u8],
	wait_id: u64,
) -> Result<()> {
	let worker = universalpubsub::WorkerId::from_bytes(
		worker_id
			.try_into()
			.context("invalid start waiter worker id")?,
	);
	// The waiting gateway may run the previous release while a deploy rolls.
	let payload = crate::sealed::seal_for_worker::<rivet_data::versioned::ActorStartNotice>(
		crate::sealed::SealedValue::StartNotice,
		&notice,
		ctx.topology().as_ref(),
		&worker,
	)?;

	ctx.ups()?
		.request_with_timeout(
			universalpubsub::RequestClass::ActorStartNotice,
			&worker,
			ActorStartNoticeSubject::new(wait_id),
			&payload,
			Duration::from_millis(ctx.config().pegboard().actor_start_notice_timeout_ms()),
		)
		.await?;

	Ok(())
}

/// Sends one command packet to an envoy connection and checks its acknowledgement.
async fn send_commands(ctx: &EngineContext, route: &Route, payload: &Bytes) -> Result<()> {
	let ack = ctx
		.ups()?
		.request(
			universalpubsub::RequestClass::EnvoyCommand,
			&route.worker,
			route.subject(),
			payload,
		)
		.await?;
	ensure!(
		ack.as_ref() != COMMANDS_REFUSED_FOREIGN_GROUP,
		"the envoy connection serves another namespace and refused the commands"
	);
	ensure!(
		ack.as_ref() != LEGACY_COMMANDS_REFUSED_NAMESPACE_MOVED,
		"the envoy connection opened before its namespace moved into its group and closes; the envoy reconnects with the group"
	);
	ensure!(ack.as_ref() == COMMANDS_ACK, "unexpected envoy ack");
	Ok(())
}

/// Legacy: whether the destroy of the unprefixed actor `actor_id` of `namespace` sends the step
/// that removes it from the finished backfill's skipped list: only when `standing`, the
/// namespace's standing as this process caches it, lists the actor. Every other unprefixed
/// destroy (an ungrouped namespace, an actor of the rollout window, the old copy a backfill move
/// destroys) sends nothing. A standing that cannot be read skips the clear rather than fail the
/// destroy, so a namespace volume that sheds or moves never holds up destroys; the cost is at
/// most one namespace read per status TTL per process on a cold cache. A missed entry, from an
/// unreadable standing or one cached before the backfill finished, is cleared by the actor API's
/// idempotent destroy of the skipped actor. Removed in 3.1 with the backfill.
pub(crate) fn legacy_destroy_clears_skipped(
	namespace: &str,
	standing: std::result::Result<&crate::NamespaceStanding, &anyhow::Error>,
	actor_id: Id,
) -> bool {
	match standing {
		Ok(standing) => crate::catalog::legacy_lists_skipped(standing, actor_id),
		Err(error) => {
			tracing::debug!(
				?error,
				namespace,
				?actor_id,
				"namespace standing unreadable at an unprefixed destroy; the actor api's destroy clears a skipped entry"
			);
			false
		}
	}
}

/// Whether the envoy record, read at `read_unix_ms`, proves the envoy holds no epoch any more.
///
/// An envoy stops every actor it holds once it has gone its lost threshold without the engine:
/// its lost timer fires after it notices the close, and on reconnecting it first stops them if
/// it last heard from the engine longer ago than that. A disconnected record's heartbeat is the
/// close time, stamped when the disconnect was written, and the record shows no reconnect
/// before `read_unix_ms`. Past the threshold plus a margin the envoy has therefore crashed,
/// stopped its actors, or will stop them before it serves another connection. A missing record
/// has no envoy behind it.
///
/// A record still marked connected is never gone, even when its worker died or its connection
/// closed without the disconnect write: its heartbeat is only the last record write, so it
/// bounds nothing about the envoy's last contact. The stop timer keeps resending until the
/// record changes.
pub(crate) fn envoy_gone(
	envoy: Option<&wire::EnvoyData>,
	read_unix_ms: i64,
	lost_threshold_ms: u64,
) -> bool {
	let Some(envoy) = envoy else {
		return true;
	};
	if envoy.connected {
		return false;
	}
	// A threshold past the timestamp range is never reached.
	let lost_threshold_ms = i64::try_from(lost_threshold_ms).unwrap_or(i64::MAX);
	let gone_after_unix_ms = envoy
		.heartbeat_unix_ms
		.saturating_add(lost_threshold_ms)
		.saturating_add(i64::try_from(ENVOY_GONE_MARGIN_MS).unwrap_or(i64::MAX));
	read_unix_ms > gone_after_unix_ms
}

/// Whether the connection a connected record names has ended, given `error`, its refusal of
/// the stop: its worker answered that it has no consumer for the connection, or the worker
/// could not be reached and its topology lease expired or was released. A worker that answered
/// in any other way is alive and may still serve the connection.
///
/// An ended lease counts only because a worker whose lease ended no longer serves envoy
/// connections: a released worker closed them first, and an expired one is assumed dead.
/// pegboard-envoy does not yet close its connections when its lease fences, so an expired
/// worker that is still alive breaks this assumption, as it does the envoy check.
async fn connection_ended(
	ctx: &EngineContext,
	envoy: &wire::EnvoyData,
	error: &anyhow::Error,
) -> Result<bool> {
	if no_responders(error) {
		return Ok(true);
	}
	if !unreachable_worker(error) {
		return Ok(false);
	}
	let route = Route::of_envoy(envoy)?;
	let ended = match ctx.topology().worker_lease(&route.worker).await {
		rivet_topology::WorkerLease::Expired | rivet_topology::WorkerLease::Released => true,
		rivet_topology::WorkerLease::Live | rivet_topology::WorkerLease::Unknown => false,
	};
	Ok(ended)
}

/// Records the envoy as disconnected now, for a record still marked connected whose
/// connection has ended (see `connection_ended`) without its disconnect write landing, as when
/// its worker crashed. The record would otherwise stay connected and never prove the envoy
/// gone. The close happened before now, and so did the envoy's last contact, so `now` is a
/// sound disconnect time. The write is conditional on the record read: a reconnect or the
/// connection's own late disconnect write wins over it, and a connection that is in fact still
/// live writes itself connected again with its next record write, which only delays a gone
/// verdict. Best effort; the next resend retries.
async fn mark_disconnected(
	ctx: &EngineContext,
	envoy_volume: &rivet_keys::keys::VolumeKey,
	stored: &wire::EnvoyData,
) {
	let result = async {
		let now_unix_ms = ctx.env().clock().observe()?.unix_ms;
		let mut data = stored.clone();
		data.connected = false;
		data.heartbeat_unix_ms = now_unix_ms.max(stored.heartbeat_unix_ms);
		let response = ctx
			.depot()
			.command::<crate::commands::EnvoyUpsert>(
				envoy_volume,
				wire::EnvoyUpsertRequest {
					data: data.clone(),
					expected: Some(stored.clone()),
				},
			)
			.await?;
		anyhow::Ok(response.data == data)
	}
	.await;
	match result {
		Ok(true) => tracing::info!(
			envoy_key = %stored.key,
			"marked an envoy disconnected whose connection ended without its disconnect write"
		),
		Ok(false) => tracing::debug!(
			envoy_key = %stored.key,
			"the envoy record changed before it could be marked disconnected"
		),
		Err(error) => tracing::debug!(
			?error,
			"could not mark the envoy disconnected; the next stop resend retries"
		),
	}
}

/// Picks an envoy for the actor by rendezvous hashing over every live envoy that serves its name
/// Chooses the envoy to run `actor_id` among the pool's active envoys serving `name`, with its
/// route from the index entry: the actor's allocation. `None` when no active envoy serves the
/// name.
pub(crate) async fn select_allocation(
	ctx: &EngineContext,
	actor_id: &[u8],
	namespace: &str,
	pool_name: &str,
	name: &str,
) -> Result<Option<wire::ActorAllocation>> {
	let envoys = index::list_active_envoys(ctx, namespace, pool_name, name).await?;
	Ok(choose_envoy(actor_id, envoys))
}

/// Rendezvous hashing of `actor_id` over the candidate allocations: the envoy whose key hashes
/// highest with the actor id wins, so two selections against the same active set agree and an
/// envoy joining or leaving moves only the actors that hash to it.
pub(crate) fn choose_envoy(
	actor_id: &[u8],
	candidates: Vec<wire::ActorAllocation>,
) -> Option<wire::ActorAllocation> {
	candidates.into_iter().max_by_key(|candidate| {
		// The actor ID has a fixed length, so concatenating it with the envoy key is
		// unambiguous.
		let mut hasher = Xxh3::with_seed(0);
		hasher.update(actor_id);
		hasher.update(candidate.envoy_key.as_bytes());
		hasher.digest()
	})
}

/// Whether the `refusals`-th refusal in a row is logged as a warning rather than at debug.
pub(super) fn refusal_warns(refusals: u32) -> bool {
	refusals >= REFUSALS_WARN_AFTER && (refusals - REFUSALS_WARN_AFTER) % REFUSALS_WARN_EVERY == 0
}

/// Logs a failed attempt of `effect`, whose failure the executor reports so the state machine
/// runs the effect again. A refusal that ran nothing, such as a full transport window or a
/// volume in transition, is counted and logged at debug, and at warn with its reason only when
/// it keeps recurring; any other failure is an error.
pub(super) fn log_effect_failure(effect: &'static str, attempt: u32, err: &anyhow::Error) {
	let Some(retry_after_ms) = depot_client::retry_after_ms(err) else {
		tracing::error!(
			?err,
			effect,
			attempt,
			"actor effect failed; the state machine runs it again"
		);
		return;
	};
	let reason = depot_client::refusal_reason(err);
	crate::metrics::ACTOR_EFFECT_REFUSALS
		.with_label_values(&[effect, reason])
		.inc();
	if refusal_warns(attempt) {
		tracing::warn!(
			?err,
			effect,
			reason,
			attempt,
			"actor effect keeps being refused; depot is shedding load on the volume it writes"
		);
	} else {
		tracing::debug!(
			effect,
			reason,
			attempt,
			retry_after_ms,
			"actor effect refused for now"
		);
	}
}

/// Sends an effect's result to the state machine through `send`, sending it again while depot
/// refuses it at admission, which proves the event ran nothing. Each resend waits the refusal's
/// hint doubled per earlier refusal, between half and all of it, so a shedding tablet's many
/// refused reports neither come back in lockstep nor at its hint's rate. After
/// [`REPORT_REFUSAL_RESENDS`] resends, or once the next would pass [`super::EFFECT_TIMEOUT_MS`]
/// after the first send, the refusal returns and the effect's own deadline timer runs the
/// effect again, so a refused result is never simply dropped. Any other failure returns at once
/// and leaves the outcome to that timer.
pub(super) async fn redeliver_refused<F, Fut>(
	clock: &dyn rivet_env::Clock,
	rng: &mut rivet_env::RngStream,
	event: &'static str,
	mut send: F,
) -> Result<()>
where
	F: FnMut() -> Fut,
	Fut: Future<Output = Result<()>>,
{
	let deadline_boot_ms = clock
		.observe()?
		.boot_ms
		.saturating_add(super::EFFECT_TIMEOUT_MS);
	let mut refusals = 0u32;
	loop {
		let err = match send().await {
			Ok(()) => return Ok(()),
			Err(err) => err,
		};
		let Some(retry_after_ms) = depot_client::retry_after_ms(&err) else {
			return Err(err);
		};

		refusals = refusals.saturating_add(1);
		let reason = depot_client::refusal_reason(&err);
		crate::metrics::ACTOR_EVENT_REPORT_REFUSALS
			.with_label_values(&[event, reason])
			.inc();
		if refusals > REPORT_REFUSAL_RESENDS {
			tracing::debug!(
				event,
				reason,
				refusals,
				"actor event still refused; the effect's timer runs the effect again"
			);
			return Err(err);
		}
		let retry_at_boot_ms = clock
			.observe()?
			.boot_ms
			.saturating_add(report_refusal_delay_ms(refusals, retry_after_ms, rng));
		if retry_at_boot_ms >= deadline_boot_ms {
			return Err(err);
		}
		tracing::debug!(
			event,
			reason,
			refusals,
			retry_after_ms,
			"actor event refused; sending it again"
		);
		clock.sleep_until(retry_at_boot_ms).await?;
	}
}

/// How many times [`redeliver_refused`] sends a refused result again before it leaves the
/// effect to its own deadline timer.
pub(super) const REPORT_REFUSAL_RESENDS: u32 = 2;

/// The wait before resending a result refused `refusals` times in a row with hint
/// `retry_after_ms`: the hint, capped at [`REPORT_REFUSAL_HINT_MAX_MS`] and doubled per earlier
/// refusal, then between half and all of that.
pub(super) fn report_refusal_delay_ms(
	refusals: u32,
	retry_after_ms: u64,
	rng: &mut rivet_env::RngStream,
) -> u64 {
	let doublings = refusals.saturating_sub(1).min(16);
	let delay_ms = retry_after_ms
		.clamp(1, REPORT_REFUSAL_HINT_MAX_MS)
		.saturating_mul(1u64 << doublings);
	let half_ms = delay_ms / 2;
	half_ms + rng.gen_range(0..delay_ms - half_ms + 1)
}

pub(crate) fn effect_backoff(attempt: u32) -> u64 {
	let Some(retry) = attempt.checked_sub(2) else {
		return 0;
	};

	let multiplier = 1u64.checked_shl(retry).unwrap_or(u64::MAX);
	EFFECT_BACKOFF_BASE_MS
		.saturating_mul(multiplier)
		.min(EFFECT_BACKOFF_MAX_MS)
}
