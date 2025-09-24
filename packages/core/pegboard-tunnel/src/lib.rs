use anyhow::*;
use async_trait::async_trait;
use bytes::Bytes;
use futures::{SinkExt, StreamExt};
use gas::prelude::*;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use hyper_tungstenite::{HyperWebsocket, tungstenite::Message as WsMessage};
use rivet_guard_core::{
	custom_serve::CustomServeTrait, proxy_service::ResponseBody, request_context::RequestContext,
};
use rivet_tunnel_protocol::{
	MessageKind, PROTOCOL_VERSION, PubSubMessage, RequestId, RunnerMessage, versioned,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;
use universalpubsub::{PublishOpts, pubsub::NextOutput};
use versioned_data_util::OwnedVersionedData as _;

pub struct PegboardTunnelCustomServe {
	ctx: StandaloneCtx,
}

impl PegboardTunnelCustomServe {
	pub fn new(ctx: StandaloneCtx) -> Self {
		Self { ctx }
	}
}

#[async_trait]
impl CustomServeTrait for PegboardTunnelCustomServe {
	async fn handle_request(
		&self,
		_req: hyper::Request<http_body_util::Full<hyper::body::Bytes>>,
		_request_context: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
		// Pegboard tunnel doesn't handle regular HTTP requests
		// Return a simple status response
		let response = Response::builder()
			.status(StatusCode::OK)
			.header("Content-Type", "text/plain")
			.body(ResponseBody::Full(Full::new(Bytes::from(
				"pegboard-tunnel WebSocket endpoint",
			))))?;

		Ok(response)
	}

	async fn handle_websocket(
		&self,
		client_ws: HyperWebsocket,
		_headers: &hyper::HeaderMap,
		path: &str,
		_request_context: &mut RequestContext,
	) -> std::result::Result<(), (HyperWebsocket, anyhow::Error)> {
		let ups = match self.ctx.pools().ups() {
			Result::Ok(u) => u,
			Err(e) => return Err((client_ws, e.into())),
		};

		// Parse URL to extract runner_id and protocol version
		let url = match url::Url::parse(&format!("ws://placeholder/{path}")) {
			Result::Ok(u) => u,
			Err(e) => return Err((client_ws, e.into())),
		};

		// Extract namespace name from query parameters (required) and resolve to namespace_id
		let namespace_name = match url
			.query_pairs()
			.find_map(|(n, v)| (n == "namespace").then_some(v))
		{
			Some(name) => name.to_string(),
			None => {
				return Err((client_ws, anyhow!("namespace query parameter is required")));
			}
		};

		// Resolve namespace name to namespace_id
		let namespace = match self
			.ctx
			.op(namespace::ops::resolve_for_name_global::Input {
				name: namespace_name.clone(),
			})
			.await
		{
			Result::Ok(Some(ns)) => ns,
			Result::Ok(None) => {
				return Err((
					client_ws,
					anyhow!("namespace '{}' not found", namespace_name),
				));
			}
			Err(e) => return Err((client_ws, e)),
		};
		let namespace_id = namespace.namespace_id;

		// Extract runner_name from query parameters (required)
		let runner_name = match url
			.query_pairs()
			.find_map(|(n, v)| (n == "runner_name").then_some(v))
		{
			Some(name) => name.to_string(),
			None => {
				return Err((
					client_ws,
					anyhow!("runner_name query parameter is required"),
				));
			}
		};

		// Extract runner_key from query parameters (required)
		let runner_key = match url
			.query_pairs()
			.find_map(|(n, v)| (n == "runner_key").then_some(v))
		{
			Some(key) => key.to_string(),
			None => {
				return Err((client_ws, anyhow!("runner_key query parameter is required")));
			}
		};

		// Extract protocol version from query parameters (required)
		let protocol_version = match url
			.query_pairs()
			.find_map(|(n, v)| (n == "protocol_version").then_some(v))
			.as_ref()
			.and_then(|v| v.parse::<u16>().ok())
		{
			Some(version) => version,
			None => {
				return Err((
					client_ws,
					anyhow!("protocol_version query parameter is required and must be a valid u16"),
				));
			}
		};

		tracing::info!(
			?namespace_id,
			?runner_name,
			?runner_key,
			?protocol_version,
			?path,
			"tunnel WebSocket connection established"
		);

		// Subscribe to pubsub topic for this runner before accepting the client websocket so
		// that failures can be retried by the proxy.
		let topic = pegboard::pubsub_subjects::TunnelRunnerReceiverSubject::new(
			namespace_id,
			&runner_name,
			&runner_key,
		)
		.to_string();
		tracing::info!(%topic, ?runner_key, "subscribing to runner receiver topic");
		let mut sub = match ups.subscribe(&topic).await {
			Result::Ok(s) => s,
			Err(e) => return Err((client_ws, e.into())),
		};

		// Accept WS
		let ws_stream = match client_ws.await {
			Result::Ok(ws) => ws,
			Err(e) => {
				// Handshake already in progress; cannot retry safely here
				tracing::error!(error=?e, "client websocket await failed");
				return std::result::Result::<(), (HyperWebsocket, anyhow::Error)>::Ok(());
			}
		};

		let (ws_write, mut ws_read) = ws_stream.split();
		let ws_write = Arc::new(tokio::sync::Mutex::new(ws_write));

		struct ActiveRequest {
			/// Subject to send replies to.
			reply_to: String,
		}

		// Active HTTP & WebSocket requests. They are separate but use the same mechanism to
		// maintain state.
		let active_requests = Arc::new(Mutex::new(HashMap::<RequestId, ActiveRequest>::new()));

		// Forward pubsub -> WebSocket
		let ws_write_pubsub_to_ws = ws_write.clone();
		let ups_clone = ups.clone();
		let active_requests_clone = active_requests.clone();
		let pubsub_to_ws = tokio::spawn(async move {
			while let Result::Ok(NextOutput::Message(ups_msg)) = sub.next().await {
				tracing::info!(
					payload_len = ups_msg.payload.len(),
					"received message from pubsub, forwarding to WebSocket"
				);

				// Parse message
				let msg = match versioned::PubSubMessage::deserialize_with_embedded_version(
					&ups_msg.payload,
				) {
					Result::Ok(x) => x,
					Err(err) => {
						tracing::error!(?err, "failed to parse tunnel message");
						continue;
					}
				};

				// Save active request
				if let Some(reply_to) = msg.reply_to {
					let mut active_requests = active_requests_clone.lock().await;
					active_requests.insert(msg.request_id, ActiveRequest { reply_to });
				}

				// If terminal, remove active request tracking
				if is_message_kind_request_close(&msg.message_kind) {
					let mut active_requests = active_requests_clone.lock().await;
					active_requests.remove(&msg.request_id);
				}

				// Forward raw message to WebSocket
				let tunnel_msg = match versioned::RunnerMessage::latest(RunnerMessage {
					request_id: msg.request_id,
					message_id: msg.message_id,
					message_kind: msg.message_kind,
				})
				.serialize_version(protocol_version)
				{
					Result::Ok(x) => x,
					Err(err) => {
						tracing::error!(?err, "failed to serialize tunnel message");
						continue;
					}
				};
				let ws_msg = WsMessage::Binary(tunnel_msg.into());
				{
					let mut stream = ws_write_pubsub_to_ws.lock().await;
					if let Err(e) = stream.send(ws_msg).await {
						tracing::error!(?e, "failed to send message to WebSocket");
						break;
					}
				}
			}
			tracing::info!("pubsub to WebSocket forwarding task ended");
		});

		// Forward WebSocket -> pubsub
		let active_requests_clone = active_requests.clone();
		let runner_key_clone = runner_key.clone();
		let ws_to_pubsub = tokio::spawn(async move {
			tracing::info!("starting WebSocket to pubsub forwarding task");
			while let Some(msg) = ws_read.next().await {
				match msg {
					Result::Ok(WsMessage::Binary(data)) => {
						tracing::info!(
							data_len = data.len(),
							"received binary message from WebSocket"
						);

						// Parse message
						let msg = match versioned::RunnerMessage::deserialize_version(
							&data,
							protocol_version,
						)
						.and_then(|x| x.into_latest())
						{
							Result::Ok(x) => x,
							Err(err) => {
								tracing::error!(?err, "failed to deserialize message");
								continue;
							}
						};

						// Determine reply to subject
						let request_id = msg.request_id;
						let reply_to = {
							let active_requests = active_requests_clone.lock().await;
							if let Some(req) = active_requests.get(&request_id) {
								req.reply_to.clone()
							} else {
								tracing::warn!(
									"no active request for tunnel message, may have timed out"
								);
								continue;
							}
						};

						// Remove active request entries when terminal
						if is_message_kind_request_close(&msg.message_kind) {
							let mut active_requests = active_requests_clone.lock().await;
							active_requests.remove(&request_id);
						}

						// Publish message to UPS
						let message_serialized =
							match versioned::PubSubMessage::latest(PubSubMessage {
								request_id: msg.request_id,
								message_id: msg.message_id,
								reply_to: None,
								message_kind: msg.message_kind,
							})
							.serialize_with_embedded_version(PROTOCOL_VERSION)
							{
								Result::Ok(x) => x,
								Err(err) => {
									tracing::error!(?err, "failed to serialize tunnel to gateway");
									continue;
								}
							};
						match ups_clone
							.publish(&reply_to, &message_serialized, PublishOpts::one())
							.await
						{
							Result::Ok(_) => {}
							Err(err) => {
								tracing::error!(?err, "error publishing ups message");
							}
						}
					}
					Result::Ok(WsMessage::Close(_)) => {
						tracing::info!(?runner_key_clone, "WebSocket closed");
						break;
					}
					Result::Ok(_) => {
						// Ignore other message types
					}
					Err(e) => {
						tracing::error!(?e, "WebSocket error");
						break;
					}
				}
			}
			tracing::info!("WebSocket to pubsub forwarding task ended");
		});

		// Wait for either task to complete
		tokio::select! {
			_ = pubsub_to_ws => {
				tracing::info!("pubsub to WebSocket task completed");
			}
			_ = ws_to_pubsub => {
				tracing::info!("WebSocket to pubsub task completed");
			}
		}

		// Clean up
		tracing::info!(?runner_key, "connection closed");

		std::result::Result::<(), (HyperWebsocket, anyhow::Error)>::Ok(())
	}
}

fn is_message_kind_request_close(kind: &MessageKind) -> bool {
	match kind {
		// HTTP terminal states
		MessageKind::ToClientResponseStart(resp) => !resp.stream,
		MessageKind::ToClientResponseChunk(chunk) => chunk.finish,
		MessageKind::ToClientResponseAbort => true,
		// WebSocket terminal states (either side closes)
		MessageKind::ToClientWebSocketClose(_) => true,
		MessageKind::ToServerWebSocketClose(_) => true,
		_ => false,
	}
}
