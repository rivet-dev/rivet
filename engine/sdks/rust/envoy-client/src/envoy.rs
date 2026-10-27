use std::collections::HashMap;
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::OnceLock;
use std::sync::atomic::Ordering;

use anyhow::Context;
#[cfg(not(target_arch = "wasm32"))]
use parking_lot::Mutex;

use crate::async_counter::AsyncCounter;
use rivet_envoy_protocol as protocol;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::Instrument;

use crate::actor::ToActor;
use crate::commands::handle_commands;
use crate::config::EnvoyConfig;
use crate::connection::{start_connection, ws_send};
use crate::context::{SharedContext, WsTxMessage};
use crate::deadlines::sleep_until_deadline;
use crate::events::{handle_ack_events, handle_send_events, resend_unacknowledged_events};
use crate::handle::EnvoyHandle;
use crate::kv::{
	KvRequestEntry, handle_kv_request, handle_kv_response, process_unsent_kv_requests,
};
use crate::metrics::METRICS;
use crate::sqlite::{
	RemoteSqliteRequest, RemoteSqliteRequestEntry, RemoteSqliteResponseEnvelope, SqliteRequest,
	SqliteRequestEntry, SqliteResponse, fail_remote_sqlite_requests_with_shutdown,
	fail_sqlite_requests_with_shutdown, handle_remote_sqlite_exec_response,
	handle_remote_sqlite_execute_batch_response, handle_remote_sqlite_execute_response,
	handle_remote_sqlite_request, handle_sqlite_commit_finalize_response,
	handle_sqlite_commit_response, handle_sqlite_commit_stage_begin_response,
	handle_sqlite_commit_stage_segment_response, handle_sqlite_connection_closed,
	handle_sqlite_get_pages_response, handle_sqlite_request, process_unsent_remote_sqlite_requests,
	process_unsent_sqlite_requests,
};
use crate::tunnel::{
	CompletedHttpRequests, HttpRequestCancellationKey, handle_tunnel_message, make_ws_key,
	resend_buffered_tunnel_messages, send_hibernatable_ws_message_ack,
};
use crate::utils::{BufferMap, EnvoyShutdownError, SleepFuture, boxed_sleep, spawn_detached};

/// Process-wide envoy slot. Holds the handle inside a mutex so a stopped
/// handle (e.g. from a shutdown-during-build race in serverless mode) can be
/// replaced on the next `start_envoy_sync` call.
#[cfg(not(target_arch = "wasm32"))]
static GLOBAL_ENVOY: OnceLock<Mutex<Option<EnvoyHandle>>> = OnceLock::new();

pub struct EnvoyContext {
	pub shared: Arc<SharedContext>,
	pub shutting_down: bool,
	pub actors: HashMap<String, HashMap<u32, ActorEntry>>,
	pub pending_events: HashMap<(String, u32), PendingActorEvents>,
	pub pending_events_ack_waiters: Vec<oneshot::Sender<()>>,
	pub buffered_actor_messages: HashMap<String, Vec<BufferedActorMessage>>,
	pub kv_requests: HashMap<u32, KvRequestEntry>,
	pub next_kv_request_id: u32,
	pub sqlite_requests: HashMap<u32, SqliteRequestEntry>,
	pub next_sqlite_request_id: u32,
	pub remote_sqlite_requests: HashMap<u32, RemoteSqliteRequestEntry>,
	pub next_remote_sqlite_request_id: u32,
	pub request_to_actor: BufferMap<WebSocketRoute>,
	pub http_request_routes: BufferMap<HttpRequestRoute>,
	pub http_message_indices: BufferMap<protocol::MessageIndex>,
	/// Recently cancelled exact-generation requests. This prevents a delayed
	/// `RequestStart` from reaching the actor after its cancellation arrived.
	pub http_request_cancellations: HashMap<HttpRequestCancellationKey, crate::time::Instant>,
	/// Recently finished requests. Gateway can still send body frames, its own body cancel,
	/// or response credit for them, and those frames must not abort the finished response.
	pub completed_http_requests: CompletedHttpRequests,
	pub buffered_messages: Vec<protocol::ToRivetTunnelMessage>,
	/// Highest stopped generation per actor, so a late duplicate start is dropped.
	pub stopped_generation_floors: crate::commands::StoppedGenerationFloors,
	/// Highest command index processed per `(actor_id, generation)`, used to
	/// drop repeated commands for a hosted generation. `remove_actor` forgets the
	/// actor's entries and `handle_commands` forgets entries of generations it
	/// does not host, so the map is bounded by the hosted generations.
	pub processed_command_idx: HashMap<(String, u32), i64>,
	/// Deadlines of in-flight requests and retired event sets. The loop sleeps until the
	/// earliest one.
	pub deadlines: crate::deadlines::Deadlines,
}

pub struct HttpRequestRoute {
	pub actor_id: String,
	pub actor_generation: Option<u32>,
	pub actor_admitted: bool,
	pub session: u64,
	pub gateway_id: protocol::GatewayId,
	pub request_id: protocol::RequestId,
}

#[derive(Clone)]
pub struct WebSocketRoute {
	pub actor_id: String,
	pub actor_generation: Option<u32>,
}

pub struct ActorEntry {
	pub handle: mpsc::UnboundedSender<ToActor>,
	pub active_http_request_count: Arc<AsyncCounter>,
	pub name: String,
	pub last_command_idx: i64,
	pub received_stop: bool,
	/// HTTP requests admitted to this generation and still in its inbox.
	pub queued_requests: Arc<crate::actor::QueuedRequests>,
}

#[derive(Default)]
pub struct PendingActorEvents {
	pub events: Vec<protocol::EventWrapper>,
	pub retired_at: Option<crate::time::Instant>,
}

pub enum BufferedActorMessage {
	WsMsg {
		message_id: protocol::MessageId,
		msg: protocol::ToEnvoyWebSocketMessage,
	},
	WsClose {
		message_id: protocol::MessageId,
		close: protocol::ToEnvoyWebSocketClose,
	},
}

pub enum ToEnvoyMessage {
	ConnMessage {
		message: protocol::ToEnvoy,
		session: u64,
	},
	ConnClose {
		evict: bool,
		session: u64,
	},
	SendEvents {
		events: Vec<protocol::EventWrapper>,
	},
	KvRequest {
		actor_id: String,
		data: protocol::KvRequestData,
		response_tx: oneshot::Sender<anyhow::Result<protocol::KvResponseData>>,
	},
	SqliteRequest {
		trace: Option<crate::sqlite::SqliteTrace>,
		request: SqliteRequest,
		response_tx: oneshot::Sender<anyhow::Result<SqliteResponse>>,
	},
	RemoteSqliteRequest {
		request: RemoteSqliteRequest,
		expected_session: Option<u64>,
		response_tx: oneshot::Sender<anyhow::Result<RemoteSqliteResponseEnvelope>>,
	},
	SendOrBufferTunnelMsg {
		msg: protocol::ToRivetTunnelMessage,
	},
	ActorIntent {
		actor_id: String,
		generation: Option<u32>,
		intent: protocol::ActorIntent,
		error: Option<String>,
	},
	SetAlarm {
		actor_id: String,
		generation: Option<u32>,
		alarm_ts: Option<i64>,
		ack_tx: Option<oneshot::Sender<()>>,
	},
	HwsAck {
		gateway_id: protocol::GatewayId,
		request_id: protocol::RequestId,
		envoy_message_index: u16,
	},
	RebindWebSocket {
		actor_id: String,
		generation: u32,
		gateway_id: protocol::GatewayId,
		request_id: protocol::RequestId,
		response_tx: oneshot::Sender<bool>,
	},
	HttpRequestComplete {
		gateway_id: protocol::GatewayId,
		request_id: protocol::RequestId,
	},
	GetActor {
		actor_id: String,
		generation: Option<u32>,
		response_tx: oneshot::Sender<Option<ActorInfo>>,
	},
	WaitForEventsAcked(oneshot::Sender<()>),
	MarkStopping(oneshot::Sender<bool>),
	Shutdown,
	Stop,
}

/// Information about an actor, returned by `EnvoyHandle::get_actor`.
#[derive(Clone)]
pub struct ActorInfo {
	pub name: String,
	pub generation: u32,
	pub active_http_request_count: Arc<AsyncCounter>,
}

impl EnvoyContext {
	pub(crate) fn rebind_websocket(
		&mut self,
		actor_id: &str,
		generation: u32,
		gateway_id: &protocol::GatewayId,
		request_id: &protocol::RequestId,
	) -> bool {
		if let Some(route) = self.request_to_actor.get_mut(&[gateway_id, request_id]) {
			if route.actor_id != actor_id {
				return false;
			}
			if route.actor_generation.is_some() {
				route.actor_generation = Some(generation);
			}
		} else {
			// A hibernating actor may be restored on a different Envoy process. Its
			// authoritative start command carries the hibernating request ids, while
			// the original process owns the old ephemeral route map.
			self.request_to_actor.insert(
				&[gateway_id, request_id],
				WebSocketRoute {
					actor_id: actor_id.to_owned(),
					actor_generation: Some(generation),
				},
			);
		}

		self.shared
			.live_tunnel_requests
			.lock()
			.expect("shared live tunnel request registry poisoned")
			.insert(make_ws_key(gateway_id, request_id), actor_id.to_owned());
		true
	}

	pub fn insert_actor(
		&mut self,
		actor_id: String,
		generation: u32,
		handle: mpsc::UnboundedSender<ToActor>,
		active_http_request_count: Arc<AsyncCounter>,
		name: String,
		last_command_idx: i64,
	) {
		let buffered_actor_id = actor_id.clone();
		let buffered_handle = handle.clone();
		self.actors
			.entry(actor_id.clone())
			.or_insert_with(HashMap::new)
			.insert(
				generation,
				ActorEntry {
					handle: handle.clone(),
					active_http_request_count: active_http_request_count.clone(),
					name,
					last_command_idx,
					received_stop: false,
					queued_requests: Default::default(),
				},
			);
		self.shared
			.actors
			.lock()
			.expect("shared actor registry poisoned")
			.entry(actor_id)
			.or_insert_with(HashMap::new)
			.insert(
				generation,
				crate::context::SharedActorEntry {
					handle,
					active_http_request_count,
				},
			);

		self.shared.actors_notify.notify_waiters();

		if let Some(messages) = self.buffered_actor_messages.remove(&buffered_actor_id) {
			for message in messages {
				match message {
					BufferedActorMessage::WsMsg { message_id, msg } => {
						let _ = buffered_handle.send(ToActor::WsMsg { message_id, msg });
					}
					BufferedActorMessage::WsClose { message_id, close } => {
						let _ = buffered_handle.send(ToActor::WsClose { message_id, close });
					}
				}
			}
		}
	}

	pub fn remove_actor(&mut self, actor_id: &str, generation: u32) {
		// The floor, not the per-generation index, keeps a stopped generation from
		// starting again, so every index entry of this actor can go.
		self.processed_command_idx
			.retain(|(id, _), _| id != actor_id);
		self.stopped_generation_floors.record(
			actor_id,
			generation,
			crate::time::Instant::now(),
			self.shared.config.stopped_generation_floor_ttl,
			self.shared.config.stopped_generation_floor_capacity,
		);
		crate::events::retire(self, actor_id, generation);
		if let Some(generations) = self.actors.get_mut(actor_id) {
			generations.remove(&generation);
			if generations.is_empty() {
				self.actors.remove(actor_id);
			}
		}

		let mut shared = self
			.shared
			.actors
			.lock()
			.expect("shared actor registry poisoned");
		if let Some(generations) = shared.get_mut(actor_id) {
			generations.remove(&generation);
			if generations.is_empty() {
				shared.remove(actor_id);
			}
		}
		self.shared.actors_notify.notify_waiters();
	}

	pub fn get_actor(&self, actor_id: &str, generation: Option<u32>) -> Option<&ActorEntry> {
		let gens = self.actors.get(actor_id)?;
		if gens.is_empty() {
			return None;
		}

		if let Some(g) = generation {
			return gens.get(&g).filter(|entry| !entry.handle.is_closed());
		}

		// Return highest generation non-closed entry
		// HashMap doesn't guarantee order, so find max key
		let mut best: Option<&ActorEntry> = None;
		let mut best_gen: u32 = 0;
		for (&g, entry) in gens {
			if !entry.handle.is_closed() && (best.is_none() || g > best_gen) {
				best = Some(entry);
				best_gen = g;
			}
		}
		best
	}

	/// Selects an actor for a new request. Exact-generation continuations use
	/// `get_actor` directly so an already admitted stream can finish during stop.
	pub fn get_actor_for_admission(
		&self,
		actor_id: &str,
		generation: Option<u32>,
	) -> Option<&ActorEntry> {
		let actor = self.get_actor(actor_id, generation)?;
		if generation.is_some() && actor.received_stop {
			None
		} else {
			Some(actor)
		}
	}

	pub fn get_actor_entry_mut(
		&mut self,
		actor_id: &str,
		generation: u32,
	) -> Option<&mut ActorEntry> {
		self.actors
			.get_mut(actor_id)
			.and_then(|gens| gens.get_mut(&generation))
	}
}

/// Starts an envoy and waits until the engine has sent its init message.
///
/// Returns an error if the envoy loop exits before init, for example when the
/// engine evicts the connection. The wait is unbounded while the engine stays
/// unreachable, so callers that must stay responsive should use
/// [`start_envoy_sync`] and bound [`EnvoyHandle::started`] themselves.
pub async fn start_envoy(config: EnvoyConfig) -> anyhow::Result<EnvoyHandle> {
	let handle = start_envoy_sync(config);
	handle
		.started()
		.await
		.context("envoy failed to start before returning handle")?;
	Ok(handle)
}

pub fn start_envoy_sync(config: EnvoyConfig) -> EnvoyHandle {
	#[cfg(target_arch = "wasm32")]
	{
		start_envoy_sync_inner(config)
	}

	#[cfg(not(target_arch = "wasm32"))]
	{
		if config.not_global {
			return start_envoy_sync_inner(config);
		}

		let slot = GLOBAL_ENVOY.get_or_init(|| Mutex::new(None));
		let mut guard = slot.lock();
		if let Some(handle) = guard.as_ref() {
			if !handle.is_stopped() {
				return handle.clone();
			}
		}
		let handle = start_envoy_sync_inner(config);
		*guard = Some(handle.clone());
		handle
	}
}

fn start_envoy_sync_inner(config: EnvoyConfig) -> EnvoyHandle {
	crate::metrics::METRICS
		.actor_queued_requests_capacity
		.set(config.max_queued_requests_per_actor as i64);
	let (envoy_tx, envoy_rx) = mpsc::unbounded_channel::<ToEnvoyMessage>();
	let (start_tx, start_rx) = tokio::sync::watch::channel(());
	let (stopped_tx, _stopped_rx) = tokio::sync::watch::channel(false);
	let (connection_session_tx, _connection_session_rx) = tokio::sync::watch::channel(0);

	let envoy_key = uuid::Uuid::new_v4().to_string();
	let shared = Arc::new(SharedContext {
		config,
		envoy_key,
		envoy_tx: envoy_tx.clone(),
		actors: Arc::new(std::sync::Mutex::new(HashMap::new())),
		actors_notify: Arc::new(tokio::sync::Notify::new()),
		live_tunnel_requests: Arc::new(std::sync::Mutex::new(HashMap::new())),
		pending_hibernation_restores: Arc::new(std::sync::Mutex::new(HashMap::new())),
		ws_tx: Arc::new(tokio::sync::Mutex::new(None)),
		http_ws_tx: Arc::new(tokio::sync::Mutex::new(None)),
		connection_session: std::sync::atomic::AtomicU64::new(0),
		next_connection_session: std::sync::atomic::AtomicU64::new(0),
		connection_session_tx,
		protocol_metadata: Arc::new(tokio::sync::Mutex::new(None)),
		shutting_down: std::sync::atomic::AtomicBool::new(false),
		last_ping_ts: std::sync::atomic::AtomicI64::new(0),
		contact_before_install_ms: std::sync::atomic::AtomicI64::new(0),
		last_contact_ms: std::sync::atomic::AtomicI64::new(0),
		stopped_tx,
	});

	let handle = EnvoyHandle {
		shared: shared.clone(),
		started_rx: start_rx,
	};

	let connection_task = start_connection(shared.clone());

	let ctx = EnvoyContext {
		shared: shared.clone(),
		shutting_down: false,
		actors: HashMap::new(),
		pending_events: HashMap::new(),
		pending_events_ack_waiters: Vec::new(),
		buffered_actor_messages: HashMap::new(),
		kv_requests: HashMap::new(),
		next_kv_request_id: 0,
		sqlite_requests: HashMap::new(),
		next_sqlite_request_id: 0,
		remote_sqlite_requests: HashMap::new(),
		next_remote_sqlite_request_id: 0,
		request_to_actor: BufferMap::new(),
		http_request_routes: BufferMap::new(),
		http_message_indices: BufferMap::new(),
		http_request_cancellations: HashMap::new(),
		completed_http_requests: CompletedHttpRequests::default(),
		buffered_messages: Vec::new(),
		stopped_generation_floors: Default::default(),
		processed_command_idx: HashMap::new(),
		deadlines: Default::default(),
	};

	tracing::info!(envoy_key = %shared.envoy_key, "starting envoy");
	let span = tracing::info_span!("envoy_client", envoy_key = %shared.envoy_key);
	spawn_detached(envoy_loop(ctx, envoy_rx, start_tx, connection_task).instrument(span));

	handle
}

async fn envoy_loop(
	mut ctx: EnvoyContext,
	mut rx: mpsc::UnboundedReceiver<ToEnvoyMessage>,
	start_tx: tokio::sync::watch::Sender<()>,
	connection_task: Option<tokio::task::JoinHandle<()>>,
) {
	let mut lost_timeout: Option<SleepFuture> = None;

	loop {
		let iter_start = crate::time::Instant::now();
		#[allow(unused_assignments)]
		let mut branch: &'static str = "unknown";
		tokio::select! {
			msg = rx.recv() => {
				branch = "envoy_msg";
				let Some(msg) = msg else {
					observe_envoy_loop_iteration(branch, iter_start);
					break;
				};
				METRICS.envoy_tx_depth.dec();

				match msg {
					ToEnvoyMessage::ConnMessage { message, session } => {
						lost_timeout = handle_conn_message(&mut ctx, &start_tx, lost_timeout, message, session).await;
					}
					ToEnvoyMessage::ConnClose { evict, session } => {
						remove_http_routes_for_session(&mut ctx, session);
						for generations in ctx.actors.values() {
							for actor in generations.values() {
								let _ = actor.handle.send(ToActor::ConnectionClosed { session });
							}
						}
						handle_sqlite_connection_closed(&mut ctx, session);
						lost_timeout = handle_conn_close(&ctx, lost_timeout);
						if evict {
							observe_envoy_loop_iteration(branch, iter_start);
							break;
						}
					}
					ToEnvoyMessage::SendEvents { events } => {
						handle_send_events(&mut ctx, events).await;
					}
					ToEnvoyMessage::KvRequest { actor_id, data, response_tx } => {
						handle_kv_request(&mut ctx, actor_id, data, response_tx).await;
					}
					ToEnvoyMessage::SqliteRequest { request, response_tx, trace } => {
						handle_sqlite_request(&mut ctx, request, response_tx, trace).await;
					}
					ToEnvoyMessage::RemoteSqliteRequest { request, expected_session, response_tx } => {
						handle_remote_sqlite_request(&mut ctx, request, expected_session, response_tx).await;
					}
					ToEnvoyMessage::SendOrBufferTunnelMsg { msg } => {
						crate::tunnel::send_or_buffer_tunnel_message(&mut ctx, msg).await;
					}
					ToEnvoyMessage::ActorIntent { actor_id, generation, intent, error } => {
						if let Some(entry) = ctx.get_actor(&actor_id, generation) {
							let _ = entry.handle.send(ToActor::Intent { intent, error });
						}
					}
					ToEnvoyMessage::SetAlarm { actor_id, generation, alarm_ts, ack_tx } => {
						if let Some(entry) = ctx.get_actor(&actor_id, generation) {
							if let Err(error) = entry.handle.send(ToActor::SetAlarm { alarm_ts, ack_tx }) {
								if let ToActor::SetAlarm { ack_tx: Some(ack_tx), .. } = error.0 {
									let _ = ack_tx.send(());
								}
							}
						} else if let Some(ack_tx) = ack_tx {
							let _ = ack_tx.send(());
						}
					}
					ToEnvoyMessage::HwsAck { gateway_id, request_id, envoy_message_index } => {
						send_hibernatable_ws_message_ack(&mut ctx, gateway_id, request_id, envoy_message_index);
					}
					ToEnvoyMessage::RebindWebSocket { actor_id, generation, gateway_id, request_id, response_tx } => {
						let rebound = ctx.rebind_websocket(
							&actor_id,
							generation,
							&gateway_id,
							&request_id,
						);
						let _ = response_tx.send(rebound);
					}
					ToEnvoyMessage::HttpRequestComplete { gateway_id, request_id } => {
						crate::tunnel::complete_http_request(&mut ctx, gateway_id, request_id);
					}
					ToEnvoyMessage::GetActor { actor_id, generation, response_tx } => {
						let info = ctx.get_actor(&actor_id, generation).map(|entry| {
							let actor_gen = generation.unwrap_or_else(|| {
								ctx.actors
									.get(&actor_id)
									.and_then(|gens| {
										gens.iter()
											.filter(|(_, e)| !e.handle.is_closed())
											.map(|(&g, _)| g)
											.max()
									})
									.unwrap_or(0)
							});
							ActorInfo {
								name: entry.name.clone(),
								generation: actor_gen,
								active_http_request_count: entry
									.active_http_request_count
									.clone(),
							}
						});
						let _ = response_tx.send(info);
					}
					ToEnvoyMessage::WaitForEventsAcked(response_tx) => {
						if ctx.pending_events.is_empty() {
							let _ = response_tx.send(());
						} else {
							ctx.pending_events_ack_waiters.push(response_tx);
						}
					}
					ToEnvoyMessage::MarkStopping(response_tx) => {
						let sent = !ws_send(&ctx.shared, protocol::ToRivet::ToRivetStopping).await;
						let _ = response_tx.send(sent);
					}
					ToEnvoyMessage::Shutdown => {
						handle_shutdown(&mut ctx).await;
					}
					ToEnvoyMessage::Stop => {
						observe_envoy_loop_iteration(branch, iter_start);
						break;
					}
				}
			}
			_ = sleep_until_deadline(ctx.deadlines.next_at()) => {
				branch = "request_deadline";
				crate::deadlines::expire_due(&mut ctx);
			}
			_ = async {
				match lost_timeout.as_mut() {
					Some(timeout) => timeout.as_mut().await,
					None => std::future::pending::<()>().await,
				}
			} => {
				branch = "lost_timeout";
				stop_all_actors_lost(&mut ctx);
				lost_timeout = None;
			}
		}
		observe_envoy_loop_iteration(branch, iter_start);
	}

	// Cleanup
	{
		let guard = ctx.shared.ws_tx.lock().await;
		if let Some(tx) = guard.as_ref() {
			let _ = tx.send(WsTxMessage::Close);
		}
	}
	if let Some(connection_task) = connection_task {
		connection_task.abort();
		let _ = connection_task.await;
	}
	let session = ctx.shared.connection_session.load(Ordering::Acquire);
	if session != 0 {
		crate::connection::remove_connection_for_session(&ctx.shared, session).await;
	}

	for (_id, request) in ctx.kv_requests.drain() {
		let _ = request
			.response_tx
			.send(Err(anyhow::anyhow!("envoy shutting down")));
	}
	fail_sqlite_requests_with_shutdown(&mut ctx);
	fail_remote_sqlite_requests_with_shutdown(&mut ctx);

	ctx.actors.clear();
	ctx.shared
		.actors
		.lock()
		.expect("shared actor registry poisoned")
		.clear();

	tracing::info!("envoy stopped");

	ctx.shared.config.callbacks.on_shutdown();

	// Latched signal: waiters on `EnvoyHandle::wait_stopped` observe this and
	// any future callers of `wait_stopped` resolve immediately because watch
	// retains the last value.
	// A local development shutdown may finish before its caller starts waiting.
	// Keep the stopped state even when no receiver is subscribed yet.
	ctx.shared.stopped_tx.send_replace(true);
}

pub(crate) fn remove_http_routes_for_session(ctx: &mut EnvoyContext, session: u64) {
	let closed_routes = ctx
		.http_request_routes
		.remove_where(|route| route.session == session);
	for route in closed_routes {
		ctx.http_message_indices
			.remove(&[&route.gateway_id, &route.request_id]);
	}
}

fn observe_envoy_loop_iteration(branch: &'static str, start: crate::time::Instant) {
	let elapsed = start.elapsed();
	METRICS
		.envoy_loop_iteration_duration_seconds
		.with_label_values(&[branch])
		.observe(elapsed.as_secs_f64());
}

/// Send a message into the envoy_loop's mpsc and bump the depth gauge.
/// Producers should prefer this over calling `shared.envoy_tx.send` directly
/// so the `envoy_tx_depth` gauge stays in sync.
pub fn send_to_envoy_tx(
	shared: &crate::context::SharedContext,
	msg: ToEnvoyMessage,
) -> Result<(), tokio::sync::mpsc::error::SendError<ToEnvoyMessage>> {
	match shared.envoy_tx.send(msg) {
		Ok(()) => {
			METRICS.envoy_tx_depth.inc();
			Ok(())
		}
		Err(e) => Err(e),
	}
}

async fn handle_conn_message(
	ctx: &mut EnvoyContext,
	start_tx: &tokio::sync::watch::Sender<()>,
	mut lost_timeout: Option<SleepFuture>,
	message: protocol::ToEnvoy,
	session: u64,
) -> Option<SleepFuture> {
	match message {
		protocol::ToEnvoy::ToEnvoyInit(init) => {
			{
				let mut guard = ctx.shared.protocol_metadata.lock().await;
				*guard = Some(init.metadata.clone());
			}
			tracing::info!(?init.metadata, "received init");

			// An envoy that went its lost threshold without a ping, for example while its
			// process was paused or its network partitioned, may notice the close only now
			// and reconnect before its lost timer runs out. The engine may already have ended
			// the epochs it holds, so they stop before anything resumes on this connection.
			// The silence is judged now, when the actors would resume, so it covers every
			// delay between the close and this init.
			let contact_ms = ctx
				.shared
				.contact_before_install_ms
				.swap(0, Ordering::AcqRel);
			let silence_ms = if contact_ms == 0 {
				0
			} else {
				crate::time::now_millis().saturating_sub(contact_ms)
			};
			if silence_ms > init.metadata.envoy_lost_threshold {
				tracing::warn!(
					silence_ms,
					lost_threshold_ms = init.metadata.envoy_lost_threshold,
					"reconnected after the envoy lost threshold without a ping; stopping all actors"
				);
				stop_all_actors_lost(ctx);
			}

			lost_timeout = None;
			resend_unacknowledged_events(ctx).await;
			process_unsent_kv_requests(ctx).await;
			process_unsent_sqlite_requests(ctx).await;
			process_unsent_remote_sqlite_requests(ctx).await;
			resend_buffered_tunnel_messages(ctx).await;

			let _ = start_tx.send(());
		}
		protocol::ToEnvoy::ToEnvoyCommands(commands) => {
			handle_commands(ctx, commands).await;
		}
		protocol::ToEnvoy::ToEnvoyAckEvents(ack) => {
			handle_ack_events(ctx, ack);
		}
		protocol::ToEnvoy::ToEnvoyKvResponse(response) => {
			handle_kv_response(ctx, response).await;
		}
		protocol::ToEnvoy::ToEnvoySqliteGetPagesResponse(response) => {
			handle_sqlite_get_pages_response(ctx, response).await;
		}
		protocol::ToEnvoy::ToEnvoySqliteCommitResponse(response) => {
			handle_sqlite_commit_response(ctx, response).await;
		}
		protocol::ToEnvoy::ToEnvoySqliteCommitStageBeginResponse(response) => {
			handle_sqlite_commit_stage_begin_response(ctx, response).await;
		}
		protocol::ToEnvoy::ToEnvoySqliteCommitStageSegmentResponse(response) => {
			handle_sqlite_commit_stage_segment_response(ctx, response).await;
		}
		protocol::ToEnvoy::ToEnvoySqliteCommitFinalizeResponse(response) => {
			handle_sqlite_commit_finalize_response(ctx, response).await;
		}
		protocol::ToEnvoy::ToEnvoySqliteExecResponse(response) => {
			handle_remote_sqlite_exec_response(ctx, response).await;
		}
		protocol::ToEnvoy::ToEnvoySqliteExecuteResponse(response) => {
			handle_remote_sqlite_execute_response(ctx, response).await;
		}
		protocol::ToEnvoy::ToEnvoySqliteExecuteBatchResponse(response) => {
			handle_remote_sqlite_execute_batch_response(ctx, response).await;
		}
		protocol::ToEnvoy::ToEnvoyTunnelMessage(tunnel_msg) => {
			handle_tunnel_message(ctx, session, tunnel_msg).await;
		}
		protocol::ToEnvoy::ToEnvoyPing(_) => {
			// Should be handled by connection task
		}
	}

	lost_timeout
}

/// Stops every actor because the envoy went its lost threshold without the engine: in-flight
/// KV and SQLite requests fail, and each actor is told it is lost and forgotten. Their
/// unacknowledged events are still resent; they name ended epochs, which the engine ignores.
fn stop_all_actors_lost(ctx: &mut EnvoyContext) {
	for (_id, request) in ctx.kv_requests.drain() {
		let _ = request
			.response_tx
			.send(Err(anyhow::anyhow!(EnvoyShutdownError)));
	}
	fail_sqlite_requests_with_shutdown(ctx);
	fail_remote_sqlite_requests_with_shutdown(ctx);

	if !ctx.actors.is_empty() {
		tracing::warn!("stopping all actors due to envoy lost threshold");
		for (_actor_id, gens) in &ctx.actors {
			for (_g, entry) in gens {
				if !entry.handle.is_closed() {
					let _ = entry.handle.send(ToActor::Lost);
				}
			}
		}
		ctx.actors.clear();
		ctx.shared
			.actors
			.lock()
			.expect("shared actor registry poisoned")
			.clear();
	}
}

fn handle_conn_close(ctx: &EnvoyContext, lost_timeout: Option<SleepFuture>) -> Option<SleepFuture> {
	if lost_timeout.is_some() {
		return lost_timeout;
	}

	// Read threshold from protocol metadata, fall back to 10 seconds
	let lost_threshold = {
		let metadata = ctx.shared.protocol_metadata.try_lock().ok();
		metadata
			.and_then(|guard| guard.as_ref().map(|m| m.envoy_lost_threshold as u64))
			.unwrap_or(10_000)
	};

	tracing::debug!(ms = lost_threshold, "starting envoy lost timeout");

	Some(boxed_sleep(std::time::Duration::from_millis(
		lost_threshold,
	)))
}

async fn handle_shutdown(ctx: &mut EnvoyContext) {
	if ctx.shutting_down {
		return;
	}
	ctx.shutting_down = true;
	ctx.shared.shutting_down.store(true, Ordering::Release);

	tracing::debug!("envoy received shutdown");

	ws_send(&ctx.shared, protocol::ToRivet::ToRivetStopping).await;

	// Wait for all actors to finish. The process manager (Docker,
	// k8s, etc.) provides the ultimate shutdown deadline.
	let actor_handles: Vec<mpsc::UnboundedSender<ToActor>> = ctx
		.actors
		.values()
		.flat_map(|gens| gens.values())
		.filter(|entry| !entry.handle.is_closed())
		.map(|entry| entry.handle.clone())
		.collect();

	let shared = ctx.shared.clone();
	let shutdown_span = tracing::debug_span!(
		parent: tracing::Span::current(),
		"envoy_graceful_shutdown",
		envoy_key = %ctx.shared.envoy_key,
	);
	spawn_detached(
		async move {
			futures_util::future::join_all(actor_handles.iter().map(|h| h.closed())).await;
			tracing::debug!("all actors stopped during graceful shutdown");
			let _ = send_to_envoy_tx(&shared, ToEnvoyMessage::Stop);
		}
		.instrument(shutdown_span),
	);
}
