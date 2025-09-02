use anyhow::*;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use gas::prelude::*;
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, StatusCode, body::Incoming as BodyIncoming};
use hyper_tungstenite::HyperWebsocket;
use pegboard::pubsub_subjects::{
	TunnelHttpResponseSubject, TunnelHttpRunnerSubject, TunnelHttpWebSocketSubject,
};
use rivet_error::*;
use rivet_guard_core::{
	custom_serve::CustomServeTrait,
	proxy_service::{ResponseBody, X_RIVET_ERROR},
	request_context::RequestContext,
};
use rivet_tunnel_protocol::{
	MessageBody, StreamFinishReason, ToServerRequestFinish, ToServerRequestStart,
	ToServerWebSocketClose, ToServerWebSocketMessage, ToServerWebSocketOpen, TunnelMessage,
	versioned,
};
use rivet_util::serde::HashableMap;
use std::result::Result::Ok as ResultOk;
use std::{
	collections::HashMap,
	sync::{
		Arc,
		atomic::{AtomicU64, Ordering},
	},
	time::Duration,
};
use tokio::{
	sync::{Mutex, oneshot},
	time::timeout,
};
use tokio_tungstenite::tungstenite::Message;
use universalpubsub::NextOutput;

const UPS_REQ_TIMEOUT: Duration = Duration::from_secs(2);

pub struct PegboardGateway {
	ctx: StandaloneCtx,
	request_counter: AtomicU64,
	actor_id: Id,
	runner_id: Id,
	port_name: String,
}

impl PegboardGateway {
	pub fn new(ctx: StandaloneCtx, actor_id: Id, runner_id: Id, port_name: String) -> Self {
		Self {
			ctx,
			request_counter: AtomicU64::new(0),
			actor_id,
			runner_id,
			port_name,
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
				if is_tunnel_closed_error(&err) {
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
				if is_tunnel_closed_error(&err) {
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
			.ok_or_else(|| anyhow!("missing x-rivet-actor header"))?
			.to_str()
			.map_err(|_| anyhow!("invalid x-rivet-actor header"))?
			.to_string();

		// Generate request ID using atomic counter
		let request_id = self.request_counter.fetch_add(1, Ordering::SeqCst);

		// Extract request parts
		let mut headers = HashableMap::new();
		for (name, value) in req.headers() {
			if let ResultOk(value_str) = value.to_str() {
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
			.map_err(|e| anyhow!("failed to read body: {}", e))?
			.to_bytes();

		// Create tunnel message
		let request_start = ToServerRequestStart {
			request_id,
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
		};

		let message = TunnelMessage {
			body: MessageBody::ToServerRequestStart(request_start),
		};

		// Serialize message
		let serialized = versioned::TunnelMessage::serialize(versioned::TunnelMessage::V1(message))
			.map_err(|e| anyhow!("failed to serialize message: {}", e))?;

		// Build NATS topic
		let tunnel_subject = TunnelHttpRunnerSubject::new(self.runner_id, &self.port_name);
		let topic = tunnel_subject.to_string();

		tracing::info!(
			?topic,
			?self.runner_id,
			?self.port_name,
			?request_id,
			"publishing request to NATS"
		);

		// Create response channel
		let (response_tx, response_rx) = oneshot::channel();
		let response_map = Arc::new(Mutex::new(HashMap::new()));
		response_map.lock().await.insert(request_id, response_tx);

		// Subscribe to response topic
		let response_subject =
			TunnelHttpResponseSubject::new(self.runner_id, &self.port_name, request_id);
		let response_topic = response_subject.to_string();

		tracing::info!(
			?response_topic,
			?request_id,
			"subscribing to response topic"
		);

		let mut subscriber = self.ctx.ups()?.subscribe(&response_topic).await?;

		// Spawn task to handle response
		let response_map_clone = response_map.clone();
		tokio::spawn(async move {
			tracing::info!("starting response handler task");
			while let ResultOk(NextOutput::Message(msg)) = subscriber.next().await {
				// Ack message
				//match msg.reply(&[]).await {
				//	Result::Ok(_) => {}
				//	Err(err) => {
				//		tracing::warn!(?err, "failed to ack gateway request response message")
				//	}
				//};

				tracing::info!(
					payload_len = msg.payload.len(),
					"received response from NATS"
				);
				if let ResultOk(tunnel_msg) = versioned::TunnelMessage::deserialize(&msg.payload) {
					match tunnel_msg.body {
						MessageBody::ToClientResponseStart(response_start) => {
							tracing::info!(request_id = ?response_start.request_id, status = response_start.status, "received response from tunnel");
							if let Some(tx) = response_map_clone
								.lock()
								.await
								.remove(&response_start.request_id)
							{
								tracing::info!(request_id = ?response_start.request_id, "sending response to handler");
								let _ = tx.send(response_start);
							} else {
								tracing::warn!(request_id = ?response_start.request_id, "no handler found for response");
							}
						}
						_ => {
							tracing::warn!("received non-response message from NATS");
						}
					}
				} else {
					tracing::error!("failed to deserialize response from NATS");
				}
			}
			tracing::info!("response handler task ended");
		});

		// Publish request
		self.ctx
			.ups()?
			.publish(&topic, &serialized)
			.await
			.map_err(|e| anyhow!("failed to publish request: {}", e))?;

		// Send finish message
		let finish_message = TunnelMessage {
			body: MessageBody::ToServerRequestFinish(ToServerRequestFinish {
				request_id,
				reason: StreamFinishReason::Complete,
			}),
		};
		let finish_serialized =
			versioned::TunnelMessage::serialize(versioned::TunnelMessage::V1(finish_message))
				.map_err(|e| anyhow!("failed to serialize finish message: {}", e))?;
		self.ctx
			.ups()?
			.publish(&topic, &finish_serialized)
			.await
			.map_err(|e| anyhow!("failed to publish finish message: {}", e))?;

		// Wait for response with timeout
		let response_start = match timeout(Duration::from_secs(30), response_rx).await {
			ResultOk(ResultOk(response)) => response,
			_ => return Err(anyhow!("request timed out")),
		};

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
			.ok_or_else(|| anyhow!("missing x-rivet-actor header"))
			.and_then(|v| {
				v.to_str()
					.map_err(|_| anyhow!("invalid x-rivet-actor header"))
			}) {
			Result::Ok(v) => v.to_string(),
			Err(err) => return Err((client_ws, err)),
		};

		// Generate WebSocket ID using atomic counter
		let websocket_id = self.request_counter.fetch_add(1, Ordering::SeqCst);

		// Extract headers
		let mut request_headers = HashableMap::new();
		for (name, value) in headers {
			if let ResultOk(value_str) = value.to_str() {
				request_headers.insert(name.to_string(), value_str.to_string());
			}
		}

		// Build NATS topic
		let tunnel_subject = TunnelHttpRunnerSubject::new(self.runner_id, &self.port_name);
		let topic = tunnel_subject.to_string();

		// Send WebSocket open message
		let open_message = TunnelMessage {
			body: MessageBody::ToServerWebSocketOpen(ToServerWebSocketOpen {
				actor_id: actor_id.clone(),
				web_socket_id: websocket_id,
				path: path.to_string(),
				headers: request_headers,
			}),
		};

		let serialized =
			match versioned::TunnelMessage::serialize(versioned::TunnelMessage::V1(open_message)) {
				Result::Ok(s) => s,
				Err(e) => {
					return Err((
						client_ws,
						anyhow!("failed to serialize websocket open: {}", e),
					));
				}
			};

		let ups = match self.ctx.ups() {
			Result::Ok(u) => u,
			Err(err) => return Err((client_ws, err.into())),
		};
		if let Err(err) = ups.publish(&topic, &serialized).await {
			return Err((client_ws, err.into()));
		}

		// Subscribe to messages from server before accepting the client websocket so that
		// failures here can be retried by the proxy.
		let ws_subject =
			TunnelHttpWebSocketSubject::new(self.runner_id, &self.port_name, websocket_id);
		let response_topic = ws_subject.to_string();
		let mut subscriber = match ups.subscribe(&response_topic).await {
			Result::Ok(sub) => sub,
			Err(err) => return Err((client_ws, err.into())),
		};

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
		tokio::spawn(async move {
			while let ResultOk(NextOutput::Message(msg)) = subscriber.next().await {
				// Ack message
				//match msg.reply(&[]).await {
				//	Result::Ok(_) => {}
				//	Err(err) => {
				//		tracing::warn!(?err, "failed to ack gateway websocket message")
				//	}
				//};

				if let ResultOk(tunnel_msg) = versioned::TunnelMessage::deserialize(&msg.payload) {
					match tunnel_msg.body {
						MessageBody::ToClientWebSocketMessage(ws_msg) => {
							if ws_msg.web_socket_id == websocket_id {
								let msg = if ws_msg.binary {
									Message::Binary(ws_msg.data.into())
								} else {
									Message::Text(
										String::from_utf8_lossy(&ws_msg.data).into_owned().into(),
									)
								};
								let _ = ws_sink.send(msg).await;
							}
						}
						MessageBody::ToClientWebSocketClose(_) => break,
						_ => {}
					}
				}
			}
		});

		// Forward messages from client to server
		let mut close_reason = None;
		while let Some(msg) = ws_stream.next().await {
			match msg {
				ResultOk(Message::Binary(data)) => {
					let ws_message = TunnelMessage {
						body: MessageBody::ToServerWebSocketMessage(ToServerWebSocketMessage {
							web_socket_id: websocket_id,
							data: data.into(),
							binary: true,
						}),
					};
					let serialized = match versioned::TunnelMessage::serialize(
						versioned::TunnelMessage::V1(ws_message),
					) {
						Result::Ok(s) => s,
						Err(_) => break,
					};
					if let Err(err) = ups.publish(&topic, &serialized).await {
						if is_tunnel_closed_error(&err) {
							tracing::warn!("tunnel closed sending binary message");
							close_reason = Some("Tunnel closed".to_string());
							break;
						} else {
							tracing::error!(?err, "error sending binary message");
						}
					}
				}
				ResultOk(Message::Text(text)) => {
					let ws_message = TunnelMessage {
						body: MessageBody::ToServerWebSocketMessage(ToServerWebSocketMessage {
							web_socket_id: websocket_id,
							data: text.as_bytes().to_vec(),
							binary: false,
						}),
					};
					let serialized = match versioned::TunnelMessage::serialize(
						versioned::TunnelMessage::V1(ws_message),
					) {
						Result::Ok(s) => s,
						Err(_) => break,
					};
					if let Err(err) = ups.publish(&topic, &serialized).await {
						if is_tunnel_closed_error(&err) {
							tracing::warn!("tunnel closed sending text message");
							close_reason = Some("Tunnel closed".to_string());
							break;
						} else {
							tracing::error!(?err, "error sending text message");
						}
					}
				}
				ResultOk(Message::Close(_)) | Err(_) => break,
				_ => {}
			}
		}

		// Send WebSocket close message
		let close_message = TunnelMessage {
			body: MessageBody::ToServerWebSocketClose(ToServerWebSocketClose {
				web_socket_id: websocket_id,
				code: None,
				reason: close_reason,
			}),
		};

		let serialized = match versioned::TunnelMessage::serialize(versioned::TunnelMessage::V1(
			close_message,
		)) {
			Result::Ok(s) => s,
			Err(_) => Vec::new(),
		};

		if let Err(err) = ups.publish(&topic, &serialized).await {
			if is_tunnel_closed_error(&err) {
				tracing::warn!("tunnel closed sending close message");
			} else {
				tracing::error!(?err, "error sending close message");
			}
		}

		std::result::Result::<(), (HyperWebsocket, anyhow::Error)>::Ok(())
	}
}

/// Determines if the tunnel is closed by if the UPS service is no longer responding.
fn is_tunnel_closed_error(err: &anyhow::Error) -> bool {
	if let Some(err) = err.chain().find_map(|x| x.downcast_ref::<RivetError>())
		&& err.group() == "ups"
		&& (err.code() == "no_responders" || err.code() == "request_timeout")
	{
		true
	} else {
		false
	}
}
