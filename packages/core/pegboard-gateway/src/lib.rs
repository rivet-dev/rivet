use std::result::Result::Ok as ResultOk;
use std::{
	collections::HashMap,
	sync::{
		Arc,
		atomic::{AtomicU64, Ordering},
	},
	time::Duration,
};

use anyhow::*;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use gas::prelude::*;
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, StatusCode};
use hyper_tungstenite::HyperWebsocket;
use rivet_guard_core::{
	custom_serve::CustomServeTrait,
	proxy_service::{ResponseBody, X_RIVET_ERROR},
	request_context::RequestContext,
};
use rivet_tunnel_protocol::{
	MessageKind, ToServerRequestStart, ToServerWebSocketClose, ToServerWebSocketMessage,
	ToServerWebSocketOpen,
};
use rivet_util::serde::HashableMap;
use tokio_tungstenite::tungstenite::Message;

use crate::shared_state::{SharedState, TunnelMessageData};

pub mod shared_state;

const UPS_REQ_TIMEOUT: Duration = Duration::from_secs(2);

pub struct PegboardGateway {
	ctx: StandaloneCtx,
	shared_state: SharedState,
	namespace_id: Id,
	runner_name: String,
	runner_key: String,
	actor_id: Id,
}

impl PegboardGateway {
	pub fn new(
		ctx: StandaloneCtx,
		shared_state: SharedState,
		namespace_id: Id,
		runner_name: String,
		runner_key: String,
		actor_id: Id,
	) -> Self {
		Self {
			ctx,
			shared_state,
			namespace_id,
			runner_name,
			runner_key,
			actor_id,
		}
	}
}

#[async_trait]
impl CustomServeTrait for PegboardGateway {
	async fn handle_request(
		&self,
		req: Request<Full<Bytes>>,
		request_context: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
		let res = self.handle_request_inner(req, request_context).await;
		match res {
			Result::Ok(x) => Ok(x),
			Err(err) => {
				if is_tunnel_service_unavailable(&err) {
					// This will force the request to be retried with a new tunnel
					Ok(Response::builder()
						.status(StatusCode::SERVICE_UNAVAILABLE)
						.header(X_RIVET_ERROR, "pegboard_gateway.tunnel_closed")
						.body(ResponseBody::Full(Full::new(Bytes::new())))?)
				} else {
					Err(err)
				}
			}
		}
	}

	async fn handle_websocket(
		&self,
		client_ws: HyperWebsocket,
		headers: &hyper::HeaderMap,
		path: &str,
		_request_context: &mut RequestContext,
	) -> std::result::Result<(), (HyperWebsocket, anyhow::Error)> {
		match self
			.handle_websocket_inner(client_ws, headers, path, _request_context)
			.await
		{
			Result::Ok(()) => std::result::Result::<(), (HyperWebsocket, anyhow::Error)>::Ok(()),
			Result::Err((client_ws, err)) => {
				if is_tunnel_service_unavailable(&err) {
					Err((
						client_ws,
						rivet_guard_core::errors::WebSocketServiceUnavailable.build(),
					))
				} else {
					Err((client_ws, err))
				}
			}
		}
	}
}

impl PegboardGateway {
	async fn handle_request_inner(
		&self,
		req: Request<Full<Bytes>>,
		_request_context: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
		// Extract actor ID for the message
		let actor_id = req
			.headers()
			.get("x-rivet-actor")
			.context("missing x-rivet-actor header")?
			.to_str()
			.context("invalid x-rivet-actor header")?
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
		let path = req
			.uri()
			.path_and_query()
			.map_or_else(|| "/".to_string(), |x| x.to_string());

		let body_bytes = req
			.into_body()
			.collect()
			.await
			.context("failed to read body")?
			.to_bytes();

		// Build subject to publish to
		let tunnel_subject = pegboard::pubsub_subjects::TunnelRunnerReceiverSubject::new(
			self.namespace_id,
			&self.runner_name,
			&self.runner_key,
		)
		.to_string();

		// Start listening for request responses
		let (request_id, mut msg_rx) = self
			.shared_state
			.start_in_flight_request(tunnel_subject)
			.await;

		// Start request
		let message = MessageKind::ToServerRequestStart(ToServerRequestStart {
			actor_id: actor_id.clone(),
			method,
			path,
			headers,
			body: if body_bytes.is_empty() {
				None
			} else {
				Some(body_bytes.to_vec())
			},
			stream: false,
		});
		self.shared_state.send_message(request_id, message).await?;

		// Wait for response
		tracing::info!("starting response handler task");
		let response_start = loop {
			let Some(msg) = msg_rx.recv().await else {
				tracing::warn!("received no message response");
				return Err(RequestError::ServiceUnavailable.into());
			};

			match msg {
				TunnelMessageData::Message(msg) => match msg {
					MessageKind::ToClientResponseStart(response_start) => {
						break response_start;
					}
					_ => {
						tracing::warn!("received non-response message from pubsub");
					}
				},
				TunnelMessageData::Timeout => {
					tracing::warn!("tunnel message timeout");
					return Err(RequestError::ServiceUnavailable.into());
				}
			}
		};
		tracing::info!("response handler task ended");

		// Build HTTP response
		let mut response_builder =
			Response::builder().status(StatusCode::from_u16(response_start.status)?);

		// Add headers
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
		client_ws: HyperWebsocket,
		headers: &hyper::HeaderMap,
		path: &str,
		_request_context: &mut RequestContext,
	) -> std::result::Result<(), (HyperWebsocket, anyhow::Error)> {
		// Extract actor ID for the message
		let actor_id = match headers
			.get("x-rivet-actor")
			.context("missing x-rivet-actor header")
			.and_then(|v| v.to_str().context("invalid x-rivet-actor header"))
		{
			Result::Ok(v) => v.to_string(),
			Err(err) => return Err((client_ws, err)),
		};

		// Extract headers
		let mut request_headers = HashableMap::new();
		for (name, value) in headers {
			if let Result::Ok(value_str) = value.to_str() {
				request_headers.insert(name.to_string(), value_str.to_string());
			}
		}

		// Build subject to publish to
		let tunnel_subject = pegboard::pubsub_subjects::TunnelRunnerReceiverSubject::new(
			self.namespace_id,
			&self.runner_name,
			&self.runner_key,
		)
		.to_string();

		// Start listening for WebSocket messages
		let (request_id, mut msg_rx) = self
			.shared_state
			.start_in_flight_request(tunnel_subject.clone())
			.await;

		// Send WebSocket open message
		let open_message = MessageKind::ToServerWebSocketOpen(ToServerWebSocketOpen {
			actor_id: actor_id.clone(),
			path: path.to_string(),
			headers: request_headers,
		});

		if let Err(err) = self
			.shared_state
			.send_message(request_id, open_message)
			.await
		{
			return Err((client_ws, err));
		}

		// Wait for WebSocket open acknowledgment
		let open_ack_received = loop {
			let Some(msg) = msg_rx.recv().await else {
				tracing::warn!("received no websocket open response");
				return Err((client_ws, RequestError::ServiceUnavailable.into()));
			};

			match msg {
				TunnelMessageData::Message(MessageKind::ToClientWebSocketOpen) => {
					break true;
				}
				TunnelMessageData::Message(MessageKind::ToClientWebSocketClose(close)) => {
					tracing::info!(?close, "websocket closed before opening");
					return Err((client_ws, RequestError::ServiceUnavailable.into()));
				}
				TunnelMessageData::Timeout => {
					tracing::warn!("websocket open timeout");
					return Err((client_ws, RequestError::ServiceUnavailable.into()));
				}
				_ => {
					tracing::warn!("received unexpected message while waiting for websocket open");
				}
			}
		};

		if !open_ack_received {
			return Err((client_ws, anyhow!("failed to open websocket")));
		}

		// Accept the WebSocket
		let ws_stream = match client_ws.await {
			Result::Ok(ws) => ws,
			Err(e) => {
				// Handshake already in progress; cannot retry safely here
				tracing::debug!(error = ?e, "client websocket await failed");
				return std::result::Result::<(), (HyperWebsocket, anyhow::Error)>::Ok(());
			}
		};
		let (mut ws_sink, mut ws_stream) = ws_stream.split();

		// Spawn task to forward messages from server to client
		let mut msg_rx_for_task = msg_rx;
		tokio::spawn(async move {
			while let Some(msg) = msg_rx_for_task.recv().await {
				match msg {
					TunnelMessageData::Message(MessageKind::ToClientWebSocketMessage(ws_msg)) => {
						let msg = if ws_msg.binary {
							Message::Binary(ws_msg.data.into())
						} else {
							Message::Text(String::from_utf8_lossy(&ws_msg.data).into_owned().into())
						};
						if let Err(e) = ws_sink.send(msg).await {
							tracing::warn!(?e, "failed to send websocket message to client");
							break;
						}
					}
					TunnelMessageData::Message(MessageKind::ToClientWebSocketClose(close)) => {
						tracing::info!(?close, "server closed websocket");
						break;
					}
					TunnelMessageData::Timeout => {
						tracing::warn!("websocket message timeout");
						break;
					}
					_ => {}
				}
			}
		});

		// Forward messages from client to server
		let mut close_reason = None;
		while let Some(msg) = ws_stream.next().await {
			match msg {
				Result::Ok(Message::Binary(data)) => {
					let ws_message =
						MessageKind::ToServerWebSocketMessage(ToServerWebSocketMessage {
							data: data.into(),
							binary: true,
						});
					if let Err(err) = self.shared_state.send_message(request_id, ws_message).await {
						if is_tunnel_service_unavailable(&err) {
							tracing::warn!("tunnel closed sending binary message");
							close_reason = Some("Tunnel closed".to_string());
							break;
						} else {
							tracing::error!(?err, "error sending binary message");
						}
					}
				}
				Result::Ok(Message::Text(text)) => {
					let ws_message =
						MessageKind::ToServerWebSocketMessage(ToServerWebSocketMessage {
							data: text.as_bytes().to_vec(),
							binary: false,
						});
					if let Err(err) = self.shared_state.send_message(request_id, ws_message).await {
						if is_tunnel_service_unavailable(&err) {
							tracing::warn!("tunnel closed sending text message");
							close_reason = Some("Tunnel closed".to_string());
							break;
						} else {
							tracing::error!(?err, "error sending text message");
						}
					}
				}
				Result::Ok(Message::Close(_)) | Err(_) => break,
				_ => {}
			}
		}

		// Send WebSocket close message
		let close_message = MessageKind::ToServerWebSocketClose(ToServerWebSocketClose {
			code: None,
			reason: close_reason,
		});

		if let Err(err) = self
			.shared_state
			.send_message(request_id, close_message)
			.await
		{
			if is_tunnel_service_unavailable(&err) {
				tracing::warn!("tunnel closed sending close message");
			} else {
				tracing::error!(?err, "error sending close message");
			}
		}

		std::result::Result::<(), (HyperWebsocket, anyhow::Error)>::Ok(())
	}
}

#[derive(thiserror::Error, Debug)]
enum RequestError {
	#[error("service unavailable")]
	ServiceUnavailable,
}

/// Determines if the tunnel is closed by if the UPS service is no longer responding.
fn is_tunnel_service_unavailable(err: &anyhow::Error) -> bool {
	err.chain().any(|x| x.is::<RequestError>())
}
