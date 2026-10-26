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
use crate::route::Route;
use crate::start_wait::{Next, StartWait, Wake};
use crate::start_waits::StartWaits;
use anyhow::Result;
use base64::Engine;
use pegboard::commands::{ActorRead, EnvoyRead, is_volume_gone};
use pegboard::keys::partition::{ActorVolume, EnvoyVolume};
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
	pub(crate) namespace: String,
	pub(crate) pool_name: String,
	pub(crate) name: String,
	pub(crate) key: String,
	/// The request's `rvt-input`, URL-safe base64 without padding.
	pub(crate) input: Option<String>,
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
			&self.namespace,
			rivet_guard_core::auth::ResourceKind::ActorGateway,
			Some(actor_id),
			rivet_guard_core::auth::OperationKind::Create,
		)
	}
}

/// Checks whether a request may reach an existing actor through its gateway.
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

/// Waits until the actor runs and opens a stream to it on `class`. With `create`, the actor is
/// created first and created again if it is found destroyed. With `pipeline`, the request may
/// follow the start command to its envoy before the actor runs; a request the envoy already
/// refused there waits for the actor to run instead. Returns the actor's epoch and the stream.
pub(crate) async fn acquire(
	ctx: &EngineContext,
	waits: &Arc<StartWaits>,
	actor_id: Id,
	create: Option<CreateActor>,
	class: universalpubsub::RequestClass,
	pipeline: bool,
	credential: &rivet_guard_core::auth::Credential,
	deadline: RoutingDeadline,
) -> Result<(u32, universalpubsub::DuplexStream)> {
	let clock = ctx.env().clock();

	let mut acquire = Acquire {
		ctx,
		class,
		deadline_unix_ms: deadline.unix_ms,
		actor_id,
		create,
		pipeline,
		credential,
		authorization_namespace: None,
		wait: StartWait::new(waits.clone(), actor_id),
		last_phase: "unread",
	};
	let acquired = rivet_env::timeout_at(clock.as_ref(), deadline.boot_ms, acquire.run()).await;
	let result = match acquired {
		Ok(result) => result,
		Err(error) => {
			tracing::warn!(%actor_id, last_phase = acquire.last_phase, "actor did not become ready before the gateway deadline");
			Err(error.into())
		}
	};
	acquire.wait.release().await;
	result
}

/// Whether a failed stream open to an envoy connection may succeed through the envoy fallback:
/// a stale route, a missing worker or a transport failure, as opposed to a request error.
pub(crate) fn retryable_open_error(error: &anyhow::Error) -> bool {
	pegboard::actor_machine::executor::retryable_ups_error(error)
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
	class: universalpubsub::RequestClass,
	deadline_unix_ms: i64,
	actor_id: Id,
	create: Option<CreateActor>,
	pipeline: bool,
	credential: &'a rivet_guard_core::auth::Credential,
	/// Actual namespace from an actor read, or the namespace of this keyed creation.
	authorization_namespace: Option<String>,
	wait: StartWait,
	last_phase: &'static str,
}

impl Acquire<'_> {
	async fn run(&mut self) -> Result<(u32, universalpubsub::DuplexStream)> {
		let ctx = self.ctx;
		let clock = ctx.env().clock();
		let mut stream_retry_delay = STREAM_RETRY_INITIAL_DELAY;
		let mut next = match self.create.is_some() {
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
						next = if matches!(
							self.credential,
							rivet_guard_core::auth::Credential::Jwt(_)
						) && self.authorization_namespace.is_none()
						{
							self.wait.wait(ctx).await?
						} else {
							self.not_running().await?
						};
						continue;
					}
					Read::Gone => {
						self.last_phase = "gone";
						next = self.create_or_not_found().await?;
						continue;
					}
				},
			};
			authorize_read(
				self.credential,
				self.ctx.env().clock().observe()?.unix_ms,
				&actor.namespace,
				self.actor_id,
			)?;
			self.authorization_namespace = Some(actor.namespace.to_string());
			last_actor = Some(actor.clone());
			self.last_phase = phase_name(&phase);

			match phase {
				wire::ActorPhase::Initializing(_) | wire::ActorPhase::Sleeping => {
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
					match envoy_check(ctx, &actor, &allocation).await? {
						EnvoyCheck::Newer(newer) => match self.open(&newer).await {
							Ok(stream) => {
								// The allocation's route is stale for the rest of the epoch
								// unless someone records what this request learned.
								repair_route(ctx, self.actor_id, epoch, &allocation, newer);
								return Ok((epoch, stream));
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
				wire::ActorPhase::Destroying(_) | wire::ActorPhase::Destroyed => {
					next = self.create_or_not_found().await?;
				}
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

	/// Notices carry no namespace, so a JWT can route only after a read or keyed creation
	/// established its scope. Fresh expiry checks also cover opens resumed from a notice.
	fn authorize_route(&self) -> Result<()> {
		match self.credential {
			rivet_guard_core::auth::Credential::Admin => Ok(()),
			rivet_guard_core::auth::Credential::Jwt(_) => {
				let namespace = self
					.authorization_namespace
					.as_deref()
					.ok_or_else(|| rivet_guard_core::auth::InsufficientPermissions.build())?;
				authorize_read(
					self.credential,
					self.ctx.env().clock().observe()?.unix_ms,
					namespace,
					self.actor_id,
				)
			}
		}
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

	async fn read(&self) -> Result<Read> {
		let response = self
			.ctx
			.depot()
			.command::<ActorRead>(&ActorVolume::new(self.actor_id).volume_key(), ())
			.await;
		let response = match response {
			Ok(response) => response,
			Err(error) if is_volume_gone(&error) => return Ok(Read::Gone),
			Err(error) => return Err(error),
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

	/// Creates the actor with this request's waiter and returns what the answer settled.
	async fn create(&mut self) -> Result<Next> {
		let Some(create) = &self.create else {
			return Err(ActorNotFound.build());
		};
		create.authorize(
			self.credential,
			self.ctx.env().clock().observe()?.unix_ms,
			self.actor_id,
		)?;
		self.authorization_namespace = Some(create.namespace.clone());
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
				&ActorVolume::new(self.actor_id).volume_key(),
				wire::ActorCreateRequest {
					init: wire::ActorCreateData {
						actor_id: self.actor_id.as_bytes().into(),
						namespace: create.namespace.clone().into(),
						pool_name: create.pool_name.clone().into(),
						name: create.name.clone().into(),
						key: Some(create.key.clone().into()),
						input: input.map(Into::into),
					},
					waiter: waiter.clone(),
				},
			)
			.await?;
		let settled = self
			.wait
			.on_outcome(waiter.as_ref(), created.outcome)
			.await?;
		Ok(settled.unwrap_or(Next::Wait))
	}

	/// A destroyed actor is created again by a request that may create, and is not found by
	/// one that may not.
	async fn create_or_not_found(&mut self) -> Result<Next> {
		if self.create.is_none() {
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
		match envoy_check(self.ctx, actor, allocation).await? {
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
			self.actor_id,
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
		wire::ActorPhase::Sleeping => "sleeping",
		wire::ActorPhase::Destroying(_) => "destroying",
		wire::ActorPhase::Destroyed => "destroyed",
	}
}

/// What one fresh read of an envoy's record, and its worker's lease, says about the envoy an
/// actor was allocated to.
enum EnvoyCheck {
	/// The envoy serves on the allocation's connection, or is disconnected for now; wait.
	Serving,
	/// The same envoy process reconnected and kept its actors; open its newer connection.
	Newer(Route),
	/// The envoy is gone, so no execution there survives.
	Lost,
	/// The envoy is draining.
	Draining,
}

/// Reads the envoy's record once and judges it against `allocation`. Nothing is cached: each
/// check is one durable read, so callers run it only after a failed open or on a fallback poll.
async fn envoy_check(
	ctx: &EngineContext,
	actor: &wire::ActorData,
	allocation: &wire::ActorAllocation,
) -> Result<EnvoyCheck> {
	let route = Route::of_allocation(allocation)?;
	let volume = EnvoyVolume::new(
		actor.namespace.to_string(),
		actor.pool_name.to_string(),
		allocation.envoy_key.to_string(),
	)
	.volume_key();
	let envoy = match ctx.depot().command::<EnvoyRead>(&volume, ()).await {
		Ok(response) => response.data,
		Err(error) if is_volume_gone(&error) => None,
		Err(error) => return Err(error),
	};
	let Some(envoy) = envoy else {
		return Ok(EnvoyCheck::Lost);
	};
	if envoy.stopping {
		return Ok(EnvoyCheck::Draining);
	}

	let now = ctx.env().clock().observe()?.unix_ms;
	let heartbeat_expired =
		envoy.heartbeat_unix_ms + ctx.config().pegboard().envoy_lost_threshold() < now;
	if !envoy.connected && heartbeat_expired {
		return Ok(EnvoyCheck::Lost);
	}

	let current = Route::of_envoy(&envoy)?;
	let lease_ended = match ctx.topology().worker_lease(&route.worker).await {
		rivet_topology::WorkerLease::Expired | rivet_topology::WorkerLease::Released => true,
		rivet_topology::WorkerLease::Live | rivet_topology::WorkerLease::Unknown => false,
	};
	// An ended worker lease only closes the envoy's connection, and the dead worker may have
	// left the record marked connected. The envoy keeps its actors until its own lost threshold,
	// which it learned from this config, so it may still reconnect with them.
	if lease_ended && heartbeat_expired && current.connection_id == route.connection_id {
		return Ok(EnvoyCheck::Lost);
	}
	if envoy.connected && current.connection_id != route.connection_id {
		return Ok(EnvoyCheck::Newer(current));
	}
	Ok(EnvoyCheck::Serving)
}

/// Records the envoy's newer route on the actor's allocation, so later requests of the epoch
/// open it first. Best effort and off the request's path: the state machine applies it only if
/// the allocation still equals `from`, and a failure costs the next request one more fallback.
fn repair_route(
	ctx: &EngineContext,
	actor_id: Id,
	epoch: u32,
	from: &wire::ActorAllocation,
	newer: Route,
) {
	let event = wire::ActorEvent::UpdateRoute(wire::UpdateRoute {
		epoch,
		from: from.clone(),
		to: newer.into_allocation(from),
	});
	let ctx2 = ctx.clone();
	let _detached = ctx.env().spawn(Box::pin(async move {
		if let Err(error) = crate::start_wait::send_event(&ctx2, actor_id, event).await {
			tracing::debug!(%actor_id, ?error, "actor route repair failed; the next request falls back again");
		}
	}));
}


