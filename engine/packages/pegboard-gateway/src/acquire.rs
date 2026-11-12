//! Acquiring an actor for one gateway request: creating it when asked, waiting for it to start
//! and opening the stream to its envoy.
//!
//! The actor state machine owns the actor's lifecycle; this reads it and nudges it. A read that
//! finds the actor not running sends a `TryStart` with this process's waiter, which wakes the
//! actor and registers for its start notices; the state machine dedupes the waiter, so every
//! read may send one. Notices and the fallback poll drive the next read. The gateway also
//! judges the envoy an actor was allocated to, since the state machine has no start or stop
//! timeout: a dead or draining envoy is reported with a `Reallocate`.
use crate::ActorNotFound;
use crate::start_wait::{Next, StartWait, Wake};
use crate::start_waits::StartWaits;
use anyhow::Result;
use base64::Engine;
use pegboard::commands::{ActorRead, is_volume_gone};
use pegboard::envoy_check::{
	EnvoyCheck, Route, envoy_check, no_responders, repair_route, retryable_ups_error,
};
use pegboard::keys::partition::ActorVolume;
use pegboard::legacy_volumes::ResolvedRead;
use pegboard::wire;
use rivet_engine_context::EngineContext;
use rivet_util::Id;
use std::sync::Arc;
use std::time::Duration;

/// First wait before retrying a stream open to a running actor that failed in transport.
const STREAM_RETRY_INITIAL_DELAY: Duration = Duration::from_millis(50);

/// Longest wait between stream open retries to a running actor.
const STREAM_RETRY_MAX_DELAY: Duration = Duration::from_millis(500);

/// How long a request waits for its actor before failing.
const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(45);

/// One routing lifetime, shared by every acquisition attempt and its durable waiters.
#[derive(Clone, Copy)]
pub(crate) struct RoutingDeadline {
	pub(crate) boot_ms: u64,
	pub(crate) unix_ms: i64,
}

impl RoutingDeadline {
	pub(crate) fn new(ctx: &EngineContext) -> Result<Self> {
		let now = ctx.env().clock().observe()?;
		Ok(Self {
			boot_ms: now
				.boot_ms
				.saturating_add(ACQUIRE_TIMEOUT.as_millis() as u64),
			unix_ms: now
				.unix_ms
				.saturating_add(ACQUIRE_TIMEOUT.as_millis() as i64),
		})
	}
}

/// What a getOrCreate request creates. A create of an existing actor wakes it and leaves its
/// own input in place; a create of a destroyed actor starts it fresh.
#[derive(Clone)]
pub(crate) struct CreateActor {
	/// The actor the create makes and the group its volumes are keyed in: always the
	/// namespace's own, from [`pegboard::actor_key::keyed_create_actor`], whatever actor the
	/// request first resolved to. Computed from `standing` again once the create is sent.
	pub(crate) actor: pegboard::actor_key::KeyedActor,
	/// The request's namespace as the create acts on it: its name, which tokens are checked
	/// against, its group, and for a namespace without a group the legacy namespace the actor
	/// records (`NamespaceStanding::legacy_create_namespace`).
	pub(crate) standing: Arc<pegboard::NamespaceStanding>,
	pub(crate) pool_name: String,
	pub(crate) name: String,
	pub(crate) key: String,
	/// The request's `rvt-input`, URL-safe base64 without padding.
	pub(crate) input: Option<String>,
	/// The request forces actor v3 while its namespace migrates in from V2, whatever
	/// `create_ratio` says.
	pub(crate) force_actor_v3: bool,
}

impl CreateActor {
	/// Authorizes the mutation before sending an actor create command.
	fn authorize(
		&self,
		credential: &rivet_guard_core::auth::Credential,
		now_unix_ms: i64,
		actor_id: Id,
	) -> Result<()> {
		credential.authorize(
			now_unix_ms,
			&self.standing.name,
			rivet_guard_core::auth::ResourceKind::ActorGateway,
			Some(actor_id),
			rivet_guard_core::auth::OperationKind::Create,
		)
	}
}

/// Checks whether a request may reach an existing actor through its gateway. `namespace` is
/// the one the request named: the record names none, and the group it was read through proves
/// which namespace it is read in.
fn authorize_read(
	credential: &rivet_guard_core::auth::Credential,
	now_unix_ms: i64,
	namespace: &str,
	actor_id: Id,
) -> Result<()> {
	credential.authorize(
		now_unix_ms,
		namespace,
		rivet_guard_core::auth::ResourceKind::ActorGateway,
		Some(actor_id),
		rivet_guard_core::auth::OperationKind::Read,
	)
}

/// Another datacenter owns the actor key. Forwarding across datacenters is not built yet.
#[derive(Debug)]
pub(crate) struct ActorKeyOwnedElsewhere {
	pub(crate) datacenter_label: u16,
}

impl std::fmt::Display for ActorKeyOwnedElsewhere {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"actor key is owned by datacenter {}",
			self.datacenter_label
		)
	}
}

impl std::error::Error for ActorKeyOwnedElsewhere {}

/// The V2 migrator holds the actor, so nothing here starts or waits for it. V2 may still serve
/// it, so routing decides where the request goes.
#[derive(Debug)]
pub(crate) struct HeldByMigrateFromV2;

impl std::fmt::Display for HeldByMigrateFromV2 {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("the actor is held by a migration from V2")
	}
}

impl std::error::Error for HeldByMigrateFromV2 {}

/// The actor's key belongs to V2 while its namespace migrates in, so V2 serves the request.
#[derive(Debug)]
pub(crate) struct DelegatedToV2;

impl std::fmt::Display for DelegatedToV2 {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str("the actor's key belongs to V2")
	}
}

impl std::error::Error for DelegatedToV2 {}

/// Where an acquisition starts, carried from one attempt of a request to the next so a retried
/// attempt resumes from where the last one left off rather than from the request's resolve.
pub(crate) struct AcquireTarget {
	/// The actor the attempt routes to: the resolved one, or the create's own once a create
	/// moved the acquisition to it.
	pub(crate) actor_id: Id,
	/// The group the actor's volumes are in: its namespace's, or `None` for an actor at the
	/// legacy unprefixed keys.
	pub(crate) group: Option<rivet_keys::keys::codec::GroupId>,
	/// What the request's resolve learned about the actor's state volume, taken by the first
	/// read so it is not read twice.
	pub(crate) resolved: Option<ResolvedRead>,
	pub(crate) create: Option<CreateActor>,
	/// Whether an attempt already sent the create, so a later attempt reads the actor first and
	/// creates again only once it finds the actor gone.
	pub(crate) create_sent: bool,
}

impl AcquireTarget {
	pub(crate) fn new(
		actor_id: Id,
		group: Option<rivet_keys::keys::codec::GroupId>,
		resolved: ResolvedRead,
		create: Option<CreateActor>,
	) -> Self {
		Self {
			actor_id,
			group,
			resolved: Some(resolved),
			create,
			create_sent: false,
		}
	}

	/// Whether the acquisition starts with its create instead of a read: only when it may
	/// create, no earlier attempt sent the create, the actor resolved is the one the create
	/// makes, and the resolve did not already find that actor's record, which the first read
	/// reuses instead. A request resolved to an actor not yet moved reads it first and creates
	/// only once it is gone.
	fn creates_first(&self) -> bool {
		let Some(create) = &self.create else {
			return false;
		};
		if self.create_sent {
			return false;
		}
		let resolved_is_created =
			create.actor.actor_id == self.actor_id && create.actor.group == self.group;
		let found = matches!(self.resolved, Some(ResolvedRead::Found(_)));
		resolved_is_created && !found
	}

	/// Records that `create` is about to be sent and moves the target to its actor. Returns
	/// whether the actor or group changed. What the resolve read is dropped either way: a create
	/// invalidates any earlier read, so the next read reads the actor again.
	fn follow_create(&mut self, create: CreateActor) -> bool {
		self.create_sent = true;
		self.resolved = None;
		let moved = create.actor.actor_id != self.actor_id || create.actor.group != self.group;
		if moved {
			self.actor_id = create.actor.actor_id;
			self.group = create.actor.group;
		}
		self.create = Some(create);
		moved
	}
}

/// Waits until the actor runs and opens a stream to it on `class`. With `create`, the actor is
/// created first and created again if it is found destroyed. With `pipeline`, the request may
/// follow the start command to its envoy before the actor runs; a request the envoy already
/// refused there waits for the actor to run instead. Returns the actor's epoch and the stream.
/// `namespace` is the request's validated namespace, which a token is checked against. `target`
/// is where the acquisition starts; it is updated in place whether the acquisition succeeds or
/// fails, so the request's next attempt starts from it. A create always makes the create's own
/// actor in the namespace's group, and the acquisition follows it there, so the answer names
/// the actor and group the stream reached.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn acquire(
	ctx: &EngineContext,
	waits: &Arc<StartWaits>,
	target: &mut AcquireTarget,
	namespace: &str,
	class: universalpubsub::RequestClass,
	pipeline: bool,
	credential: &rivet_guard_core::auth::Credential,
	deadline: RoutingDeadline,
) -> Result<Acquired> {
	let clock = ctx.env().clock();

	let mut acquire = Acquire {
		ctx,
		waits: waits.clone(),
		class,
		deadline_unix_ms: deadline.unix_ms,
		wait: StartWait::new(waits.clone(), target.actor_id, target.group),
		target,
		namespace,
		pipeline,
		credential,
		authorization_namespace: None,
		last_phase: "unread",
	};
	let acquired = rivet_env::timeout_at(clock.as_ref(), deadline.boot_ms, acquire.run()).await;
	let result = match acquired {
		Ok(Ok((epoch, stream))) => Ok(Acquired {
			epoch,
			stream,
			actor_id: acquire.target.actor_id,
			group: acquire.target.group,
		}),
		Ok(Err(error)) => Err(error),
		Err(rivet_env::TimeoutError::Elapsed) => {
			tracing::warn!(actor_id = %acquire.target.actor_id, last_phase = acquire.last_phase, "actor did not become ready before the gateway deadline");
			Err(crate::RequestTimeout.build())
		}
		Err(error) => Err(error.into()),
	};
	acquire.wait.release().await;
	result
}

/// A stream to a running actor: its epoch, and the actor and group it reached, which differ from
/// the requested ones only after a create made the namespace's own actor.
pub(crate) struct Acquired {
	pub(crate) epoch: u32,
	pub(crate) stream: universalpubsub::DuplexStream,
	pub(crate) actor_id: Id,
	pub(crate) group: Option<rivet_keys::keys::codec::GroupId>,
}

/// Whether a failed stream open to an envoy connection may succeed through the envoy fallback:
/// a stale route, a missing worker or a transport failure, as opposed to a request error.
pub(crate) fn retryable_open_error(error: &anyhow::Error) -> bool {
	retryable_ups_error(error)
}

/// What a read of the actor found.
enum Read {
	/// The actor's data and phase.
	Actor {
		actor: wire::ActorData,
		epoch: u32,
		phase: wire::ActorPhase,
	},
	/// Created but its data is not written yet: it is initializing.
	Initializing,
	/// No state volume: the actor never existed or was destroyed and deleted.
	Gone,
}

/// One request's acquisition, kept outside the deadline so its wait is released however it ends.
struct Acquire<'a> {
	ctx: &'a EngineContext,
	waits: Arc<StartWaits>,
	class: universalpubsub::RequestClass,
	deadline_unix_ms: i64,
	/// Where this attempt routes, shared with the request's later attempts.
	target: &'a mut AcquireTarget,
	/// The namespace the request named, the only scope a token is checked against.
	namespace: &'a str,
	pipeline: bool,
	credential: &'a rivet_guard_core::auth::Credential,
	/// The request's namespace once an actor read or this keyed creation authorized it; `None`
	/// until then, so a JWT does not route on a notice alone.
	authorization_namespace: Option<String>,
	wait: StartWait,
	last_phase: &'static str,
}

impl Acquire<'_> {
	async fn run(&mut self) -> Result<(u32, universalpubsub::DuplexStream)> {
		let ctx = self.ctx;
		let clock = ctx.env().clock();
		let mut stream_retry_delay = STREAM_RETRY_INITIAL_DELAY;
		let mut next = match self.target.creates_first() {
			true => self.create().await?,
			false => Next::Read,
		};
		// The actor's data from the last read, which a notice does not carry.
		let mut last_actor: Option<wire::ActorData> = None;

		loop {
			let (actor, epoch, phase) = match std::mem::replace(&mut next, Next::Read) {
				Next::Running { epoch, allocation } => match &last_actor {
					Some(actor) => (
						actor.clone(),
						epoch,
						wire::ActorPhase::Running(wire::Running { allocation }),
					),
					// The notice names where the actor runs, so route there before reading
					// it. Only a failed open needs the actor's data, for the envoy fallback.
					None => match self.open_noticed(&allocation).await? {
						Some(stream) => return Ok((epoch, stream)),
						None => {
							next = Next::Read;
							continue;
						}
					},
				},
				// The start command already went to the allocation, so the request follows it
				// there instead of waiting for the actor to run. A failed open falls back to the
				// wait. The actor keeps its waiter after a starting notice, so a request that
				// may not pipeline waits for the running notice that follows.
				Next::Starting { .. } if !self.pipeline => {
					next = Next::Wait;
					continue;
				}
				Next::Starting { epoch, allocation } => match self.open_noticed(&allocation).await?
				{
					Some(stream) => return Ok((epoch, stream)),
					None => {
						next = Next::Wait;
						continue;
					}
				},
				Next::KeyOwnedElsewhere { reservation_id } => {
					return Err(ActorKeyOwnedElsewhere {
						datacenter_label: reservation_id.label(),
					}
					.into());
				}
				Next::MigratingFromV2 => return Err(HeldByMigrateFromV2.into()),
				Next::DelegatedToV2 => return Err(DelegatedToV2.into()),
				Next::Wait => {
					next = self.wait.wait(ctx).await?;
					continue;
				}
				Next::Read => match self.read().await? {
					Read::Actor {
						actor,
						epoch,
						phase,
					} => (actor, epoch, phase),
					Read::Initializing => {
						self.last_phase = "initializing";
						next = if self.credential.is_namespace_scoped()
							&& self.authorization_namespace.is_none()
						{
							self.wait.wait(ctx).await?
						} else {
							self.not_running().await?
						};
						continue;
					}
					Read::Gone => {
						self.last_phase = "gone";
						let before = self.target.actor_id;
						next = self.create_or_not_found().await?;
						if self.target.actor_id != before {
							last_actor = None;
						}
						continue;
					}
				},
			};
			// Without a group the unprefixed key space is shared, so a record of another
			// namespace is refused as missing before anything authorizes or wakes it.
			if !pegboard::catalog::actor_in_namespace(self.target.group, self.namespace, &actor) {
				return Err(ActorNotFound.build());
			}
			// The read found the actor through the request namespace's group, so the request's
			// namespace is the scope.
			authorize_read(
				self.credential,
				self.ctx.env().clock().observe()?.unix_ms,
				self.namespace,
				self.target.actor_id,
			)?;
			self.authorization_namespace = Some(self.namespace.to_owned());
			last_actor = Some(actor.clone());
			self.last_phase = phase_name(&phase);

			match phase {
				wire::ActorPhase::Initializing(_) | wire::ActorPhase::Sleeping(_) => {
					next = self.not_running().await?;
				}
				// The state machine's start and stop timers end these phases eventually, but a
				// dead envoy is found sooner by the gateway, and a draining one moves the
				// actor. The gateway judges the envoy on its fallback polls only, so a burst
				// of starts does not read every envoy record.
				wire::ActorPhase::Starting(wire::Starting { allocation })
				| wire::ActorPhase::Stopping(wire::Stopping { allocation }) => {
					next = match self.check_envoy_on_poll(&actor, epoch, &allocation).await? {
						Some(reallocated) => reallocated,
						None => self.not_running().await?,
					};
				}
				wire::ActorPhase::Running(wire::Running { allocation }) => {
					let route = Route::of_allocation(&allocation)?;
					let error = match self.open(&route).await {
						Ok(stream) => return Ok((epoch, stream)),
						Err(error) => error,
					};
					if !retryable_open_error(&error) {
						return Err(error);
					}

					// The failed open is the liveness check: read the envoy's record once.
					tracing::debug!(%error, "actor channel open failed; reading the envoy record");
					match envoy_check(
						ctx,
						self.namespace,
						&actor,
						&allocation,
						no_responders(&error),
					)
					.await?
					{
						EnvoyCheck::Newer(newer) => match self.open(&newer).await {
							Ok(stream) => {
								// The allocation's route is stale for the rest of the epoch
								// unless someone records what this request learned.
								repair_route(
									ctx,
									self.target.group,
									self.target.actor_id,
									epoch,
									&allocation,
									newer,
								);
								return Ok((epoch, stream));
							}
							Err(error) if no_responders(&error) => {
								// This newer route ended too. The stop executor reads the record
								// again and waits for the old epoch to stop before replacing it.
								next = self.reallocate(epoch, true).await?;
							}
							Err(error) if retryable_open_error(&error) => {
								tracing::debug!(%error, "actor channel open failed on the envoy's newer connection");
								rivet_env::sleep(clock.as_ref(), stream_retry_delay).await?;
								stream_retry_delay =
									(stream_retry_delay * 2).min(STREAM_RETRY_MAX_DELAY);
							}
							Err(error) => return Err(error),
						},
						EnvoyCheck::Lost => {
							next = self.reallocate(epoch, true).await?;
						}
						EnvoyCheck::Draining => {
							next = self.reallocate(epoch, false).await?;
						}
						EnvoyCheck::Serving => {
							// Transport failures persist for a while, so retrying at once only
							// adds another open per attempt.
							rivet_env::sleep(clock.as_ref(), stream_retry_delay).await?;
							stream_retry_delay =
								(stream_retry_delay * 2).min(STREAM_RETRY_MAX_DELAY);
						}
					}
				}
				// A create of a key delegated to V2 reports the delegation while its namespace
				// migrates in from V2, and creates the actor here once it no longer does.
				wire::ActorPhase::Destroying(_)
				| wire::ActorPhase::Destroyed
				| wire::ActorPhase::DelegatedToV2 => {
					let before = self.target.actor_id;
					next = self.create_or_not_found().await?;
					if self.target.actor_id != before {
						last_actor = None;
					}
				}
				// The V2 migrator holds the actor, and V2 may still serve it, so nothing here
				// starts or waits for it.
				wire::ActorPhase::MigratingFromV2(_) => return Err(HeldByMigrateFromV2.into()),
			}
		}
	}

	/// Opens a stream to `route` on this request's class.
	async fn open(&self, route: &Route) -> Result<universalpubsub::DuplexStream> {
		self.authorize_route()?;
		let stream = self
			.ctx
			.ups()?
			.open_bidirectional_stream(
				&route.worker,
				route.subject(),
				universalpubsub::StreamOptions::new(self.class),
			)
			.await?;
		self.authorize_route()?;
		Ok(stream)
	}

	/// Notices carry no namespace, so a namespace-scoped credential can route only after a read
	/// or keyed creation established its scope. Fresh expiry checks also cover opens resumed
	/// from a notice.
	fn authorize_route(&self) -> Result<()> {
		if !self.credential.is_namespace_scoped() {
			return Ok(());
		}
		let namespace = self
			.authorization_namespace
			.as_deref()
			.ok_or_else(|| rivet_guard_core::auth::InsufficientPermissions.build())?;
		authorize_read(
			self.credential,
			self.ctx.env().clock().observe()?.unix_ms,
			namespace,
			self.target.actor_id,
		)
	}

	/// Opens a stream to the allocation a start notice or `TryStart` answer named, with no
	/// actor read. Returns `None` when the open failed in a way the envoy fallback handles, so
	/// the caller reads the actor and takes the usual running path.
	async fn open_noticed(
		&self,
		allocation: &wire::ActorAllocation,
	) -> Result<Option<universalpubsub::DuplexStream>> {
		let route = Route::of_allocation(allocation)?;
		match self.open(&route).await {
			Ok(stream) => Ok(Some(stream)),
			Err(error) if retryable_open_error(&error) => {
				tracing::debug!(%error, "noticed actor channel open failed; reading the actor");
				Ok(None)
			}
			Err(error) => Err(error),
		}
	}

	async fn read(&mut self) -> Result<Read> {
		let resolved = self.target.resolved.take().unwrap_or(ResolvedRead::Unread);
		let response = match resolved {
			ResolvedRead::Found(response) => response,
			ResolvedRead::Gone => return Ok(Read::Gone),
			ResolvedRead::Unread => {
				let response = self
					.ctx
					.depot()
					.command::<ActorRead>(
						&ActorVolume::in_namespace(self.target.group, self.target.actor_id),
						(),
					)
					.await;
				match response {
					Ok(response) => response,
					Err(error) if is_volume_gone(&error) => return Ok(Read::Gone),
					Err(error) => return Err(error),
				}
			}
		};
		Ok(match (response.data, response.phase) {
			(Some(actor), Some(phase)) => Read::Actor {
				actor,
				epoch: response.epoch,
				phase,
			},
			(None, _) | (_, None) => Read::Initializing,
		})
	}

	/// Creates the actor with this request's waiter and returns what the answer settled. The
	/// create makes its own actor in the namespace's group; when the request had resolved to
	/// another, such as a not yet moved actor at the legacy unprefixed keys, the acquisition
	/// moves to the created actor first, with a start wait for it in place of the old one.
	/// Legacy: a create whose standing has no group reads the namespace again first and creates
	/// at the new id in the group when the namespace has moved into one; see
	/// [`pegboard::actor_key::keyed_create_standing`]. Removed in 3.1 with the backfill.
	async fn create(&mut self) -> Result<Next> {
		let Some(mut create) = self.target.create.clone() else {
			return Err(ActorNotFound.build());
		};
		let standing =
			pegboard::actor_key::keyed_create_standing(self.ctx, &create.standing).await?;
		create.actor = pegboard::actor_key::keyed_create_actor(
			self.ctx.config().dc_label(),
			&standing,
			&create.name,
			&create.key,
		);
		create.standing = standing;
		create.authorize(
			self.credential,
			self.ctx.env().clock().observe()?.unix_ms,
			create.actor.actor_id,
		)?;
		self.authorization_namespace = Some(create.standing.name.clone());
		let legacy_namespace = create.standing.legacy_create_namespace();
		if self.target.follow_create(create.clone()) {
			let wait = StartWait::new(self.waits.clone(), self.target.actor_id, self.target.group);
			std::mem::replace(&mut self.wait, wait).release().await;
		}
		let input = create
			.input
			.as_deref()
			.map(|input| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(input))
			.transpose()?;
		let waiter = self.wait.waiter(self.ctx, self.deadline_unix_ms).await?;
		let created = self
			.ctx
			.depot()
			.command::<pegboard::commands::ActorCreate>(
				&ActorVolume::in_namespace(self.target.group, self.target.actor_id),
				wire::ActorCreateRequest {
					init: wire::ActorCreateData {
						actor_id: self.target.actor_id.as_bytes().into(),
						pool_name: create.pool_name.clone().into(),
						name: create.name.clone().into(),
						key: Some(create.key.clone().into()),
						input: input.map(Into::into),
						legacy_namespace: legacy_namespace.map(Into::into),
						fork_from: None,
						legacy_move: None,
					},
					waiter: waiter.clone(),
					force_actor_v3: create.force_actor_v3,
				},
			)
			.await
			.map_err(pegboard::commands::legacy_v3_create_refusal)?;
		let settled = self
			.wait
			.on_outcome(waiter.as_ref(), created.outcome)
			.await?;
		Ok(settled.unwrap_or(Next::Wait))
	}

	/// A destroyed actor is created again by a request that may create, and is not found by
	/// one that may not.
	async fn create_or_not_found(&mut self) -> Result<Next> {
		if self.target.create.is_none() {
			return Err(ActorNotFound.build());
		}
		self.create().await
	}

	/// Wakes the actor and registers for its notices, then waits for a notice or the fallback
	/// poll unless the answer already settled the next step.
	async fn not_running(&mut self) -> Result<Next> {
		match self.wait.try_start(self.ctx, self.deadline_unix_ms).await? {
			Some(next) => Ok(next),
			None => self.wait.wait(self.ctx).await,
		}
	}

	/// Judges the envoy of an actor on its start or stop path, only when a fallback poll woke
	/// this request. Returns the next step when the envoy is gone or draining and the actor was
	/// told to reallocate.
	async fn check_envoy_on_poll(
		&mut self,
		actor: &wire::ActorData,
		epoch: u32,
		allocation: &wire::ActorAllocation,
	) -> Result<Option<Next>> {
		if self.wait.last_wake() != Some(Wake::Poll) {
			return Ok(None);
		}
		match envoy_check(self.ctx, self.namespace, actor, allocation, false).await? {
			EnvoyCheck::Serving | EnvoyCheck::Newer(_) => Ok(None),
			EnvoyCheck::Lost => Ok(Some(self.reallocate(epoch, true).await?)),
			EnvoyCheck::Draining => Ok(Some(self.reallocate(epoch, false).await?)),
		}
	}

	/// Tells the actor its epoch's envoy is lost or draining, with this request's waiter so it
	/// follows the reallocation's notices, and returns what the answer settled.
	async fn reallocate(&mut self, epoch: u32, lost: bool) -> Result<Next> {
		let waiter = self.wait.waiter(self.ctx, self.deadline_unix_ms).await?;
		let response = crate::start_wait::send_event(
			self.ctx,
			self.target.group,
			self.target.actor_id,
			wire::ActorEvent::Reallocate(wire::Reallocate {
				epoch,
				lost,
				waiter: waiter.clone(),
			}),
		)
		.await?;
		match response.outcome {
			Some(outcome) => Ok(self
				.wait
				.on_outcome(waiter.as_ref(), outcome)
				.await?
				.unwrap_or(Next::Wait)),
			None => Ok(Next::Read),
		}
	}
}

/// Bounded phase names for diagnostics.
fn phase_name(phase: &wire::ActorPhase) -> &'static str {
	match phase {
		wire::ActorPhase::Initializing(_) => "initializing",
		wire::ActorPhase::Starting(_) => "starting",
		wire::ActorPhase::Running(_) => "running",
		wire::ActorPhase::Stopping(_) => "stopping",
		wire::ActorPhase::Sleeping(_) => "sleeping",
		wire::ActorPhase::Destroying(_) => "destroying",
		wire::ActorPhase::Destroyed => "destroyed",
		wire::ActorPhase::MigratingFromV2(_) => "migrating_from_v2",
		wire::ActorPhase::DelegatedToV2 => "delegated_to_v2",
	}
}
