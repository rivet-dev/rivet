use anyhow::{Context, Result, bail, ensure};
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use pegboard::{
	commands::EnvoyRead,
	envoy_check::ACTOR_STOPPED_DETAIL,
	epoch::{wrapping_ge, wrapping_gt, wrapping_le},
	index::{IndexDeleteActiveEnvoy, IndexInsertEnvoy, IndexUpsertActiveEnvoy},
	keys::{
		partition::ActiveEnvoyIndexVolume, partition::EnvoyIndexVolume, partition::EnvoyVolume,
		ups::EnvoySubject, ups::EnvoySupersedeSubject,
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
use std::sync::{
	Arc,
	atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering},
};
use tokio::sync::{Mutex, mpsc};
use universalpubsub::{RequestClass, Turn, Turns};

use uuid::Uuid;
use vbare::Versioned;

mod inbound;

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
	/// Last pong timestamp whose pool admission renewal was durably acknowledged.
	pool_heartbeat_unix_ms: AtomicI64,
	/// Whether a newer version superseded this envoy, and the eviction of its actors.
	version_drain: crate::version_drain::State,
	to_envoy_tx: crate::queue::Sender<ToEnvoyFrame>,
	/// Small control replies have their own bounded lane so data backpressure cannot block
	/// the socket reader before its next pong. The writer fairly drains both lanes.
	control_tx: crate::queue::Sender<ToEnvoyFrame>,
	/// Latched protocol rejection: reader and ordered dispatcher ignore later frames.
	rejected: AtomicBool,
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
	/// This process's pool names refused node connects, for the sustained refusal warning.
	missing_pools: Arc<crate::missing_pools::MissingPools>,
	connection_id: Uuid,
	version: u16,
	/// Legacy: set once this connection logged a warning for an event naming another
	/// namespace's legacy actor; later ones log at debug. Removed in 3.1 with the backfill.
	legacy_foreign_event_logged: AtomicBool,
}

impl Connection {
	/// Admission is not delivery. Lifecycle callers use this only after their durable
	/// delivery or proof that ownership ended; writer pressure retains the ACK.
	async fn ack_events(&self, checkpoints: Vec<p::ActorCheckpoint>) -> Result<()> {
		self.control_tx
			.send(p::ToEnvoy::ToEnvoyAckEvents(p::ToEnvoyAckEvents {
				last_event_checkpoints: checkpoints,
			}))
			.await?;
		Ok(())
	}

	fn send_control(&self, frame: impl Into<ToEnvoyFrame>) -> Result<()> {
		self.control_tx
			.try_send(frame.into())
			.map_err(control_send_error)
	}

	/// Only for failures known to precede execution. If even the refusal cannot be
	/// queued, let that caller time out rather than aborting unrelated accepted work.
	/// Successes, lifecycle acknowledgements and protocol rejections remain strict.
	fn send_storage_refusal(&self, reply: p::ToEnvoy) -> Result<()> {
		match self.control_tx.try_send(reply.into()) {
			Ok(()) => Ok(()),
			Err(mpsc::error::TrySendError::Full(_)) => {
				if let Ok(now) = self.ctx.env().clock().observe()
					&& self.queue_pressure.claim_warning(now.boot_ms)
				{
					tracing::warn!(
						"envoy storage refusal queue full; unadmitted request will time out"
					);
				}
				Ok(())
			}
			Err(mpsc::error::TrySendError::Closed(_)) => {
				bail!("envoy control reply writer closed")
			}
		}
	}
}

pub async fn run(
	ctx: &EngineContext,
	socket: WebSocket,
	envoy_key: String,
	scope: crate::routes::EnvoyScope,
	version: u16,
	envoy_version: u32,
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
			log_connect_rejection(ctx, &service.missing_pools, &scope, &rejection).await;
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
		envoy_version,
		now,
	));

	let queue_pressure = Arc::new(crate::queue::QueuePressure::new(ctx.env().clock()));
	let (to_envoy_tx, mut to_envoy_rx) = crate::queue::channel(
		crate::queue::QueueKind::Socket,
		ctx.config().pegboard().envoy_socket_queue_capacity(),
		queue_pressure.clone(),
	);
	let (control_tx, mut control_rx) = crate::queue::channel(
		crate::queue::QueueKind::SocketControl,
		ctx.config().pegboard().envoy_socket_queue_capacity(),
		queue_pressure.clone(),
	);
	let (inbound_tx, mut inbound_rx) = inbound::channel(
		ctx.config().pegboard().envoy_socket_queue_capacity(),
		queue_pressure.clone(),
	);
	let inbound_bytes = Arc::new(tokio::sync::Semaphore::new(SOCKET_INBOUND_BYTES));
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
			scope.group,
			legacy_namespace_check(ctx, &scope.namespace),
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
		)?
		.with_legacy_fallback(scope.legacy_fallback.clone()),
		envoy_key,
		scope,
		record,
		pool_config: Default::default(),
		last_heartbeat_unix_ms: AtomicI64::new(0),
		pool_heartbeat_unix_ms: AtomicI64::new(0),
		version_drain: Default::default(),
		to_envoy_tx,
		control_tx,
		rejected: AtomicBool::new(false),
		event_failure,
		queue_pressure,
		last_pong_unix_ms: AtomicI64::new(now),
		last_pong_boot_ms: AtomicU64::new(started.boot_ms),
		ping,
		gateway_handles: scc::HashMap::new(),
		actor_handles: scc::HashMap::new(),
		actor_handle_budget,
		missing_pools: service.missing_pools.clone(),
		connection_id,
		version,
		legacy_foreign_event_logged: AtomicBool::new(false),
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
			let peer = stream.peer_worker_id().clone();
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
				let result = handle_sub_stream(
					ctx3, conn3, tasks3, class, peer, stream_tx, stream_rx, arrival,
				)
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
		write_socket(
			&mut socket_tx,
			&mut to_envoy_rx,
			&mut control_rx,
			&mut ping_rx,
			version,
		)
		.await
	});

	let conn2 = conn.clone();
	let ctx2 = ctx.clone();
	let socket_reader_task = tasks.spawn_result(async move {
		read_socket(&ctx2, &conn2, &mut socket_rx, &inbound_tx, &inbound_bytes).await
	});
	let conn2 = conn.clone();
	let ctx2 = ctx.clone();
	let tasks2 = tasks.clone();
	let inbound_dispatch_task = tasks.spawn_result(async move {
		dispatch_inbound(&conn2, &mut inbound_rx, |message| {
			handle_ws_msg(&ctx2, &conn2, &tasks2, message)
		})
		.await
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
		run_ping_loop(
			clock, period_ms, &conn2.ping,
			|| async {
				match renew_pool_admission(&ctx2, &conn2).await {
					Ok(()) => Ok(()),
					Err(error) if error.is::<crate::envoy_record::PoolAdmissionLost>() => {
						tracing::info!("closing node connection whose pool admission expired; the runtime reconnects for fresh admission");
						Err(error)
					}
					Err(error) => {
						tracing::debug!(?error, "pool admission renewal failed; the next pong retries it");
						Ok(())
					}
				}
			},
			|| async {
				match write_heartbeat(&ctx2, &conn2).await {
					Ok(()) => {
						if let Err(error) = evict_actors_from_superseded_envoy(&ctx2, &conn2).await {
							tracing::debug!(?error, "eviction from a superseded envoy failed; the next heartbeat retries it");
						}
					}
					Err(error) if error.is::<crate::envoy_record::Superseded>() => {
						tracing::info!("closing envoy connection whose active index entries belong to a newer connection");
						return Err(error);
					}
					Err(error) => tracing::debug!(?error, "envoy heartbeat write failed; the next pong retries it"),
				}
				Ok(())
			},
		).await
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
			res = inbound_dispatch_task => res?.context("task cancelled")?,
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
	}

	res
}

/// Legacy: the uncached read of `namespace`'s current group that a legacy-bound storage open on
/// a connection without a group makes before it opens (see `sqlite::legacy_open_resolve`).
/// Removed in 3.1 with the backfill.
fn legacy_namespace_check(
	ctx: &EngineContext,
	namespace: &str,
) -> crate::sqlite::LegacyNamespaceCheck {
	let read_ctx = ctx.clone();
	let read_namespace = namespace.to_owned();
	let standing_ctx = ctx.clone();
	let standing_namespace = namespace.to_owned();
	crate::sqlite::LegacyNamespaceCheck::new(
		namespace.to_owned(),
		Arc::new(move || {
			let ctx = read_ctx.clone();
			let namespace = read_namespace.clone();
			Box::pin(async move {
				pegboard::catalog::legacy_read_namespace_group(&ctx, &namespace).await
			})
		}),
		Arc::new(move || {
			let ctx = standing_ctx.clone();
			let namespace = standing_namespace.clone();
			Box::pin(
				async move { pegboard::catalog::cached_namespace_standing(&ctx, &namespace).await },
			)
		}),
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
async fn write_heartbeat(ctx: &EngineContext, conn: &Connection) -> Result<()> {
	let (connection_epoch, version, names, heartbeat_unix_ms) = {
		let record = conn.record.lock().await;
		if !record.connected {
			return Ok(());
		}
		// Advertise only a confirmed pool lease, never a newer unconfirmed pong.
		let Some(heartbeat_unix_ms) = live_pool_heartbeat(
			conn.pool_heartbeat_unix_ms.load(Ordering::Acquire),
			ctx.env().clock().observe()?.unix_ms,
		) else {
			return Ok(());
		};
		if heartbeat_unix_ms <= conn.last_heartbeat_unix_ms.load(Ordering::Acquire) {
			return Ok(());
		}
		if record.entry.stopping {
			conn.last_heartbeat_unix_ms
				.fetch_max(heartbeat_unix_ms, Ordering::AcqRel);
			return Ok(());
		}
		(
			record.entry.connection_epoch,
			record.entry.version,
			record.entry.names.clone(),
			heartbeat_unix_ms,
		)
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
				version,
				heartbeat_unix_ms,
				names,
			},
		)
		.await?;
	if response.superseded {
		return Err(crate::envoy_record::Superseded.into());
	}
	// A late index write carries the confirmed timestamp and therefore expires with
	// that admission. Do not act on its response without current confirmed admission.
	if live_pool_heartbeat(
		conn.pool_heartbeat_unix_ms.load(Ordering::Acquire),
		ctx.env().clock().observe()?.unix_ms,
	)
	.is_none()
	{
		return Ok(());
	}
	conn.last_heartbeat_unix_ms
		.fetch_max(heartbeat_unix_ms, Ordering::AcqRel);
	if let Some(config) = conn.pool_config.get() {
		conn.version_drain
			.observe(
				&conn.scope,
				&conn.envoy_key,
				version,
				response.newest_live_version,
				config.drain_on_version_upgrade,
				ctx.env().clock().observe()?.boot_ms,
			)
			.await;
	}
	Ok(())
}

/// Asks the actors due in this heartbeat's eviction batch to move off this envoy, when a newer
/// envoy version superseded it in a pool that drains on version upgrade. The pool configuration
/// is the one captured at connect. Each move request is fenced by the actor's epoch.
async fn evict_actors_from_superseded_envoy(ctx: &EngineContext, conn: &Connection) -> Result<()> {
	let Some(config) = conn.pool_config.get() else {
		return Ok(());
	};
	if !config.drain_on_version_upgrade || !conn.version_drain.is_draining().await {
		return Ok(());
	}
	if live_pool_heartbeat(
		conn.pool_heartbeat_unix_ms.load(Ordering::Acquire),
		ctx.env().clock().observe()?.unix_ms,
	)
	.is_none()
	{
		return Ok(());
	}
	let mut bound = Vec::new();
	conn.actor_handles
		.iter_async(|actor_id, handle| {
			if let Some(epoch) = handle.epoch.get() {
				bound.push(crate::version_drain::BoundActor {
					actor_id: actor_id.clone(),
					epoch,
				});
			}
			true
		})
		.await;
	conn.version_drain
		.evict_due(
			ctx,
			&conn.scope,
			&conn.envoy_key,
			crate::version_drain::Pacing::of_pool(config),
			bound,
			ctx.env().clock().observe()?.boot_ms,
		)
		.await
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
		p::ToEnvoy::ToEnvoyCommands(_) | p::ToEnvoy::ToEnvoySqliteProveTail(_) => {
			Ok(universalpubsub::RequestClass::EnvoyCommand)
		}
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
	peer: universalpubsub::WorkerId,
	mut stream_tx: universalpubsub::PacketSender,
	mut stream_rx: universalpubsub::PacketReceiver,
	arrival: Option<CommandArrival>,
) -> Result<()> {
	let (to_recipient_tx, mut to_recipient_rx) = crate::queue::channel(
		crate::queue::QueueKind::Tunnel,
		ctx.config().pegboard().envoy_tunnel_queue_capacity(),
		conn.queue_pressure.clone(),
	);

	// The version the sender wrote its first message at, which it therefore reads. Commands
	// get no reply in the envoy protocol, so theirs is not needed.
	let (first_msg, mut turns, request_version) = match arrival {
		Some(arrival) => (
			arrival.message,
			arrival.turns,
			p::versioned::MIN_PROTOCOL_VERSION,
		),
		None => {
			let first_payload = stream_rx.recv().await?.context("expected request")?;
			let message = p::versioned::ToEnvoy::deserialize_with_embedded_version(&first_payload)?;
			let request_version = embedded_version(&first_payload)?;
			(message, Vec::new(), request_version)
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
				// so forwarding it could only start a epoch the engine no longer tracks.
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
					now_boot_ms + conn.ctx.config().pegboard().envoy_lost_threshold()
				});
				// Actor ids repeat across a namespace and its forks, so a command naming another
				// group was routed on another namespace's allocation and must not reach the actor
				// this connection hosts under the same id. Legacy: a connection opened before its
				// namespace moved into the named group closes instead, so the envoy reconnects with
				// the group and the next start reaches it. Removed in 3.1 with the backfill.
				let mut legacy_group_read = None;
				for command in &commands {
					let named = command.group.as_deref();
					let read_group = || {
						pegboard::catalog::legacy_read_namespace_group(
							&conn.ctx,
							&conn.scope.namespace,
						)
					};
					let checked =
						check_group(&conn.scope, named, &mut legacy_group_read, read_group).await?;
					let refusal = match checked {
						GroupCheck::Serves => continue,
						GroupCheck::Foreign => {
							tracing::warn!(
								actor_id = %command.checkpoint.actor_id,
								epoch = command.checkpoint.generation,
								"refusing actor commands for another namespace's actor with the same id; the sender's allocation names an envoy of another namespace"
							);
							pegboard::actor_machine::executor::COMMANDS_REFUSED_FOREIGN_GROUP
						}
						GroupCheck::LegacyNamespaceMoved => {
							tracing::info!(
								actor_id = %command.checkpoint.actor_id,
								namespace = %conn.scope.namespace,
								"actor command for a grouped actor reached a connection opened before its namespace moved into its group; closing it so the envoy reconnects with the group"
							);
							legacy_close_moved_namespace(&conn);
							pegboard::actor_machine::executor::LEGACY_COMMANDS_REFUSED_NAMESPACE_MOVED
						}
					};
					stream_tx.send(Bytes::from_static(refusal)).await?;
					stream_tx.finish().await?;
					return anyhow::Ok(());
				}
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
					let started_epoch = match &command.inner {
						p::Command::CommandStartActor(_) => Some(command.checkpoint.generation),
						p::Command::CommandStopActor(_) => None,
					};
					if let p::Command::CommandStartActor(start) = &command.inner {
						// The group check above admitted only a command naming this connection's
						// group, or none for a legacy actor, so that is the group the instance's
						// storage opens in for its whole life.
						let group = match command.group {
							Some(_) => conn.scope.group,
							None => None,
						};
						handle.bind_epoch(crate::sqlite::OwnedEpoch {
							epoch: command.checkpoint.generation,
							create_unix_ms: start.config.create_ts,
							group,
						});
						let id = rivet_util_id::Id::parse(&command.checkpoint.actor_id);
						// A start whose storage was never written, as only a first epoch's
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
							let epoch = command.checkpoint.generation;
							let create_unix_ms = start.config.create_ts;
							// Reserve before spawning and forwarding the start, so a stop can
							// retire its lane even when the open has not been polled yet.
							match conn
								.sqlite
								.prefetch(conn.ctx.depot(), id, epoch, create_unix_ms, group)
								.await
							{
								Ok(prefetch) => {
									tasks.spawn(prefetch);
								}
								Err(error) => tracing::debug!(%id, ?error, "actor database could not be prefetched; its first request opens it"),
							}
						}
					}

					handle.recv_command(command, deadline_boot_ms).await?;
					// The command is on its actor's lane, so the actor's next command may follow.
					drop(turn);
					// Requests held for this start may follow it now: it is ahead of them on the lane.
					if let Some(epoch) = started_epoch {
						handle.start_queued(epoch);
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
		p::ToEnvoy::ToEnvoySqliteProveTail(request) => {
			Box::pin(prove_sqlite_tail(&conn, stream_tx, request)).await?
		}
		p::ToEnvoy::ToEnvoyTunnelMessage(mut tunnel_msg) => {
			let mut last_message_id = tunnel_msg.message_id.clone();
			let message_key = (
				tunnel_msg.message_id.gateway_id,
				tunnel_msg.message_id.request_id,
			);

			// Figure out recipient actor. The group is the engine's routing check and is taken
			// off the message, so the envoy receives it as before.
			let (is_websocket, actor_id, actor_epoch, group) = match &mut tunnel_msg.message_kind {
				p::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(start) => (
					false,
					start.actor_id.clone(),
					start.actor_generation.context("expected epoch to be set")?,
					start.group.take(),
				),
				p::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(open) => (
					true,
					open.actor_id.clone(),
					open.actor_generation.context("expected epoch to be set")?,
					open.group.take(),
				),
				_ => bail!("unexpected first tunnel message, unable to determine recipient"),
			};

			// Actor ids repeat across a namespace and its forks, so a request naming another
			// group was routed on another namespace's allocation. It is refused before the envoy
			// sees it, as the envoy refuses an actor it does not host, and the gateway reads the
			// actor again. Legacy: a connection opened before its namespace moved into the named
			// group also closes, so the envoy reconnects with the group. Removed in 3.1 with the
			// backfill.
			let checked = check_group(&conn.scope, group.as_deref(), &mut None, || {
				pegboard::catalog::legacy_read_namespace_group(&conn.ctx, &conn.scope.namespace)
			})
			.await?;
			let refused = match checked {
				GroupCheck::Serves => false,
				GroupCheck::Foreign => {
					tracing::warn!(
						%actor_id,
						epoch = actor_epoch,
						"refusing a gateway request for another namespace's actor with the same id; the gateway routed it on an allocation of another namespace"
					);
					true
				}
				GroupCheck::LegacyNamespaceMoved => {
					tracing::info!(
						%actor_id,
						namespace = %conn.scope.namespace,
						"gateway request for a grouped actor reached a connection opened before its namespace moved into its group; closing it so the envoy reconnects with the group"
					);
					legacy_close_moved_namespace(&conn);
					true
				}
			};
			if refused {
				let reply_version =
					pegboard::envoy_protocol::reply_version(&ctx, &peer, request_version);
				let refusal = foreign_group_refusal(
					tunnel_msg.message_id.clone(),
					is_websocket,
					&actor_id,
					actor_epoch,
				)?;
				let payload = Bytes::from(p::versioned::ToRivet::serialize_with_embedded_version(
					&refusal,
					reply_version,
				)?);
				Box::pin(stream_tx.send(payload)).await?;
				return Ok(());
			}

			// Store stream sender in connection
			let gateway_handle = Arc::new(GatewayHandle {
				tx: to_recipient_tx,
				actor_id: actor_id.clone().to_string(),
				actor_epoch,
				stopped: tokio::sync::Notify::new(),
				overloaded: tokio::sync::Notify::new(),
			});
			{
				let scc::hash_map::Entry::Vacant(entry) =
					Box::pin(conn.gateway_handles.entry_async(message_key)).await
				else {
					bail!("duplicate gateway request");
				};

				entry.insert_entry(gateway_handle.clone());
			}

			// Replies to the gateway are encoded at one version for the whole stream, the one the
			// gateway's worker reads; see `pegboard::envoy_protocol`.
			let reply_version =
				pegboard::envoy_protocol::reply_version(&ctx, &peer, request_version);
			let mut stream_finalized_by_reply = false;
			let handler = async {
				let actor_handle = Box::pin(actor_handle(&conn, &tasks, &actor_id)).await?;

				// A gateway's stream can overtake its actor's start. Wait until that start
				// is on the lane ahead of it; overload cancellation also covers this setup.
				Box::pin(actor_handle.await_start(&conn, &actor_id, actor_epoch)).await;
				Box::pin(actor_handle.recv_tunnel_msg(tunnel_msg)).await?;
				loop {
					rivet_env::select!( ctx.env().rng("envoy.subs");
						// The execution stopped. Its tunnels end with it; the gateway closes the
						// client socket so the client reconnects and wakes the actor.
						_ = gateway_handle.stopped.notified() => {
							last_message_id.message_index = last_message_id.message_index.wrapping_add(1);
							let message_kind = if is_websocket {
								p::ToRivetTunnelMessageKind::ToRivetWebSocketClose(p::ToRivetWebSocketClose {
									code: Some(1012),
									reason: Some(ACTOR_STOPPED_DETAIL.into()),
									hibernate: false,
								})
							} else {
								p::ToRivetTunnelMessageKind::ToRivetResponseAbort(p::ToRivetResponseAbort {
									reason: p::HttpStreamAbortReason {
										kind: p::HttpStreamAbortReasonKind::Cancelled,
										detail: Some(ACTOR_STOPPED_DETAIL.into()),
									},
								})
							};
							let payload = Bytes::from(
								p::versioned::ToRivet::serialize_with_embedded_version(&p::ToRivet::ToRivetTunnelMessage(
									p::ToRivetTunnelMessage {
										message_id: last_message_id.clone(),
										message_kind,
									},
								), reply_version)?
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
							let payload = reply.into_payload(reply_version)?;

							Box::pin(stream_tx.send(payload)).await?;

							if reply_is_final {
								stream_finalized_by_reply = true;
								break;
							}
						},
					);
				}

				anyhow::Ok(())
			};
			let handler_res = gateway_handle.run(handler).await;

			Box::pin(
				conn.gateway_handles
					.remove_if_async(&message_key, |current| {
						Arc::ptr_eq(current, &gateway_handle)
					}),
			)
			.await;

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
						actor_generation: Some(actor_epoch),
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

// The public WebSocket already limits one frame to 32 MiB. Bound the *whole*
// queued/in-progress backlog of each class to that size, not 128 maximum-size frames.
// Lifecycle memory is reserved separately so one executing storage frame cannot starve it.
const SOCKET_INBOUND_BYTES: usize = 32 * 1024 * 1024;

struct SocketInbound {
	message: p::ToRivet,
	_bytes: tokio::sync::OwnedSemaphorePermit,
}

fn queue_inbound(
	tx: &inbound::Sender,
	bytes: &Arc<tokio::sync::Semaphore>,
	message: p::ToRivet,
	wire_bytes: usize,
) -> Result<Vec<p::ToRivet>> {
	tx.ensure_open()?;
	let count = u32::try_from(wire_bytes).context("envoy inbound frame exceeds byte budget")?;
	let storage = inbound::is_storage(&message);
	let budget = if storage { bytes } else { tx.control_bytes() };
	let permit = match budget.clone().try_acquire_many_owned(count) {
		Ok(permit) => permit,
		Err(tokio::sync::TryAcquireError::NoPermits) if storage => return Ok(vec![message]),
		Err(tokio::sync::TryAcquireError::NoPermits) => {
			bail!("envoy lifecycle byte budget exhausted; reconnect to replay unacknowledged work")
		}
		Err(tokio::sync::TryAcquireError::Closed) => bail!("envoy inbound byte budget closed"),
	};
	tx.try_send(SocketInbound {
		message,
		_bytes: permit,
	})
}

/// Only storage requests removed before ownership lookup or execution may be refused.
/// Lifecycle admission retains its reports or returns an explicit terminal resource error.
fn refuse_inbound(conn: &Connection, message: p::ToRivet) -> Result<()> {
	let reply = match message {
		p::ToRivet::ToRivetKvRequest(request) => crate::kv::error_response(
			request.request_id,
			format!(
				"depot.overloaded: node inbound queue is full (reason=node_inbound, retry_after_ms={STORAGE_LANE_RETRY_AFTER_MS})"
			),
		),
		p::ToRivet::ToRivetSqliteGetPagesRequest(_)
		| p::ToRivet::ToRivetSqliteCommitRequest(_)
		| p::ToRivet::ToRivetSqliteCommitStageBeginRequest(_)
		| p::ToRivet::ToRivetSqliteCommitStageSegmentRequest(_)
		| p::ToRivet::ToRivetSqliteCommitFinalizeRequest(_)
		| p::ToRivet::ToRivetSqliteExecRequest(_)
		| p::ToRivet::ToRivetSqliteExecuteRequest(_)
		| p::ToRivet::ToRivetSqliteExecuteBatchRequest(_) => {
			crate::sqlite::inbound_overloaded_failure(&message, STORAGE_LANE_RETRY_AFTER_MS)?
		}
		_ => bail!("envoy inbound queue full; reconnect to replay unacknowledged work"),
	};
	conn.send_storage_refusal(reply)
}

/// Reads heartbeat traffic without waiting for actor ownership lookup, registration,
/// durable event delivery or response queue capacity. All other ordered work uses one
/// bounded dispatcher; storage admission failures are refused without disconnecting
/// accepted execution. Pending storage may be refused to retain lifecycle reports.
/// Only exhaustion of lifecycle resources itself requires reconnect/replay.
async fn read_socket<S, E>(
	ctx: &EngineContext,
	conn: &Arc<Connection>,
	socket_rx: &mut S,
	inbound_tx: &inbound::Sender,
	inbound_bytes: &Arc<tokio::sync::Semaphore>,
) -> Result<()>
where
	S: futures_util::Stream<Item = std::result::Result<Message, E>> + Unpin,
	E: std::error::Error + Send + Sync + 'static,
{
	while let Some(message) = socket_rx.next().await {
		match message? {
			Message::Binary(_) if conn.rejected.load(Ordering::Acquire) => {}
			Message::Binary(payload) => {
				let wire_bytes = payload.len();
				// The envoy speaks the connection's version without a header. At the latest
				// version the message is viewed in place, and tunnel replies are forwarded
				// to their gateway without being decoded.
				let decoded = if conn.version == p::PROTOCOL_VERSION {
					match p::versioned::ToRivet::open_body(payload, conn.version) {
						Ok(view) => {
							if let p::ToRivetRef::ToRivetTunnelMessage(message) = view.get() {
								let message_id = message.message_id();
								let message_key =
									(*message_id.gateway_id(), *message_id.request_id());
								forward_tunnel_reply(conn, message_key, TunnelReply::Encoded(view))
									.await?;
								continue;
							}
							Ok(view.to_owned())
						}
						Err(error) => Err(error),
					}
				} else {
					p::versioned::ToRivet::decode_body(payload, conn.version)
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
						// A known protocol failure must not wait for a pending ownership
						// lookup. Earlier unacknowledged work remains replayable.
						conn.rejected.store(true, Ordering::Release);
						conn.send_control(ToEnvoyFrame::Reject(rejection))?;
						continue;
					}
				};
				match msg {
					p::ToRivet::ToRivetPong(_) => record_pong(ctx, conn)?,
					// Command ACKs currently have no effects or ordering requirements.
					p::ToRivet::ToRivetAckCommands(_) => {}
					msg => {
						for refused in queue_inbound(inbound_tx, inbound_bytes, msg, wire_bytes)? {
							refuse_inbound(conn, refused)?;
						}
						// Buffered frames can all be ready without another I/O poll. Give the
						// dispatcher a turn; this never waits on its storage work.
						rivet_env::tokio::task::yield_now().await;
					}
				}
			}
			Message::Close(_) => break,
			_ => {}
		}
	}

	Ok(())
}

/// A single FIFO preserves admission order even when a start concurrently binds a handle.
/// An unknown actor is validated before it can allocate retained actor lanes.
async fn dispatch_inbound<F, Fut>(
	conn: &Connection,
	rx: &mut inbound::Receiver,
	mut handle: F,
) -> Result<()>
where
	F: FnMut(p::ToRivet) -> Fut,
	Fut: std::future::Future<Output = Result<Option<crate::rejection::Rejection>>>,
{
	while let Some(SocketInbound { message, _bytes }) = rx.recv().await {
		if conn.rejected.load(Ordering::Acquire) {
			continue;
		}
		let rejection = handle(message).await?;
		if let Some(rejection) = rejection {
			log_connect_rejection(&conn.ctx, &conn.missing_pools, &conn.scope, &rejection).await;
			conn.rejected.store(true, Ordering::Release);
			let namespace_moved = rejection
				.error
				.is::<crate::lifecycle::LegacyNamespaceMoved>();
			let sent = conn.control_tx.try_send(ToEnvoyFrame::Reject(rejection));
			// Legacy: a moved namespace's close that finds the control queue full ends the
			// connection with the typed error, which is expected, like the event lane's. A closed
			// writer is not a full queue and fails below. Removed in 3.1 with the backfill.
			if namespace_moved && matches!(sent, Err(mpsc::error::TrySendError::Full(_))) {
				tracing::info!(
					namespace = %conn.scope.namespace,
					"envoy control queue full while closing a moved namespace's connection; closing it without a reason so the envoy reconnects and replays"
				);
				return Err(crate::lifecycle::LegacyNamespaceMoved.into());
			}
			sent.map_err(control_send_error)?;
		}
	}
	Ok(())
}

/// The error for a control frame the envoy's control queue did not accept.
fn control_send_error<T>(error: mpsc::error::TrySendError<T>) -> anyhow::Error {
	match error {
		mpsc::error::TrySendError::Full(_) => {
			anyhow::anyhow!(
				"envoy control reply queue full; reconnect to replay unacknowledged work"
			)
		}
		mpsc::error::TrySendError::Closed(_) => {
			anyhow::anyhow!("envoy control reply writer closed")
		}
	}
}

fn record_pong(ctx: &EngineContext, conn: &Connection) -> Result<()> {
	let now = ctx.env().clock().observe()?;
	conn.last_pong_unix_ms.store(now.unix_ms, Ordering::Release);
	conn.last_pong_boot_ms.store(now.boot_ms, Ordering::Release);
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

			conn.control_tx
				.send(p::ToEnvoy::ToEnvoyInit(p::ToEnvoyInit {
					metadata: p::ProtocolMetadata {
						envoy_lost_threshold: i64::try_from(
							ctx.config().pegboard().envoy_lost_threshold(),
						)
						.context("pegboard.node_lost_threshold exceeds the protocol's range")?,
						actor_stop_threshold: i64::try_from(
							ctx.config().pegboard().actor_stop_threshold(),
						)
						.context("pegboard.actor_stop_threshold exceeds the protocol's range")?,
						max_response_payload_size: 32 * 1024 * 1024,
					},
				}))
				.await?;
		}
		p::ToRivet::ToRivetPong(_) => record_pong(ctx, conn)?,
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
			// This ordered dispatcher is separate from the socket reader. Wait for bounded
			// actor-lane capacity instead of disconnecting on a legitimate replay batch.
			// Delivery still owns the ACK, so admission cannot acknowledge past a failure.
			let mut orphan_checkpoints = Vec::new();
			let mut legacy_group_checked = false;
			for event in events {
				let Some(handle) = inbound_actor_handle(
					conn,
					tasks,
					&event.checkpoint.actor_id,
					OwnershipUse::Events,
				)
				.await?
				else {
					let unowned = unowned_event(&conn.scope, &mut legacy_group_checked, || {
						pegboard::catalog::legacy_read_namespace_group(ctx, &conn.scope.namespace)
					})
					.await?;
					match unowned {
						UnownedEvent::Acknowledge => {}
						UnownedEvent::LegacyNamespaceMoved => {
							tracing::info!(
								actor_id = %event.checkpoint.actor_id,
								namespace = %conn.scope.namespace,
								epoch = event.checkpoint.generation,
								"replayed actor event reached a connection opened before its namespace moved into its group; closing it so the envoy reconnects and replays"
							);
							return Ok(Some(crate::rejection::Rejection::legacy_namespace_moved()));
						}
					}
					// A replay from an epoch whose ownership already ended has no side effects.
					// Acknowledge it without retaining lanes for an actor no longer hosted here.
					orphan_checkpoints.push(event.checkpoint);
					continue;
				};
				if let Err(error) = handle.recv_event(event).await {
					return Ok(Some(crate::rejection::Rejection {
						frame: CloseFrame {
							code: crate::routes::TRY_AGAIN_LATER,
							reason: "envoy.event_lane_closed;retry_after_ms=100".into(),
						},
						error,
					}));
				}
			}
			if !orphan_checkpoints.is_empty() {
				conn.ack_events(orphan_checkpoints).await?;
			}
		}
		p::ToRivet::ToRivetTunnelMessage(message) => {
			let message_key = (message.message_id.gateway_id, message.message_id.request_id);
			forward_tunnel_reply(conn, message_key, TunnelReply::from(message)).await?;
		}
		p::ToRivet::ToRivetKvRequest(request) => {
			let Some(handle) =
				inbound_actor_handle(conn, tasks, &request.actor_id, OwnershipUse::Storage).await?
			else {
				conn.send_storage_refusal(crate::kv::error_response(
					request.request_id,
					"actor is not running on this envoy".into(),
				))?;
				return Ok(None);
			};
			if let Some(refused) = handle.try_recv_kv(request)? {
				conn.send_storage_refusal(crate::kv::error_response(
					refused.request_id,
					storage_lane_full_message(),
				))?;
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
			let Some(handle) =
				inbound_actor_handle(conn, tasks, &actor_id, OwnershipUse::Storage).await?
			else {
				conn.send_storage_refusal(crate::sqlite::not_running_failure(&msg)?)?;
				return Ok(None);
			};
			if let Some(refused) = handle.try_recv_sqlite(msg)? {
				conn.send_storage_refusal(crate::sqlite::overloaded_failure(
					&refused,
					STORAGE_LANE_RETRY_AFTER_MS,
				)?)?;
			}
		}
	}

	Ok(None)
}

async fn log_connect_rejection(
	ctx: &EngineContext,
	missing_pools: &crate::missing_pools::MissingPools,
	scope: &crate::routes::EnvoyScope,
	rejection: &crate::rejection::Rejection,
) {
	// Legacy: a moved namespace's close is expected and logged where it is decided. Removed in
	// 3.1 with the backfill.
	if rejection
		.error
		.is::<crate::lifecycle::LegacyNamespaceMoved>()
	{
		return;
	}
	// A node that connects before its pool is written is an expected race of deploys and
	// scripts that start nodes and write pools concurrently; the close reason tells the node,
	// and the reconnect after the pool write registers it. Refusals of one pool name that
	// persist warn once, since that node's pool may never appear.
	if expected_connect_rejection(&rejection.error) {
		let refusal = match ctx.env().clock().observe() {
			Ok(now) => {
				missing_pools
					.refused(&scope.namespace, &scope.pool_name, now.boot_ms)
					.await
			}
			Err(error) => {
				tracing::debug!(
					?error,
					"clock unavailable; not tracking a missing pool refusal"
				);
				crate::missing_pools::Refusal::Expected
			}
		};
		match refusal {
			crate::missing_pools::Refusal::Expected => tracing::debug!(
				code = rejection.frame.code,
				reason = %rejection.frame.reason,
				"refusing envoy connect for a pool that does not exist yet; the envoy reconnects"
			),
			crate::missing_pools::Refusal::Sustained {
				count,
				refused_for_ms,
			} => tracing::warn!(
				namespace = %scope.namespace,
				pool_name = %scope.pool_name,
				count,
				refused_for_ms,
				"node connects keep being refused because their pool does not exist; create the pool or fix the nodes' pool name"
			),
		}
		return;
	}
	tracing::warn!(
		code = rejection.frame.code,
		reason = %rejection.frame.reason,
		error = ?rejection.error,
		"refusing envoy connect that storage could not register; the envoy reconnects"
	);
}

/// Whether a connect refusal is an expected race rather than a storage failure: the node's pool
/// does not exist yet.
fn expected_connect_rejection(error: &anyhow::Error) -> bool {
	let error = rivet_error::RivetError::extract(error);
	error.group() == "pool" && error.code() == "not_found"
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
		// A missing pool or admission record requires a new registration. A stale
		// heartbeat fails the attempt above and the next pong can retry.
		_ if renew => Err(crate::envoy_record::PoolAdmissionLost.into()),
		_ => Err(pegboard::validation::PoolNotFound.build()),
	}
}

/// Captures the pool configuration, writes the connection's record and indexes, then asks
/// the connection it replaced, if any, to close. Configuration stays fixed until disconnect.
async fn connect(ctx: &EngineContext, conn: &Arc<Connection>, names: Vec<String>) -> Result<()> {
	pegboard::catalog::ensure_namespace(ctx, &conn.scope.namespace).await?;
	let admitted_unix_ms = ctx.env().clock().observe()?.unix_ms;
	let config = register_pool_connection(ctx, conn, admitted_unix_ms, false).await?;
	conn.pool_heartbeat_unix_ms
		.fetch_max(admitted_unix_ms, Ordering::AcqRel);
	conn.missing_pools
		.admitted(&conn.scope.namespace, &conn.scope.pool_name)
		.await;
	let _ = conn.pool_config.set(config);

	let is_new = conn.record.lock().await.is_new();
	write_record(ctx, conn, true, |entry| {
		entry.names = names.into_iter().map(Into::into).collect();
	})
	.await?;

	// The listing index is keyed by creation time, which the record keeps across connections,
	// so an envoy enters it once. A retry of a lost reply writes the same key.
	let create_unix_ms = conn.record.lock().await.entry.create_unix_ms;
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
	write_heartbeat(ctx, conn).await?;

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

/// The catalog lease is the authority for advertising this node, not a recent pong alone.
fn live_pool_heartbeat(confirmed_unix_ms: i64, now_unix_ms: i64) -> Option<i64> {
	(confirmed_unix_ms > 0
		&& now_unix_ms < confirmed_unix_ms.saturating_add(pegboard::ENVOY_HEARTBEAT_TIMEOUT_MS))
	.then_some(confirmed_unix_ms)
}

/// One renewal at a time, serialized with connection record changes. Index work does
/// not hold this lock while awaiting its store, so a slow index cannot expire admission.
async fn renew_pool_admission(ctx: &EngineContext, conn: &Connection) -> Result<()> {
	let record = conn.record.lock().await;
	if !record.connected {
		return Ok(());
	}
	let heartbeat_unix_ms = conn.last_pong_unix_ms.load(Ordering::Acquire);
	if heartbeat_unix_ms <= conn.pool_heartbeat_unix_ms.load(Ordering::Acquire) {
		return Ok(());
	}
	register_pool_connection(ctx, conn, heartbeat_unix_ms, true).await?;
	conn.pool_heartbeat_unix_ms
		.fetch_max(heartbeat_unix_ms, Ordering::AcqRel);
	Ok(())
}

/// Publishes pings and renews admission independently of the serialized index refresh.
/// An old active index entry makes a node ineligible for new allocations; it does not
/// revoke actors on a connected node. Epoch teardown still requires a stopped report or
/// proof of disconnection (`pegboard::actor_machine::executor::envoy_gone`).
async fn run_ping_loop<R, Renew, F, Fut>(
	clock: Arc<dyn rivet_env::Clock>,
	period_ms: u64,
	ping: &tokio::sync::watch::Sender<i64>,
	mut renew: R,
	mut refresh: F,
) -> Result<()>
where
	R: FnMut() -> Renew,
	Renew: std::future::Future<Output = Result<()>>,
	F: FnMut() -> Fut,
	Fut: std::future::Future<Output = Result<()>>,
{
	let publish = async {
		let mut interval = rivet_env::interval(clock.clone(), period_ms);
		loop {
			interval.tick().await?;
			ping.send_replace(clock.observe()?.unix_ms);
		}
		#[allow(unreachable_code)]
		anyhow::Ok(())
	};
	let admission = async {
		let mut interval = rivet_env::interval(clock.clone(), period_ms);
		loop {
			interval.tick().await?;
			renew().await?;
		}
		#[allow(unreachable_code)]
		anyhow::Ok(())
	};
	let persist = async {
		let mut interval = rivet_env::interval(clock.clone(), period_ms);
		loop {
			interval.tick().await?;
			refresh().await?;
		}
		#[allow(unreachable_code)]
		anyhow::Ok(())
	};
	// At most one renewal and one index refresh run. Terminal errors end the
	// connection; its owner drops all three futures, without detached work.
	rivet_env::select_biased!("terminal admission or supersession failure ends local liveness",
		result = admission => result,
		result = persist => result,
		result = publish => result,
	)
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

/// Fairly drains the bounded data and control lanes until both close or a close frame
/// is written. Pings go first; control replies wait behind at most one data frame, and
/// sustained control traffic cannot starve data responses.
async fn write_socket<S>(
	socket_tx: &mut S,
	to_envoy_rx: &mut crate::queue::Receiver<ToEnvoyFrame>,
	control_rx: &mut crate::queue::Receiver<ToEnvoyFrame>,
	ping_rx: &mut tokio::sync::watch::Receiver<i64>,
	version: u16,
) -> Result<()>
where
	S: futures_util::Sink<Message> + Unpin,
	S::Error: std::error::Error + Send + Sync + 'static,
{
	let data = futures_util::stream::unfold(to_envoy_rx, |rx| async {
		rx.recv().await.map(|frame| (frame, rx))
	});
	let control = futures_util::stream::unfold(control_rx, |rx| async {
		rx.recv().await.map(|frame| (frame, rx))
	});
	futures_util::pin_mut!(data, control);
	// `select` alternates the first polled stream: neither control nor data can starve
	// the other, and closing either lane still drains the remaining one.
	let mut frames = futures_util::stream::select(data, control);
	loop {
		let next = rivet_env::select_biased!("a pending ping goes ahead of queued frames",
			changed = ping_rx.changed() => match changed {
				Ok(()) => WriterNext::Ping(*ping_rx.borrow_and_update()),
				// The connection owns the sender, so it outlives this writer.
				Err(_) => WriterNext::Frame(None),
			},
			frame = frames.next() => WriterNext::Frame(frame),
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
				// A rejection may overtake queued data. Unanswered requests and events
				// remain unacknowledged when the connection closes.
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

/// What the socket reader does with an envoy event for an actor this connection does not own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnownedEvent {
	/// Acknowledge the event: a replay from an epoch whose ownership already ended has no side
	/// effects.
	Acknowledge,
	/// Legacy: close the connection and leave the event unacknowledged, since the connection's
	/// namespace moved into its group after connect and the actor may live there. Removed in 3.1
	/// with the backfill.
	LegacyNamespaceMoved,
}

/// Decides what happens to an event for an actor this connection does not own. A grouped scope
/// acknowledges it. Legacy: a scope without a group reads only unprefixed volumes, so a grouped
/// actor reads as not owned there; `namespace_group` reads the namespace's group now (see
/// `pegboard::catalog::legacy_read_namespace_group`), and a group closes the connection
/// instead of acknowledging. A read that found no group is remembered in `legacy_group_checked`
/// for the rest of the batch, so a batch costs at most one read. The legacy behavior acknowledged
/// every such event; the new behavior closes when the namespace moved since connect. Removed in
/// 3.1 with the backfill.
///
/// The read goes straight to the namespace volume and does not depend on this process's view of
/// the fleet or on the legacy catalog row: a process whose view lags would otherwise skip the
/// volume, find the row the move deleted missing, and acknowledge a grouped actor's replayed
/// event, dropping it. It runs only on a miss in a scope without a group, which is a replay, at
/// most once per batch, and a failed read ends the connection without acknowledging. A grouped
/// actor exists only after its namespace's move stored the group on the namespace volume, and
/// the read runs after the batch carrying its events arrived, so it sees the group.
async fn unowned_event<F, Fut>(
	scope: &crate::routes::EnvoyScope,
	legacy_group_checked: &mut bool,
	namespace_group: F,
) -> Result<UnownedEvent>
where
	F: FnOnce() -> Fut,
	Fut: std::future::Future<Output = Result<Option<rivet_keys::keys::codec::GroupId>>>,
{
	if scope.group.is_some() || *legacy_group_checked {
		return Ok(UnownedEvent::Acknowledge);
	}
	match namespace_group().await? {
		Some(_) => Ok(UnownedEvent::LegacyNamespaceMoved),
		None => {
			*legacy_group_checked = true;
			Ok(UnownedEvent::Acknowledge)
		}
	}
}

/// Only inbound messages for an actor owned by this Envoy may allocate retained lanes.
/// Reconnected actors resolve their ownership from storage before a handle exists. `purpose`
/// is what the message does: a handle made for an event through the legacy moved-actor
/// fallback is left unbound, so a KV or SQLite request on it resolves its ownership again
/// without the fallback and is refused.
async fn inbound_actor_handle(
	conn: &Arc<Connection>,
	tasks: &Arc<TaskScope>,
	actor_id: &str,
	purpose: OwnershipUse,
) -> Result<Option<Arc<ActorHandle>>> {
	let existing = conn
		.actor_handles
		.read_async(actor_id, |_, handle| handle.clone())
		.await;
	let unbound = BoundEpoch::default();
	let bound = existing
		.as_ref()
		.map_or(&unbound, |handle| handle.epoch.as_ref());
	let Some(owned) = resolve_owner(
		&conn.scope,
		&conn.envoy_key,
		actor_id,
		bound,
		conn.as_ref(),
		purpose,
	)
	.await?
	else {
		return Ok(None);
	};
	// The actor's own record binds the epoch; the legacy fallback never does.
	let bind = match bound.owned() {
		Some(_) => Some(owned),
		None => None,
	};
	match existing {
		Some(handle) => Ok(Some(handle)),
		None => actor_handle_with_incarnation(conn, tasks, actor_id, bind)
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
	owned: Option<crate::sqlite::OwnedEpoch>,
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
			if let Some(owned) = owned {
				handle.bind_epoch(owned);
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
	epoch: Arc<BoundEpoch>,
	/// The newest epoch whose start command this connection has queued on the actor's
	/// lane. Requests held by `await_start` watch it.
	start_queued: tokio::sync::watch::Sender<Option<u32>>,
	/// The newest epoch the actor's record showed already started, as one more than it or
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

		let epoch = Arc::new(BoundEpoch::default());
		let conn2 = conn.clone();
		let epoch2 = epoch.clone();
		let storage_actor_id = actor_id.clone();
		// The event lane keeps a sender to the storage lane for its stop barrier, so the
		// storage lane ends only after the event lane.
		let barrier_tx = storage_tx.clone();
		tasks.spawn(async move {
			if let Err(err) = ActorHandle::run_inbound(
				&conn2,
				&actor_id,
				&epoch2,
				inbound_rx,
				Some(&barrier_tx),
			)
			.await
			{
				// Legacy: a connection whose namespace moved into its group closes with a typed
				// reason after every frame queued before it, so the envoy reconnects with the
				// grouped scope and replays. A full socket queue falls back to the plain failure.
				// Removed in 3.1 with the backfill.
				//
				// The Reject path does not report to the owner, since the owner would cancel the
				// writer before it sends the close. The connection then ends when the writer
				// reaches the Reject, after the frames queued before it; the other lanes keep
				// running until then, and a ping timeout bounds a writer the envoy stopped
				// draining. A stale lane cannot acknowledge a grouped actor's event meanwhile,
				// since its delivery closes the same way.
				if err.is::<crate::lifecycle::LegacyNamespaceMoved>() {
					let rejection = crate::rejection::Rejection::legacy_namespace_moved();
					if conn2.to_envoy_tx.try_send(ToEnvoyFrame::Reject(rejection)).is_ok() {
						return;
					}
					tracing::info!(%actor_id, "envoy socket queue full while closing a moved namespace's connection; closing it without a reason so the envoy reconnects and replays");
				} else {
					tracing::error!(%actor_id, ?err, "actor event lane failed; closing connection to replay unacknowledged events");
				}
				// Only the first failure is needed. The owner cancels and joins all lanes,
				// even if the socket queue is full or its writer is blocked.
				let _ = conn2.event_failure.try_send(err);
			}
		});

		let conn2 = conn.clone();
		let epoch2 = epoch.clone();
		tasks.spawn(async move {
			if let Err(err) =
				ActorHandle::run_inbound(&conn2, &storage_actor_id, &epoch2, storage_rx, None).await
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
			epoch,
			start_queued: tokio::sync::watch::Sender::new(None),
			started_elsewhere: AtomicU64::new(0),
		}
	}

	/// Records that `epoch`'s start command is queued on the lane.
	fn start_queued(&self, epoch: u32) {
		self.start_queued.send_if_modified(|current| {
			if current.is_some_and(|current| wrapping_ge(current, epoch)) {
				return false;
			}
			*current = Some(epoch);
			true
		});
	}

	/// Holds a gateway request for `epoch` until its start command is queued ahead of it,
	/// for at most `pegboard.envoy_start_wait_ms`. A start this connection did not forward, such
	/// as one delivered before a reconnect, is found in the actor's record instead. The request
	/// is forwarded however the wait ends; an Envoy without the actor refuses it as not found.
	///
	/// Ordered UPS lanes do not make this hold unnecessary. They order requests from one sender
	/// to one destination, but the start comes from the pegboard executor as a request and the
	/// gateway request comes from a gateway as a stream, so nothing orders one behind the other.
	async fn await_start(&self, conn: &Connection, actor_id: &str, epoch: u32) {
		let is_queued =
			|queued: &Option<u32>| queued.is_some_and(|queued| wrapping_ge(queued, epoch));
		if is_queued(&self.start_queued.borrow())
			|| decode_epoch(self.started_elsewhere.load(Ordering::Acquire))
				.is_some_and(|started| wrapping_ge(started, epoch))
			|| self.epoch().is_some_and(|bound| wrapping_gt(bound, epoch))
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
			started_elsewhere(conn, actor_id, epoch),
		)
		.await;
		if let Ok(Ok(true)) = started {
			advance_epoch(&self.started_elsewhere, epoch);
			return;
		}

		let forwarded = rivet_env::timeout_at(clock.as_ref(), deadline, queued.wait_for(is_queued))
			.await
			.is_ok();
		if !forwarded {
			tracing::debug!(
				%actor_id,
				epoch,
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
					// TODO: also send `CommandStopActor` for any epoch that is no longer
					// current when its storage request or event is refused, so a ghost epoch
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
		epoch: &BoundEpoch,
		mut rx: crate::queue::Receiver<QueuedInbound>,
		storage: Option<&crate::queue::Sender<QueuedInbound>>,
	) -> Result<()> {
		// Each message is handled in a boxed future so the lane task, which lives as long as
		// its actor and is idle almost all the time, does not reserve space for the largest
		// request inline. Inline, the SQLite path made every held actor's lane future about
		// 90 KiB.
		while let Some(QueuedInbound { msg }) = rx.recv().await {
			Box::pin(Self::handle_inbound(conn, actor_id, epoch, msg, storage)).await?;
		}

		Ok(())
	}

	async fn handle_inbound(
		conn: &Arc<Connection>,
		actor_id: &str,
		epoch: &BoundEpoch,
		msg: InboundMsg,
		storage: Option<&crate::queue::Sender<QueuedInbound>>,
	) -> Result<()> {
		match msg {
			InboundMsg::Drained(barrier) => reach_stop_barrier(epoch, *barrier),
			InboundMsg::Kv(request) => {
				let reply = kv_request(conn, actor_id, epoch, *request).await;
				conn.to_envoy_tx.send(reply).await?;
			}
			InboundMsg::Sqlite(request) => {
				if let Some(expected) = crate::sqlite::expected_epoch(&request)
					&& epoch.is_stopped(expected)
				{
					let reply = crate::sqlite::stopped_failure(&request, expected)?;
					conn.to_envoy_tx.send(reply).await?;
					return Ok(());
				}
				let owned = owned_epoch(conn, actor_id, epoch).await;
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
				// when both shared one lane: the stopped epoch's writes finish before the
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
					// forever. The stopped epoch is tombstoned so its queued requests fail
					// closed, and its session is retired without waiting for the stuck request.
					if !drained {
						tracing::warn!(
							%actor_id,
							epoch = checkpoint.generation,
							"actor storage lane did not drain before the stop's delivery budget; a storage request is stuck, failing the stopped epoch's remaining requests"
						);
						epoch.stop(checkpoint.generation);
						if let Ok(id) = rivet_util_id::Id::parse(actor_id) {
							conn.sqlite.retire_now(id).await;
						}
					}
				}
				// The stopped epoch's session is retired before the stop is delivered, so
				// no later epoch can have taken it yet, and only a session of this
				// epoch or older is retired. Its database volume closes.
				if stopped && let Ok(id) = rivet_util_id::Id::parse(actor_id) {
					conn.sqlite.stop(id, checkpoint.generation).await;
				}
				// Stop this lane before any later checkpoint can cumulatively acknowledge
				// past this event. Its owner closes the connection to trigger SDK replay.
				ensure!(
					crate::lifecycle::deliver(
						&conn.ctx,
						&conn.scope,
						&conn.legacy_foreign_event_logged,
						actor_id,
						*event
					)
					.await?,
					"actor lifecycle event delivery exhausted"
				);

				conn.ack_events(vec![checkpoint.clone()]).await?;

				// A stopped epoch needs no lanes or SQLite session on this
				// connection. Dropping the handle closes its lanes once their queued
				// work drains.
				if stopped {
					stop_tunnels(conn, actor_id, checkpoint.generation).await;
					conn.actor_handles
						.remove_if_async(actor_id, |handle| {
							handle
								.epoch()
								.is_none_or(|epoch| epoch == checkpoint.generation)
						})
						.await;
				}
			}
		}

		Ok(())
	}

	fn bind_epoch(&self, owned: crate::sqlite::OwnedEpoch) {
		self.epoch.bind(owned);
	}

	fn epoch(&self) -> Option<u32> {
		self.epoch.get()
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
			.send(QueuedInbound::new(InboundMsg::Event(Box::new(event))))
			.await
			.map_err(|_| anyhow::anyhow!("actor event lane closed; reconnect to replay"))
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

/// The epoch this connection started for one actor, with the incarnation and group it was
/// bound to. Storage requests carry the epoch so the actor volume can reject requests from
/// an execution that no longer owns the actor; the incarnation and group name the database
/// volume, which the epoch opens for its whole life without resolving it again.
///
/// A stopped epoch leaves a tombstone: it and every older epoch own nothing on this
/// connection afterward, whatever was bound or the actor's record says, so their late storage
/// requests fail closed.
///
/// The envoy protocol calls the actor epoch a generation, so every `checkpoint.generation` and
/// `expected_generation` read off a message is the epoch tracked here.
#[derive(Default)]
struct BoundEpoch {
	/// The bound epoch, or `None` before a bind. A bind replaces all its fields at once.
	/// Forced-sync: read on every storage request from sync accessors and never held across an
	/// await.
	bound: parking_lot::Mutex<Option<crate::sqlite::OwnedEpoch>>,
	/// One more than the highest stopped epoch, or 0.
	stopped: AtomicU64,
}

impl BoundEpoch {
	fn bind(&self, owned: crate::sqlite::OwnedEpoch) {
		*self.bound.lock() = Some(owned);
	}

	/// The bound epoch with its incarnation and group, unless it stopped.
	fn owned(&self) -> Option<crate::sqlite::OwnedEpoch> {
		let owned = (*self.bound.lock())?;
		match self.is_stopped(owned.epoch) {
			true => None,
			false => Some(owned),
		}
	}

	fn get(&self) -> Option<u32> {
		self.owned().map(|owned| owned.epoch)
	}

	/// Records that `epoch` and every older one stopped on this connection.
	fn stop(&self, epoch: u32) {
		advance_epoch(&self.stopped, epoch);
	}

	/// Whether `epoch` stopped here. An epoch the protocol encodes outside the epoch
	/// range never stopped; admission rejects it separately.
	fn is_stopped(&self, epoch: impl Into<u64>) -> bool {
		let Ok(epoch) = u32::try_from(epoch.into()) else {
			return false;
		};
		decode_epoch(self.stopped.load(Ordering::Acquire))
			.is_some_and(|stopped| wrapping_le(epoch, stopped))
	}
}

/// The epoch an atomic holds as one more than it, or `None` for 0.
fn decode_epoch(encoded: u64) -> Option<u32> {
	match encoded {
		0 => None,
		encoded => u32::try_from(encoded - 1).ok(),
	}
}

/// Advances an atomic that holds a epoch as one more than it to `epoch`, unless it
/// already holds a later one on the epoch ring.
fn advance_epoch(slot: &AtomicU64, epoch: u32) {
	let _ = slot.try_update(
		Ordering::AcqRel,
		Ordering::Acquire,
		|current| match decode_epoch(current) {
			Some(current) if wrapping_ge(current, epoch) => None,
			Some(_) | None => Some(u64::from(epoch) + 1),
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

/// Whether the actor's record shows `epoch` already started, or a newer epoch, so
/// its start was delivered on another connection and no start is coming on this one.
async fn started_elsewhere(conn: &Connection, actor_id: &str, epoch: u32) -> Result<bool> {
	let id = rivet_util_id::Id::parse(actor_id)?;
	let (_group, read) = conn.scope.read_actor(&conn.ctx, id).await?;
	// A destroyed actor's state volume is deleted; no start is coming for it here.
	let Some(read) = read else {
		return Ok(false);
	};
	if wrapping_gt(read.epoch, epoch) {
		return Ok(true);
	}
	let started = read.epoch == epoch
		&& match read.phase {
			Some(wire::ActorPhase::Running(_)) | Some(wire::ActorPhase::Stopping(_)) => true,
			Some(wire::ActorPhase::Starting(_))
			| Some(wire::ActorPhase::Initializing(_))
			| Some(wire::ActorPhase::Sleeping(_))
			| Some(wire::ActorPhase::Destroying(_))
			| Some(wire::ActorPhase::Destroyed)
			| Some(wire::ActorPhase::MigratingFromV2(_))
			| Some(wire::ActorPhase::DelegatedToV2)
			| None => false,
		};
	Ok(started)
}

/// What an ownership lookup admits the epoch it finds to do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OwnershipUse {
	/// KV and SQLite requests, which open the actor's database. Only the epoch the
	/// actor's own current record places on this envoy owns it.
	Storage,
	/// Lifecycle event delivery. Legacy: a moved random-id actor's old incarnation still
	/// stopping on this envoy also owns its events, so its stopped report reaches it. Removed in
	/// 3.1 with the backfill.
	Events,
}

/// The actor records an ownership lookup reads.
trait OwnerReads {
	/// The actor's current record as this connection resolves it, with the group it lives in,
	/// or `None` once the actor is destroyed.
	async fn current(
		&self,
		actor_id: rivet_util_id::Id,
	) -> Result<(
		Option<rivet_keys::keys::codec::GroupId>,
		Option<wire::ActorReadResponse>,
	)>;

	/// Legacy: the actor's unprefixed record, or `None` once its volume is gone. Removed in 3.1
	/// with the backfill.
	async fn legacy_unprefixed(
		&self,
		actor_id: rivet_util_id::Id,
	) -> Result<Option<wire::ActorReadResponse>>;
}

impl OwnerReads for Connection {
	async fn current(
		&self,
		actor_id: rivet_util_id::Id,
	) -> Result<(
		Option<rivet_keys::keys::codec::GroupId>,
		Option<wire::ActorReadResponse>,
	)> {
		self.scope.read_actor(&self.ctx, actor_id).await
	}

	async fn legacy_unprefixed(
		&self,
		actor_id: rivet_util_id::Id,
	) -> Result<Option<wire::ActorReadResponse>> {
		let read = self
			.ctx
			.depot()
			.command::<pegboard::commands::ActorRead>(
				&pegboard::keys::partition::ActorVolume::in_namespace(None, actor_id),
				(),
			)
			.await;
		match read {
			Ok(read) => Ok(Some(read)),
			Err(error) if pegboard::commands::is_volume_gone(&error) => Ok(None),
			Err(error) => Err(error),
		}
	}
}

/// Resolves the epoch that owns the actor's storage on this envoy, with the incarnation whose
/// database volume it uses. A connection that did not deliver the start, such as one opened
/// after a reconnect, learns both from the actor's record.
async fn owned_epoch(
	conn: &Connection,
	actor_id: &str,
	bound: &BoundEpoch,
) -> Result<Option<crate::sqlite::OwnedEpoch>> {
	resolve_owner(
		&conn.scope,
		&conn.envoy_key,
		actor_id,
		bound,
		conn,
		OwnershipUse::Storage,
	)
	.await
}

/// Resolves the epoch that owns the actor on this envoy for `purpose`, with the incarnation
/// whose database volume it uses. An epoch the actor's current record places here is bound to
/// `bound`, so later requests skip the read.
///
/// Legacy: for [`OwnershipUse::Events`] only, when the current record is the backfill's copy of
/// a random-id actor, which keeps the actor's id, and this connection does not host it, the
/// legacy actor it was moved from may still be stopping on this envoy, as after a reconnect
/// once the backfill is done, when the scope no longer falls back for the actor. Its record
/// decides, so the envoy's stopped report reaches it instead of being acknowledged as unowned,
/// which would leave it stopping with its stop resent forever. That epoch is never bound
/// and never owns storage: its storage ended with the move, and the copy keeps the legacy
/// actor's id and incarnation, so only the copy's epochs open that database. The read runs
/// only on such a miss. Removed in 3.1 with the backfill.
async fn resolve_owner(
	scope: &crate::routes::EnvoyScope,
	envoy_key: &str,
	actor_id: &str,
	bound: &BoundEpoch,
	reads: &impl OwnerReads,
	purpose: OwnershipUse,
) -> Result<Option<crate::sqlite::OwnedEpoch>> {
	if let Some(owned) = bound.owned() {
		return Ok(Some(owned));
	}

	let id = rivet_util_id::Id::parse(actor_id)?;
	let (group, read) = reads.current(id).await?;
	// A destroyed actor's state volume is deleted; it owns nothing here.
	let Some(read) = read else {
		return Ok(None);
	};
	let moved_copy = legacy_moved_copy_of(group, &read, id);
	if let Some(owned) = record_owned_epoch(scope, group, envoy_key, read, bound) {
		return Ok(Some(owned));
	}
	match purpose {
		OwnershipUse::Storage => return Ok(None),
		OwnershipUse::Events => {}
	}
	if !moved_copy {
		return Ok(None);
	}
	let Some(legacy) = reads.legacy_unprefixed(id).await? else {
		return Ok(None);
	};
	Ok(record_owner(scope, None, envoy_key, legacy, bound))
}

/// Legacy: whether `read`, the record of `actor_id` in `group`, is a backfill copy moved from
/// the unprefixed actor with the same id, as every moved random-id actor's is. Removed in 3.1
/// with the backfill.
fn legacy_moved_copy_of(
	group: Option<rivet_keys::keys::codec::GroupId>,
	read: &wire::ActorReadResponse,
	actor_id: rivet_util_id::Id,
) -> bool {
	group.is_some()
		&& read
			.data
			.as_ref()
			.and_then(|data| data.legacy_moved_from.as_ref())
			.is_some_and(|moved_from| moved_from.as_ref() == actor_id.as_bytes().as_slice())
}

/// [`record_owner`], binding the epoch it finds to `bound`.
fn record_owned_epoch(
	scope: &crate::routes::EnvoyScope,
	group: Option<rivet_keys::keys::codec::GroupId>,
	envoy_key: &str,
	read: wire::ActorReadResponse,
	bound: &BoundEpoch,
) -> Option<crate::sqlite::OwnedEpoch> {
	let owned = record_owner(scope, group, envoy_key, read, bound)?;
	bound.bind(owned);
	Some(owned)
}

/// The epoch and incarnation `read`, the actor's record in `group`, places on this envoy,
/// bound to `group`, unless `bound` already saw that epoch stop.
fn record_owner(
	scope: &crate::routes::EnvoyScope,
	group: Option<rivet_keys::keys::codec::GroupId>,
	envoy_key: &str,
	read: wire::ActorReadResponse,
	bound: &BoundEpoch,
) -> Option<crate::sqlite::OwnedEpoch> {
	let (Some(data), Some(phase)) = (read.data, read.phase) else {
		return None;
	};
	let allocation = match phase {
		wire::ActorPhase::Starting(wire::Starting { allocation, .. })
		| wire::ActorPhase::Running(wire::Running { allocation })
		| wire::ActorPhase::Stopping(wire::Stopping { allocation, .. }) => allocation,
		wire::ActorPhase::Initializing(_)
		| wire::ActorPhase::Sleeping(_)
		| wire::ActorPhase::Destroying(_)
		| wire::ActorPhase::Destroyed
		| wire::ActorPhase::MigratingFromV2(_)
		| wire::ActorPhase::DelegatedToV2 => return None,
	};
	// The allocation names the envoy, not the connection: after a reconnect the recorded
	// connection is stale until a gateway repairs it, but the actor still runs here.
	if allocation.envoy_key != envoy_key || data.pool_name != scope.pool_name {
		return None;
	}
	// An ungrouped volume key carries no namespace, so the record's legacy namespace is what
	// keeps an envoy of another namespace with the same key from binding the actor. A grouped
	// key names the group, which proves the namespace; see `actor_in_namespace`.
	if !pegboard::catalog::actor_in_namespace(group, &scope.namespace, &data) {
		return None;
	}

	// A record that still shows an epoch stopped here, before the stop was delivered,
	// grants nothing.
	if bound.is_stopped(read.epoch) {
		return None;
	}
	Some(crate::sqlite::OwnedEpoch {
		epoch: read.epoch,
		create_unix_ms: data.create_unix_ms,
		group,
	})
}

/// Runs one KV request against the actor volume. Failures become request-scoped KV errors so
/// one actor's storage trouble never closes the shared socket.
async fn kv_request(
	conn: &Connection,
	actor_id: &str,
	bound: &BoundEpoch,
	request: p::ToRivetKvRequest,
) -> p::ToEnvoy {
	let request_id = request.request_id;
	let result = async {
		let Some(owned) = owned_epoch(conn, actor_id, bound).await? else {
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
				owned.epoch,
				owned.create_unix_ms,
				owned.group,
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

/// The version header of a payload encoded with its embedded version.
fn embedded_version(payload: &[u8]) -> Result<u16> {
	let header = payload
		.get(..2)
		.context("envoy message shorter than its version header")?;
	Ok(u16::from_le_bytes([header[0], header[1]]))
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

	/// The UPS packet for the gateway: a version header and a `ToRivet` body at `version`. An
	/// encoded reply at that version is copied once behind the header instead of being
	/// re-encoded; at an older one it is decoded and converted down.
	fn into_payload(self, version: u16) -> Result<Bytes> {
		match self {
			Self::Message(reply) => Ok(Bytes::from(
				p::versioned::ToRivet::serialize_with_embedded_version(
					&p::ToRivet::ToRivetTunnelMessage(*reply),
					version,
				)?,
			)),
			Self::Encoded(view) if version != p::PROTOCOL_VERSION => Ok(Bytes::from(
				p::versioned::ToRivet::serialize_with_embedded_version(&view.to_owned(), version)?,
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

/// Hands a reply to its gateway without waiting for the gateway to read. A full queue cancels
/// only that stream, including a handler blocked on a peer send; pongs and other actors keep
/// flowing on the application socket. Replies to removed streams are dropped.
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
	if let Some(route) = handle {
		match route.tx.try_send(reply) {
			Ok(()) => return Ok(()),
			Err(mpsc::error::TrySendError::Full(_)) => {
				route.overloaded.notify_one();
			}
			Err(mpsc::error::TrySendError::Closed(_)) => {}
		}
		// Remove this exact registration. A replacement installed under the same request key
		// must not be removed by the old stream's late reply.
		tracing::debug!(
			?message_key,
			"dropped a tunnel reply for a closed or full gateway route"
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
	/// before it ran; the lane then refuses the stopped epoch's later requests and answers.
	Drained(Box<StopBarrier>),
}

/// A stop's barrier on the storage lane.
struct StopBarrier {
	/// The epoch that stopped; it and older ones are refused once the lane reaches this.
	epoch: u32,
	done: tokio::sync::oneshot::Sender<()>,
}

/// Runs when the storage lane reaches a stop's barrier: every request queued before it ran, so
/// the stopped epoch is closed to later requests before the stop is delivered.
fn reach_stop_barrier(bound: &BoundEpoch, barrier: StopBarrier) {
	bound.stop(barrier.epoch);
	let _ = barrier.done.send(());
}

/// [`drain_storage_lane`] bounded by `budget`. Returns whether the lane drained in time.
async fn drain_storage_lane_within(
	clock: &dyn rivet_env::Clock,
	budget: std::time::Duration,
	storage: &crate::queue::Sender<QueuedInbound>,
	epoch: u32,
) -> bool {
	rivet_env::timeout(clock, budget, drain_storage_lane(storage, epoch))
		.await
		.is_ok()
}

/// Waits until the storage lane has run every request queued on it so far. A closed lane has
/// nothing left to run.
async fn drain_storage_lane(storage: &crate::queue::Sender<QueuedInbound>, epoch: u32) {
	let (done_tx, done_rx) = tokio::sync::oneshot::channel();
	let barrier = StopBarrier {
		epoch,
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
	actor_epoch: u32,
	/// Notified once the execution behind this tunnel stopped.
	stopped: tokio::sync::Notify,
	/// Cancels this stream even while its gateway has stopped reading a pending send.
	overloaded: tokio::sync::Notify,
}

impl GatewayHandle {
	/// Cancel the whole stream on overload, including setup and a blocked peer send. Merely
	/// checking between messages would leave a slow gateway's accepted work stuck forever.
	async fn run(&self, transfer: impl std::future::Future<Output = Result<()>>) -> Result<()> {
		rivet_env::select_biased!("a full gateway reply queue cancels only its stream",
			_ = self.overloaded.notified() => Err(anyhow::anyhow!("gateway reply queue is full")),
			result = transfer => result,
		)
	}
}

/// Ends every tunnel to `epoch` of the actor. The envoy runs no execution for them anymore.
async fn stop_tunnels(conn: &Connection, actor_id: &str, epoch: u32) {
	conn.gateway_handles
		.iter_async(|_, handle| {
			if handle.actor_id == actor_id && handle.actor_epoch == epoch {
				handle.stopped.notify_one();
			}
			true
		})
		.await;
}

/// The part of a tail proof request's stream deadline kept back from the proof for sending
/// its acknowledgement.
const PROVE_TAIL_ACK_MARGIN_MS: u64 = 250;

/// Answers pegboard's request to prove the tail of the actor database volume this connection
/// holds for `request.generation`, so a fork of the running actor can capture it. The volume
/// commits one empty entry, and the stream is acknowledged once that is committed. The proof
/// gets the sender's deadline instead of being cancelled at it, because dropping a proof
/// between its admission and commit would fence the running actor's volume. A connection that
/// does not hold the volume for that epoch, or a proof still undecided at its deadline,
/// answers nothing and logs at debug; any other failure ends the stream without an
/// acknowledgement. Pegboard treats a missing
/// acknowledgement as not proven, and the fork is refused again and resends after its backoff.
/// Nothing here retries.
async fn prove_sqlite_tail(
	conn: &Arc<Connection>,
	mut stream_tx: universalpubsub::PacketSender,
	request: p::ToEnvoySqliteProveTail,
) -> Result<()> {
	// A request past its sender's deadline gets no answer; the sender has already given up.
	let deadline_boot_ms = stream_tx.deadline_boot_ms();
	let now_boot_ms = conn.ctx.env().clock().observe()?.boot_ms;
	if command_expired(deadline_boot_ms, now_boot_ms) {
		tracing::debug!(
			actor_id = %request.actor_id,
			generation = request.generation,
			"dropping a SQLite tail proof request whose sender's deadline passed"
		);
		return Ok(());
	}

	let id = rivet_util_id::Id::parse(&request.actor_id)
		.context("SQLite tail proof request names an invalid actor id")?;
	// The proof stops short of the stream's deadline, so a proof that commits at its own
	// deadline can still send the acknowledgement before the sender gives up.
	let proof_deadline_boot_ms =
		deadline_boot_ms.map(|deadline| deadline.saturating_sub(PROVE_TAIL_ACK_MARGIN_MS));
	// A proof that fenced the volume fails the stream at error; name the actor and generation
	// so the log says whose volume a tail proof fenced.
	let committed = match conn
		.sqlite
		.prove_tail(
			conn.ctx.env().clock().as_ref(),
			id,
			request.generation,
			proof_deadline_boot_ms,
		)
		.await
		.with_context(|| {
			format!(
				"proving the SQLite tail of actor {} generation {}",
				request.actor_id, request.generation
			)
		})? {
		crate::sqlite::TailProof::Proven { committed } => committed,
		crate::sqlite::TailProof::NotHeld { reason } => {
			tracing::debug!(
				actor_id = %request.actor_id,
				generation = request.generation,
				reason,
				"not proving a SQLite tail this connection does not hold"
			);
			return Ok(());
		}
		// Expected under a slow quorum. The sender reads the missing acknowledgement as not
		// proven and asks again on its next resend.
		crate::sqlite::TailProof::Undecided { error } => {
			tracing::debug!(
				%error,
				actor_id = %request.actor_id,
				generation = request.generation,
				"a SQLite tail proof did not decide before its deadline"
			);
			return Ok(());
		}
	};
	tracing::debug!(
		actor_id = %request.actor_id,
		generation = request.generation,
		committed,
		"proved an actor database volume's tail for a fork"
	);

	stream_tx
		.send(Bytes::from_static(
			pegboard::actor_machine::executor::TAIL_PROVEN_ACK,
		))
		.await?;
	stream_tx.finish().await?;
	Ok(())
}

/// A command stream's first message, read in arrival order, with one actor turn per command.
struct CommandArrival {
	message: p::ToEnvoy,
	turns: Vec<Turn>,
}

/// Reads a command stream's first message and takes each command's turn on its actor. Called
/// from the subscriber loop, so turns follow the order UPS delivered the streams in, which is
/// the order of the sender's lane tickets. A stream whose first message is not commands takes
/// no turns: a SQLite tail proof request runs on its own, and anything else is rejected by its
/// handler.
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

/// Whether a message naming the volume group `group` may reach an actor this connection hosts.
#[derive(Debug, PartialEq, Eq)]
enum GroupCheck {
	/// The message names no group, from a v8 sender or for an actor at the legacy unprefixed
	/// keys, or names this connection's group.
	Serves,
	/// The message names another group than this connection's namespace has, so it was routed on
	/// another namespace's allocation and is refused.
	Foreign,
	/// Legacy: the connection has no group because it opened before its namespace moved into the
	/// group the message names. The connection closes so the envoy reconnects with the group, and
	/// the message is refused. Removed in 3.1 with the backfill.
	LegacyNamespaceMoved,
}

/// Checks the group a message names against this connection's scope. Legacy: a connection
/// without a group cannot tell a foreign group from its own namespace's new one, so it reads the
/// namespace's group now with `namespace_group` (see
/// `pegboard::catalog::legacy_read_namespace_group`) and remembers the answer in
/// `legacy_group_read` for the rest of the message batch. The legacy behavior refused every such
/// message, which left a grouped actor hashed to this envoy never starting after its namespace's
/// move; the new behavior closes the connection when the namespace moved into the named group.
/// The read runs only when a groupless connection gets a grouped message, which happens only
/// after its namespace moved. Removed in 3.1 with the backfill.
async fn check_group<F, Fut>(
	scope: &crate::routes::EnvoyScope,
	group: Option<&[u8]>,
	legacy_group_read: &mut Option<Option<rivet_keys::keys::codec::GroupId>>,
	namespace_group: F,
) -> Result<GroupCheck>
where
	F: FnOnce() -> Fut,
	Fut: std::future::Future<Output = Result<Option<rivet_keys::keys::codec::GroupId>>>,
{
	let Some(group) = group else {
		return Ok(GroupCheck::Serves);
	};
	if let Some(own) = scope.group {
		return match own.as_slice() == group {
			true => Ok(GroupCheck::Serves),
			false => Ok(GroupCheck::Foreign),
		};
	}
	let current = match legacy_group_read {
		Some(current) => *current,
		None => {
			let current = namespace_group().await?;
			*legacy_group_read = Some(current);
			current
		}
	};
	match current {
		Some(current) if current.as_slice() == group => Ok(GroupCheck::LegacyNamespaceMoved),
		Some(_) | None => Ok(GroupCheck::Foreign),
	}
}

/// Legacy: closes a connection opened before its namespace moved into its group, after every
/// frame queued before the close, as the event lane does. A full socket queue ends the connection
/// through the event failure channel without a reason instead; the envoy reconnects either way.
/// Removed in 3.1 with the backfill.
fn legacy_close_moved_namespace(conn: &Connection) {
	let rejection = crate::rejection::Rejection::legacy_namespace_moved();
	if conn
		.to_envoy_tx
		.try_send(ToEnvoyFrame::Reject(rejection))
		.is_ok()
	{
		return;
	}
	tracing::info!(
		namespace = %conn.scope.namespace,
		"envoy socket queue full while closing a moved namespace's connection; closing it without a reason so the envoy reconnects"
	);
	let _ = conn
		.event_failure
		.try_send(crate::lifecycle::LegacyNamespaceMoved.into());
}

/// The answer to a gateway request this connection refused for naming another namespace's
/// group: the envoy's own refusal of an actor it does not host, a 503 `envoy.actor_not_found`
/// for HTTP and a close with that reason for a WebSocket, which the gateway treats as a refusal
/// before admission.
fn foreign_group_refusal(
	message_id: p::MessageId,
	is_websocket: bool,
	actor_id: &str,
	actor_epoch: u32,
) -> Result<p::ToRivet> {
	let message_kind = if is_websocket {
		p::ToRivetTunnelMessageKind::ToRivetWebSocketClose(p::ToRivetWebSocketClose {
			code: Some(1011),
			reason: Some("envoy.actor_not_found".into()),
			hibernate: false,
		})
	} else {
		let body = serde_json::to_vec(&serde_json::json!({
			"group": "envoy",
			"code": "actor_not_found",
			"message": "Actor not found",
			"actor": {
				"actorId": actor_id,
				"generation": actor_epoch,
			},
		}))?;
		let headers = [
			("x-rivet-error", "envoy.actor_not_found".to_owned()),
			("content-type", "application/json".to_owned()),
			("content-length", body.len().to_string()),
		];
		p::ToRivetTunnelMessageKind::ToRivetResponseStart(p::ToRivetResponseStart {
			status: 503,
			headers: headers
				.into_iter()
				.map(|(name, value)| (name.to_owned().into(), value.into()))
				.collect(),
			body: Some(body.into()),
			stream: false,
		})
	};
	Ok(p::ToRivet::ToRivetTunnelMessage(p::ToRivetTunnelMessage {
		message_id: p::MessageId {
			message_index: 0,
			..message_id
		},
		message_kind,
	}))
}

/// Rewrites a start's actor key, and its fork source's key, from the canonical form the engine
/// names actors by into the slash form the actor runtime reads, and takes off the group, the
/// engine's routing check. A stop or a keyless start keeps its key.
fn command_for_envoy(mut command: p::CommandWrapper) -> p::CommandWrapper {
	command.group = None;
	if let p::Command::CommandStartActor(start) = &mut command.inner {
		start.config.key = start
			.config
			.key
			.as_deref()
			.map(actor_key::to_slash)
			.map(Into::into);
		if let Some(fork) = &mut start.fork {
			fork.key = fork.key.as_deref().map(actor_key::to_slash).map(Into::into);
			for link in &mut fork.chain {
				link.key = link.key.as_deref().map(actor_key::to_slash).map(Into::into);
			}
		}
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
		epoch = command.checkpoint.generation,
		stage,
		kind,
		"dropping an actor command whose sender's deadline passed; the actor state machine retries it if it is still wanted"
	);
}
