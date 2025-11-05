use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::TryStreamExt;
use gas::prelude::*;
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, StatusCode};
use rivet_error::*;
use rivet_guard_core::{
	WebSocketHandle,
	custom_serve::CustomServeTrait,
	errors::{
		ServiceUnavailable, WebSocketServiceRetry, WebSocketServiceTimeout,
		WebSocketServiceUnavailable,
	},
	proxy_service::ResponseBody,
	request_context::RequestContext,
};
use rivet_runner_protocol as protocol;
use rivet_util::serde::HashableMap;
use std::time::Duration;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::{
	Message,
	protocol::frame::{CloseFrame, coding::CloseCode},
};

use crate::shared_state::{SharedState, TunnelMessageData};

pub mod shared_state;

const TUNNEL_ACK_TIMEOUT: Duration = Duration::from_secs(2);

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
	shared_state: SharedState,
	runner_id: Id,
	actor_id: Id,
	path: String,
}

impl PegboardGateway {
	#[tracing::instrument(skip_all, fields(?actor_id, ?runner_id, ?path))]
	pub fn new(shared_state: SharedState, runner_id: Id, actor_id: Id, path: String) -> Self {
		Self {
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

		// Build subject to publish to
		let tunnel_subject =
			pegboard::pubsub_subjects::RunnerReceiverSubject::new(self.runner_id).to_string();

		// Start listening for request responses
		let request_id = Uuid::new_v4().into_bytes();
		let mut msg_rx = self
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
			while let Some(msg) = msg_rx.recv().await {
				match msg {
					TunnelMessageData::Message(msg) => match msg {
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
					},
					TunnelMessageData::Timeout => {
						tracing::warn!("tunnel message timeout");
						return Err(ServiceUnavailable.build());
					}
				}
			}

			tracing::warn!(request_id=?Uuid::from_bytes(request_id), "received no message response during request init");
			Err(ServiceUnavailable.build())
		};
		let response_start = tokio::time::timeout(TUNNEL_ACK_TIMEOUT, fut)
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
		unique_request_id: Uuid,
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

		// Build subject to publish to
		let tunnel_subject =
			pegboard::pubsub_subjects::RunnerReceiverSubject::new(self.runner_id).to_string();

		// Start listening for WebSocket messages
		let request_id = unique_request_id.into_bytes();
		let mut msg_rx = self
			.shared_state
			.start_in_flight_request(tunnel_subject.clone(), request_id)
			.await;

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
			while let Some(msg) = msg_rx.recv().await {
				match msg {
					TunnelMessageData::Message(
						protocol::ToServerTunnelMessageKind::ToServerWebSocketOpen(msg),
					) => {
						return anyhow::Ok(msg);
					}
					TunnelMessageData::Message(
						protocol::ToServerTunnelMessageKind::ToServerWebSocketClose(close),
					) => {
						tracing::warn!(?close, "websocket closed before opening");
						return Err(WebSocketServiceUnavailable.build());
					}
					TunnelMessageData::Timeout => {
						tracing::warn!("websocket open timeout");
						return Err(WebSocketServiceUnavailable.build());
					}
					_ => {
						tracing::warn!(
							"received unexpected message while waiting for websocket open"
						);
					}
				}
			}

			tracing::warn!(request_id=?Uuid::from_bytes(request_id), "received no message response during ws init");
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

		// Send reclaimed messages
		self.shared_state
			.resend_pending_websocket_messages(request_id, open_msg.last_msg_index)
			.await?;

		let ws_rx = client_ws.recv();

		let (tunnel_to_ws_abort_tx, mut tunnel_to_ws_abort_rx) = watch::channel(());
		let (ws_to_tunnel_abort_tx, mut ws_to_tunnel_abort_rx) = watch::channel(());

		// Spawn task to forward messages from tunnel to ws
		let shared_state = self.shared_state.clone();
		let tunnel_to_ws = tokio::spawn(
			async move {
				loop {
					tokio::select! {
						res = msg_rx.recv() => {
							if let Some(msg) = res {
								match msg {
									TunnelMessageData::Message(
										protocol::ToServerTunnelMessageKind::ToServerWebSocketMessage(ws_msg),
									) => {
										let msg = if ws_msg.binary {
											Message::Binary(ws_msg.data.into())
										} else {
											Message::Text(
												String::from_utf8_lossy(&ws_msg.data).into_owned().into(),
											)
										};
										client_ws.send(msg).await?;
									}
									TunnelMessageData::Message(
										protocol::ToServerTunnelMessageKind::ToServerWebSocketMessageAck(ack),
									) => {
										shared_state
											.ack_pending_websocket_messages(request_id, ack.index)
											.await?;
									}
									TunnelMessageData::Message(
										protocol::ToServerTunnelMessageKind::ToServerWebSocketClose(close),
									) => {
										tracing::debug!(?close, "server closed websocket");


										if open_msg.can_hibernate && close.retry {
											// Successful closure
											return Err(WebSocketServiceRetry.build());
										} else {
											return Ok(LifecycleResult::ServerClose(close));
										}
									}
									TunnelMessageData::Timeout => {
										tracing::warn!("websocket message timeout");
										return Err(WebSocketServiceTimeout.build());
									}
									_ => {}
								}
							} else {
								tracing::debug!("tunnel sub closed");
								return Err(WebSocketServiceRetry.build());
							}
						}
						_ = tunnel_to_ws_abort_rx.changed() => {
							tracing::debug!("task aborted");
							return Ok(LifecycleResult::Aborted);
						}
					}
				}
			}
			.instrument(tracing::info_span!("tunnel_to_ws_task")),
		);

		// Spawn task to forward messages from ws to tunnel
		let shared_state_clone = self.shared_state.clone();
		let ws_to_tunnel = tokio::spawn(
			async move {
				let mut ws_rx = ws_rx.lock().await;

				loop {
					tokio::select! {
						res = ws_rx.try_next() => {
							if let Some(msg) = res? {
								match msg {
									Message::Binary(data) => {
										let ws_message =
											protocol::ToClientTunnelMessageKind::ToClientWebSocketMessage(
												protocol::ToClientWebSocketMessage {
													// NOTE: This gets set in shared_state.ts
													index: 0,
													data: data.into(),
													binary: true,
												},
											);
										shared_state_clone
											.send_message(request_id, ws_message)
											.await?;
									}
									Message::Text(text) => {
										let ws_message =
											protocol::ToClientTunnelMessageKind::ToClientWebSocketMessage(
												protocol::ToClientWebSocketMessage {
													// NOTE: This gets set in shared_state.ts
													index: 0,
													data: text.as_bytes().to_vec(),
													binary: false,
												},
											);
										shared_state_clone
											.send_message(request_id, ws_message)
											.await?;
									}
									Message::Close(close) => {
										return Ok(LifecycleResult::ClientClose(close));
									}
									_ => {}
								}
							} else {
								tracing::debug!("websocket stream closed");
								return Ok(LifecycleResult::ClientClose(None));
							}
						}
						_ = ws_to_tunnel_abort_rx.changed() => {
							tracing::debug!("task aborted");
							return Ok(LifecycleResult::Aborted);
						}
					};
				}
			}
			.instrument(tracing::info_span!("ws_to_tunnel_task")),
		);

		// Wait for both tasks to complete
		let (tunnel_to_ws_res, ws_to_tunnel_res) = tokio::join!(
			async {
				let res = tunnel_to_ws.await?;

				// Abort other if not aborted
				if !matches!(res, Ok(LifecycleResult::Aborted)) {
					tracing::debug!(?res, "tunnel to ws task completed, aborting counterpart");

					drop(ws_to_tunnel_abort_tx);
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

					drop(tunnel_to_ws_abort_tx);
				} else {
					tracing::debug!(?res, "ws to tunnel task completed");
				}

				res
			}
		);

		// Determine single result from both tasks
		let mut lifecycle_res = match (tunnel_to_ws_res, ws_to_tunnel_res) {
			// Prefer error
			(_, Err(err)) => Err(err),
			(Err(err), _) => Err(err),
			// Prefer non aborted result if both succeed
			(Ok(res), Ok(LifecycleResult::Aborted)) => Ok(res),
			(Ok(LifecycleResult::Aborted), Ok(res)) => Ok(res),
			// Prefer tunnel to ws if both succeed (unlikely case)
			(res, _) => res,
		};

		// Send WebSocket close message to runner
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
