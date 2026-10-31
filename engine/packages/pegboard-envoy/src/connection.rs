use anyhow::{Context, Result, bail, ensure};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use pegboard::{
	commands::EnvoyRead,
	epoch::{wrapping_ge, wrapping_gt, wrapping_le},
	index::{IndexDeleteActiveEnvoy, IndexInsertEnvoy, IndexUpsertActiveEnvoy},
	keys::{
		partition::ActiveEnvoyIndexVolume, partition::ActorVolume, partition::EnvoyIndexVolume,
		partition::EnvoyVolume, ups::EnvoySubject, ups::EnvoySupersedeSubject,
	},
	wire,
};
use rivet_engine_context::EngineContext;
use rivet_env::{
	TaskScope,
	capabilities::websocket::{CloseFrame, Message, WebSocket},
};
use rivet_envoy_protocol as p;
use rivetkit_shared_types::actor_key;
use std::collections::HashSet;
use std::sync::{
	Arc,
	atomic::{AtomicI64, AtomicU64, Ordering},
};
use tokio::sync::{Mutex, mpsc};
use universalpubsub::{RequestClass, Turn, Turns};

use uuid::Uuid;
use vbare::Versioned;

struct Connection {
	ctx: EngineContext,
	sqlite: crate::sqlite::Sessions,
	envoy_key: String,
	/// The namespace and pool whose indexes hold this envoy.
	scope: crate::routes::EnvoyScope,
	/// This connection's record. Its writes are serialized by this lock.
	record: Mutex<crate::envoy_record::RecordState>,
	/// Configuration captured once when this connection is admitted.
	pool_config: std::sync::OnceLock<wire::NodePoolConfig>,
	/// The heartbeat this connection last wrote to the active envoy index, or 0 before the
	/// first. Allocation picks envoys from that index, so it is kept inside the heartbeat
	/// window while the envoy answers pings and removed when it stops or disconnects.
	last_heartbeat_unix_ms: AtomicI64,
	to_envoy_tx: crate::queue::Sender<ToEnvoyFrame>,
	/// A failed event lane ends the connection so its unacknowledged checkpoints replay.
	/// Separate from the socket queue: backpressure must not prevent this close.
	event_failure: mpsc::Sender<anyhow::Error>,
	/// Shared by every queue on this connection so its near-capacity warning is rate-limited
	/// once per connection.
	queue_pressure: Arc<crate::queue::QueuePressure>,
	/// Unix time of the last pong, or of the connection's start before the first one.
	last_pong_unix_ms: AtomicI64,
	/// Boot time of the last pong, or of the connection's start before the first one. The pong
	/// watchdog uses this monotonic clock, so a wall-clock step cannot move its deadline.
	last_pong_boot_ms: AtomicU64,
	/// The latest ping to send. The socket writer sends it ahead of queued frames, so a ping
	/// never waits for a slot in the socket queue and a busy but reading envoy still answers.
	ping: tokio::sync::watch::Sender<i64>,
	gateway_handles: scc::HashMap<([u8; 4], [u8; 4]), Arc<GatewayHandle>>,
	actor_handles: scc::HashMap<String, Arc<ActorHandle>>,
	actor_handle_budget: crate::admission::ActorHandleBudget,
	/// This process's lost checks for closed connections.
	lost_checks: Arc<crate::lost_checks::LostChecks>,
	/// Generations a closed connection of this envoy hosted, taken over when this connection
	/// cancelled that connection's lost check. They join this connection's own check at close.
	inherited: Mutex<Vec<crate::lost_checks::HostedActor>>,
	connection_id: Uuid,
	version: u16,
}

pub async fn run(
	ctx: &EngineContext,
	socket: WebSocket,
	envoy_key: String,
	scope: crate::routes::EnvoyScope,
	version: u16,
	service: crate::Service,
	connection_id: Option<Uuid>,
) -> Result<()> {
	tracing::debug!(%envoy_key, namespace = %scope.namespace, pool_name = %scope.pool_name, "envoy connecting");
	let stored = match ctx
		.depot()
		.command::<EnvoyRead>(&envoy_volume(&scope, &envoy_key), ())
		.await
	{
		Ok(response) => response.data,
		Err(error) if pegboard::commands::is_volume_gone(&error) => None,
		Err(error) => {
			// Close with a reason instead of dropping the socket, so the envoy can tell a
			// storage outage from a network fault and retry after the hint.
			let mut socket = socket;
			let rejection = crate::rejection::Rejection::connect(
				error,
				ctx.config().pegboard().envoy_connect_retry_after_ms(),
			);
			log_connect_rejection(&rejection);
			if let Err(error) = socket.send(Message::Close(Some(rejection.frame))).await {
				tracing::debug!(?error, "failed to send envoy connect rejection");
			}
			return Err(rejection.error);
		}
	};

	let connection_id =
		connection_id.unwrap_or_else(|| ctx.env().rng("envoy.connection_id").uuid_v4());
	let started = ctx.env().clock().observe()?;
	let now = started.unix_ms;
	let record = Mutex::new(crate::envoy_record::RecordState::new(
		stored,
		envoy_key.clone(),
		ctx.ups()?.worker_id().as_bytes().to_vec(),
		connection_id.as_bytes().to_vec(),
		now,
	));

	let queue_pressure = Arc::new(crate::queue::QueuePressure::new(ctx.env().clock()));
	let (to_envoy_tx, mut to_envoy_rx) = crate::queue::channel(
		crate::queue::QueueKind::Socket,
		ctx.config().pegboard().envoy_socket_queue_capacity(),
		queue_pressure.clone(),
	);
	let (mut socket_tx, mut socket_rx) = socket.split();
	let (event_failure, mut event_failures) = mpsc::channel(1);
	let (ping, mut ping_rx) = tokio::sync::watch::channel(0);

	let actor_handle_budget = crate::admission::ActorHandleBudget::new(
		ctx.config().pegboard().envoy_actor_handles_capacity(),
		queue_pressure.clone(),
	);
	let conn = Arc::new(Connection {
		ctx: ctx.clone(),
		sqlite: crate::sqlite::Sessions::new(
			ctx.config().pegboard().sqlite_sessions(),
			crate::sqlite::RemoteLimits {
				idle_ms: ctx.config().pegboard().sqlite_remote_idle_ms(),
				max_response_bytes: ctx.config().pegboard().sqlite_remote_max_response_bytes(),
				pool: service.sqlite_pool.clone(),
			},
			{
				let config = ctx.config().clone();
				Arc::new(move || config.dynamic().pegboard().sqlite_server_prefetch())
			},
		)?,
		envoy_key,
		scope,
		record,
		pool_config: Default::default(),
		last_heartbeat_unix_ms: AtomicI64::new(0),
		to_envoy_tx,
		event_failure,
		queue_pressure,
		last_pong_unix_ms: AtomicI64::new(now),
		last_pong_boot_ms: AtomicU64::new(started.boot_ms),
		ping,
		gateway_handles: scc::HashMap::new(),
		actor_handles: scc::HashMap::new(),
		actor_handle_budget,
		lost_checks: service.lost_checks.clone(),
		inherited: Mutex::new(Vec::new()),
		connection_id,
		version,
	});

	let tasks = Arc::new(ctx.env().task_scope());

	let mut incoming_sub = ctx
		.ups()?
		.subscribe_stream(EnvoySubject::new(connection_id))?;
	let ctx2 = ctx.clone();
	let conn2 = conn.clone();
	let tasks2 = tasks.clone();
	let mut shutdown = service.stopping.subscribe();
	let incoming_sub_task = tasks.spawn_result(async move {
		let http = rivet_engine_context::ingress::IngressDrain::default();
		let mut draining = false;
		// Spawn a new task for every gateway stream opened, as well as any other msgs sent to this envoy
		loop {
			let next = if draining {
				incoming_sub.next().await?
			} else {
				rivet_env::select_biased!("withdraw runner admission before draining accepted streams",
					_ = shutdown.wait_for(|stopping| *stopping) => {
						incoming_sub.close();
						draining = true;
						continue;
					},
					next = incoming_sub.next() => next?,
				)
			};
			let Some(stream) = next else {
				break;
			};
			let ctx3 = ctx2.clone();
			let tasks3 = tasks2.clone();
			let conn3 = conn2.clone();

			// Commands take their actors' turns here, in the order UPS delivered them, before
			// any task runs. Their input is complete on arrival, so reading it never waits on
			// the network.
			let class = stream.class();
			let (stream_tx, mut stream_rx) = stream.split();
			let arrival = if class == RequestClass::EnvoyCommand {
				match command_arrival(&conn2, &tasks2, &mut stream_rx).await {
					Ok(arrival) => Some(arrival),
					Err(err) => {
						tracing::warn!(
							?err,
							"dropping an unreadable command stream; the sender times out and retries"
						);
						continue;
					}
				}
			} else {
				None
			};

			// Reserve before spawn, including handlers not yet polled at shutdown.
			let http_owner = if class == RequestClass::GatewayHttp {
				http.enter()
			} else {
				None
			};
			tasks2.spawn(async move {
				let _http_owner = http_owner;
				let result =
					handle_sub_stream(ctx3, conn3, tasks3, class, stream_tx, stream_rx, arrival)
						.await;
				let end = SubStreamEnd::classify(&result);

				// Display formatting keeps the expected close free of a backtrace.
				if let Err(err) = result {
					match end {
						SubStreamEnd::Cancelled => {
							tracing::debug!(%err, "gateway cancelled sub stream");
						}
						SubStreamEnd::Finished | SubStreamEnd::Failed => {
							tracing::error!(?err, "failed handling sub stream");
						}
					}
				}
			});
		}

		if draining {
			http.drain().await;
			tracing::info!(
				"accepted HTTP requests drained; closing runner connection for shutdown"
			);
			let clock = conn2.ctx.env().clock();
			rivet_env::timeout(
				clock.as_ref(),
				SHUTDOWN_CLOSE_QUEUE_TIMEOUT,
				conn2
					.to_envoy_tx
					.send(ToEnvoyFrame::Close(shutdown_close())),
			)
			.await
			.context("runner socket queue stayed full during shutdown")??;
			// Only writer/reader completion may end the connection. Returning here
			// would cancel the writer before it sent the queued Close frame.
			std::future::pending::<()>().await;
		}
		anyhow::Ok(())
	});

	let socket_writer_task = tasks.spawn_result(async move {
		write_socket(&mut socket_tx, &mut to_envoy_rx, &mut ping_rx, version).await
	});

	let conn2 = conn.clone();
	let ctx2 = ctx.clone();
	let tasks2 = tasks.clone();
	let socket_reader_task = tasks.spawn_result(async move {
		// Set once a message is rejected. The socket writer then closes the connection, and
		// messages that arrive before it does are ignored.
		let mut rejected = false;
		while let Some(message) = socket_rx.next().await {
			match message? {
				Message::Binary(_) if rejected => {}
				Message::Binary(payload) => {
					// The envoy speaks the connection's version without a header. At the latest
					// version the message is viewed in place, and tunnel replies are forwarded
					// to their gateway without being decoded.
					let decoded = if conn2.version == p::PROTOCOL_VERSION {
						match p::versioned::ToRivet::open_body(payload, conn2.version) {
							Ok(view) => {
								if let p::ToRivetRef::ToRivetTunnelMessage(message) = view.get() {
									let message_id = message.message_id();
									let message_key =
										(*message_id.gateway_id(), *message_id.request_id());
									forward_tunnel_reply(
										&conn2,
										message_key,
										TunnelReply::Encoded(view),
									)
									.await?;
									continue;
								}
								Ok(view.to_owned())
							}
							Err(error) => Err(error),
						}
					} else {
						p::versioned::ToRivet::decode_body(payload, conn2.version)
					};

					let msg = match decoded {
						Ok(msg) => msg,
						Err(error) => {
							let rejection = crate::rejection::Rejection::decode(error);
							tracing::debug!(
								code = rejection.frame.code,
								reason = %rejection.frame.reason,
								"rejecting undecodable envoy message"
							);
							conn2
								.to_envoy_tx
								.send(ToEnvoyFrame::Reject(rejection))
								.await
								.ok()
								.context("envoy socket writer closed before rejecting a message")?;
							rejected = true;
							continue;
						}
					};
					if let Some(rejection) = handle_ws_msg(&ctx2, &conn2, &tasks2, msg).await? {
						log_connect_rejection(&rejection);
						conn2
							.to_envoy_tx
							.try_send(ToEnvoyFrame::Reject(rejection))
							.ok()
							.context("envoy socket writer closed before rejecting a connect")?;
						rejected = true;
					}
				}
				Message::Close(_) => break,
				_ => {}
			}
		}

		anyhow::Ok(())
	});

	// A newer connection of this envoy asks this one to close, so commands sent to this connection
	// are not acknowledged onto a lane nobody reads.
	let mut supersede_sub = ctx
		.ups()?
		.subscribe_request(EnvoySupersedeSubject::new(connection_id))?;
	let supersede_task = tasks.spawn_result(async move {
		if let Some(request) = supersede_sub.next().await? {
			let _ = request.reply(&[]);
			tracing::info!("closing envoy connection superseded by a newer connection");
			return Err(crate::envoy_record::Superseded.into());
		}
		anyhow::Ok(())
	});

	// Envoys only report liveness in reply to pings. A connection without a pong for the ping
	// timeout is closed, which drops its subscription so senders get no responders. The
	// watchdog waits only on the clock, so an envoy that stopped reading cannot postpone its own
	// close by filling the socket queue. Each pong since the last tick advances the envoy's
	// heartbeat in the active index, off the socket reader, so a slow index write never delays
	// the envoy's messages.
	let conn2 = conn.clone();
	let ctx2 = ctx.clone();
	let pong_watchdog_task = tasks.spawn_result(async move {
		let clock = ctx2.env().clock();
		let ping_timeout_ms = ctx2.config().pegboard().envoy_ping_timeout();
		pong_watchdog(clock.as_ref(), &conn2.last_pong_boot_ms, ping_timeout_ms).await
	});
	let conn2 = conn.clone();
	let ctx2 = ctx.clone();
	let ping_task = tasks.spawn_result(async move {
		let clock = ctx2.env().clock();
		let period_ms = ctx2.config().pegboard().envoy_update_ping_interval();
		let mut interval = rivet_env::interval(clock.clone(), period_ms);
		loop {
			interval.tick().await?;
			let ts = clock.observe()?.unix_ms;
			conn2.ping.send_replace(ts);

			let last_pong_unix_ms = conn2.last_pong_unix_ms.load(Ordering::Acquire);
			if last_pong_unix_ms > conn2.last_heartbeat_unix_ms.load(Ordering::Acquire) {
				match write_heartbeat(&ctx2, &conn2, last_pong_unix_ms).await {
					Ok(()) => {}
					Err(error) if error.is::<crate::envoy_record::Superseded>() => {
						tracing::info!(
							"closing envoy connection whose active index entries belong to a newer connection"
						);
						return Err(error);
					}
					Err(error) if error.is::<crate::envoy_record::PoolAdmissionLost>() => {
						tracing::info!(
							"closing node connection whose pool admission expired; the runtime reconnects for fresh admission"
						);
						return Err(error);
					}
					Err(error) => {
						tracing::debug!(
							?error,
							"envoy heartbeat write failed; the next pong retries it"
						);
					}
				}
			}
		}
		#[allow(unreachable_code)]
		anyhow::Ok(())
	});

	// Remote SQL sessions expire on one deadline queue, owned and joined with the connection.
	let conn2 = conn.clone();
	let ctx2 = ctx.clone();
	let sqlite_expiry_task =
		tasks.spawn_result(async move { conn2.sqlite.run_expiry(ctx2.env().clone()).await });

	let periodic = async {
		rivet_env::select!( ctx.env().rng("envoy.periodic");
			res = pong_watchdog_task => res?.context("task cancelled")?,
			res = ping_task => res?.context("task cancelled")?,
			res = sqlite_expiry_task => res?.context("task cancelled")?,
		)
	};
	let control = async {
		rivet_env::select!( ctx.env().rng("envoy.control");
			res = supersede_task => res?.context("task cancelled")?,
			error = event_failures.recv() => Err(error.context("actor event failure channel closed")?),
		)
	};
	let background = async {
		rivet_env::select!( ctx.env().rng("envoy.background");
			res = incoming_sub_task => res?.context("task cancelled")?,
			res = control => res,
			res = periodic => res,
		)
	};
	let res = rivet_env::select!( ctx.env().rng("envoy.tasks");
		res = background => res,
		res = socket_writer_task => res?.context("task cancelled")?,
		res = socket_reader_task => res?.context("task cancelled")?,
	);

	tasks.shutdown().await;

	// Clear the record's connection within the shutdown budget. A superseded connection no
	// longer owns the record; a final failure is covered by readers' delivery rules.
	let superseded = res
		.as_ref()
		.err()
		.is_some_and(|error| error.is::<crate::envoy_record::Superseded>());
	if !superseded {
		disconnect(ctx, &conn).await;
		schedule_lost_check(ctx, &conn).await;
	}

	res
}

/// Schedules the lost check for the generations this connection hosted, so a quiet actor on an
/// envoy that never reconnects is not left running. A superseded connection needs none: a newer
/// connection of the envoy hosts its actors.
async fn schedule_lost_check(ctx: &EngineContext, conn: &Connection) {
	if !conn.record.lock().await.connected {
		return;
	}

	let mut actors = std::mem::take(&mut *conn.inherited.lock().await);
	let mut hosted = Vec::new();
	conn.actor_handles
		.iter_async(|actor_id, handle| {
			if let Some(generation) = handle.generation() {
				hosted.push(crate::lost_checks::HostedActor {
					actor_id: actor_id.clone(),
					generation,
				});
			}
			true
		})
		.await;
	// A generation this connection hosts replaces the one it inherited for the same actor.
	let hosted_ids: HashSet<&str> = hosted.iter().map(|actor| actor.actor_id.as_str()).collect();
	actors.retain(|inherited| !hosted_ids.contains(inherited.actor_id.as_str()));
	drop(hosted_ids);
	actors.extend(hosted);
	if actors.is_empty() {
		return;
	}

	let now_boot_ms = match ctx.env().clock().observe() {
		Ok(clocks) => clocks.boot_ms,
		Err(error) => {
			tracing::warn!(
				?error,
				"could not schedule the envoy lost check; its actors are found lost on their next request"
			);
			return;
		}
	};
	// The envoy keeps its actors for its lost threshold after the socket closes, so they are
	// not judged lost sooner.
	let delay_ms = u64::try_from(ctx.config().pegboard().envoy_lost_threshold()).unwrap_or(0);
	conn.lost_checks
		.schedule(
			lost_check_key(conn),
			crate::lost_checks::Check {
				connection_id: conn.connection_id.as_bytes().to_vec(),
				actors,
			},
			now_boot_ms,
			delay_ms,
		)
		.await;
}

/// The key of this connection's envoy in the lost-check queue.
fn lost_check_key(conn: &Connection) -> crate::lost_checks::EnvoyKey {
	(
		conn.scope.namespace.clone(),
		conn.scope.pool_name.clone(),
		conn.envoy_key.clone(),
	)
}

/// The volume of the envoy's record.
fn envoy_volume(scope: &crate::routes::EnvoyScope, envoy_key: &str) -> rivet_keys::keys::VolumeKey {
	EnvoyVolume::new(
		scope.namespace.clone(),
		scope.pool_name.clone(),
		envoy_key.to_owned(),
	)
	.volume_key()
}

/// Leaves the active envoy index, so allocation stops picking this envoy. Idempotent: the index
/// deletes whatever this connection's epoch wrote, keeps a newer connection's entries, and
/// treats an absent envoy as already gone.
async fn leave_active_index(ctx: &EngineContext, conn: &Connection) -> Result<()> {
	if conn.last_heartbeat_unix_ms.load(Ordering::Acquire) == 0 {
		return Ok(());
	}
	let connection_epoch = conn.record.lock().await.entry.connection_epoch;
	ctx.depot()
		.command::<IndexDeleteActiveEnvoy>(
			&ActiveEnvoyIndexVolume::new(
				conn.scope.namespace.clone(),
				conn.scope.pool_name.clone(),
			)
			.volume_key(),
			wire::IndexDeleteActiveEnvoyRequest {
				envoy_key: conn.envoy_key.clone().into(),
				connection_epoch,
			},
		)
		.await
}

/// Moves the envoy's active index entries to `heartbeat_unix_ms`, under the names it serves and
/// with this connection's route. A connection that is stopping or not yet connected writes
/// nothing: it must not come back into allocation. An index owned by a newer connection of the
/// envoy means this one is superseded and must close.
async fn write_heartbeat(
	ctx: &EngineContext,
	conn: &Connection,
	heartbeat_unix_ms: i64,
) -> Result<()> {
	let (connection_epoch, names) = {
		let record = conn.record.lock().await;
		if !record.connected {
			return Ok(());
		}
		register_pool_connection(ctx, conn, heartbeat_unix_ms, true).await?;
		if record.entry.stopping {
			conn.last_heartbeat_unix_ms
				.fetch_max(heartbeat_unix_ms, Ordering::AcqRel);
			return Ok(());
		}
		(record.entry.connection_epoch, record.entry.names.clone())
	};
	let response = ctx
		.depot()
		.command::<IndexUpsertActiveEnvoy>(
			&ActiveEnvoyIndexVolume::new(
				conn.scope.namespace.clone(),
				conn.scope.pool_name.clone(),
			)
			.volume_key(),
			wire::IndexUpsertActiveEnvoyRequest {
				envoy_key: conn.envoy_key.clone().into(),
				connection_epoch,
				worker_id: ctx.ups()?.worker_id().as_bytes().to_vec().into(),
				connection_id: conn.connection_id.as_bytes().to_vec().into(),
				heartbeat_unix_ms,
				names,
			},
		)
		.await?;
	if response.superseded {
		return Err(crate::envoy_record::Superseded.into());
	}
	conn.last_heartbeat_unix_ms
		.fetch_max(heartbeat_unix_ms, Ordering::AcqRel);
	Ok(())
}

/// Writes the disconnect and leaves the active index, retrying with a short backoff within a
/// fixed budget.
async fn disconnect(ctx: &EngineContext, conn: &Connection) {
	const ATTEMPTS: u32 = 4;
	let clock = ctx.env().clock();
	let mut backoff = std::time::Duration::from_millis(100);
	for attempt in 1..=ATTEMPTS {
		let result = async {
			pegboard::catalog::pool(
				ctx,
				&conn.scope.namespace,
				&conn.scope.pool_name,
				wire::PoolOperation::PoolDisconnect(conn.connection_id.as_bytes().to_vec().into()),
			)
			.await?;
			leave_active_index(ctx, conn).await?;
			write_record(ctx, conn, false, |entry| {
				entry.connected = false;
			})
			.await
		}
		.await;
		match result {
			Ok(()) => return,
			Err(error) if error.is::<crate::envoy_record::Superseded>() => return,
			Err(error) if attempt == ATTEMPTS => {
				tracing::warn!(
					?error,
					"envoy disconnect write failed; readers treat the record as disconnected once the worker's lease ends"
				);
				return;
			}
			Err(error) => {
				tracing::debug!(?error, attempt, "retrying envoy disconnect write");
			}
		}
		if rivet_env::sleep(clock.as_ref(), backoff).await.is_err() {
			return;
		}
		backoff *= 2;
	}
}

/// The request class a stream must be opened under, decided by its first message. Commands are
/// `EnvoyCommand`; a tunnel starts with `ToEnvoyRequestStart` (`GatewayHttp`) or
/// `ToEnvoyWebSocketOpen` (`GatewayWebSocket`). Any other first message is invalid.
fn expected_class(first: &p::ToEnvoy) -> Result<universalpubsub::RequestClass> {
	match first {
		p::ToEnvoy::ToEnvoyCommands(_) => Ok(universalpubsub::RequestClass::EnvoyCommand),
		p::ToEnvoy::ToEnvoyTunnelMessage(message) => match &message.message_kind {
			p::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(_) => {
				Ok(universalpubsub::RequestClass::GatewayHttp)
			}
			p::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(_) => {
				Ok(universalpubsub::RequestClass::GatewayWebSocket)
			}
			p::ToEnvoyTunnelMessageKind::ToEnvoyRequestChunk(_)
			| p::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort(_)
			| p::ToEnvoyTunnelMessageKind::ToEnvoyRequestBodyCancel
			| p::ToEnvoyTunnelMessageKind::ToEnvoyResponseBodyWindowUpdate(_)
			| p::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketMessage(_)
			| p::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(_) => {
				bail!("unexpected first tunnel message, unable to determine recipient")
			}
		},
		p::ToEnvoy::ToEnvoyInit(_)
		| p::ToEnvoy::ToEnvoyAckEvents(_)
		| p::ToEnvoy::ToEnvoyKvResponse(_)
		| p::ToEnvoy::ToEnvoyPing(_)
		| p::ToEnvoy::ToEnvoySqliteGetPagesResponse(_)
		| p::ToEnvoy::ToEnvoySqliteCommitResponse(_)
		| p::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(_)
		| p::ToEnvoy::ToEnvoySqliteCommitStageSegmentResponse(_)
		| p::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(_)
		| p::ToEnvoy::ToEnvoySqliteExecResponse(_)
		| p::ToEnvoy::ToEnvoySqliteExecuteResponse(_)
		| p::ToEnvoy::ToEnvoySqliteExecuteBatchResponse(_) => {
			bail!("unexpected first message on an envoy stream")
		}
	}
}

/// How one gateway or command stream on this connection ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SubStreamEnd {
	Finished,
	/// The peer reset the HTTP/2 stream with `CANCEL`. The gateway does this whenever its
	/// client request or WebSocket closes, so it is a normal close rather than a failure.
	Cancelled,
	Failed,
}

impl SubStreamEnd {
	fn classify(result: &Result<()>) -> Self {
		let Err(err) = result else {
			return SubStreamEnd::Finished;
		};

		let cancelled = err.chain().any(|cause| {
			cause
				.downcast_ref::<h2::Error>()
				.is_some_and(|h2_err| h2_err.reason() == Some(h2::Reason::CANCEL))
		});
		if cancelled {
			SubStreamEnd::Cancelled
		} else {
			SubStreamEnd::Failed
		}
	}
}

/// A held tunnel's task lives as long as its client socket, so every await other than the
/// loop's own select is boxed: inline, the largest of them would be reserved for every held
/// socket while it is idle.
async fn handle_sub_stream(
	ctx: EngineContext,
	conn: Arc<Connection>,
	tasks: Arc<TaskScope>,
	class: RequestClass,
	mut stream_tx: universalpubsub::PacketSender,
	mut stream_rx: universalpubsub::PacketReceiver,
	arrival: Option<CommandArrival>,
) -> Result<()> {
	let (to_recipient_tx, mut to_recipient_rx) = crate::queue::channel(
		crate::queue::QueueKind::Tunnel,
		ctx.config().pegboard().envoy_tunnel_queue_capacity(),
		conn.queue_pressure.clone(),
	);

	let (first_msg, mut turns) = match arrival {
		Some(arrival) => (arrival.message, arrival.turns),
		None => {
			let first_payload = stream_rx.recv().await?.context("expected request")?;
			let message = p::versioned::ToEnvoy::deserialize_with_embedded_version(&first_payload)?;
			(message, Vec::new())
		}
	};

	// A stream admitted under another class's caps would let one kind of traffic spend another's
	// budget, so it is rejected before any work starts.
	let expected = expected_class(&first_msg)?;
	if expected != class {
		tracing::warn!(
			class = class.as_str(),
			kind = expected.as_str(),
			"rejecting envoy stream whose class does not match its first message; the sender used the wrong request class"
		);
		bail!("envoy stream class does not match its first message");
	}

	match first_msg {
		// If we receive commands, we don't expect to receive more than one payload from the same stream
		p::ToEnvoy::ToEnvoyCommands(commands) => {
			Box::pin(async move {
				// The sender's attempt deadline travels with the UPS request. A command that
				// arrives after it is one the sender already gave up on and retries on its own,
				// so forwarding it could only start a generation the engine no longer tracks.
				let deadline_boot_ms = stream_tx.deadline_boot_ms();
				let now_boot_ms = conn.ctx.env().clock().observe()?.boot_ms;
				if command_expired(deadline_boot_ms, now_boot_ms) {
					for command in &commands {
						drop_expired_command(command, "arrival");
					}
					// No acknowledgement: the sender's attempt has already ended.
					return anyhow::Ok(());
				}
				// Each command was given its actor's turn at arrival. Waiting for it keeps one
				// actor's commands in arrival order on its lane, while other actors' commands go
				// ahead.
				let turn_deadline_boot_ms = deadline_boot_ms.unwrap_or_else(|| {
					now_boot_ms + conn.ctx.config().pegboard().envoy_lost_threshold() as u64
				});
				let mut turns = turns.drain(..);
				for command in commands {
					let turn = turns.next().context("command arrived without its turn")?;
					if turn
						.wait(conn.ctx.env().clock().as_ref(), turn_deadline_boot_ms)
						.await
						.is_err()
					{
						drop_expired_command(&command, "arrival");
						// No acknowledgement: the sender's attempt has already ended.
						return anyhow::Ok(());
					}
					let handle = actor_handle(&conn, &tasks, &command.checkpoint.actor_id).await?;
					let started_generation = match &command.inner {
						p::Command::CommandStartActor(_) => Some(command.checkpoint.generation),
						p::Command::CommandStopActor(_) => None,
					};
					if let p::Command::CommandStartActor(start) = &command.inner {
						handle
							.bind_generation(command.checkpoint.generation, start.config.create_ts);
						let id = rivet_util_id::Id::parse(&command.checkpoint.actor_id);
						// A start whose storage was never written, as only a first generation's
						// preload shows, creates its database volume with its first SQLite
						// commit instead of opening it ahead of time.
						if start.preloaded_kv.is_some()
							&& let Ok(id) = id
						{
							if let Err(error) = conn
								.sqlite
								.expect_fresh(id, command.checkpoint.generation)
								.await
							{
								tracing::debug!(
									%id,
									?error,
									"actor database could not be marked fresh; its first request opens it"
								);
							}
						} else if let Ok(id) = id {
							// The database volume opens while the command reaches the Envoy and
							// the runtime starts, instead of at the actor's first storage request.
							let conn = conn.clone();
							let generation = command.checkpoint.generation;
							let create_unix_ms = start.config.create_ts;
							tasks.spawn(async move {
								conn.sqlite
									.prefetch(conn.ctx.depot(), id, generation, create_unix_ms)
									.await;
							});
						}
					}

					handle.recv_command(command, deadline_boot_ms).await?;
					// The command is on its actor's lane, so the actor's next command may follow.
					drop(turn);
					// Requests held for this start may follow it now: it is ahead of them on the lane.
					if let Some(generation) = started_generation {
						handle.start_queued(generation);
					}
				}

				// The sender waits for this acknowledgement before treating the commands as
				// delivered, so a connection that closed mid-request is retried.
				stream_tx
					.send(Bytes::from_static(
						pegboard::actor_machine::executor::COMMANDS_ACK,
					))
					.await?;
				stream_tx.finish().await?;
				anyhow::Ok(())
			})
			.await?
		}
		p::ToEnvoy::ToEnvoyTunnelMessage(tunnel_msg) => {
			let mut last_message_id = tunnel_msg.message_id.clone();
			let message_key = (
				tunnel_msg.message_id.gateway_id,
				tunnel_msg.message_id.request_id,
			);

			// Figure out recipient actor
			let (is_websocket, actor_id, actor_generation) = match &tunnel_msg.message_kind {
				p::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(start) => (
					false,
					start.actor_id.clone(),
					start
						.actor_generation
						.context("expected generation to be set")?,
				),
				p::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(open) => (
					true,
					open.actor_id.clone(),
					open.actor_generation
						.context("expected generation to be set")?,
				),
				_ => bail!("unexpected first tunnel message, unable to determine recipient"),
			};

			// Store stream sender in connection
			let gateway_handle = Arc::new(GatewayHandle {
				tx: to_recipient_tx,
				actor_id: actor_id.clone().to_string(),
				actor_generation,
				stopped: tokio::sync::Notify::new(),
			});
			{
				let scc::hash_map::Entry::Vacant(entry) =
					Box::pin(conn.gateway_handles.entry_async(message_key)).await
				else {
					bail!("duplicate gateway request");
				};

				entry.insert_entry(gateway_handle.clone());
			}

			let actor_handle = Box::pin(actor_handle(&conn, &tasks, &actor_id)).await?;

			// A gateway sends a request right behind its actor's start command, over another
			// stream, so it can arrive first. It waits here until the start is on the lane ahead of
			// it, bounded, so the Envoy finds the actor.
			Box::pin(actor_handle.await_start(&conn, &actor_id, actor_generation)).await;
			Box::pin(actor_handle.recv_tunnel_msg(tunnel_msg)).await?;

			let mut stream_finalized_by_reply = false;
			let handler_res = async {
				loop {
					rivet_env::select!( ctx.env().rng("envoy.subs");
						// The execution stopped. Its tunnels end with it; the gateway closes the
						// client socket so the client reconnects and wakes the actor.
						_ = gateway_handle.stopped.notified() => {
							last_message_id.message_index = last_message_id.message_index.wrapping_add(1);
							let message_kind = if is_websocket {
								p::ToRivetTunnelMessageKind::ToRivetWebSocketClose(p::ToRivetWebSocketClose {
									code: Some(1012),
									reason: Some("actor.stopped".into()),
									hibernate: false,
								})
							} else {
								p::ToRivetTunnelMessageKind::ToRivetResponseAbort(p::ToRivetResponseAbort {
									reason: p::HttpStreamAbortReason {
										kind: p::HttpStreamAbortReasonKind::Cancelled,
										detail: Some("actor.stopped".into()),
									},
								})
							};
							let payload = Bytes::from(
								p::versioned::ToRivet::serialize_with_embedded_version(&p::ToRivet::ToRivetTunnelMessage(
									p::ToRivetTunnelMessage {
										message_id: last_message_id.clone(),
										message_kind,
									},
								), p::PROTOCOL_VERSION)?
							);
							Box::pin(stream_tx.send(payload)).await?;
							stream_finalized_by_reply = true;
							break;
						},
						// Continue receiving the rest of the stream
						next = stream_rx.recv() => {
							let Some(payload) = next? else {
								break;
							};

							let (message_id, tunnel_msg) =
								gateway_tunnel_message(payload, message_key, conn.version)?;
							last_message_id = message_id;
							match tunnel_msg {
								GatewayTunnelMessage::Encoded(payload) => {
									Box::pin(actor_handle.recv_encoded_tunnel_msg(payload)).await?;
								}
								GatewayTunnelMessage::Message(tunnel_msg) => {
									Box::pin(actor_handle.recv_tunnel_msg(tunnel_msg)).await?;
								}
							}
						},
						reply = to_recipient_rx.recv() => {
							let Some(reply) = reply else {
								break;
							};

							let reply_is_final = reply.is_final()?;
							let actor_close = reply.actor_websocket_close()?;
							let payload = reply.into_payload()?;

							Box::pin(stream_tx.send(payload)).await?;

							// The actor's close handler runs only when a close comes back to it,
							// as for a client close. This stream ends with the actor's close, so
							// the gateway cannot answer it, and the close returns from here.
							if let Some(close) = actor_close {
								last_message_id.message_index = last_message_id.message_index.wrapping_add(1);
								let _ = Box::pin(conn.to_envoy_tx.send(p::ToEnvoy::ToEnvoyTunnelMessage(
									p::ToEnvoyTunnelMessage {
										message_id: last_message_id.clone(),
										message_kind: p::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(close),
									},
								)))
								.await;
							}

							if reply_is_final {
								stream_finalized_by_reply = true;
								break;
							}
						},
					);
				}

				anyhow::Ok(())
			}
			.await;

			Box::pin(conn.gateway_handles.remove_async(&message_key)).await;

			// Handle ungraceful termination with best effort closure
			if !stream_finalized_by_reply {
				last_message_id.message_index = last_message_id.message_index.wrapping_add(1);

				let message_kind = if is_websocket {
					p::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(p::ToEnvoyWebSocketClose {
						code: Some(1001),
						reason: Some("gateway disconnected".into()),
					})
				} else {
					p::ToEnvoyTunnelMessageKind::ToEnvoyRequestAbort(p::ToEnvoyRequestAbort {
						actor_id: Some(actor_id),
						actor_generation: Some(actor_generation),
						reason: p::HttpStreamAbortReason {
							kind: p::HttpStreamAbortReasonKind::Cancelled,
							detail: Some("gateway disconnected".into()),
						},
					})
				};

				let _ = Box::pin(conn.to_envoy_tx.send(p::ToEnvoy::ToEnvoyTunnelMessage(
					p::ToEnvoyTunnelMessage {
						message_id: last_message_id,
						message_kind,
					},
				)))
				.await;
			}

			handler_res?;
		}
		_ => bail!("unsupported envoy msg"),
	}

	Ok(())
}

/// Handles one envoy message. Returns the rejection to close the connection with when the
/// envoy's connect could not be registered.
async fn handle_ws_msg(
	ctx: &EngineContext,
	conn: &Arc<Connection>,
	tasks: &Arc<TaskScope>,
	msg: p::ToRivet,
) -> Result<Option<crate::rejection::Rejection>> {
	match msg {
		p::ToRivet::ToRivetMetadata(metadata) => {
			let names: Vec<String> = metadata
				.prepopulate_actor_names
				.unwrap_or_default()
				.into_keys()
				.map(|name| name.to_string())
				.collect();

			// Init waits for the connect writes. Init clears the envoy's lost timer, and until
			// the record names this connection, commands and gateway requests still go to the
			// old one while the record's heartbeat ages toward a lost verdict. An earlier Init
			// could let the engine reallocate actors the envoy still runs.
			match connect(ctx, conn, names).await {
				Ok(()) => {}
				Err(error) if error.is::<crate::envoy_record::Superseded>() => return Err(error),
				Err(error) => {
					let mut rejection = crate::rejection::Rejection::connect(
						error,
						ctx.config().pegboard().envoy_connect_retry_after_ms(),
					);
					let structured = rivet_error::RivetError::extract(&rejection.error);
					if (structured.group(), structured.code()) == ("pool", "not_found") {
						rejection.frame.code = 1008;
					}
					return Ok(Some(rejection));
				}
			}

			conn.to_envoy_tx
				.send(p::ToEnvoy::ToEnvoyInit(p::ToEnvoyInit {
					metadata: p::ProtocolMetadata {
						envoy_lost_threshold: ctx.config().pegboard().envoy_lost_threshold(),
						// TODO: Remove?
						actor_stop_threshold: 30000,
						max_response_payload_size: 32 * 1024 * 1024,
					},
				}))
				.await?;
		}
		p::ToRivet::ToRivetPong(_) => {
			let now = ctx.env().clock().observe()?;
			conn.last_pong_unix_ms.store(now.unix_ms, Ordering::Release);
			conn.last_pong_boot_ms.store(now.boot_ms, Ordering::Release);
		}
		p::ToRivet::ToRivetStopping => {
			// The record is written first so a heartbeat racing this cannot re-enter the index.
			write_record(ctx, conn, false, |entry| {
				entry.stopping = true;
			})
			.await?;
			leave_active_index(ctx, conn).await?;
		}
		p::ToRivet::ToRivetAckCommands(_) => {}
		p::ToRivet::ToRivetEvents(events) => {
			// Never wait behind an actor's durable delivery on the socket reader. Refusing
			// admission ends this connection: the SDK replays unacknowledged events on
			// reconnect, and no later checkpoint may acknowledge past the refused event.
			for event in events {
				let Some(handle) =
					inbound_actor_handle(conn, tasks, &event.checkpoint.actor_id).await?
				else {
					// A replay from an epoch whose ownership already ended has no side effects.
					// Acknowledge it without retaining lanes for an actor no longer hosted here.
					conn.to_envoy_tx
						.try_send(
							p::ToEnvoy::ToEnvoyAckEvents(p::ToEnvoyAckEvents {
								last_event_checkpoints: vec![event.checkpoint],
							})
							.into(),
						)
						.ok()
						.context("envoy socket queue unavailable")?;
					continue;
				};
				if let Err(error) = handle.recv_event(event).await {
					return Ok(Some(crate::rejection::Rejection {
						frame: CloseFrame {
							code: crate::routes::TRY_AGAIN_LATER,
							reason: "envoy.event_lane_full;retry_after_ms=100".into(),
						},
						error,
					}));
				}
			}
		}
		p::ToRivet::ToRivetTunnelMessage(message) => {
			let message_key = (message.message_id.gateway_id, message.message_id.request_id);
			forward_tunnel_reply(conn, message_key, TunnelReply::from(message)).await?;
		}
		p::ToRivet::ToRivetKvRequest(request) => {
			let Some(handle) = inbound_actor_handle(conn, tasks, &request.actor_id).await? else {
				conn.to_envoy_tx
					.try_send(
						crate::kv::error_response(
							request.request_id,
							"actor is not running on this envoy".into(),
						)
						.into(),
					)
					.ok()
					.context("envoy socket queue unavailable")?;
				return Ok(None);
			};
			if let Some(refused) = handle.try_recv_kv(request)? {
				conn.to_envoy_tx
					.send(crate::kv::error_response(
						refused.request_id,
						storage_lane_full_message(),
					))
					.await?;
			}
		}
		p::ToRivet::ToRivetSqliteGetPagesRequest(_)
		| p::ToRivet::ToRivetSqliteCommitRequest(_)
		| p::ToRivet::ToRivetSqliteCommitStageBeginRequest(_)
		| p::ToRivet::ToRivetSqliteCommitStageSegmentRequest(_)
		| p::ToRivet::ToRivetSqliteCommitFinalizeRequest(_)
		| p::ToRivet::ToRivetSqliteExecRequest(_)
		| p::ToRivet::ToRivetSqliteExecuteRequest(_)
		| p::ToRivet::ToRivetSqliteExecuteBatchRequest(_) => {
			let actor_id = crate::sqlite::actor_id(&msg)?.to_owned();
			let Some(handle) = inbound_actor_handle(conn, tasks, &actor_id).await? else {
				conn.to_envoy_tx
					.try_send(crate::sqlite::not_running_failure(&msg)?.into())
					.ok()
					.context("envoy socket queue unavailable")?;
				return Ok(None);
			};
			if let Some(refused) = handle.try_recv_sqlite(msg)? {
				conn.to_envoy_tx
					.send(crate::sqlite::overloaded_failure(
						&refused,
						STORAGE_LANE_RETRY_AFTER_MS,
					)?)
					.await?;
			}
		}
	}

	Ok(None)
}

fn log_connect_rejection(rejection: &crate::rejection::Rejection) {
	tracing::warn!(
		code = rejection.frame.code,
		reason = %rejection.frame.reason,
		error = ?rejection.error,
		"refusing envoy connect that storage could not register; the envoy reconnects"
	);
}

/// Admission and lease renewal share the pool's writer with DELETE, closing their race.
async fn register_pool_connection(
	ctx: &EngineContext,
	conn: &Connection,
	heartbeat_unix_ms: i64,
	renew: bool,
) -> Result<wire::NodePoolConfig> {
	let connection = wire::PoolConnection {
		connection_id: conn.connection_id.as_bytes().to_vec().into(),
		heartbeat_unix_ms,
	};
	let operation = if renew {
		wire::PoolOperation::PoolRenew(connection)
	} else {
		wire::PoolOperation::PoolConnect(connection)
	};
	let reply =
		pegboard::catalog::pool(ctx, &conn.scope.namespace, &conn.scope.pool_name, operation)
			.await?;
	match (reply.status, reply.config) {
		(wire::PoolStatus::Ok, Some(config)) => Ok(config),
		// Renewal cannot revive an expired or removed admission. Retrying on the
		// same live socket would keep pongs flowing but never restore scheduling.
		_ if renew => Err(crate::envoy_record::PoolAdmissionLost.into()),
		_ => Err(pegboard::validation::PoolNotFound.build()),
	}
}

/// Captures the pool configuration, writes the connection's record and indexes, then asks
/// the connection it replaced, if any, to close. Configuration stays fixed until disconnect.
async fn connect(ctx: &EngineContext, conn: &Arc<Connection>, names: Vec<String>) -> Result<()> {
	pegboard::catalog::ensure_namespace(ctx, &conn.scope.namespace).await?;
	let config =
		register_pool_connection(ctx, conn, ctx.env().clock().observe()?.unix_ms, false).await?;
	let _ = conn.pool_config.set(config);

	let is_new = conn.record.lock().await.is_new();
	write_record(ctx, conn, true, |entry| {
		entry.names = names.into_iter().map(Into::into).collect();
	})
	.await?;

	// The listing index is keyed by creation time, which the record keeps across connections,
	// so an envoy enters it once. A retry of a lost reply writes the same key.
	let (create_unix_ms, heartbeat_unix_ms) = {
		let record = conn.record.lock().await;
		(record.entry.create_unix_ms, record.entry.heartbeat_unix_ms)
	};
	if is_new {
		ctx.depot()
			.command::<IndexInsertEnvoy>(
				&EnvoyIndexVolume::new(conn.scope.namespace.clone(), conn.scope.pool_name.clone())
					.volume_key(),
				wire::IndexInsertEnvoyRequest {
					create_unix_ms,
					envoy_key: conn.envoy_key.clone().into(),
				},
			)
			.await?;
	}
	write_heartbeat(ctx, conn, heartbeat_unix_ms).await?;

	// The envoy is back before its closed connection's lost check ran, so its actors survived.
	if let Some(check) = conn.lost_checks.cancel(&lost_check_key(conn)).await {
		conn.inherited.lock().await.extend(check.actors);
	}

	let previous = conn.record.lock().await.previous.take();
	if let Some(previous) = previous.filter(|previous| previous.connected) {
		// Best effort: without it the old connection closes at its ping timeout.
		let ctx = ctx.clone();
		ctx.env().clone().spawn(Box::pin(async move {
			if let Err(error) = supersede(&ctx, &previous).await {
				tracing::debug!(?error, "envoy supersede notice was not delivered");
			}
		}));
	}
	Ok(())
}

/// Asks the connection `previous` names to close.
async fn supersede(ctx: &EngineContext, previous: &wire::EnvoyData) -> Result<()> {
	let worker = universalpubsub::WorkerId::from_bytes(
		previous
			.worker_id
			.as_ref()
			.try_into()
			.context("invalid envoy worker id")?,
	);
	let connection_id =
		Uuid::from_slice(&previous.connection_id).context("invalid connection id")?;
	ctx.ups()?
		.request(
			universalpubsub::RequestClass::EnvoyCommand,
			&worker,
			EnvoySupersedeSubject::new(connection_id),
			&[],
		)
		.await?;
	Ok(())
}

/// Writes one change to this connection's record, fenced by the stored record's equality. Only
/// the connect write (`first`) may run before the first write landed; later writes before it
/// are skipped. A record of a newer connection ends this one.
async fn write_record(
	ctx: &EngineContext,
	conn: &Connection,
	first: bool,
	change: impl FnOnce(&mut wire::EnvoyData),
) -> Result<()> {
	let mut record = conn.record.lock().await;
	if !first && !record.connected {
		return Ok(());
	}
	let now = ctx.env().clock().observe()?.unix_ms;
	record.next(now, change);

	let volume = envoy_volume(&conn.scope, &conn.envoy_key);
	// One retry adopts this connection's own write that landed after a lost reply.
	for _ in 0..2 {
		let response = ctx
			.depot()
			.command::<pegboard::commands::EnvoyUpsert>(
				&volume,
				wire::EnvoyUpsertRequest {
					data: record.entry.clone(),
					expected: record.expected.clone(),
				},
			)
			.await?;
		if response.data == record.entry {
			record.written();
			return Ok(());
		}
		record.resolve_mismatch(&response.data)?;
	}
	bail!("envoy record changed during write retry")
}

/// Whether the connection went longer than `ping_timeout_ms` without a pong.
pub(crate) fn ping_timed_out(
	last_pong_boot_ms: u64,
	now_boot_ms: u64,
	ping_timeout_ms: u64,
) -> bool {
	now_boot_ms.saturating_sub(last_pong_boot_ms) > ping_timeout_ms
}

/// Returns once no pong has arrived within `ping_timeout_ms`, as an error that ends the
/// connection. It sleeps until the last pong would expire and checks again, so a newer pong
/// only moves the next wake. Nothing here waits on the socket queue, the socket writer or a
/// heartbeat write, so an envoy that stopped reading is still declared lost on time.
async fn pong_watchdog(
	clock: &dyn rivet_env::Clock,
	last_pong_boot_ms: &AtomicU64,
	ping_timeout_ms: i64,
) -> Result<()> {
	let ping_timeout_ms = u64::try_from(ping_timeout_ms).unwrap_or(0);
	loop {
		let now_boot_ms = clock.boot_ms()?;
		let last_pong = last_pong_boot_ms.load(Ordering::Acquire);
		if ping_timed_out(last_pong, now_boot_ms, ping_timeout_ms) {
			tracing::warn!(
				since_last_pong_ms = now_boot_ms.saturating_sub(last_pong),
				ping_timeout_ms,
				"closing envoy connection with no pong within the ping timeout; the envoy or the network to it is unresponsive"
			);
			bail!("envoy ping timeout");
		}

		// The first instant this pong counts as timed out.
		let expires_at = last_pong.saturating_add(ping_timeout_ms).saturating_add(1);
		clock.sleep_until(expires_at).await?;
	}
}

/// Writes queued frames to the envoy's socket until the queue closes or a close frame is
/// written. A pending ping goes ahead of queued frames, so traffic that keeps the queue full
/// never starves the ping an envoy must answer to stay connected.
async fn write_socket<S>(
	socket_tx: &mut S,
	to_envoy_rx: &mut crate::queue::Receiver<ToEnvoyFrame>,
	ping_rx: &mut tokio::sync::watch::Receiver<i64>,
	version: u16,
) -> Result<()>
where
	S: futures_util::Sink<Message> + Unpin,
	S::Error: std::error::Error + Send + Sync + 'static,
{
	loop {
		let next = rivet_env::select_biased!("a pending ping goes ahead of queued frames",
			changed = ping_rx.changed() => match changed {
				Ok(()) => WriterNext::Ping(*ping_rx.borrow_and_update()),
				// The connection owns the sender, so it outlives this writer.
				Err(_) => WriterNext::Frame(None),
			},
			frame = to_envoy_rx.recv() => WriterNext::Frame(frame),
		);
		let frame = match next {
			WriterNext::Ping(ts) => {
				let ping = p::ToEnvoy::ToEnvoyPing(p::ToEnvoyPing { ts });
				let payload = p::versioned::ToEnvoy::serialize(&ping, version)?;
				socket_tx.send(Message::Binary(payload.into())).await?;
				continue;
			}
			WriterNext::Frame(Some(frame)) => frame,
			WriterNext::Frame(None) => break,
		};
		let payload = match frame {
			ToEnvoyFrame::Message(msg) => p::versioned::ToEnvoy::serialize(&msg, version)?.into(),
			ToEnvoyFrame::Encoded(payload) => payload,
			ToEnvoyFrame::Close(frame) => {
				socket_tx.send(Message::Close(Some(frame))).await?;
				break;
			}
			ToEnvoyFrame::Reject(rejection) => {
				// Frames queued before the rejection are written before the close. Replies
				// whose handlers are still running are dropped with the connection.
				if let Err(error) = socket_tx.send(Message::Close(Some(rejection.frame))).await {
					tracing::debug!(?error, "failed to send envoy message rejection");
				}
				return Err(rejection.error);
			}
		};
		socket_tx.send(Message::Binary(payload)).await?;
	}

	Ok(())
}

/// What the socket writer sends next.
enum WriterNext {
	Ping(i64),
	/// A queued frame, or `None` once the queue has closed.
	Frame(Option<ToEnvoyFrame>),
}

/// Only inbound messages for an actor owned by this Envoy may allocate retained lanes.
/// Reconnected actors resolve their ownership from storage before a handle exists.
async fn inbound_actor_handle(
	conn: &Arc<Connection>,
	tasks: &Arc<TaskScope>,
	actor_id: &str,
) -> Result<Option<Arc<ActorHandle>>> {
	let existing = conn
		.actor_handles
		.read_async(actor_id, |_, handle| handle.clone())
		.await;
	let unbound = BoundGeneration::default();
	let bound = existing
		.as_ref()
		.map_or(&unbound, |handle| handle.generation.as_ref());
	let Some(owned) = owned_generation(conn, actor_id, bound).await? else {
		return Ok(None);
	};
	match existing {
		Some(handle) => Ok(Some(handle)),
		None => actor_handle_with_incarnation(conn, tasks, actor_id, Some(owned))
			.await
			.map(Some),
	}
}

/// Trusted starts and gateway streams share the same bounded handle table.
async fn actor_handle(
	conn: &Arc<Connection>,
	tasks: &Arc<TaskScope>,
	actor_id: &str,
) -> Result<Arc<ActorHandle>> {
	actor_handle_with_incarnation(conn, tasks, actor_id, None).await
}

async fn actor_handle_with_incarnation(
	conn: &Arc<Connection>,
	tasks: &Arc<TaskScope>,
	actor_id: &str,
	owned: Option<(u32, i64)>,
) -> Result<Arc<ActorHandle>> {
	if let Some(handle) = conn
		.actor_handles
		.read_async(actor_id, |_, handle| handle.clone())
		.await
	{
		return Ok(handle);
	}
	let permit = conn
		.actor_handle_budget
		.try_admit(conn.ctx.env().clock().boot_ms()?)?;
	Ok(conn
		.actor_handles
		.entry_async(actor_id.to_owned())
		.await
		.or_insert_with(|| {
			let handle = ActorHandle::new(conn, tasks, actor_id.to_owned(), permit);
			if let Some((generation, incarnation)) = owned {
				handle.bind_generation(generation, incarnation);
			}
			Arc::new(handle)
		})
		.clone())
}

/// Per-actor lanes on one connection. Outbound commands and tunnel messages keep their order on
/// one lane; inbound lifecycle events keep theirs on another, so slow storage for inbound work
/// never delays traffic toward the actor.
struct ActorHandle {
	_admission: crate::admission::ActorHandlePermit,
	tx: crate::queue::Sender<ActorMsg>,
	/// The actor's events, applied in order.
	inbound_tx: crate::queue::Sender<QueuedInbound>,
	/// The actor's KV and SQLite requests, applied in order on a lane of their own so a storage
	/// request never waits behind an event's durable delivery. A stop event drains it first.
	storage_tx: crate::queue::Sender<QueuedInbound>,
	/// Arrival order of the actor's command streams. Each stream is handled in its own task, so
	/// a command waits for its turn before it enters `tx`.
	command_turns: Turns,
	generation: Arc<BoundGeneration>,
	/// The newest generation whose start command this connection has queued on the actor's
	/// lane. Requests held by `await_start` watch it.
	start_queued: tokio::sync::watch::Sender<Option<u32>>,
	/// The newest generation the actor's record showed already started, as one more than it or
	/// 0, so a connection that never forwarded the start reads the record once per actor.
	started_elsewhere: AtomicU64,
}

impl ActorHandle {
	fn new(
		conn: &Arc<Connection>,
		tasks: &Arc<TaskScope>,
		actor_id: String,
		permit: crate::admission::ActorHandlePermit,
	) -> ActorHandle {
		let pegboard = conn.ctx.config().pegboard();
		let (tx, rx) = crate::queue::channel(
			crate::queue::QueueKind::ActorLane,
			pegboard.envoy_actor_lane_capacity(),
			conn.queue_pressure.clone(),
		);
		let (inbound_tx, inbound_rx) = crate::queue::channel(
			crate::queue::QueueKind::ActorInbound,
			pegboard.envoy_actor_inbound_capacity(),
			conn.queue_pressure.clone(),
		);
		let (storage_tx, storage_rx) = crate::queue::channel(
			crate::queue::QueueKind::ActorStorage,
			pegboard.envoy_actor_inbound_capacity(),
			conn.queue_pressure.clone(),
		);

		let conn2 = conn.clone();
		tasks.spawn(async move {
			if let Err(err) = ActorHandle::run(&conn2, rx).await {
				tracing::error!(?err, "actor handle failed");
			}
		});

		let generation = Arc::new(BoundGeneration::default());
		let conn2 = conn.clone();
		let generation2 = generation.clone();
		let storage_actor_id = actor_id.clone();
		// The event lane keeps a sender to the storage lane for its stop barrier, so the
		// storage lane ends only after the event lane.
		let barrier_tx = storage_tx.clone();
		tasks.spawn(async move {
			if let Err(err) = ActorHandle::run_inbound(
				&conn2,
				&actor_id,
				&generation2,
				inbound_rx,
				Some(&barrier_tx),
			)
			.await
			{
				tracing::error!(%actor_id, ?err, "actor event lane failed; closing connection to replay unacknowledged events");
				// Only the first failure is needed. The owner cancels and joins all lanes,
				// even if the socket queue is full or its writer is blocked.
				let _ = conn2.event_failure.try_send(err);
			}
		});

		let conn2 = conn.clone();
		let generation2 = generation.clone();
		tasks.spawn(async move {
			if let Err(err) =
				ActorHandle::run_inbound(&conn2, &storage_actor_id, &generation2, storage_rx, None)
					.await
			{
				tracing::error!(actor_id = %storage_actor_id, ?err, "actor storage lane failed");
			}
		});

		ActorHandle {
			_admission: permit,
			tx,
			inbound_tx,
			storage_tx,
			command_turns: Turns::new(),
			generation,
			start_queued: tokio::sync::watch::Sender::new(None),
			started_elsewhere: AtomicU64::new(0),
		}
	}

	/// Records that `generation`'s start command is queued on the lane.
	fn start_queued(&self, generation: u32) {
		self.start_queued.send_if_modified(|current| {
			if current.is_some_and(|current| wrapping_ge(current, generation)) {
				return false;
			}
			*current = Some(generation);
			true
		});
	}

	/// Holds a gateway request for `generation` until its start command is queued ahead of it,
	/// for at most `pegboard.envoy_start_wait_ms`. A start this connection did not forward, such
	/// as one delivered before a reconnect, is found in the actor's record instead. The request
	/// is forwarded however the wait ends; an Envoy without the actor refuses it as not found.
	///
	/// Ordered UPS lanes do not make this hold unnecessary. They order requests from one sender
	/// to one destination, but the start comes from the pegboard executor as a request and the
	/// gateway request comes from a gateway as a stream, so nothing orders one behind the other.
	async fn await_start(&self, conn: &Connection, actor_id: &str, generation: u32) {
		let is_queued =
			|queued: &Option<u32>| queued.is_some_and(|queued| wrapping_ge(queued, generation));
		if is_queued(&self.start_queued.borrow())
			|| decode_generation(self.started_elsewhere.load(Ordering::Acquire))
				.is_some_and(|started| wrapping_ge(started, generation))
			|| self
				.generation()
				.is_some_and(|bound| wrapping_gt(bound, generation))
		{
			return;
		}

		let clock = conn.ctx.env().clock();
		let wait =
			std::time::Duration::from_millis(conn.ctx.config().pegboard().envoy_start_wait_ms());
		let Ok(deadline) = rivet_env::deadline_after(clock.as_ref(), wait) else {
			return;
		};
		let mut queued = self.start_queued.subscribe();

		// The start usually follows within a few milliseconds, so the record is read only if it
		// does not.
		let grace = rivet_env::deadline_after(clock.as_ref(), START_READ_GRACE)
			.map_or(deadline, |grace| grace.min(deadline));
		let forwarded = rivet_env::timeout_at(clock.as_ref(), grace, queued.wait_for(is_queued))
			.await
			.is_ok();
		if forwarded {
			return;
		}

		let started = rivet_env::timeout_at(
			clock.as_ref(),
			deadline,
			started_elsewhere(conn, actor_id, generation),
		)
		.await;
		if let Ok(Ok(true)) = started {
			advance_generation(&self.started_elsewhere, generation);
			return;
		}

		let forwarded = rivet_env::timeout_at(clock.as_ref(), deadline, queued.wait_for(is_queued))
			.await
			.is_ok();
		if !forwarded {
			tracing::debug!(
				%actor_id,
				generation,
				"actor start was not forwarded on this connection in time; forwarding the request anyway"
			);
		}
	}

	async fn run(conn: &Arc<Connection>, mut rx: crate::queue::Receiver<ActorMsg>) -> Result<()> {
		while let Some(msg) = rx.recv().await {
			match msg {
				ActorMsg::Command {
					command,
					deadline_boot_ms,
				} => {
					let starts = matches!(command.inner, p::Command::CommandStartActor(_));
					// Only a start is dangerous late: it creates a runtime instance, while a late
					// stop is ignored or deduplicated by the Envoy. So a start that waited in this
					// lane past its sender's deadline is dropped, and a stop is always forwarded.
					//
					// TODO: also send `CommandStopActor` for any generation that is no longer
					// current when its storage request or event is refused, so a ghost generation
					// that slipped through (for example after a runtime restart lost its floor)
					// stops on its first write or event instead of running user code.
					if starts {
						let now_boot_ms = conn.ctx.env().clock().observe()?.boot_ms;
						if !forwards_command(&command, deadline_boot_ms, now_boot_ms) {
							drop_expired_command(&command, "forward");
							continue;
						}
					}
					conn.to_envoy_tx
						.send(p::ToEnvoy::ToEnvoyCommands(vec![command_for_envoy(
							*command,
						)]))
						.await?;
				}
				ActorMsg::TunnelMessage(msg) => {
					conn.to_envoy_tx
						.send(p::ToEnvoy::ToEnvoyTunnelMessage(*msg))
						.await?;
				}
				ActorMsg::EncodedTunnelMessage(payload) => {
					conn.to_envoy_tx
						.send(ToEnvoyFrame::Encoded(payload))
						.await?;
				}
			}
		}

		Ok(())
	}

	async fn run_inbound(
		conn: &Arc<Connection>,
		actor_id: &str,
		generation: &BoundGeneration,
		mut rx: crate::queue::Receiver<QueuedInbound>,
		storage: Option<&crate::queue::Sender<QueuedInbound>>,
	) -> Result<()> {
		// Each message is handled in a boxed future so the lane task, which lives as long as
		// its actor and is idle almost all the time, does not reserve space for the largest
		// request inline. Inline, the SQLite path made every held actor's lane future about
		// 90 KiB.
		while let Some(QueuedInbound { msg }) = rx.recv().await {
			Box::pin(Self::handle_inbound(
				conn, actor_id, generation, msg, storage,
			))
			.await?;
		}

		Ok(())
	}

	async fn handle_inbound(
		conn: &Arc<Connection>,
		actor_id: &str,
		generation: &BoundGeneration,
		msg: InboundMsg,
		storage: Option<&crate::queue::Sender<QueuedInbound>>,
	) -> Result<()> {
		match msg {
			InboundMsg::Drained(barrier) => reach_stop_barrier(generation, *barrier),
			InboundMsg::Kv(request) => {
				let reply = kv_request(conn, actor_id, generation, *request).await;
				conn.to_envoy_tx.send(reply).await?;
			}
			InboundMsg::Sqlite(request) => {
				if let Some(expected) = crate::sqlite::expected_generation(&request)
					&& generation.is_stopped(expected)
				{
					let reply = crate::sqlite::stopped_failure(&request, expected)?;
					conn.to_envoy_tx.send(reply).await?;
					return Ok(());
				}
				let owned = owned_generation(conn, actor_id, generation).await;
				let span = tracing::trace_span!(target: "sqlite_prefetch::trace", "sqlite_connection",
                    envoy_key = %conn.envoy_key, connection_id = %conn.connection_id);
				let reply = crate::sqlite::trace_request(
					span,
					crate::sqlite::request(&conn.ctx, &conn.sqlite, actor_id, owned, *request),
				)
				.await?;
				conn.to_envoy_tx.send(reply).await?;
			}
			InboundMsg::Event(event) => {
				let checkpoint = event.checkpoint.clone();
				let stopped = crate::lifecycle::is_stopped(&event);
				// A stop is delivered only after every storage request queued before it ran, as
				// when both shared one lane: the stopped generation's writes finish before the
				// actor may start elsewhere, and none of them can reopen the volume afterward.
				if stopped && let Some(storage) = storage {
					let budget = std::time::Duration::from_millis(
						conn.ctx
							.config()
							.pegboard()
							.envoy_event_delivery_timeout_ms(),
					);
					let drained = drain_storage_lane_within(
						conn.ctx.env().clock().as_ref(),
						budget,
						storage,
						checkpoint.generation,
					)
					.await;
					// A storage request that outlives the budget must not hold the stop back
					// forever. The stopped generation is tombstoned so its queued requests fail
					// closed, and its session is retired without waiting for the stuck request.
					if !drained {
						tracing::warn!(
							%actor_id,
							generation = checkpoint.generation,
							"actor storage lane did not drain before the stop's delivery budget; a storage request is stuck, failing the stopped generation's remaining requests"
						);
						generation.stop(checkpoint.generation);
						if let Ok(id) = rivet_util_id::Id::parse(actor_id) {
							conn.sqlite.retire_now(id).await;
						}
					}
				}
				// The stopped generation's session is retired before the stop is delivered, so
				// no later generation can have taken it yet, and only a session of this
				// generation or older is retired. Its database volume closes.
				if stopped && let Ok(id) = rivet_util_id::Id::parse(actor_id) {
					conn.sqlite.stop(id, checkpoint.generation).await;
				}
				// Stop this lane before any later checkpoint can cumulatively acknowledge
				// past this event. Its owner closes the connection to trigger SDK replay.
				ensure!(
					crate::lifecycle::deliver(&conn.ctx, actor_id, *event).await?,
					"actor lifecycle event delivery exhausted"
				);

				conn.to_envoy_tx
					.send(p::ToEnvoy::ToEnvoyAckEvents(p::ToEnvoyAckEvents {
						last_event_checkpoints: vec![checkpoint.clone()],
					}))
					.await?;

				// A stopped generation needs no lanes or SQLite session on this
				// connection. Dropping the handle closes its lanes once their queued
				// work drains.
				if stopped {
					stop_tunnels(conn, actor_id, checkpoint.generation).await;
					conn.actor_handles
						.remove_if_async(actor_id, |handle| {
							handle
								.generation()
								.is_none_or(|generation| generation == checkpoint.generation)
						})
						.await;
				}
			}
		}

		Ok(())
	}

	fn bind_generation(&self, generation: u32, create_unix_ms: i64) {
		self.generation.bind(generation, create_unix_ms);
	}

	fn generation(&self) -> Option<u32> {
		self.generation.get()
	}

	async fn recv_command(
		&self,
		cmd: p::CommandWrapper,
		deadline_boot_ms: Option<u64>,
	) -> Result<()> {
		self.tx
			.send(ActorMsg::Command {
				command: Box::new(cmd),
				deadline_boot_ms,
			})
			.await?;
		Ok(())
	}

	async fn recv_tunnel_msg(&self, cmd: p::ToEnvoyTunnelMessage) -> Result<()> {
		self.tx.send(ActorMsg::TunnelMessage(Box::new(cmd))).await?;
		Ok(())
	}

	/// Queues a tunnel message already encoded as a latest-version `ToEnvoy` body, in order with
	/// this actor's other messages.
	async fn recv_encoded_tunnel_msg(&self, payload: Bytes) -> Result<()> {
		self.tx
			.send(ActorMsg::EncodedTunnelMessage(payload))
			.await?;
		Ok(())
	}

	async fn recv_event(&self, event: p::EventWrapper) -> Result<()> {
		self.inbound_tx
			.try_send(QueuedInbound::new(InboundMsg::Event(Box::new(event))))
			.map_err(|_| anyhow::anyhow!("actor event lane unavailable; reconnect to replay"))
	}

	/// Queues a KV request on the storage lane without waiting. A full lane hands the request
	/// back so the socket reader can refuse it instead of stalling every actor on the socket.
	fn try_recv_kv(&self, request: p::ToRivetKvRequest) -> Result<Option<p::ToRivetKvRequest>> {
		match self
			.storage_tx
			.try_send(QueuedInbound::new(InboundMsg::Kv(Box::new(request))))
		{
			Ok(()) => Ok(None),
			Err(mpsc::error::TrySendError::Full(queued)) => match queued.msg {
				InboundMsg::Kv(request) => Ok(Some(*request)),
				InboundMsg::Event(_) | InboundMsg::Sqlite(_) | InboundMsg::Drained(_) => {
					anyhow::bail!("storage lane returned a different message than it was given")
				}
			},
			Err(mpsc::error::TrySendError::Closed(_)) => {
				anyhow::bail!("actor storage lane closed")
			}
		}
	}

	/// Queues a SQLite request on the storage lane without waiting; see [`Self::try_recv_kv`].
	fn try_recv_sqlite(&self, request: p::ToRivet) -> Result<Option<p::ToRivet>> {
		match self
			.storage_tx
			.try_send(QueuedInbound::new(InboundMsg::Sqlite(Box::new(request))))
		{
			Ok(()) => Ok(None),
			Err(mpsc::error::TrySendError::Full(queued)) => match queued.msg {
				InboundMsg::Sqlite(request) => Ok(Some(*request)),
				InboundMsg::Event(_) | InboundMsg::Kv(_) | InboundMsg::Drained(_) => {
					anyhow::bail!("storage lane returned a different message than it was given")
				}
			},
			Err(mpsc::error::TrySendError::Closed(_)) => {
				anyhow::bail!("actor storage lane closed")
			}
		}
	}
}

/// The generation this connection started for one actor, and the incarnation it belongs to.
/// Storage requests carry the generation so the actor volume can reject requests from an
/// execution that no longer owns the actor; the incarnation names the database volume.
///
/// A stopped generation leaves a tombstone: it and every older generation own nothing on this
/// connection afterward, whatever was bound or the actor's record says, so their late storage
/// requests fail closed.
#[derive(Default)]
struct BoundGeneration {
	/// One more than the bound generation, or 0.
	bound: AtomicU64,
	/// The bound generation's incarnation, the actor's `create_unix_ms`, or 0 before a bind.
	create_unix_ms: AtomicI64,
	/// One more than the highest stopped generation, or 0.
	stopped: AtomicU64,
}

impl BoundGeneration {
	fn bind(&self, generation: u32, create_unix_ms: i64) {
		self.create_unix_ms.store(create_unix_ms, Ordering::Release);
		self.bound
			.store(u64::from(generation) + 1, Ordering::Release);
	}

	/// The bound generation with its incarnation, unless it stopped.
	fn get_incarnation(&self) -> Option<(u32, i64)> {
		let generation = self.get()?;
		Some((generation, self.create_unix_ms.load(Ordering::Acquire)))
	}

	fn get(&self) -> Option<u32> {
		match self.bound.load(Ordering::Acquire) {
			0 => None,
			bound => u32::try_from(bound - 1)
				.ok()
				.filter(|generation| !self.is_stopped(*generation)),
		}
	}

	/// Records that `generation` and every older one stopped on this connection.
	fn stop(&self, generation: u32) {
		advance_generation(&self.stopped, generation);
	}

	/// Whether `generation` stopped here. A generation the protocol encodes outside the epoch
	/// range never stopped; admission rejects it separately.
	fn is_stopped(&self, generation: impl Into<u64>) -> bool {
		let Ok(generation) = u32::try_from(generation.into()) else {
			return false;
		};
		decode_generation(self.stopped.load(Ordering::Acquire))
			.is_some_and(|stopped| wrapping_le(generation, stopped))
	}
}

/// The generation an atomic holds as one more than it, or `None` for 0.
fn decode_generation(encoded: u64) -> Option<u32> {
	match encoded {
		0 => None,
		encoded => u32::try_from(encoded - 1).ok(),
	}
}

/// Advances an atomic that holds a generation as one more than it to `generation`, unless it
/// already holds a later one on the epoch ring.
fn advance_generation(slot: &AtomicU64, generation: u32) {
	let _ = slot.try_update(
		Ordering::AcqRel,
		Ordering::Acquire,
		|current| match decode_generation(current) {
			Some(current) if wrapping_ge(current, generation) => None,
			Some(_) | None => Some(u64::from(generation) + 1),
		},
	);
}

/// How long a held request waits for its start before reading the actor's record.
/// How long an actor whose storage lane was full should wait before sending the request again.
const STORAGE_LANE_RETRY_AFTER_MS: u64 = 100;

/// The KV error text for a request refused because its actor's storage lane was full. KV
/// errors carry only a message, so the text names the typed refusal and its retry hint.
fn storage_lane_full_message() -> String {
	format!(
		"depot.overloaded: actor storage lane is full (reason=actor_storage_lane, retry_after_ms={STORAGE_LANE_RETRY_AFTER_MS})"
	)
}

/// How long a held request waits for a queued start before checking the actor's record for a
/// start delivered on another connection.
const START_READ_GRACE: std::time::Duration = std::time::Duration::from_millis(20);

/// Whether the actor's record shows `generation` already started, or a newer generation, so
/// its start was delivered on another connection and no start is coming on this one.
async fn started_elsewhere(conn: &Connection, actor_id: &str, generation: u32) -> Result<bool> {
	let id = rivet_util_id::Id::parse(actor_id)?;
	let read = conn
		.ctx
		.depot()
		.command::<pegboard::commands::ActorRead>(&ActorVolume::new(id).volume_key(), ())
		.await?;
	if wrapping_gt(read.epoch, generation) {
		return Ok(true);
	}
	let started = read.epoch == generation
		&& match read.phase {
			Some(wire::ActorPhase::Running(_)) | Some(wire::ActorPhase::Stopping(_)) => true,
			Some(wire::ActorPhase::Starting(_))
			| Some(wire::ActorPhase::Initializing(_))
			| Some(wire::ActorPhase::Sleeping)
			| Some(wire::ActorPhase::Destroying(_))
			| Some(wire::ActorPhase::Destroyed)
			| None => false,
		};
	Ok(started)
}

/// Resolves the generation that owns the actor on this envoy, with the incarnation whose
/// database volume it uses. A connection that did not deliver the start, such as one opened
/// after a reconnect, learns both from the actor's record.
async fn owned_generation(
	conn: &Connection,
	actor_id: &str,
	bound: &BoundGeneration,
) -> Result<Option<(u32, i64)>> {
	if let Some(owned) = bound.get_incarnation() {
		return Ok(Some(owned));
	}

	let id = rivet_util_id::Id::parse(actor_id)?;
	let read = conn
		.ctx
		.depot()
		.command::<pegboard::commands::ActorRead>(&ActorVolume::new(id).volume_key(), ())
		.await;
	// A destroyed actor's state volume is deleted; it owns nothing here.
	let read = match read {
		Ok(read) => read,
		Err(error) if pegboard::commands::is_volume_gone(&error) => return Ok(None),
		Err(error) => return Err(error),
	};
	Ok(record_owned_generation(
		&conn.scope,
		&conn.envoy_key,
		read,
		bound,
	))
}

fn record_owned_generation(
	scope: &crate::routes::EnvoyScope,
	envoy_key: &str,
	read: wire::ActorReadResponse,
	bound: &BoundGeneration,
) -> Option<(u32, i64)> {
	let (Some(data), Some(phase)) = (read.data, read.phase) else {
		return None;
	};
	let allocation = match phase {
		wire::ActorPhase::Starting(wire::Starting { allocation, .. })
		| wire::ActorPhase::Running(wire::Running { allocation })
		| wire::ActorPhase::Stopping(wire::Stopping { allocation, .. }) => allocation,
		wire::ActorPhase::Initializing(_)
		| wire::ActorPhase::Sleeping
		| wire::ActorPhase::Destroying(_)
		| wire::ActorPhase::Destroyed => return None,
	};
	// The allocation names the envoy, not the connection: after a reconnect the recorded
	// connection is stale until a gateway repairs it, but the actor still runs here.
	if allocation.envoy_key != envoy_key
		|| data.namespace != scope.namespace
		|| data.pool_name != scope.pool_name
	{
		return None;
	}

	// A record that still shows an epoch stopped here, before the stop was delivered,
	// grants nothing.
	if bound.is_stopped(read.epoch) {
		return None;
	}
	bound.bind(read.epoch, data.create_unix_ms);
	Some((read.epoch, data.create_unix_ms))
}

/// Runs one KV request against the actor volume. Failures become request-scoped KV errors so
/// one actor's storage trouble never closes the shared socket.
async fn kv_request(
	conn: &Connection,
	actor_id: &str,
	bound: &BoundGeneration,
	request: p::ToRivetKvRequest,
) -> p::ToEnvoy {
	let request_id = request.request_id;
	let result = async {
		let Some((generation, create_unix_ms)) = owned_generation(conn, actor_id, bound).await?
		else {
			return Ok(crate::kv::error_response(
				request_id,
				"actor is not running on this envoy".into(),
			));
		};
		let id = rivet_util_id::Id::parse(actor_id)?;
		let data = conn
			.sqlite
			.kv(
				conn.ctx.depot(),
				id,
				generation,
				create_unix_ms,
				request.data,
			)
			.await?;
		anyhow::Ok(p::ToEnvoy::ToEnvoyKvResponse(p::ToEnvoyKvResponse {
			request_id,
			data,
		}))
	}
	.await;

	match result {
		Ok(reply) => reply,
		Err(error) => {
			tracing::warn!(%actor_id, ?error, "actor KV request failed");
			crate::kv::error_response(request_id, error.to_string())
		}
	}
}

/// Large variants are boxed: every held actor's lane channel allocates a block of slots sized
/// by the largest variant, and the common encoded variant is small.
enum ActorMsg {
	Command {
		command: Box<p::CommandWrapper>,
		/// The sender's attempt deadline on this host's monotonic clock.
		deadline_boot_ms: Option<u64>,
	},
	TunnelMessage(Box<p::ToEnvoyTunnelMessage>),
	/// A latest-version `ToEnvoy` body holding a tunnel message, forwarded without decoding.
	EncodedTunnelMessage(Bytes),
}

/// A gateway's tunnel message after its identity check, ready for the actor's queue.
enum GatewayTunnelMessage {
	/// The latest-version `ToEnvoy` body, written to an envoy on that version as is.
	Encoded(Bytes),
	/// Decoded for an envoy on an older version, which the socket writer converts down.
	Message(p::ToEnvoyTunnelMessage),
}

/// Checks a gateway's follow-up tunnel message in place against the stream's identity. For an
/// envoy on the latest version the encoded body is kept and nothing is decoded.
fn gateway_tunnel_message(
	payload: Bytes,
	message_key: ([u8; 4], [u8; 4]),
	envoy_version: u16,
) -> Result<(p::MessageId, GatewayTunnelMessage)> {
	let view = p::versioned::ToEnvoy::open(payload)?;
	let p::ToEnvoyRef::ToEnvoyTunnelMessage(tunnel_msg) = view.get() else {
		bail!("unexpected envoy msg in tunnel");
	};
	let message_id = tunnel_msg.message_id();
	ensure!(
		(*message_id.gateway_id(), *message_id.request_id()) == message_key,
		"tunnel identity changed",
	);
	let message_id = message_id.to_owned();
	if envoy_version == p::PROTOCOL_VERSION {
		return Ok((message_id, GatewayTunnelMessage::Encoded(view.into_bytes())));
	}
	let p::ToEnvoy::ToEnvoyTunnelMessage(tunnel_msg) = view.to_owned() else {
		bail!("unexpected envoy msg in tunnel");
	};
	Ok((message_id, GatewayTunnelMessage::Message(tunnel_msg)))
}

/// One message queued for the envoy socket.
enum ToEnvoyFrame {
	/// Encoded by the socket writer at the connection's protocol version.
	Message(p::ToEnvoy),
	/// Already encoded at the connection's protocol version, written as is.
	Encoded(Bytes),
	/// Closes the socket after every frame queued before it; the writer stops here.
	Close(CloseFrame),
	/// Closes the socket with the rejection's close frame and ends the connection with its error.
	Reject(crate::rejection::Rejection),
}

/// WebSocket close code for an endpoint that is going away, as on server shutdown.
const GOING_AWAY: u16 = 1001;

/// How long the shutdown close waits for room in the socket queue.
const SHUTDOWN_CLOSE_QUEUE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// The close an engine sends its envoys when it starts shutting down. The envoy reconnects
/// after it, and the reason names the cause in the `<group>.<code>` form envoys parse.
pub(crate) fn shutdown_close() -> CloseFrame {
	CloseFrame {
		code: GOING_AWAY,
		reason: "envoy.engine_shutting_down".into(),
	}
}

impl From<p::ToEnvoy> for ToEnvoyFrame {
	fn from(msg: p::ToEnvoy) -> Self {
		Self::Message(msg)
	}
}

/// A tunnel reply on its way from the envoy to its gateway stream.
/// The decoded variant is boxed so each tunnel's reply channel slot stays small; the common
/// path from a current envoy is the encoded view.
enum TunnelReply {
	/// Decoded from an envoy speaking an older version, or built here.
	Message(Box<p::ToRivetTunnelMessage>),
	/// A latest-version `ToRivet` body holding a tunnel message, received from the envoy.
	Encoded(vbare::View<p::ToRivetWire>),
}

impl TunnelReply {
	/// Whether this reply ends its stream.
	fn is_final(&self) -> Result<bool> {
		Ok(match self {
			Self::Message(reply) => match &reply.message_kind {
				p::ToRivetTunnelMessageKind::ToRivetResponseStart(start) => !start.stream,
				p::ToRivetTunnelMessageKind::ToRivetResponseChunk(chunk) => chunk.finish,
				p::ToRivetTunnelMessageKind::ToRivetWebSocketClose(_)
				| p::ToRivetTunnelMessageKind::ToRivetResponseAbort(_) => true,
				p::ToRivetTunnelMessageKind::ToRivetRequestBodyWindowUpdate(_)
				| p::ToRivetTunnelMessageKind::ToRivetRequestBodyCancel
				| p::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(_)
				| p::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(_)
				| p::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(_) => false,
			},
			Self::Encoded(view) => {
				let p::ToRivetRef::ToRivetTunnelMessage(reply) = view.get() else {
					bail!("encoded tunnel reply holds another message");
				};
				match reply.message_kind() {
					p::ToRivetTunnelMessageKindRef::ToRivetResponseStart(start) => !start.stream(),
					p::ToRivetTunnelMessageKindRef::ToRivetResponseChunk(chunk) => chunk.finish(),
					p::ToRivetTunnelMessageKindRef::ToRivetWebSocketClose(_)
					| p::ToRivetTunnelMessageKindRef::ToRivetResponseAbort(_) => true,
					p::ToRivetTunnelMessageKindRef::ToRivetRequestBodyWindowUpdate(_)
					| p::ToRivetTunnelMessageKindRef::ToRivetRequestBodyCancel
					| p::ToRivetTunnelMessageKindRef::ToRivetWebSocketOpen(_)
					| p::ToRivetTunnelMessageKindRef::ToRivetWebSocketMessage(_)
					| p::ToRivetTunnelMessageKindRef::ToRivetWebSocketMessageAck(_) => false,
				}
			}
		})
	}

	/// The close to return to the actor when this reply is the actor closing its WebSocket. A
	/// close that hibernates the WebSocket does not end it for the actor.
	fn actor_websocket_close(&self) -> Result<Option<p::ToEnvoyWebSocketClose>> {
		Ok(match self {
			Self::Message(reply) => match &reply.message_kind {
				p::ToRivetTunnelMessageKind::ToRivetWebSocketClose(close) if !close.hibernate => {
					Some(p::ToEnvoyWebSocketClose {
						code: close.code,
						reason: close.reason.clone(),
					})
				}
				p::ToRivetTunnelMessageKind::ToRivetWebSocketClose(_)
				| p::ToRivetTunnelMessageKind::ToRivetResponseStart(_)
				| p::ToRivetTunnelMessageKind::ToRivetResponseChunk(_)
				| p::ToRivetTunnelMessageKind::ToRivetResponseAbort(_)
				| p::ToRivetTunnelMessageKind::ToRivetRequestBodyWindowUpdate(_)
				| p::ToRivetTunnelMessageKind::ToRivetRequestBodyCancel
				| p::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(_)
				| p::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(_)
				| p::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(_) => None,
			},
			Self::Encoded(view) => {
				let p::ToRivetRef::ToRivetTunnelMessage(reply) = view.get() else {
					bail!("encoded tunnel reply holds another message");
				};
				match reply.message_kind() {
					p::ToRivetTunnelMessageKindRef::ToRivetWebSocketClose(close)
						if !close.hibernate() =>
					{
						Some(p::ToEnvoyWebSocketClose {
							code: close.code(),
							reason: close.reason().map(|reason| reason.to_owned().into()),
						})
					}
					p::ToRivetTunnelMessageKindRef::ToRivetWebSocketClose(_)
					| p::ToRivetTunnelMessageKindRef::ToRivetResponseStart(_)
					| p::ToRivetTunnelMessageKindRef::ToRivetResponseChunk(_)
					| p::ToRivetTunnelMessageKindRef::ToRivetResponseAbort(_)
					| p::ToRivetTunnelMessageKindRef::ToRivetRequestBodyWindowUpdate(_)
					| p::ToRivetTunnelMessageKindRef::ToRivetRequestBodyCancel
					| p::ToRivetTunnelMessageKindRef::ToRivetWebSocketOpen(_)
					| p::ToRivetTunnelMessageKindRef::ToRivetWebSocketMessage(_)
					| p::ToRivetTunnelMessageKindRef::ToRivetWebSocketMessageAck(_) => None,
				}
			}
		})
	}

	/// The UPS packet for the gateway: a version header and a latest-version `ToRivet` body.
	/// An encoded reply is copied once behind the header instead of being re-encoded.
	fn into_payload(self) -> Result<Bytes> {
		match self {
			Self::Message(reply) => Ok(Bytes::from(
				p::versioned::ToRivet::serialize_with_embedded_version(
					&p::ToRivet::ToRivetTunnelMessage(*reply),
					p::PROTOCOL_VERSION,
				)?,
			)),
			Self::Encoded(view) => {
				let body = view.as_bytes();
				let mut payload = bytes::BytesMut::with_capacity(2 + body.len());
				payload.extend_from_slice(&p::PROTOCOL_VERSION.to_le_bytes());
				payload.extend_from_slice(body);
				Ok(payload.freeze())
			}
		}
	}
}

impl From<p::ToRivetTunnelMessage> for TunnelReply {
	fn from(reply: p::ToRivetTunnelMessage) -> Self {
		Self::Message(Box::new(reply))
	}
}

/// Hands a tunnel reply to the gateway stream it belongs to. Replies for a closed stream are
/// dropped.
async fn forward_tunnel_reply(
	conn: &Connection,
	message_key: ([u8; 4], [u8; 4]),
	reply: TunnelReply,
) -> Result<()> {
	let handle = conn
		.gateway_handles
		.get_async(&message_key)
		.await
		.map(|entry| entry.clone());
	if let Some(route) = handle
		&& route.tx.send(reply).await.is_err()
	{
		// The route's request handler ended while this reply waited for room on its queue, as
		// when the client went away or the actor stopped mid-response, so nobody can deliver the
		// reply. Dropping it, and the route if it is still listed, keeps one late reply from
		// closing the connection under every other actor on it.
		tracing::debug!(
			?message_key,
			"dropped a tunnel reply for a closed gateway route"
		);
		conn.gateway_handles
			.remove_if_async(&message_key, |current| Arc::ptr_eq(current, &route))
			.await;
	}
	Ok(())
}

/// An inbound message on one of the actor's lanes.
struct QueuedInbound {
	msg: InboundMsg,
}

impl QueuedInbound {
	fn new(msg: InboundMsg) -> Self {
		QueuedInbound { msg }
	}
}

/// Variants are boxed so each held actor's inbound channel block holds pointers, not the
/// largest request inline.
enum InboundMsg {
	Event(Box<p::EventWrapper>),
	Kv(Box<p::ToRivetKvRequest>),
	/// One SQLite page or execution request; see [`crate::sqlite::actor_id`].
	Sqlite(Box<p::ToRivet>),
	/// A stop's barrier on the storage lane. When the lane reaches it, every request queued
	/// before it ran; the lane then refuses the stopped generation's later requests and answers.
	Drained(Box<StopBarrier>),
}

/// A stop's barrier on the storage lane.
struct StopBarrier {
	/// The generation that stopped; it and older ones are refused once the lane reaches this.
	generation: u32,
	done: tokio::sync::oneshot::Sender<()>,
}

/// Runs when the storage lane reaches a stop's barrier: every request queued before it ran, so
/// the stopped generation is closed to later requests before the stop is delivered.
fn reach_stop_barrier(bound: &BoundGeneration, barrier: StopBarrier) {
	bound.stop(barrier.generation);
	let _ = barrier.done.send(());
}

/// [`drain_storage_lane`] bounded by `budget`. Returns whether the lane drained in time.
async fn drain_storage_lane_within(
	clock: &dyn rivet_env::Clock,
	budget: std::time::Duration,
	storage: &crate::queue::Sender<QueuedInbound>,
	generation: u32,
) -> bool {
	rivet_env::timeout(clock, budget, drain_storage_lane(storage, generation))
		.await
		.is_ok()
}

/// Waits until the storage lane has run every request queued on it so far. A closed lane has
/// nothing left to run.
async fn drain_storage_lane(storage: &crate::queue::Sender<QueuedInbound>, generation: u32) {
	let (done_tx, done_rx) = tokio::sync::oneshot::channel();
	let barrier = StopBarrier {
		generation,
		done: done_tx,
	};
	if storage
		.send(QueuedInbound::new(InboundMsg::Drained(Box::new(barrier))))
		.await
		.is_err()
	{
		return;
	}
	let _ = done_rx.await;
}

struct GatewayHandle {
	tx: crate::queue::Sender<TunnelReply>,
	actor_id: String,
	actor_generation: u32,
	/// Notified once the execution behind this tunnel stopped.
	stopped: tokio::sync::Notify,
}

/// Ends every tunnel to `generation` of the actor. The envoy runs no execution for them anymore.
async fn stop_tunnels(conn: &Connection, actor_id: &str, generation: u32) {
	conn.gateway_handles
		.iter_async(|_, handle| {
			if handle.actor_id == actor_id && handle.actor_generation == generation {
				handle.stopped.notify_one();
			}
			true
		})
		.await;
}

/// A command stream's first message, read in arrival order, with one actor turn per command.
struct CommandArrival {
	message: p::ToEnvoy,
	turns: Vec<Turn>,
}

/// Reads a command stream's first message and takes each command's turn on its actor. Called
/// from the subscriber loop, so turns follow the order UPS delivered the streams in, which is
/// the order of the sender's lane tickets. A stream whose first message is not commands takes
/// no turns and is rejected by its handler.
async fn command_arrival(
	conn: &Arc<Connection>,
	tasks: &Arc<TaskScope>,
	stream_rx: &mut universalpubsub::PacketReceiver,
) -> Result<CommandArrival> {
	let payload = stream_rx.recv().await?.context("expected request")?;
	let message = p::versioned::ToEnvoy::deserialize_with_embedded_version(&payload)?;
	let mut turns = Vec::new();
	if let p::ToEnvoy::ToEnvoyCommands(commands) = &message {
		for command in commands {
			let handle = actor_handle(conn, tasks, &command.checkpoint.actor_id).await?;
			turns.push(handle.command_turns.take());
		}
	}
	Ok(CommandArrival { message, turns })
}

/// Whether a command whose sender's deadline is `deadline_boot_ms` has passed it at
/// `now_boot_ms`. A command without a deadline never expires.
fn command_expired(deadline_boot_ms: Option<u64>, now_boot_ms: u64) -> bool {
	deadline_boot_ms.is_some_and(|deadline| now_boot_ms >= deadline)
}

/// Whether the actor lane forwards `command` to the Envoy at `now_boot_ms`. Only a start
/// past its sender's deadline is dropped; a stop is always forwarded.
fn forwards_command(
	command: &p::CommandWrapper,
	deadline_boot_ms: Option<u64>,
	now_boot_ms: u64,
) -> bool {
	match command.inner {
		p::Command::CommandStartActor(_) => !command_expired(deadline_boot_ms, now_boot_ms),
		p::Command::CommandStopActor(_) => true,
	}
}

/// Rewrites a start's actor key from the canonical form the engine names the actor by into the
/// slash form the actor runtime reads. A stop or a keyless start is returned unchanged.
fn command_for_envoy(mut command: p::CommandWrapper) -> p::CommandWrapper {
	if let p::Command::CommandStartActor(start) = &mut command.inner {
		start.config.key = start
			.config
			.key
			.as_deref()
			.map(actor_key::to_slash)
			.map(Into::into);
	}
	command
}

/// Logs one actor command dropped at `stage` because its sender's deadline passed.
fn drop_expired_command(command: &p::CommandWrapper, stage: &'static str) {
	let kind = match command.inner {
		p::Command::CommandStartActor(_) => "start",
		p::Command::CommandStopActor(_) => "stop",
	};
	tracing::debug!(
		actor_id = %command.checkpoint.actor_id,
		generation = command.checkpoint.generation,
		stage,
		kind,
		"dropping an actor command whose sender's deadline passed; the actor state machine retries it if it is still wanted"
	);
}




