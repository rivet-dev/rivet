use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::TryStreamExt;
use gas::prelude::*;
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, StatusCode};
use pegboard::tunnel::id::{self as tunnel_id, RequestId};
use rand::Rng;
use rivet_error::*;
use rivet_guard_core::{
	custom_serve::{CustomServeTrait, HibernationResult},
	errors::{ServiceUnavailable, WebSocketServiceUnavailable},
	proxy_service::{is_ws_hibernate, ResponseBody},
	request_context::RequestContext,
	websocket_handle::WebSocketReceiver,
	WebSocketHandle,
};
use rivet_runner_protocol as protocol;
use rivet_util::serde::HashableMap;
use std::{sync::Arc, time::Duration};
use tokio::{
	sync::{watch, Mutex},
	task::JoinHandle,
};
use tokio_tungstenite::tungstenite::{
	protocol::frame::{coding::CloseCode, CloseFrame},
	Message,
};

use crate::shared_state::{InFlightRequestHandle, SharedState};

mod metrics;
mod ping_task;
pub mod shared_state;
mod tunnel_to_ws_task;
mod ws_to_tunnel_task;

const WEBSOCKET_OPEN_TIMEOUT: Duration = Duration::from_secs(15);
const TUNNEL_ACK_TIMEOUT: Duration = Duration::from_secs(5);
const UPDATE_PING_INTERVAL: Duration = Duration::from_secs(3);

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"guard",
	"websocket_pending_limit_reached",
	"Reached limit on pending websocket messages, aborting connection."
)]
pub struct WebsocketPendingLimitReached;

#[derive(Debug)]
enum LifecycleResult {
	ServerClose(protocol::ToServerWebSocketClose),
	ClientClose(Option<CloseFrame>),
	Aborted,
}

pub struct PegboardGateway {
	ctx: StandaloneCtx,
	shared_state: SharedState,
	runner_id: Id,
	actor_id: Id,
	path: String,
}

impl PegboardGateway {
	#[tracing::instrument(skip_all, fields(?actor_id, ?runner_id, ?path))]
	pub fn new(
		ctx: StandaloneCtx,
		shared_state: SharedState,
		runner_id: Id,
		actor_id: Id,
		path: String,
	) -> Self {
		Self {
			ctx,
			shared_state,
			runner_id,
			actor_id,
			path,
		}
	}
}

#[async_trait]
impl CustomServeTrait for PegboardGateway {
	#[tracing::instrument(skip_all, fields(actor_id=?self.actor_id, runner_id=?self.runner_id))]
	async fn handle_request(
		&self,
		req: Request<Full<Bytes>>,
		_request_context: &mut RequestContext,
		request_id: RequestId,
	) -> Result<Response<ResponseBody>> {
		// Use the actor ID from the gateway instance
		let actor_id = self.actor_id.to_string();

		// Extract origin for CORS (before consuming request)
		// When credentials: true, we must echo back the actual origin, not "*"
		let origin = req
			.headers()
			.get("origin")
			.and_then(|v| v.to_str().ok())
			.unwrap_or("*")
			.to_string();

		// Extract request parts
		let mut headers = HashableMap::new();
		for (name, value) in req.headers() {
			if let Result::Ok(value_str) = value.to_str() {
				headers.insert(name.to_string(), value_str.to_string());
			}
		}

		// Extract method and path before consuming the request
		let method = req.method().to_string();

		// Handle CORS preflight OPTIONS requests at gateway level
		//
		// We need to do this in Guard because there is no way of sending an OPTIONS request to the
		// actor since we don't have the `x-rivet-token` header. This implementation allows
		// requests from anywhere and lets the actor handle CORS manually in `onBeforeConnect`.
		// This had the added benefit of also applying to WebSockets.
		if req.method() == hyper::Method::OPTIONS {
			tracing::debug!("handling OPTIONS preflight request at gateway");

			// Extract requested headers
			let requested_headers = req
				.headers()
				.get("access-control-request-headers")
				.and_then(|v| v.to_str().ok())
				.unwrap_or("*");

			let mut response = Response::builder()
				.status(StatusCode::NO_CONTENT)
				.header("access-control-allow-origin", &origin)
				.header("access-control-allow-credentials", "true")
				.header(
					"access-control-allow-methods",
					"GET, POST, PUT, DELETE, OPTIONS, PATCH",
				)
				.header("access-control-allow-headers", requested_headers)
				.header("access-control-expose-headers", "*")
				.header("access-control-max-age", "86400");

			// Add Vary header to prevent cache poisoning when echoing origin
			if origin != "*" {
				response = response.header("vary", "Origin");
			}

			return Ok(response.body(ResponseBody::Full(Full::new(Bytes::new())))?);
		}

		let body_bytes = req
			.into_body()
			.collect()
			.await
			.context("failed to read body")?
			.to_bytes();

		let mut stopped_sub = self
			.ctx
			.subscribe::<pegboard::workflows::actor::Stopped>(("actor_id", self.actor_id))
			.await?;

		// Build subject to publish to
		let tunnel_subject =
			pegboard::pubsub_subjects::RunnerReceiverSubject::new(self.runner_id).to_string();

		// Start listening for request responses
		let InFlightRequestHandle {
			mut msg_rx,
			mut drop_rx,
		} = self
			.shared_state
			.start_in_flight_request(tunnel_subject, request_id)
			.await;

		// Start request
		let message = protocol::ToClientTunnelMessageKind::ToClientRequestStart(
			protocol::ToClientRequestStart {
				actor_id: actor_id.clone(),
				method,
				path: self.path.clone(),
				headers,
				body: if body_bytes.is_empty() {
					None
				} else {
					Some(body_bytes.to_vec())
				},
				stream: false,
			},
		);
		self.shared_state.send_message(request_id, message).await?;

		// Wait for response
		tracing::debug!("gateway waiting for response from tunnel");
		let fut = async {
			loop {
				tokio::select! {
					res = msg_rx.recv() => {
						if let Some(msg) = res {
							match msg {
								protocol::ToServerTunnelMessageKind::ToServerResponseStart(
									response_start,
								) => {
									return anyhow::Ok(response_start);
								}
								protocol::ToServerTunnelMessageKind::ToServerResponseAbort => {
									tracing::warn!("request aborted");
									return Err(ServiceUnavailable.build());
								}
								_ => {
									tracing::warn!("received non-response message from pubsub");
								}
							}
						} else {
							tracing::warn!(
								request_id=?tunnel_id::request_id_to_string(&request_id),
								"received no message response during request init",
							);
							break;
						}
					}
					_ = stopped_sub.next() => {
						tracing::debug!("actor stopped while waiting for request response");
						return Err(ServiceUnavailable.build());
					}
					_ = drop_rx.changed() => {
						tracing::warn!("tunnel message timeout");
						return Err(ServiceUnavailable.build());
					}
				}
			}

			Err(ServiceUnavailable.build())
		};
		let response_start = tokio::time::timeout(WEBSOCKET_OPEN_TIMEOUT, fut)
			.await
			.map_err(|_| {
				tracing::warn!("timed out waiting for tunnel ack");

				ServiceUnavailable.build()
			})??;
		tracing::debug!("response handler task ended");

		// Build HTTP response
		let mut response_builder =
			Response::builder().status(StatusCode::from_u16(response_start.status)?);

		// Add headers from actor
		for (key, value) in response_start.headers {
			response_builder = response_builder.header(key, value);
		}

		// Add CORS headers to actual request
		response_builder = response_builder
			.header("access-control-allow-origin", &origin)
			.header("access-control-allow-credentials", "true")
			.header("access-control-expose-headers", "*");

		// Add Vary header to prevent cache poisoning when echoing origin
		if origin != "*" {
			response_builder = response_builder.header("vary", "Origin");
		}

		// Add body
		let body = response_start.body.unwrap_or_default();
		let response = response_builder.body(ResponseBody::Full(Full::new(Bytes::from(body))))?;

		Ok(response)
	}

	#[tracing::instrument(skip_all, fields(actor_id=?self.actor_id, runner_id=?self.runner_id))]
	async fn handle_websocket(
		&self,
		client_ws: WebSocketHandle,
		headers: &hyper::HeaderMap,
		_path: &str,
		_request_context: &mut RequestContext,
		unique_request_id: RequestId,
		after_hibernation: bool,
	) -> Result<Option<CloseFrame>> {
		// Use the actor ID from the gateway instance
		let actor_id = self.actor_id.to_string();

		// Extract headers
		let mut request_headers = HashableMap::new();
		for (name, value) in headers {
			if let Result::Ok(value_str) = value.to_str() {
				request_headers.insert(name.to_string(), value_str.to_string());
			}
		}

		let mut stopped_sub = self
			.ctx
			.subscribe::<pegboard::workflows::actor::Stopped>(("actor_id", self.actor_id))
			.await?;

		// Build subject to publish to
		let tunnel_subject =
			pegboard::pubsub_subjects::RunnerReceiverSubject::new(self.runner_id).to_string();

		// Start listening for WebSocket messages
		let request_id = unique_request_id;
		let InFlightRequestHandle {
			mut msg_rx,
			mut drop_rx,
		} = self
			.shared_state
			.start_in_flight_request(tunnel_subject.clone(), request_id)
			.await;

		// If we are reconnecting after hibernation, don't send an open message
		let can_hibernate = if after_hibernation {
			true
		} else {
			// Send WebSocket open message
			let open_message = protocol::ToClientTunnelMessageKind::ToClientWebSocketOpen(
				protocol::ToClientWebSocketOpen {
					actor_id: actor_id.clone(),
					path: self.path.clone(),
					headers: request_headers,
				},
			);

			self.shared_state
				.send_message(request_id, open_message)
				.await?;

			tracing::debug!("gateway waiting for websocket open from tunnel");

			// Wait for WebSocket open acknowledgment
			let fut = async {
				loop {
					tokio::select! {
						res = msg_rx.recv() => {
							if let Some(msg) = res {
								match msg {
									protocol::ToServerTunnelMessageKind::ToServerWebSocketOpen(msg) => {
										return anyhow::Ok(msg);
									}
									protocol::ToServerTunnelMessageKind::ToServerWebSocketClose(close) => {
										tracing::warn!(?close, "websocket closed before opening");
										return Err(WebSocketServiceUnavailable.build());
									}
									_ => {
										tracing::warn!(
											"received unexpected message while waiting for websocket open"
										);
									}
								}
							} else {
								tracing::warn!(
									request_id=?tunnel_id::request_id_to_string(&request_id),
									"received no message response during ws init",
								);
								break;
							}
						}
						_ = stopped_sub.next() => {
							tracing::debug!("actor stopped while waiting for websocket open");
							return Err(WebSocketServiceUnavailable.build());
						}
						_ = drop_rx.changed() => {
							tracing::warn!("websocket open timeout");
							return Err(WebSocketServiceUnavailable.build());
						}
					}
				}

				Err(WebSocketServiceUnavailable.build())
			};

			let open_msg = tokio::time::timeout(TUNNEL_ACK_TIMEOUT, fut)
				.await
				.map_err(|_| {
					tracing::warn!("timed out waiting for tunnel ack");

					WebSocketServiceUnavailable.build()
				})??;

			self.shared_state
				.toggle_hibernation(request_id, open_msg.can_hibernate)
				.await?;

			open_msg.can_hibernate
		};

		// Send pending messages
		self.shared_state
			.resend_pending_websocket_messages(request_id)
			.await?;

		let ws_rx = client_ws.recv();

		let (tunnel_to_ws_abort_tx, tunnel_to_ws_abort_rx) = watch::channel(());
		let (ws_to_tunnel_abort_tx, ws_to_tunnel_abort_rx) = watch::channel(());
		let (ping_abort_tx, ping_abort_rx) = watch::channel(());

		let tunnel_to_ws = tokio::spawn(tunnel_to_ws_task::task(
			self.shared_state.clone(),
			client_ws,
			request_id,
			stopped_sub,
			msg_rx,
			drop_rx,
			can_hibernate,
			tunnel_to_ws_abort_rx,
		));
		let ws_to_tunnel = tokio::spawn(ws_to_tunnel_task::task(
			self.shared_state.clone(),
			request_id,
			ws_rx,
			ws_to_tunnel_abort_rx,
		));
		let ping = tokio::spawn(ping_task::task(
			self.shared_state.clone(),
			request_id,
			ping_abort_rx,
		));

		let tunnel_to_ws_abort_tx2 = tunnel_to_ws_abort_tx.clone();
		let ws_to_tunnel_abort_tx2 = ws_to_tunnel_abort_tx.clone();
		let ping_abort_tx2 = ping_abort_tx.clone();

		// Wait for both tasks to complete
		let (tunnel_to_ws_res, ws_to_tunnel_res, ping_res) = tokio::join!(
			async {
				let res = tunnel_to_ws.await?;

				// Abort other if not aborted
				if !matches!(res, Ok(LifecycleResult::Aborted)) {
					tracing::debug!(?res, "tunnel to ws task completed, aborting counterpart");

					let _ = ping_abort_tx.send(());
					let _ = ws_to_tunnel_abort_tx.send(());
				} else {
					tracing::debug!(?res, "tunnel to ws task completed");
				}

				res
			},
			async {
				let res = ws_to_tunnel.await?;

				// Abort other if not aborted
				if !matches!(res, Ok(LifecycleResult::Aborted)) {
					tracing::debug!(?res, "ws to tunnel task completed, aborting counterpart");

					let _ = ping_abort_tx2.send(());
					let _ = tunnel_to_ws_abort_tx.send(());
				} else {
					tracing::debug!(?res, "ws to tunnel task completed");
				}

				res
			},
			async {
				let res = ping.await?;

				// Abort others if not aborted
				if !matches!(res, Ok(LifecycleResult::Aborted)) {
					tracing::debug!(?res, "ping task completed, aborting others");

					let _ = ws_to_tunnel_abort_tx2.send(());
					let _ = tunnel_to_ws_abort_tx2.send(());
				} else {
					tracing::debug!(?res, "ping task completed");
				}

				res
			},
		);

		// Determine single result from all tasks
		let mut lifecycle_res = match (tunnel_to_ws_res, ws_to_tunnel_res, ping_res) {
			// Prefer error
			(Err(err), _, _) => Err(err),
			(_, Err(err), _) => Err(err),
			(_, _, Err(err)) => Err(err),
			// Prefer non aborted result if both succeed
			(Ok(res), Ok(LifecycleResult::Aborted), _) => Ok(res),
			(Ok(LifecycleResult::Aborted), Ok(res), _) => Ok(res),
			// Unlikely case
			(res, _, _) => res,
		};

		// Send close frame to runner if not hibernating
		if !&lifecycle_res
			.as_ref()
			.map_or_else(is_ws_hibernate, |_| false)
		{
			let (close_code, close_reason) = match &mut lifecycle_res {
				// Taking here because it won't be used again
				Ok(LifecycleResult::ClientClose(Some(close))) => {
					(close.code, Some(std::mem::take(&mut close.reason)))
				}
				Ok(_) => (CloseCode::Normal.into(), None),
				Err(_) => (CloseCode::Error.into(), Some("ws.downstream_closed".into())),
			};
			let close_message = protocol::ToClientTunnelMessageKind::ToClientWebSocketClose(
				protocol::ToClientWebSocketClose {
					code: Some(close_code.into()),
					reason: close_reason.map(|x| x.as_str().to_string()),
				},
			);

			if let Err(err) = self
				.shared_state
				.send_message(request_id, close_message)
				.await
			{
				tracing::error!(?err, "error sending close message");
			}
		}

		// Send WebSocket close message to client
		match lifecycle_res {
			Ok(LifecycleResult::ServerClose(close)) => {
				if let Some(code) = close.code {
					Ok(Some(CloseFrame {
						code: code.into(),
						reason: close.reason.unwrap_or_default().into(),
					}))
				} else {
					Ok(None)
				}
			}
			Ok(_) => Ok(None),
			Err(err) => Err(err),
		}
	}

	#[tracing::instrument(skip_all, fields(actor_id=?self.actor_id))]
	async fn handle_websocket_hibernation(
		&self,
		client_ws: WebSocketHandle,
		unique_request_id: RequestId,
	) -> Result<HibernationResult> {
		let request_id = unique_request_id;

		// Immediately rewake if we have pending messages
		if self
			.shared_state
			.has_pending_websocket_messages(request_id)
			.await?
		{
			tracing::debug!(
				?unique_request_id,
				"detected pending requests on websocket hibernation, rewaking actor"
			);
			return Ok(HibernationResult::Continue);
		}

		// Start keepalive task
		let ctx = self.ctx.clone();
		let actor_id = self.actor_id;
		let gateway_id = self.shared_state.gateway_id();
		let request_id = unique_request_id;
		let keepalive_handle: JoinHandle<Result<()>> = tokio::spawn(async move {
			let mut ping_interval = tokio::time::interval(Duration::from_millis(
				(ctx.config()
					.pegboard()
					.hibernating_request_eligible_threshold()
					/ 2)
				.try_into()?,
			));
			ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

			loop {
				ping_interval.tick().await;

				// Jitter sleep to prevent stampeding herds
				let jitter = { rand::thread_rng().gen_range(0..128) };
				tokio::time::sleep(Duration::from_millis(jitter)).await;

				ctx.op(pegboard::ops::actor::hibernating_request::upsert::Input {
					actor_id,
					gateway_id,
					request_id,
				})
				.await?;
			}
		});

		let res = self.handle_websocket_hibernation_inner(client_ws).await;

		keepalive_handle.abort();

		match &res {
			Ok(HibernationResult::Continue) => {}
			Ok(HibernationResult::Close) | Err(_) => {
				// No longer an active hibernating request, delete entry
				self.ctx
					.op(pegboard::ops::actor::hibernating_request::delete::Input {
						actor_id: self.actor_id,
						gateway_id: self.shared_state.gateway_id(),
						request_id: unique_request_id,
					})
					.await?;
			}
		}

		res
	}
}

impl PegboardGateway {
	async fn handle_websocket_hibernation_inner(
		&self,
		client_ws: WebSocketHandle,
	) -> Result<HibernationResult> {
		let mut ready_sub = self
			.ctx
			.subscribe::<pegboard::workflows::actor::Ready>(("actor_id", self.actor_id))
			.await?;

		let res = tokio::select! {
			_ = ready_sub.next() => {
				tracing::debug!("actor became ready during hibernation");

				HibernationResult::Continue
			}
			hibernation_res = hibernate_ws(client_ws.recv()) => {
				let hibernation_res = hibernation_res?;

				match &hibernation_res {
					HibernationResult::Continue => {
						tracing::debug!("received message during hibernation");
					}
					HibernationResult::Close => {
						tracing::debug!("websocket stream closed during hibernation");
					}
				}

				hibernation_res
			}
		};

		Ok(res)
	}
}

async fn hibernate_ws(ws_rx: Arc<Mutex<WebSocketReceiver>>) -> Result<HibernationResult> {
	let mut guard = ws_rx.lock().await;
	let mut pinned = std::pin::Pin::new(&mut *guard);

	loop {
		if let Some(msg) = pinned.as_mut().peek().await {
			match msg {
				Ok(Message::Binary(_)) | Ok(Message::Text(_)) => {
					return Ok(HibernationResult::Continue);
				}
				// We don't care about the close frame because we're currently hibernating; there is no
				// downstream to send the close frame to.
				Ok(Message::Close(_)) => return Ok(HibernationResult::Close),
				// Ignore rest
				_ => {
					pinned.try_next().await?;
				}
			}
		} else {
			return Ok(HibernationResult::Close);
		}
	}
}
