use std::{
	collections::{BTreeMap, HashMap},
	sync::Arc,
};

use crate::async_counter::AsyncCounter;
use rivet_envoy_protocol as protocol;
use tokio::sync::oneshot::error::TryRecvError;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use tracing::Instrument;

use crate::connection::ws_send;
use crate::context::SharedContext;
use crate::handle::EnvoyHandle;
use crate::http::HttpRequest;

use crate::stringify::stringify_to_rivet_tunnel_message_kind;
use crate::utils::{
	BufferMap, id_to_str, spawn_detached, wrapping_add_u16, wrapping_lte_u16, wrapping_sub_u16,
};
use crate::websocket::{WebSocketHandler, WebSocketMessage, WebSocketSender, WsOutgoing};

mod http;

/// HTTP requests admitted to one actor and not yet taken by its task. Shared by the actor's
/// entry, which admits against the cap, and the requests in its inbox.
#[derive(Default)]
pub struct QueuedRequests(std::sync::atomic::AtomicUsize);

impl QueuedRequests {
	/// Admits one request unless `cap` are already queued. The returned place is released when
	/// it drops.
	pub fn admit(self: &Arc<Self>, cap: usize) -> Option<QueuedRequest> {
		use std::sync::atomic::Ordering;
		let admitted = self
			.0
			.try_update(Ordering::AcqRel, Ordering::Acquire, |queued| {
				(queued < cap).then_some(queued + 1)
			});
		let queued = match admitted {
			Ok(previous) => previous + 1,
			Err(_) => return None,
		};
		if queued == cap {
			tracing::warn!(
				cap,
				"actor reached its queued request cap; later requests are refused until it takes them, which usually means a slow actor start"
			);
		}
		Some(QueuedRequest {
			queue: self.clone(),
			_gauge: rivet_metrics::GaugeGuardExt::inc_guard(
				&crate::metrics::METRICS.actor_queued_requests,
			),
		})
	}
}

/// One admitted request's place in its actor's queue.
pub struct QueuedRequest {
	queue: Arc<QueuedRequests>,
	_gauge: rivet_metrics::IntGaugeGuard,
}

impl Drop for QueuedRequest {
	fn drop(&mut self) {
		self.queue
			.0
			.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
	}
}

pub enum ToActor {
	Intent {
		intent: protocol::ActorIntent,
		error: Option<String>,
	},
	Stop {
		command_idx: i64,
		reason: protocol::StopActorReason,
	},
	Lost,
	SetAlarm {
		alarm_ts: Option<i64>,
		ack_tx: Option<oneshot::Sender<()>>,
	},
	ReqStart {
		message_id: protocol::MessageId,
		req: protocol::ToEnvoyRequestStart,
		connection_session: u64,
		/// The request's place in the actor's queued request count, released when the actor
		/// task takes it, or when the message is dropped with the inbox.
		queued: Option<QueuedRequest>,
	},
	ConnectionClosed {
		session: u64,
	},
	ReqChunk {
		message_id: protocol::MessageId,
		chunk: protocol::ToEnvoyRequestChunk,
	},
	ReqAbort {
		message_id: protocol::MessageId,
		reason: protocol::HttpStreamAbortReason,
	},
	ReqProtocolViolation {
		message_id: protocol::MessageId,
		detail: String,
	},
	ReqBodyCancelled {
		message_id: protocol::MessageId,
	},
	ReqBodyCancel {
		message_id: protocol::MessageId,
	},
	ResponseBodyWindowUpdate {
		message_id: protocol::MessageId,
		consumed_bytes: u64,
	},
	ReqComplete {
		message_id: protocol::MessageId,
	},
	WsOpen {
		message_id: protocol::MessageId,
		path: String,
		headers: BTreeMap<String, String>,
	},
	WsMsg {
		message_id: protocol::MessageId,
		msg: protocol::ToEnvoyWebSocketMessage,
	},
	WsClose {
		message_id: protocol::MessageId,
		close: protocol::ToEnvoyWebSocketClose,
	},
	HwsAck {
		gateway_id: protocol::GatewayId,
		request_id: protocol::RequestId,
		envoy_message_index: u16,
	},
}

struct WebSocketRequestState {
	envoy_message_index: u16,
}

struct WsEntry {
	is_hibernatable: bool,
	rivet_message_index: u16,
	ws_handler: Option<WebSocketHandler>,
	outgoing_tx: mpsc::UnboundedSender<WsOutgoing>,
}

struct ActorContext {
	shared: Arc<SharedContext>,
	tx: mpsc::UnboundedSender<ToActor>,
	actor_id: String,
	generation: u32,
	command_idx: i64,
	event_index: i64,
	error: Option<String>,
	http_requests: http::HttpRequests,
	websocket_requests: BufferMap<WebSocketRequestState>,
	ws_entries: BufferMap<WsEntry>,
	hibernating_requests: Vec<protocol::HibernatingRequest>,
	active_http_request_count: Arc<AsyncCounter>,
}

struct PendingStop {
	completion_rx: oneshot::Receiver<anyhow::Result<()>>,
	stop_code: protocol::StopCode,
	stop_message: Option<String>,
}

enum StopProgress {
	Stopped,
	Pending(PendingStop),
}

/// Closes the inbox of an actor whose start failed and answers every HTTP request queued in it
/// while it started, so none waits for a reply that never comes. Later requests find the inbox
/// closed and are refused at admission the same way.
async fn reject_queued_requests(
	shared: &Arc<SharedContext>,
	actor_id: &str,
	generation: u32,
	rx: &mut mpsc::UnboundedReceiver<ToActor>,
) {
	rx.close();
	while let Ok(message) = rx.try_recv() {
		if let ToActor::ReqStart {
			message_id,
			req,
			connection_session,
			queued,
		} = message
		{
			drop(queued);
			http::reject_unstarted(
				shared.clone(),
				actor_id,
				generation,
				message_id,
				&req,
				connection_session,
			)
			.await;
		}
	}
}

pub fn create_actor(
	shared: Arc<SharedContext>,
	actor_id: String,
	generation: u32,
	config: protocol::ActorConfig,
	hibernating_requests: Vec<protocol::HibernatingRequest>,
	preloaded_kv: Option<protocol::PreloadedKv>,
) -> (mpsc::UnboundedSender<ToActor>, Arc<AsyncCounter>) {
	// The start command is handled synchronously on receipt, so this is its receipt time.
	let received = crate::time::Instant::now();
	let (tx, rx) = mpsc::unbounded_channel();
	let active_http_request_count = Arc::new(AsyncCounter::new());
	spawn_detached(actor_inner(
		shared,
		received,
		actor_id,
		generation,
		config,
		hibernating_requests,
		preloaded_kv,
		tx.clone(),
		rx,
		active_http_request_count.clone(),
	));
	(tx, active_http_request_count)
}

#[tracing::instrument(
	skip_all,
	fields(
		envoy_key = %shared.envoy_key,
		actor_id = %actor_id,
		generation = generation,
		actor_key = %config.key.as_deref().unwrap_or(""),
	),
)]
async fn actor_inner(
	shared: Arc<SharedContext>,
	received: crate::time::Instant,
	actor_id: String,
	generation: u32,
	config: protocol::ActorConfig,
	hibernating_requests: Vec<protocol::HibernatingRequest>,
	preloaded_kv: Option<protocol::PreloadedKv>,
	tx: mpsc::UnboundedSender<ToActor>,
	mut rx: mpsc::UnboundedReceiver<ToActor>,
	active_http_request_count: Arc<AsyncCounter>,
) {
	let handle = EnvoyHandle {
		shared: shared.clone(),
		// Fake channel, don't care
		started_rx: tokio::sync::watch::channel(None).1,
	};

	let mut ctx = ActorContext {
		shared: shared.clone(),
		tx,
		actor_id: actor_id.clone(),
		generation,
		command_idx: 0,
		event_index: 0,
		error: None,
		http_requests: http::HttpRequests::new(),
		websocket_requests: BufferMap::new(),
		ws_entries: BufferMap::new(),
		hibernating_requests,
		active_http_request_count,
	};
	let mut http_request_tasks = JoinSet::new();
	let mut pending_stop: Option<PendingStop> = None;
	let mut rx_closed = false;

	// Call on_actor_start
	let start_result = shared
		.config
		.callbacks
		.on_actor_start(
			handle.clone(),
			actor_id.clone(),
			generation,
			config,
			preloaded_kv,
		)
		.await;

	if let Err(error) = start_result {
		let error_chain = error.chain().map(ToString::to_string).collect::<Vec<_>>();
		tracing::error!(?error, error_chain = ?error_chain, "actor start failed");
		reject_queued_requests(&shared, &actor_id, generation, &mut rx).await;
		send_event(
			&mut ctx,
			protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate {
				state: protocol::ActorState::ActorStateStopped(protocol::ActorStateStopped {
					code: protocol::StopCode::Error,
					message: Some((format!("{error:#}")).into()),
				}),
			}),
		);
		return;
	}

	if let Some(meta_entries) = handle.take_pending_hibernation_restore(&actor_id) {
		if let Err(error) = handle_hws_restore(&mut ctx, &handle, meta_entries).await {
			tracing::error!(?error, "actor hibernation restore failed");
			reject_queued_requests(&shared, &actor_id, generation, &mut rx).await;
			send_event(
				&mut ctx,
				protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate {
					state: protocol::ActorState::ActorStateStopped(protocol::ActorStateStopped {
						code: protocol::StopCode::Error,
						message: Some((format!("{error:#}")).into()),
					}),
				}),
			);
			return;
		}
	}

	// Send running state
	crate::metrics::METRICS
		.actor_start_seconds
		.observe(received.elapsed().as_secs_f64());
	send_event(
		&mut ctx,
		protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate {
			state: protocol::ActorState::ActorStateRunning,
		}),
	);

	loop {
		tokio::select! {
			maybe_task = async {
				if http_request_tasks.is_empty() {
					std::future::pending().await
				} else {
					http_request_tasks.join_next().await
				}
			} => {
				if let Some(result) = maybe_task {
					http::handle_task_result(result);
				}
			}
			msg = async {
				if rx_closed {
					std::future::pending::<Option<ToActor>>().await
				} else {
					rx.recv().await
				}
			} => {
				let Some(msg) = msg else {
					if pending_stop.is_some() {
						rx_closed = true;
						continue;
					}
					break;
				};

				match msg {
					ToActor::Intent { intent, error } => {
						send_event(
							&mut ctx,
							protocol::Event::EventActorIntent(protocol::EventActorIntent { intent }),
						);
						if error.is_some() {
							ctx.error = error;
						}
					}
					ToActor::Stop {
						command_idx,
						reason,
					} => {
						if pending_stop.is_some() {
							tracing::warn!(
								command_idx,
								"ignoring duplicate stop while actor teardown is in progress"
							);
							continue;
						}
						if command_idx <= ctx.command_idx {
							tracing::warn!(command_idx, "ignoring already seen command");
							continue;
						}
						ctx.command_idx = command_idx;
						match begin_stop(&mut ctx, &handle, &mut http_request_tasks, reason).await {
							StopProgress::Stopped => break,
							StopProgress::Pending(stop) => pending_stop = Some(stop),
						}
					}
					ToActor::Lost => {
						if pending_stop.is_some() {
							tracing::warn!(
								"ignoring lost signal while actor teardown is in progress"
							);
							continue;
						}

						ctx.error = Some("actor lost due to timeout".to_string());

						match begin_stop(
							&mut ctx,
							&handle,
							&mut http_request_tasks,
							protocol::StopActorReason::SleepIntent,
						)
						.await
						{
							StopProgress::Stopped => break,
							StopProgress::Pending(stop) => pending_stop = Some(stop),
						}
					}
					ToActor::SetAlarm { alarm_ts, ack_tx } => {
						send_event(
							&mut ctx,
							protocol::Event::EventActorSetAlarm(protocol::EventActorSetAlarm { alarm_ts }),
						);
						if let Some(ack_tx) = ack_tx {
							let _ = ack_tx.send(());
						}
					}
					ToActor::ReqStart {
						message_id,
						req,
						connection_session,
						queued,
					} => {
						drop(queued);
						http::handle_req_start(
							&mut ctx,
							&handle,
							&mut http_request_tasks,
							message_id,
							req,
							connection_session,
						);
					}
					ToActor::ConnectionClosed { session } => {
						http::handle_connection_closed(&mut ctx, session).await;
					}
					ToActor::ReqChunk { message_id, chunk } => {
						http::handle_req_chunk(&mut ctx, message_id, chunk);
					}
					ToActor::ReqAbort { message_id, reason } => {
						http::handle_req_abort(&mut ctx, message_id, reason);
					}
					ToActor::ReqProtocolViolation { message_id, detail } => {
						http::handle_protocol_violation(&mut ctx, message_id, detail).await;
					}
					ToActor::ReqBodyCancelled { message_id } => {
						http::handle_req_body_cancelled(&mut ctx, message_id);
					}
					ToActor::ReqBodyCancel { message_id } => {
						http::handle_req_body_cancel(&mut ctx, message_id);
					}
					ToActor::ResponseBodyWindowUpdate {
						message_id,
						consumed_bytes,
					} => {
						http::handle_response_body_window_update(
							&mut ctx,
							message_id,
							consumed_bytes,
						)
						.await;
					}
					ToActor::ReqComplete { message_id } => {
						http::handle_req_complete(&mut ctx, message_id);
					}
					ToActor::WsOpen {
						message_id,
						path,
						headers,
					} => {
						handle_ws_open(&mut ctx, &handle, message_id, path, headers).await;
					}
					ToActor::WsMsg { message_id, msg } => {
						handle_ws_message(&mut ctx, message_id, msg).await;
					}
					ToActor::WsClose { message_id, close } => {
						handle_ws_close(&mut ctx, message_id, close).await;
					}
					ToActor::HwsAck {
						gateway_id,
						request_id,
						envoy_message_index,
					} => {
						handle_hws_ack(&mut ctx, gateway_id, request_id, envoy_message_index).await;
					}
				}
			}
			stop_result = async {
				let pending = pending_stop
					.as_mut()
					.expect("pending stop must exist when waiting for stop completion");
				(&mut pending.completion_rx).await
			}, if pending_stop.is_some() => {
				let pending = pending_stop
					.take()
					.expect("pending stop must exist when stop completion resolves");
				http::abort_and_join_tasks(&mut ctx, &mut http_request_tasks).await;
				finalize_stop(&mut ctx, pending, stop_result);
				break;
			}
		}
	}

	http::abort_and_join_tasks(&mut ctx, &mut http_request_tasks).await;
	tracing::debug!("envoy actor stopped");
}

fn send_event(ctx: &mut ActorContext, inner: protocol::Event) {
	let checkpoint = increment_checkpoint(ctx);
	let _ = crate::envoy::send_to_envoy_tx(
		&ctx.shared,
		crate::envoy::ToEnvoyMessage::SendEvents {
			events: vec![protocol::EventWrapper { checkpoint, inner }],
		},
	);
}

async fn begin_stop(
	ctx: &mut ActorContext,
	handle: &EnvoyHandle,
	_http_request_tasks: &mut JoinSet<()>,
	reason: protocol::StopActorReason,
) -> StopProgress {
	// A Lost stop must surface as Stopped(Error). The runner side detected its
	// own WS to pegboard-envoy was unhealthy and gave up on the actor; that is
	// not a graceful exit. If we emitted Stopped(Ok) here, pegboard's
	// `handle_stopped` would see Stopped(Ok) from `Transition::Running` (no
	// prior `ActorIntent` was sent) and take `Decision::Destroy`, wiping the
	// actor and its KV after every transient WS flap that exceeds
	// `envoy_lost_threshold`.
	let (mut stop_code, mut stop_message) = if let Some(err) = ctx.error.clone() {
		(protocol::StopCode::Error, Some(err))
	} else if matches!(reason, protocol::StopActorReason::Lost) {
		(
			protocol::StopCode::Error,
			Some("envoy connection lost".to_string()),
		)
	} else {
		(protocol::StopCode::Ok, None)
	};
	let (stop_tx, mut stop_rx) = oneshot::channel();

	let stop_result = ctx
		.shared
		.config
		.callbacks
		.on_actor_stop_with_completion(
			handle.clone(),
			ctx.actor_id.clone(),
			ctx.generation,
			reason.clone(),
			crate::callbacks::ActorStopHandle::new(stop_tx),
		)
		.await;

	if let Err(error) = stop_result {
		tracing::error!(?error, "actor stop failed");
		stop_code = protocol::StopCode::Error;
		if stop_message.is_none() {
			stop_message = Some(format!("{error:#}"));
		}
		send_stopped_event(ctx, stop_code, stop_message);
		return StopProgress::Stopped;
	}

	match stop_rx.try_recv() {
		Ok(stop_result) => {
			send_stopped_event_for_result(ctx, stop_code, stop_message, stop_result);
			StopProgress::Stopped
		}
		Err(TryRecvError::Empty) => StopProgress::Pending(PendingStop {
			completion_rx: stop_rx,
			stop_code,
			stop_message,
		}),
		Err(TryRecvError::Closed) => {
			send_stopped_event(ctx, stop_code, stop_message);
			StopProgress::Stopped
		}
	}
}

fn finalize_stop(
	ctx: &mut ActorContext,
	pending: PendingStop,
	stop_result: Result<anyhow::Result<()>, oneshot::error::RecvError>,
) {
	match stop_result {
		Ok(stop_result) => {
			send_stopped_event_for_result(
				ctx,
				pending.stop_code,
				pending.stop_message,
				stop_result,
			);
		}
		Err(error) => {
			tracing::warn!(
				?error,
				"actor stop completion handle dropped before signaling teardown result"
			);
			send_stopped_event(ctx, pending.stop_code, pending.stop_message);
		}
	}
}

fn send_stopped_event_for_result(
	ctx: &mut ActorContext,
	mut stop_code: protocol::StopCode,
	mut stop_message: Option<String>,
	stop_result: anyhow::Result<()>,
) {
	if let Err(error) = stop_result {
		tracing::error!(?error, "actor stop completion failed");
		stop_code = protocol::StopCode::Error;
		if stop_message.is_none() {
			stop_message = Some(format!("{error:#}"));
		}
	}

	send_stopped_event(ctx, stop_code, stop_message);
}

fn send_stopped_event(
	ctx: &mut ActorContext,
	stop_code: protocol::StopCode,
	stop_message: Option<String>,
) {
	send_event(
		ctx,
		protocol::Event::EventActorStateUpdate(protocol::EventActorStateUpdate {
			state: protocol::ActorState::ActorStateStopped(protocol::ActorStateStopped {
				code: stop_code,
				message: stop_message.map(Into::into),
			}),
		}),
	);
}

fn spawn_ws_outgoing_task(
	shared: Arc<SharedContext>,
	gateway_id: protocol::GatewayId,
	request_id: protocol::RequestId,
	mut outgoing_rx: mpsc::UnboundedReceiver<WsOutgoing>,
) {
	let ws_task = async move {
		let mut idx: u16 = 0;
		while let Some(msg) = outgoing_rx.recv().await {
			idx += 1;
			match msg {
				WsOutgoing::Message { data, binary } => {
					ws_send(
						&shared,
						protocol::ToRivet::ToRivetTunnelMessage(protocol::ToRivetTunnelMessage {
							message_id: protocol::MessageId {
								gateway_id,
								request_id,
								message_index: idx,
							},
							message_kind:
								protocol::ToRivetTunnelMessageKind::ToRivetWebSocketMessage(
									protocol::ToRivetWebSocketMessage {
										data: data.into(),
										binary,
									},
								),
						}),
					)
					.await;
				}
				WsOutgoing::Flush { tx } => {
					let _ = tx.send(());
				}
				WsOutgoing::Close { code, reason } => {
					ws_send(
						&shared,
						protocol::ToRivet::ToRivetTunnelMessage(protocol::ToRivetTunnelMessage {
							message_id: protocol::MessageId {
								gateway_id,
								request_id,
								message_index: 0,
							},
							message_kind: protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
								protocol::ToRivetWebSocketClose {
									code,
									reason: reason.map(Into::into),
									hibernate: false,
								},
							),
						}),
					)
					.await;
					break;
				}
			}
		}
	};
	spawn_detached(ws_task.in_current_span());
}

async fn handle_ws_open(
	ctx: &mut ActorContext,
	handle: &EnvoyHandle,
	message_id: protocol::MessageId,
	path: String,
	headers: BTreeMap<String, String>,
) {
	let restored_ws = ctx
		.ws_entries
		.remove(&[&message_id.gateway_id, &message_id.request_id]);
	let is_restoring_hibernatable = restored_ws
		.as_ref()
		.map(|ws| ws.is_hibernatable)
		.unwrap_or(false);

	if !is_restoring_hibernatable {
		ctx.websocket_requests.insert(
			&[&message_id.gateway_id, &message_id.request_id],
			WebSocketRequestState {
				envoy_message_index: 0,
			},
		);
	}

	let mut full_headers: HashMap<String, String> = headers.into_iter().collect();
	full_headers.insert("Upgrade".to_string(), "websocket".to_string());
	full_headers.insert("Connection".to_string(), "Upgrade".to_string());

	let request = HttpRequest {
		method: "GET".to_string(),
		path: path.clone(),
		headers: full_headers.clone(),
		body: None,
		body_stream: None,
	};

	let is_hibernatable = if is_restoring_hibernatable {
		true
	} else {
		match ctx
			.shared
			.config
			.callbacks
			.can_hibernate(
				&ctx.actor_id,
				&message_id.gateway_id,
				&message_id.request_id,
				&request,
			)
			.await
		{
			Ok(is_hibernatable) => is_hibernatable,
			Err(error) => {
				tracing::error!(?error, "error checking websocket hibernation");

				send_actor_message(
					ctx,
					message_id.gateway_id,
					message_id.request_id,
					protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
						protocol::ToRivetWebSocketClose {
							code: Some(1011),
							reason: Some(("Server Error".to_string()).into()),
							hibernate: false,
						},
					),
				)
				.await;

				ctx.websocket_requests
					.remove(&[&message_id.gateway_id, &message_id.request_id]);
				return;
			}
		}
	};

	// Create outgoing channel BEFORE calling websocket() so the sender is available immediately
	let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel::<WsOutgoing>();
	let sender = WebSocketSender {
		tx: outgoing_tx.clone(),
	};

	let ws_result = if is_restoring_hibernatable {
		ctx.shared
			.config
			.callbacks
			.websocket(
				handle.clone(),
				ctx.actor_id.clone(),
				message_id.gateway_id,
				message_id.request_id,
				request,
				path,
				full_headers,
				true,
				true,
				sender,
			)
			.await
	} else {
		ctx.shared
			.config
			.callbacks
			.websocket(
				handle.clone(),
				ctx.actor_id.clone(),
				message_id.gateway_id,
				message_id.request_id,
				request,
				path,
				full_headers,
				is_hibernatable,
				false,
				sender,
			)
			.await
	};

	match ws_result {
		Ok(ws_handler) => {
			ctx.ws_entries.insert(
				&[&message_id.gateway_id, &message_id.request_id],
				WsEntry {
					is_hibernatable,
					rivet_message_index: message_id.message_index,
					ws_handler: Some(ws_handler),
					outgoing_tx,
				},
			);

			spawn_ws_outgoing_task(
				ctx.shared.clone(),
				message_id.gateway_id,
				message_id.request_id,
				outgoing_rx,
			);

			// Gateway wake flows still wait for a websocket-open ack before they
			// resume forwarding buffered client messages, even when the request is
			// being restored after actor hibernation.
			send_actor_message(
				ctx,
				message_id.gateway_id,
				message_id.request_id,
				protocol::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(
					protocol::ToRivetWebSocketOpen {
						can_hibernate: is_hibernatable,
					},
				),
			)
			.await;

			// Call on_open if provided
			if let Some(ws) = ctx
				.ws_entries
				.get_mut(&[&message_id.gateway_id, &message_id.request_id])
			{
				if let Some(handler) = &mut ws.ws_handler {
					if let Some(on_open) = handler.on_open.take() {
						let sender = WebSocketSender {
							tx: ws.outgoing_tx.clone(),
						};

						on_open(sender).await;
					}
				}
			}
		}
		Err(error) => {
			tracing::error!(?error, "error handling websocket open");

			send_actor_message(
				ctx,
				message_id.gateway_id,
				message_id.request_id,
				protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
					protocol::ToRivetWebSocketClose {
						code: Some(1011),
						reason: Some(("Server Error".to_string()).into()),
						hibernate: false,
					},
				),
			)
			.await;

			ctx.websocket_requests
				.remove(&[&message_id.gateway_id, &message_id.request_id]);
			ctx.ws_entries
				.remove(&[&message_id.gateway_id, &message_id.request_id]);
		}
	}
}

async fn handle_ws_message(
	ctx: &mut ActorContext,
	message_id: protocol::MessageId,
	msg: protocol::ToEnvoyWebSocketMessage,
) {
	let ws = ctx
		.ws_entries
		.get_mut(&[&message_id.gateway_id, &message_id.request_id]);

	if let Some(ws) = ws {
		// Validate message index for hibernatable websockets
		if ws.is_hibernatable {
			let previous_index = ws.rivet_message_index;
			let received_index = message_id.message_index;

			if wrapping_lte_u16(received_index, previous_index) {
				tracing::info!(
					request_id = id_to_str(&message_id.request_id),
					previous_index,
					received_index,
					"received duplicate hibernating websocket message"
				);
				return;
			}

			let expected_index = wrapping_add_u16(previous_index, 1);
			if received_index != expected_index {
				tracing::warn!(
					request_id = id_to_str(&message_id.request_id),
					previous_index,
					expected_index,
					received_index,
					gap = wrapping_sub_u16(wrapping_sub_u16(received_index, previous_index), 1),
					"hibernatable websocket message index out of sequence, closing connection"
				);

				send_actor_message(
					ctx,
					message_id.gateway_id,
					message_id.request_id,
					protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
						protocol::ToRivetWebSocketClose {
							code: Some(1008),
							reason: Some(("ws.message_index_skip".to_string()).into()),
							hibernate: false,
						},
					),
				)
				.await;
				return;
			}

			ws.rivet_message_index = received_index;
		}

		if let Some(handler) = &ws.ws_handler {
			let sender = WebSocketSender {
				tx: ws.outgoing_tx.clone(),
			};
			let ws_msg = WebSocketMessage {
				data: msg.data.to_vec(),
				binary: msg.binary,
				gateway_id: message_id.gateway_id,
				request_id: message_id.request_id,
				message_index: message_id.message_index,
				sender,
			};
			(handler.on_message)(ws_msg).await;
		}
	} else {
		tracing::warn!("received message for unknown ws");
	}
}

async fn handle_ws_close(
	ctx: &mut ActorContext,
	message_id: protocol::MessageId,
	close: protocol::ToEnvoyWebSocketClose,
) {
	let ws = ctx
		.ws_entries
		.remove(&[&message_id.gateway_id, &message_id.request_id]);

	if let Some(ws) = ws {
		if let Some(handler) = &ws.ws_handler {
			let code = close.code.unwrap_or(1000);
			let reason = close.reason.unwrap_or_default();
			(handler.on_close)(code, reason.to_string()).await;
		}
		ctx.websocket_requests
			.remove(&[&message_id.gateway_id, &message_id.request_id]);
	} else {
		tracing::warn!("received close for unknown ws");
	}
}

async fn handle_hws_restore(
	ctx: &mut ActorContext,
	handle: &EnvoyHandle,
	meta_entries: Vec<crate::tunnel::HibernatingWebSocketMetadata>,
) -> anyhow::Result<()> {
	tracing::debug!(
		requests = ctx.hibernating_requests.len(),
		"restoring hibernating requests"
	);

	let hibernating_requests = std::mem::take(&mut ctx.hibernating_requests);

	for hib_req in &hibernating_requests {
		let meta = meta_entries.iter().find(|entry| {
			entry.gateway_id == hib_req.gateway_id && entry.request_id == hib_req.request_id
		});

		if let Some(meta) = meta {
			ctx.websocket_requests.insert(
				&[&hib_req.gateway_id, &hib_req.request_id],
				WebSocketRequestState {
					envoy_message_index: meta.envoy_message_index,
				},
			);

			let mut full_headers = meta.headers.clone();
			full_headers.insert("Upgrade".to_string(), "websocket".to_string());
			full_headers.insert("Connection".to_string(), "Upgrade".to_string());

			let request = HttpRequest {
				method: "GET".to_string(),
				path: meta.path.clone(),
				headers: full_headers.clone(),
				body: None,
				body_stream: None,
			};

			let (hws_outgoing_tx, hws_outgoing_rx) = mpsc::unbounded_channel();
			let hws_sender = WebSocketSender {
				tx: hws_outgoing_tx.clone(),
			};

			let ws_result = ctx
				.shared
				.config
				.callbacks
				.websocket(
					handle.clone(),
					ctx.actor_id.clone(),
					hib_req.gateway_id,
					hib_req.request_id,
					request,
					meta.path.clone(),
					full_headers,
					true,
					true,
					hws_sender,
				)
				.await;

			match ws_result {
				Ok(ws_handler) => {
					ctx.ws_entries.insert(
						&[&hib_req.gateway_id, &hib_req.request_id],
						WsEntry {
							is_hibernatable: true,
							rivet_message_index: meta.rivet_message_index,
							ws_handler: Some(ws_handler),
							outgoing_tx: hws_outgoing_tx,
						},
					);
					if !handle
						.rebind_hibernating_websocket(
							ctx.actor_id.clone(),
							ctx.generation,
							hib_req.gateway_id,
							hib_req.request_id,
						)
						.await
					{
						tracing::warn!(
							request_id = id_to_str(&hib_req.request_id),
							"hibernating websocket route disappeared before restore"
						);
						ctx.websocket_requests
							.remove(&[&hib_req.gateway_id, &hib_req.request_id]);
						ctx.ws_entries
							.remove(&[&hib_req.gateway_id, &hib_req.request_id]);
						continue;
					}
					// Gateway wake flows wait for the websocket-open ack before
					// they resume forwarding buffered client messages.
					send_actor_message(
						ctx,
						hib_req.gateway_id,
						hib_req.request_id,
						protocol::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(
							protocol::ToRivetWebSocketOpen {
								can_hibernate: true,
							},
						),
					)
					.await;
					// Writes made by the restored callback are already queued in
					// `hws_outgoing_rx`. Start forwarding them only after the exact route
					// is rebound and the restore Open frame has entered the ordered tunnel.
					spawn_ws_outgoing_task(
						ctx.shared.clone(),
						hib_req.gateway_id,
						hib_req.request_id,
						hws_outgoing_rx,
					);
					if let Some(ws) = ctx
						.ws_entries
						.get_mut(&[&hib_req.gateway_id, &hib_req.request_id])
					{
						if let Some(handler) = &mut ws.ws_handler {
							if let Some(on_open) = handler.on_open.take() {
								let sender = WebSocketSender {
									tx: ws.outgoing_tx.clone(),
								};

								on_open(sender).await;
							}
						}
					}
					tracing::info!(
						request_id = id_to_str(&hib_req.request_id),
						"connection successfully restored"
					);
				}
				Err(error) => {
					tracing::error!(
						request_id = id_to_str(&hib_req.request_id),
						?error,
						"error creating websocket during restore"
					);

					send_actor_message(
						ctx,
						hib_req.gateway_id,
						hib_req.request_id,
						protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
							protocol::ToRivetWebSocketClose {
								code: Some(1011),
								reason: Some(("ws.restore_error".to_string()).into()),
								hibernate: false,
							},
						),
					)
					.await;

					ctx.websocket_requests
						.remove(&[&hib_req.gateway_id, &hib_req.request_id]);
				}
			}
		} else {
			tracing::warn!(
				request_id = id_to_str(&hib_req.request_id),
				"closing websocket that is not persisted"
			);

			send_actor_message(
				ctx,
				hib_req.gateway_id,
				hib_req.request_id,
				protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(
					protocol::ToRivetWebSocketClose {
						code: Some(1000),
						reason: Some(("ws.meta_not_found_during_restore".to_string()).into()),
						hibernate: false,
					},
				),
			)
			.await;
		}
	}

	// Process loaded but not connected (stale)
	for meta in &meta_entries {
		let is_connected = hibernating_requests
			.iter()
			.any(|req| req.gateway_id == meta.gateway_id && req.request_id == meta.request_id);

		if !is_connected {
			tracing::debug!(
				request_id = id_to_str(&meta.request_id),
				"removing stale persisted websocket"
			);

			let full_headers = meta.headers.clone();
			let request = HttpRequest {
				method: "GET".to_string(),
				path: meta.path.clone(),
				headers: full_headers.clone(),
				body: None,
				body_stream: None,
			};

			let (stale_tx, _) = mpsc::unbounded_channel();
			let stale_sender = WebSocketSender { tx: stale_tx };

			let ws_result = ctx
				.shared
				.config
				.callbacks
				.websocket(
					handle.clone(),
					ctx.actor_id.clone(),
					meta.gateway_id,
					meta.request_id,
					request,
					meta.path.clone(),
					full_headers,
					true,
					true,
					stale_sender,
				)
				.await;

			if let Ok(handler) = ws_result {
				(handler.on_close)(1000, "ws.stale_metadata".to_string()).await;
			}
		}
	}

	ctx.hibernating_requests = hibernating_requests;
	tracing::info!("restored hibernatable websockets");
	Ok(())
}

async fn handle_hws_ack(
	ctx: &mut ActorContext,
	gateway_id: protocol::GatewayId,
	request_id: protocol::RequestId,
	envoy_message_index: u16,
) {
	tracing::debug!(
		request_id = id_to_str(&request_id),
		index = envoy_message_index,
		"ack ws msg"
	);

	send_actor_message(
		ctx,
		gateway_id,
		request_id,
		protocol::ToRivetTunnelMessageKind::ToRivetWebSocketMessageAck(
			protocol::ToRivetWebSocketMessageAck {
				index: envoy_message_index,
			},
		),
	)
	.await;
}

fn increment_checkpoint(ctx: &mut ActorContext) -> protocol::ActorCheckpoint {
	let index = ctx.event_index;
	ctx.event_index += 1;
	protocol::ActorCheckpoint {
		actor_id: ctx.actor_id.clone().into(),
		generation: ctx.generation,
		index,
	}
}

async fn send_actor_message(
	ctx: &mut ActorContext,
	gateway_id: protocol::GatewayId,
	request_id: protocol::RequestId,
	message_kind: protocol::ToRivetTunnelMessageKind,
) {
	let req = ctx.websocket_requests.get_mut(&[&gateway_id, &request_id]);
	let envoy_message_index = if let Some(req) = req {
		let idx = req.envoy_message_index;
		req.envoy_message_index += 1;
		idx
	} else {
		tracing::warn!(
			gateway_id = id_to_str(&gateway_id),
			request_id = id_to_str(&request_id),
			"missing pending request for send message"
		);
		return;
	};

	let msg = protocol::ToRivetTunnelMessage {
		message_id: protocol::MessageId {
			gateway_id,
			request_id,
			message_index: envoy_message_index,
		},
		message_kind: message_kind.clone(),
	};

	let buffer_msg = msg.clone();
	let failed = ws_send(&ctx.shared, protocol::ToRivet::ToRivetTunnelMessage(msg)).await;

	if failed {
		if tracing::enabled!(tracing::Level::DEBUG) {
			tracing::debug!(
				request_id = id_to_str(&request_id),
				message = stringify_to_rivet_tunnel_message_kind(&message_kind),
				"buffering tunnel message, socket not connected to engine"
			);
		}
		let _ = crate::envoy::send_to_envoy_tx(
			&ctx.shared,
			crate::envoy::ToEnvoyMessage::SendOrBufferTunnelMsg { msg: buffer_msg },
		);
	}
}




