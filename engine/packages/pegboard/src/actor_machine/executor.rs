use crate::actor_machine::{ActorStateMachine, Effect, Event, State, WriteEffect};
use crate::commands::EnvoyRead;
use crate::index;
use crate::keys::{
	self,
	ups::{ActorStartNoticeSubject, EnvoySubject},
};
use crate::wire;
use anyhow::{Context, Result, ensure};
use bytes::Bytes;
use depot_worker::command::{KvHandle, Transaction};
use rivet_engine_context::EngineContext;
use rivet_envoy_protocol as p;
use rivet_keys::keys::{KeyAddress, codec::FormalKey};
use rivet_state_machine::EffectContext;
use rivet_state_machine_durable::RuntimeEvent;
use rivet_util::Id;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;
use uuid::Uuid;
use vbare::Versioned;
use xxhash_rust::xxh3::Xxh3;

/// The acknowledgement an envoy connection sends after queueing routed commands on its socket.
pub const COMMANDS_ACK: &[u8] = b"ok";
/// The wait before an effect's first retry.
const EFFECT_BACKOFF_BASE_MS: u64 = 250;
/// The longest wait between two attempts of one effect.
const EFFECT_BACKOFF_MAX_MS: u64 = 30_000;
/// How long envoy selection may take before the event fails. Every event of the actor waits
/// behind it, so it is short; the caller retries.
const SELECT_TIMEOUT: Duration = Duration::from_secs(5);
/// How far past its lost threshold a disconnected envoy must be before it is judged gone. It
/// covers the delay between the engine's close and the envoy noticing it, and clock skew
/// between the worker that stamped the disconnect and the one judging it.
const ENVOY_GONE_MARGIN_MS: i64 = 5_000;

#[derive(Clone)]
pub struct Executor {
	ctx: EngineContext,
	db: KvHandle,
	actor_id: Id,
}

impl Executor {
	pub fn new(ctx: &EngineContext, db: KvHandle, actor_id: Id) -> Self {
		Executor {
			ctx: ctx.clone(),
			db,
			actor_id,
		}
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
}

impl rivet_state_machine_durable::Executor<ActorStateMachine> for Executor {
	/// Selects the envoy for a start `event` causes, as `needs_allocation` names them, reading
	/// the pool from the create's own data or the actor's data record.
	fn prepare<'a>(
		&'a self,
		state: &'a State,
		event: &'a RuntimeEvent<Event, wire::ActorTimer>,
	) -> Pin<Box<dyn Future<Output = Result<Option<wire::ActorAllocation>>> + Send + 'a>> {
		Box::pin(async move {
			if !crate::actor_machine::needs_allocation(&state.0, event) {
				return Ok(None);
			}

			let (actor_id, namespace, pool_name, name) = match event {
				RuntimeEvent::Input(Event(wire::ActorEvent::Create(create))) => (
					create.init.actor_id.clone(),
					create.init.namespace.clone(),
					create.init.pool_name.clone(),
					create.init.name.clone(),
				),
				_ => {
					let data = self.read_data().await?;
					(data.actor_id, data.namespace, data.pool_name, data.name)
				}
			};

			rivet_env::timeout(
				&*self.ctx.env().clock(),
				SELECT_TIMEOUT,
				select_allocation(&self.ctx, &actor_id, &namespace, &pool_name, &name),
			)
			.await
			.context("envoy selection timed out")?
		})
	}

	/// Puts the actor's data record where every later effect reads it, in the create's own
	/// transaction.
	fn write<'a>(
		&'a self,
		effect: WriteEffect,
		tx: &'a mut Transaction,
	) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
		Box::pin(async move {
			match effect {
				WriteEffect::WriteData { data } => {
					let data_key = keys::kv::ActorDataKey::new();
					tx.put(&data_key.pack(), &data_key.serialize(data)?)?;
					Ok(())
				}
			}
		})
	}

	fn execute(&self, effect: Effect, ctx: EffectContext<()>) {
		match effect {
			Effect::ReserveEpoxy {
				namespace,
				name,
				key,
				attempt,
				create_unix_ms,
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

						match crate::actor_key::reserve(&self2.ctx, namespace, name, key).await? {
							crate::actor_key::ReserveResult::Reserved { .. } => {
								anyhow::Ok(wire::ReserveEpoxyPhase::ReserveEpoxyPhaseReserved)
							}
							crate::actor_key::ReserveResult::AlreadyReserved { reservation_id } => {
								anyhow::Ok(
									wire::ReserveEpoxyPhase::ReserveEpoxyPhaseAlreadyReserved(
										wire::ReserveEpoxyPhaseAlreadyReserved {
											reservation_id: reservation_id.as_bytes().into(),
										},
									),
								)
							}
						}
					}
					.await;

					let res = async {
						let phase = match res {
							Ok(phase) => phase,
							Err(err) => {
								tracing::error!(?err, "failed to reserve epoxy");
								wire::ReserveEpoxyPhase::ReserveEpoxyPhaseFailed
							}
						};

						self2
							.ctx
							.depot()
							.command::<crate::commands::ActorEvent>(
								&keys::partition::ActorVolume::new(self2.actor_id).volume_key(),
								wire::ActorEvent::ReserveEpoxyState(wire::ReserveEpoxyState {
									phase,
									attempt,
									create_unix_ms,
								}),
							)
							.await?;

						anyhow::Ok(())
					}
					.await;

					if let Err(err) = res {
						tracing::error!(?err, "send actor event failed");
					}
				}));
			}
			Effect::WriteIndex {
				namespace,
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

						self2
							.ctx
							.depot()
							.command::<crate::index::IndexInsertActor>(
								&keys::partition::ActorIndexVolume::for_actor(
									namespace,
									name,
									self2.actor_id,
									index_bucket_count,
								)
								.volume_key(),
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
								tracing::error!(?err, "failed to write index");
								wire::WriteIndexPhase::WriteIndexPhaseFailed
							}
						};

						self2
							.ctx
							.depot()
							.command::<crate::commands::ActorEvent>(
								&keys::partition::ActorVolume::new(self2.actor_id).volume_key(),
								wire::ActorEvent::WriteIndexState(wire::WriteIndexState {
									phase,
									attempt,
									create_unix_ms,
								}),
							)
							.await?;

						anyhow::Ok(())
					}
					.await;

					if let Err(err) = res {
						tracing::error!(?err, "send actor event failed");
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

						let payload =
							Bytes::from(p::versioned::ToEnvoy::serialize_with_embedded_version(
								&p::ToEnvoy::ToEnvoyCommands(vec![p::CommandWrapper {
									checkpoint: p::ActorCheckpoint {
										actor_id: self2.actor_id.to_string().into(),
										generation: epoch,
										index: command_index,
									},
									inner: p::Command::CommandStopActor(p::CommandStopActor {
										reason,
									}),
								}]),
								p::PROTOCOL_VERSION,
							)?);

						// The allocation's route is the common case and costs no read. Only a
						// transport failure there is worth the envoy record: the envoy may have
						// reconnected on a newer connection, possibly to another worker.
						let (worker_id, connection_id) = route_of(&allocation)?;
						let first =
							send_commands(&self2.ctx, &worker_id, connection_id, &payload).await;
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

						let envoy_volume = keys::partition::EnvoyVolume::new(
							data.namespace.to_string(),
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
							let (worker_id, connection_id) = route_of_envoy(envoy)?;
							let second =
								send_commands(&self2.ctx, &worker_id, connection_id, &payload)
									.await;
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
							.ctx
							.depot()
							.command::<crate::commands::ActorEvent>(
								&keys::partition::ActorVolume::new(self2.actor_id).volume_key(),
								wire::ActorEvent::EnvoyGone(wire::EnvoyGone { epoch }),
							)
							.await?;
						anyhow::Ok(())
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
				let self2 = self.clone();
				ctx.spawn(Box::pin(async move {
					let preloaded_kv = if new_storage {
						Some(p::preloaded_kv::empty_keyspace())
					} else {
						None
					};
					let sent = async {
						let data = self2.read_data().await?;
						let (worker_id, connection_id) = route_of(&allocation)?;

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
									}),
								}]),
								p::PROTOCOL_VERSION,
							)?);

						send_commands(&self2.ctx, &worker_id, connection_id, &payload).await
					}
					.await;

					// A failed send may still have been delivered, so the state machine stops
					// the allocation before it moves on. A failed report is left to the start
					// timer, which takes the same path.
					let Err(err) = sent else {
						return;
					};
					tracing::warn!(
						?err,
						"start command send failed; reporting the start failed"
					);
					let reported = self2
						.ctx
						.depot()
						.command::<crate::commands::ActorEvent>(
							&keys::partition::ActorVolume::new(self2.actor_id).volume_key(),
							wire::ActorEvent::StartFailed(wire::StartFailed { epoch }),
						)
						.await;
					if let Err(err) = reported {
						tracing::error!(
							?err,
							"could not report a failed start; the start timer covers it"
						);
					}
				}));
			}
			Effect::Destroy {
				attempt,
				index_bucket_count,
				create_unix_ms,
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
						// namespace and name are read, and those never change for a key. The
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

						self2
							.ctx
							.depot()
							.command::<crate::index::IndexDeleteActor>(
								&keys::partition::ActorIndexVolume::for_actor(
									data.namespace.to_string(),
									data.name.to_string(),
									self2.actor_id,
									index_bucket_count,
								)
								.volume_key(),
								wire::IndexDeleteActorRequest {
									actor_id: self2.actor_id.as_bytes().into(),
									create_unix_ms,
								},
							)
							.await?;

						self2
							.ctx
							.depot()
							.delete_current(
								keys::partition::ActorDatabaseVolume::new(
									self2.actor_id,
									create_unix_ms,
								)
								.volume_key(),
							)
							.await?;

						anyhow::Ok(())
					}
					.await;

					let res = async {
						let phase = match res {
							Ok(()) => wire::DestroyPhase::DestroyPhaseSuccess,
							Err(err) => {
								tracing::error!(?err, "failed to destroy");
								wire::DestroyPhase::DestroyPhaseFailed
							}
						};

						self2
							.ctx
							.depot()
							.command::<crate::commands::ActorEvent>(
								&keys::partition::ActorVolume::new(self2.actor_id).volume_key(),
								wire::ActorEvent::DestroyState(wire::DestroyState {
									phase,
									attempt,
									create_unix_ms,
								}),
							)
							.await?;

						anyhow::Ok(())
					}
					.await;

					if let Err(err) = res {
						tracing::error!(?err, "send actor event failed");
					}
				}));
			}
		}
	}
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
	let payload = rivet_data::versioned::ActorStartNotice::serialize_with_embedded_version(
		&notice,
		rivet_data::generated::PEGBOARD_ACTOR_VERSION,
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
async fn send_commands(
	ctx: &EngineContext,
	worker_id: &universalpubsub::WorkerId,
	connection_id: Uuid,
	payload: &Bytes,
) -> Result<()> {
	let ack = ctx
		.ups()?
		.request(
			universalpubsub::RequestClass::EnvoyCommand,
			worker_id,
			EnvoySubject::new(connection_id),
			payload,
		)
		.await?;
	ensure!(ack.as_ref() == COMMANDS_ACK, "unexpected envoy ack");
	Ok(())
}

/// The UPS route an allocation records for its envoy.
fn route_of(allocation: &wire::ActorAllocation) -> Result<(universalpubsub::WorkerId, Uuid)> {
	parse_route(&allocation.worker_id, &allocation.connection_id)
}

/// The UPS route an envoy record names for its current connection.
fn route_of_envoy(envoy: &wire::EnvoyData) -> Result<(universalpubsub::WorkerId, Uuid)> {
	parse_route(&envoy.worker_id, &envoy.connection_id)
}

fn parse_route(
	worker_id: &[u8],
	connection_id: &[u8],
) -> Result<(universalpubsub::WorkerId, Uuid)> {
	let worker_id = universalpubsub::WorkerId::from_bytes(
		worker_id.try_into().context("invalid envoy worker id")?,
	);
	let connection_id = Uuid::from_slice(connection_id).context("invalid envoy connection id")?;
	Ok((worker_id, connection_id))
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
	lost_threshold_ms: i64,
) -> bool {
	let Some(envoy) = envoy else {
		return true;
	};
	if envoy.connected {
		return false;
	}
	let gone_after_unix_ms = envoy
		.heartbeat_unix_ms
		.saturating_add(lost_threshold_ms.max(0))
		.saturating_add(ENVOY_GONE_MARGIN_MS);
	read_unix_ms > gone_after_unix_ms
}

/// Whether a UPS request failed because the worker that answered has no consumer for the
/// subject: for an envoy connection's subject, the connection's task has ended and its socket
/// is closed.
fn no_responders(error: &anyhow::Error) -> bool {
	let structured = rivet_error::RivetError::extract(error);
	structured.group() == "ups" && structured.code() == "no_responders"
}

/// Whether the connection a connected record names has ended, given `error`, its refusal of
/// the stop: its worker answered that it has no consumer for the connection, or the worker
/// could not be reached and its topology lease expired or was released. A worker that answered
/// in any other way is alive and may still serve the connection.
///
/// An ended lease counts only because a worker whose lease ended no longer serves envoy
/// connections: a released worker closed them first, and an expired one is assumed dead.
/// pegboard-envoy does not yet close its connections when its lease fences, so an expired
/// worker that is still alive breaks this assumption, as it does the gateway's lost check.
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
	let (worker_id, _) = route_of_envoy(envoy)?;
	let ended = match ctx.topology().worker_lease(&worker_id).await {
		rivet_topology::WorkerLease::Expired | rivet_topology::WorkerLease::Released => true,
		rivet_topology::WorkerLease::Live | rivet_topology::WorkerLease::Unknown => false,
	};
	Ok(ended)
}

/// Whether a UPS request failed without the worker answering: its route is unknown, the
/// transport failed, or the request timed out.
fn unreachable_worker(error: &anyhow::Error) -> bool {
	let structured = rivet_error::RivetError::extract(error);
	structured.group() == "ups"
		&& matches!(
			structured.code(),
			"unknown_worker" | "transport" | "timeout"
		)
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

/// Whether a failed UPS request to an envoy connection may succeed on another route: a stale
/// route, a missing worker or a transport failure, as opposed to a refused or malformed request.
pub fn retryable_ups_error(error: &anyhow::Error) -> bool {
	let structured = rivet_error::RivetError::extract(error);
	structured.group() == "ups"
		&& matches!(
			structured.code(),
			"no_responders"
				| "unknown_worker"
				| "wrong_worker"
				| "timeout" | "consumer_closed"
				| "transport"
		)
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

pub(crate) fn effect_backoff(attempt: u32) -> u64 {
	let Some(retry) = attempt.checked_sub(2) else {
		return 0;
	};

	let multiplier = 1u64.checked_shl(retry).unwrap_or(u64::MAX);
	EFFECT_BACKOFF_BASE_MS
		.saturating_mul(multiplier)
		.min(EFFECT_BACKOFF_MAX_MS)
}
