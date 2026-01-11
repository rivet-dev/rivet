use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::TryStreamExt;
use gas::prelude::*;
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, StatusCode, body::Body};
use rivet_error::*;
use rivet_guard_core::{
	ResponseBody, WebSocketHandle,
	custom_serve::{CustomServeTrait, HibernationResult},
	errors::{RequestBodyTooLarge, ServiceUnavailable, WebSocketServiceUnavailable},
	request_context::{CorsConfig, RequestContext},
	utils::is_ws_hibernate,
	websocket_handle::WebSocketReceiver,
};
use rivet_runner_protocol::{self as protocol, PROTOCOL_MK1_VERSION};
use rivet_util::serde::HashableMap;
use std::{
	sync::{Arc, atomic::AtomicU64},
	time::Duration,
};
use tokio::sync::{Mutex, watch};
use tokio_tungstenite::tungstenite::{
	Message,
	protocol::frame::{CloseFrame, coding::CloseCode},
};
use universaldb::utils::IsolationLevel::*;

use crate::shared_state::{InFlightRequestHandle, SharedState};

mod keepalive_task;
mod metrics;
mod metrics_task;
mod ping_task;
pub mod shared_state;
mod tunnel_to_ws_task;
mod ws_to_tunnel_task;

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"guard",
	"websocket_pending_limit_reached",
	"Reached limit on pending websocket messages, aborting connection."
)]
pub struct WebsocketPendingLimitReached;

const UPDATE_METRICS_INTERVAL: Duration = Duration::from_secs(15);

#[derive(Debug)]
enum LifecycleResult {
	ServerClose(protocol::mk2::ToServerWebSocketClose),
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

impl PegboardGateway {
	async fn handle_request_inner(
		&self,
		ctx: &StandaloneCtx,
		req: Request<Full<Bytes>>,
		req_ctx: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
		// Use the actor ID from the gateway instance
		let actor_id = self.actor_id.to_string();
		let request_id = req_ctx.in_flight_request_id()?;

		// Extract origin for CORS (before consuming request)
		// When credentials: true, we must echo back the actual origin, not "*"
		let origin = req
			.headers()
			.get("origin")
			.and_then(|v| v.to_str().ok())
			.unwrap_or("*")
			.to_string();

		// Extract request parts
		let headers = req
			.headers()
			.iter()
			.filter_map(|(name, value)| {
				value
					.to_str()
					.ok()
					.map(|value_str| (name.to_string(), value_str.to_string()))
			})
			.collect::<HashableMap<_, _>>();

		// Handle CORS preflight OPTIONS requests at gateway level
		//
		// We need to do this in the gateway because there is no way of sending an OPTIONS request to the
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

			req_ctx.set_cors(CorsConfig {
				allow_origin: origin.clone(),
				allow_credentials: true,
				expose_headers: "*".to_string(),
				allow_methods: Some("GET, POST, PUT, DELETE, OPTIONS, PATCH".to_string()),
				allow_headers: Some(requested_headers.to_string()),
				max_age: Some(86400),
			});

			return Ok(Response::builder()
				.status(StatusCode::NO_CONTENT)
				.body(ResponseBody::Full(Full::new(Bytes::new())))?);
		}

		// Set CORS headers through guard
		req_ctx.set_cors(CorsConfig {
			allow_origin: origin.clone(),
			allow_credentials: true,
			expose_headers: "*".to_string(),
			// Not an options req, not required
			allow_methods: None,
			allow_headers: None,
			max_age: None,
		});

		let body_bytes = req
			.into_body()
			.collect()
			.await
			.context("failed to read body")?
			.to_bytes();

		// Check request body size limit for requests to actors
		let max_request_body_size = self
			.ctx
			.config()
			.pegboard()
			.gateway_http_max_request_body_size();
		if body_bytes.len() > max_request_body_size {
			return Err(RequestBodyTooLarge {
				size: body_bytes.len(),
				max_size: max_request_body_size,
			}
			.build());
		}

		let (mut stopped_sub, runner_protocol_version) = tokio::try_join!(
			ctx.subscribe::<pegboard::workflows::actor::Stopped>(("actor_id", self.actor_id)),
			get_runner_protocol_version(&ctx, self.runner_id),
		)?;

		// Build subject to publish to
		let tunnel_subject =
			pegboard::pubsub_subjects::RunnerReceiverSubject::new(self.runner_id).to_string();

		// Start listening for request responses
		let InFlightRequestHandle {
			mut msg_rx,
			mut drop_rx,
			..
		} = self
			.shared_state
			.start_in_flight_request(tunnel_subject, runner_protocol_version, request_id)
			.await;

		// Start request
		let message = protocol::mk2::ToClientTunnelMessageKind::ToClientRequestStart(
			protocol::mk2::ToClientRequestStart {
				actor_id: actor_id.clone(),
				method: req_ctx.method().to_string(),
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
								protocol::mk2::ToServerTunnelMessageKind::ToServerResponseStart(
									response_start,
								) => {
									return anyhow::Ok(response_start);
								}
								protocol::mk2::ToServerTunnelMessageKind::ToServerResponseAbort => {
									tracing::warn!("request aborted");
									return Err(ServiceUnavailable.build());
								}
								_ => {
									tracing::warn!("received non-response message from pubsub");
								}
							}
						} else {
							tracing::warn!(
								request_id=%protocol::util::id_to_string(&request_id),
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
						tracing::warn!(reason=?drop_rx.borrow(), "tunnel message timeout");
						return Err(ServiceUnavailable.build());
					}
				}
			}

			Err(ServiceUnavailable.build())
		};
		let response_start_timeout = Duration::from_millis(
			self.ctx
				.config()
				.pegboard()
				.gateway_response_start_timeout_ms(),
		);
		let response_start = tokio::time::timeout(response_start_timeout, fut)
			.await
			.map_err(|_| {
				tracing::warn!("timed out waiting for response start from runner");

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

		// Add body
		let body = response_start.body.unwrap_or_default();
		let response = response_builder.body(ResponseBody::Full(Full::new(Bytes::from(body))))?;

		Ok(response)
	}

	async fn handle_websocket_inner(
		&self,
		ctx: &StandaloneCtx,
		req_ctx: &mut RequestContext,
		client_ws: WebSocketHandle,
		after_hibernation: bool,
	) -> Result<Option<CloseFrame>> {
		let request_id = req_ctx.in_flight_request_id()?;

		// Extract headers
		let mut request_headers = HashableMap::new();
		for (name, value) in req_ctx.headers() {
			if let Result::Ok(value_str) = value.to_str() {
				request_headers.insert(name.to_string(), value_str.to_string());
			}
		}

		let (mut stopped_sub, runner_protocol_version) = tokio::try_join!(
			ctx.subscribe::<pegboard::workflows::actor::Stopped>(("actor_id", self.actor_id)),
			get_runner_protocol_version(&ctx, self.runner_id),
		)?;

		// Build subject to publish to
		let tunnel_subject =
			pegboard::pubsub_subjects::RunnerReceiverSubject::new(self.runner_id).to_string();

		// Start listening for WebSocket messages
		let InFlightRequestHandle {
			mut msg_rx,
			mut drop_rx,
			new,
		} = self
			.shared_state
			.start_in_flight_request(tunnel_subject.clone(), runner_protocol_version, request_id)
			.await;

		ensure!(
			!after_hibernation || !new,
			"should not be creating a new in flight entry after hibernation"
		);

		// If we are reconnecting after hibernation, don't send an open message
		let can_hibernate = if after_hibernation {
			true
		} else {
			// Send WebSocket open message
			let open_message = protocol::mk2::ToClientTunnelMessageKind::ToClientWebSocketOpen(
				protocol::mk2::ToClientWebSocketOpen {
					actor_id: self.actor_id.to_string(),
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
									protocol::mk2::ToServerTunnelMessageKind::ToServerWebSocketOpen(msg) => {
										return anyhow::Ok(msg);
									}
									protocol::mk2::ToServerTunnelMessageKind::ToServerWebSocketClose(close) => {
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
									request_id=%protocol::util::id_to_string(&request_id),
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
							tracing::warn!(reason=?drop_rx.borrow(), "websocket open timeout");
							return Err(WebSocketServiceUnavailable.build());
						}
					}
				}

				Err(WebSocketServiceUnavailable.build())
			};

			let websocket_open_timeout = Duration::from_millis(
				self.ctx
					.config()
					.pegboard()
					.gateway_websocket_open_timeout_ms(),
			);
			let open_msg = tokio::time::timeout(websocket_open_timeout, fut)
				.await
				.map_err(|_| {
					tracing::warn!("timed out waiting for websocket open from runner");

					WebSocketServiceUnavailable.build()
				})??;

			self.shared_state
				.toggle_hibernation(request_id, open_msg.can_hibernate)
				.await?;

			open_msg.can_hibernate
		};

		let ingress_bytes = Arc::new(AtomicU64::new(0));
		let egress_bytes = Arc::new(AtomicU64::new(0));

		// Send pending messages
		self.shared_state
			.resend_pending_websocket_messages(request_id)
			.await?;

		let ws_rx = client_ws.recv();

		let (tunnel_to_ws_abort_tx, tunnel_to_ws_abort_rx) = watch::channel(());
		let (ws_to_tunnel_abort_tx, ws_to_tunnel_abort_rx) = watch::channel(());
		let (ping_abort_tx, ping_abort_rx) = watch::channel(());
		let (keepalive_abort_tx, keepalive_abort_rx) = watch::channel(());
		let (metrics_abort_tx, metrics_abort_rx) = watch::channel(());

		let tunnel_to_ws = tokio::spawn(tunnel_to_ws_task::task(
			self.shared_state.clone(),
			client_ws,
			request_id,
			stopped_sub,
			msg_rx,
			drop_rx,
			can_hibernate,
			egress_bytes.clone(),
			tunnel_to_ws_abort_rx,
		));
		let ws_to_tunnel = tokio::spawn(ws_to_tunnel_task::task(
			self.shared_state.clone(),
			request_id,
			ws_rx,
			ingress_bytes.clone(),
			ws_to_tunnel_abort_rx,
		));
		let update_ping_interval = Duration::from_millis(
			self.ctx
				.config()
				.pegboard()
				.gateway_update_ping_interval_ms(),
		);
		let ping = tokio::spawn(ping_task::task(
			self.shared_state.clone(),
			request_id,
			ping_abort_rx,
			update_ping_interval,
		));
		let metrics = tokio::spawn(metrics_task::task(
			ctx.clone(),
			self.actor_id,
			self.runner_id,
			ingress_bytes,
			egress_bytes,
			metrics_abort_rx,
		));
		let keepalive = if can_hibernate {
			Some(tokio::spawn(keepalive_task::task(
				self.shared_state.clone(),
				ctx.clone(),
				self.actor_id,
				self.shared_state.gateway_id(),
				request_id,
				keepalive_abort_rx,
			)))
		} else {
			None
		};

		// Wait for all tasks to complete
		let (tunnel_to_ws_res, ws_to_tunnel_res, ping_res, keepalive_res, metrics_res) = tokio::join!(
			async {
				let res = tunnel_to_ws.await?;

				// Abort other if not aborted
				if !matches!(res, Ok(LifecycleResult::Aborted)) {
					tracing::debug!(?res, "tunnel to ws task completed, aborting counterpart");

					let _ = ping_abort_tx.send(());
					let _ = ws_to_tunnel_abort_tx.send(());
					let _ = keepalive_abort_tx.send(());
					let _ = metrics_abort_tx.send(());
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

					let _ = ping_abort_tx.send(());
					let _ = tunnel_to_ws_abort_tx.send(());
					let _ = keepalive_abort_tx.send(());
					let _ = metrics_abort_tx.send(());
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

					let _ = ws_to_tunnel_abort_tx.send(());
					let _ = tunnel_to_ws_abort_tx.send(());
					let _ = keepalive_abort_tx.send(());
					let _ = metrics_abort_tx.send(());
				} else {
					tracing::debug!(?res, "ping task completed");
				}

				res
			},
			async {
				let Some(keepalive) = keepalive else {
					return Ok(LifecycleResult::Aborted);
				};

				let res = keepalive.await?;

				// Abort others if not aborted
				if !matches!(res, Ok(LifecycleResult::Aborted)) {
					tracing::debug!(?res, "keepalive task completed, aborting others");

					let _ = ws_to_tunnel_abort_tx.send(());
					let _ = tunnel_to_ws_abort_tx.send(());
					let _ = ping_abort_tx.send(());
					let _ = metrics_abort_tx.send(());
				} else {
					tracing::debug!(?res, "keepalive task completed");
				}

				res
			},
			async {
				let res = metrics.await?;

				// Abort others if not aborted
				if !matches!(res, Ok(LifecycleResult::Aborted)) {
					tracing::debug!(?res, "metrics task completed, aborting others");

					let _ = ws_to_tunnel_abort_tx.send(());
					let _ = tunnel_to_ws_abort_tx.send(());
					let _ = ping_abort_tx.send(());
					let _ = keepalive_abort_tx.send(());
				} else {
					tracing::debug!(?res, "metrics task completed");
				}

				res
			},
		);

		// Determine single result from all tasks
		let mut lifecycle_res = match (
			tunnel_to_ws_res,
			ws_to_tunnel_res,
			ping_res,
			keepalive_res,
			metrics_res,
		) {
			// Prefer error
			(Err(err), _, _, _, _) => Err(err),
			(_, Err(err), _, _, _) => Err(err),
			(_, _, Err(err), _, _) => Err(err),
			(_, _, _, Err(err), _) => Err(err),
			(_, _, _, _, Err(err)) => Err(err),
			// Prefer non aborted result if all succeed
			(Ok(res), Ok(LifecycleResult::Aborted), _, _, _) => Ok(res),
			(Ok(LifecycleResult::Aborted), Ok(res), _, _, _) => Ok(res),
			// Unlikely case
			(res, _, _, _, _) => res,
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
			let close_message = protocol::mk2::ToClientTunnelMessageKind::ToClientWebSocketClose(
				protocol::mk2::ToClientWebSocketClose {
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
}

#[async_trait]
impl CustomServeTrait for PegboardGateway {
	#[tracing::instrument(skip_all, fields(actor_id=?self.actor_id, runner_id=?self.runner_id))]
	async fn handle_request(
		&self,
		req: Request<Full<Bytes>>,
		req_ctx: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
		let ctx = self.ctx.with_ray(req_ctx.ray_id(), req_ctx.req_id())?;
		let req_body_size_hint = req.body().size_hint();

		let (res, metrics_res) = tokio::join!(
			self.handle_request_inner(&ctx, req, req_ctx),
			record_req_metrics(
				&ctx,
				self.runner_id,
				self.actor_id,
				Metric::HttpIngress(
					req_body_size_hint
						.upper()
						.unwrap_or(req_body_size_hint.lower()) as usize
				),
			),
		);

		let response_size = match &res {
			Ok(res) => res.size_hint().upper().unwrap_or(res.size_hint().lower()),
			Err(_) => 0,
		};

		if let Err(err) = metrics_res {
			tracing::error!(?err, "http req ingress metrics failed");
		} else {
			if let Err(err) = record_req_metrics(
				&ctx,
				self.runner_id,
				self.actor_id,
				Metric::HttpEgress(response_size as usize),
			)
			.await
			{
				tracing::error!(
					?err,
					runner_id=?self.runner_id,
					"http req egress metrics failed, likely corrupt now",
				);
			}
		}

		res
	}

	#[tracing::instrument(skip_all, fields(actor_id=?self.actor_id, runner_id=?self.runner_id))]
	async fn handle_websocket(
		&self,
		req_ctx: &mut RequestContext,
		client_ws: WebSocketHandle,
		after_hibernation: bool,
	) -> Result<Option<CloseFrame>> {
		let ctx = self.ctx.with_ray(req_ctx.ray_id(), req_ctx.req_id())?;
		let (res, metrics_res) = tokio::join!(
			self.handle_websocket_inner(&ctx, req_ctx, client_ws, after_hibernation),
			record_req_metrics(&ctx, self.runner_id, self.actor_id, Metric::WebsocketOpen),
		);

		if let Err(err) = metrics_res {
			tracing::error!(?err, "ws open metrics failed");
		} else {
			if let Err(err) =
				record_req_metrics(&ctx, self.runner_id, self.actor_id, Metric::WebsocketClose)
					.await
			{
				tracing::error!(
					?err,
					runner_id=?self.runner_id,
					"ws close metrics failed, likely corrupt now",
				);
			}
		}

		res
	}

	#[tracing::instrument(skip_all, fields(actor_id=?self.actor_id))]
	async fn handle_websocket_hibernation(
		&self,
		req_ctx: &mut RequestContext,
		client_ws: WebSocketHandle,
	) -> Result<HibernationResult> {
		let ctx = self.ctx.with_ray(req_ctx.ray_id(), req_ctx.req_id())?;
		let request_id = req_ctx.in_flight_request_id()?;

		// Immediately rewake if we have pending messages
		if self
			.shared_state
			.has_pending_websocket_messages(request_id)
			.await?
		{
			tracing::debug!("exiting hibernating due to pending messages");

			return Ok(HibernationResult::Continue);
		}

		// Start keepalive task
		let (keepalive_abort_tx, keepalive_abort_rx) = watch::channel(());
		let keepalive_handle = tokio::spawn(keepalive_task::task(
			self.shared_state.clone(),
			ctx.clone(),
			self.actor_id,
			self.shared_state.gateway_id(),
			request_id,
			keepalive_abort_rx,
		));

		let (res, metrics_res) = tokio::join!(
			self.handle_websocket_hibernation_inner(client_ws),
			record_req_metrics(
				&ctx,
				self.runner_id,
				self.actor_id,
				Metric::WebsocketHibernate
			)
		);

		let _ = keepalive_abort_tx.send(());
		let _ = keepalive_handle.await;

		let (delete_res, _) = tokio::join!(
			async {
				match &res {
					Ok(HibernationResult::Continue) => {}
					Ok(HibernationResult::Close) | Err(_) => {
						// No longer an active hibernating request, delete entry
						ctx.op(pegboard::ops::actor::hibernating_request::delete::Input {
							actor_id: self.actor_id,
							gateway_id: self.shared_state.gateway_id(),
							request_id,
						})
						.await?;
					}
				}

				anyhow::Ok(())
			},
			async {
				if let Err(err) = metrics_res {
					tracing::error!(?err, "ws hibernate metrics failed");
				} else {
					if let Err(err) = record_req_metrics(
						&ctx,
						self.runner_id,
						self.actor_id,
						Metric::WebsocketStopHibernate,
					)
					.await
					{
						tracing::error!(
							?err,
							runner_id=?self.runner_id,
							"ws stop hibernate metrics failed, likely corrupt now"
						);
					}
				}
			},
		);

		delete_res?;

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

		// Fetch actor info after sub to prevent race condition
		if let Some(actor) = self
			.ctx
			.op(pegboard::ops::actor::get_for_gateway::Input {
				actor_id: self.actor_id,
			})
			.await?
		{
			if actor.runner_id.is_some() {
				tracing::debug!("actor became ready during hibernation");

				return Ok(HibernationResult::Continue);
			}
		}

		let res = tokio::select! {
			_ = ready_sub.next() => {
				tracing::debug!("actor became ready during hibernation");

				HibernationResult::Continue
			}
			hibernation_res = hibernate_ws(client_ws.recv()) => {
				let hibernation_res = hibernation_res?;

				match &hibernation_res {
					HibernationResult::Continue => {
						tracing::debug!("received websocket message during hibernation");
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

async fn get_runner_protocol_version(ctx: &StandaloneCtx, runner_id: Id) -> Result<u16> {
	ctx.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(pegboard::keys::subspace());

			let protocol_version_entry = tx
				.read_opt(
					&pegboard::keys::runner::ProtocolVersionKey::new(runner_id),
					Serializable,
				)
				.await?;

			Ok(protocol_version_entry.unwrap_or(PROTOCOL_MK1_VERSION))
		})
		.await
}

enum Metric {
	HttpIngress(usize),
	HttpEgress(usize),
	WebsocketOpen,
	// Ingress, Egress
	WebsocketTransfer(usize, usize),
	WebsocketClose,
	WebsocketHibernate,
	WebsocketStopHibernate,
}

async fn record_req_metrics(
	ctx: &StandaloneCtx,
	runner_id: Id,
	actor_id: Id,
	metric: Metric,
) -> Result<u16> {
	let metric = &metric;
	// Read runner protocol version
	let (protocol_version, has_name, actor_workflow_id) = ctx
		.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(pegboard::keys::subspace());

			let protocol_version_key = pegboard::keys::runner::ProtocolVersionKey::new(runner_id);
			let namespace_id_key = pegboard::keys::runner::NamespaceIdKey::new(runner_id);
			let actor_name_key = pegboard::keys::actor::NameKey::new(actor_id);
			let actor_workflow_id_key = pegboard::keys::actor::WorkflowIdKey::new(actor_id);
			let (protocol_version_entry, namespace_id, actor_name_entry, actor_workflow_id) = tokio::try_join!(
				tx.read_opt(&protocol_version_key, Serializable),
				tx.read(&namespace_id_key, Serializable),
				tx.read_opt(&actor_name_key, Serializable),
				tx.read(&actor_workflow_id_key, Serializable),
			)?;
			let has_name = actor_name_entry.is_some();

			if let Some(name) = &actor_name_entry {
				metric_inc(&tx, namespace_id, name, metric);
			}

			Ok((
				protocol_version_entry.unwrap_or(PROTOCOL_MK1_VERSION),
				has_name,
				actor_workflow_id,
			))
		})
		.await?;

	// NOTE: The name key was added via backfill. If the actor has not backfilled the key yet (key is none),
	// we need to fetch it from the actor state
	if !has_name {
		let wf = ctx
			.workflow::<pegboard::workflows::actor::Input>(actor_workflow_id)
			.get()
			.await?
			.context("actor not found")?;
		let actor_state = &wf
			.parse_state::<Option<pegboard::workflows::actor::State>>()?
			.context("actor did not initialize state yet")?;

		// Record metrics
		ctx.udb()?
			.run(|tx| async move {
				let tx = tx.with_subspace(pegboard::keys::subspace());
				metric_inc(&tx, actor_state.namespace_id, &actor_state.name, metric);

				Ok(())
			})
			.await?;
	}

	Ok(protocol_version)
}

fn metric_inc(tx: &universaldb::Transaction, namespace_id: Id, name: &str, metric: &Metric) {
	match metric {
		Metric::HttpIngress(size) => {
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::GatewayIngress(
					name.to_string(),
					"http".to_string(),
				),
				(*size).try_into().unwrap_or_default(),
			);
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::Requests(name.to_string(), "http".to_string()),
				1,
			);
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::ActiveRequests(
					name.to_string(),
					"http".to_string(),
				),
				1,
			);
		}
		Metric::HttpEgress(size) => {
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::GatewayEgress(
					name.to_string(),
					"http".to_string(),
				),
				(*size).try_into().unwrap_or_default(),
			);
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::ActiveRequests(
					name.to_string(),
					"http".to_string(),
				),
				-1,
			);
		}
		Metric::WebsocketOpen => {
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::Requests(name.to_string(), "ws".to_string()),
				1,
			);
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::ActiveRequests(name.to_string(), "ws".to_string()),
				1,
			);
		}
		Metric::WebsocketTransfer(ingress_size, egress_size) => {
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::GatewayIngress(name.to_string(), "ws".to_string()),
				(*ingress_size).try_into().unwrap_or_default(),
			);
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::GatewayEgress(name.to_string(), "ws".to_string()),
				(*egress_size).try_into().unwrap_or_default(),
			);
		}
		Metric::WebsocketClose => {
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::ActiveRequests(name.to_string(), "ws".to_string()),
				-1,
			);
		}
		Metric::WebsocketHibernate => {
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::Requests(name.to_string(), "hws".to_string()),
				1,
			);
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::ActiveRequests(
					name.to_string(),
					"hws".to_string(),
				),
				1,
			);
		}
		Metric::WebsocketStopHibernate => {
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::ActiveRequests(
					name.to_string(),
					"hws".to_string(),
				),
				-1,
			);
		}
	}
}
