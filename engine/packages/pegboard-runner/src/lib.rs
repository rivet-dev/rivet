use anyhow::Context;
use async_trait::async_trait;
use bytes::Bytes;
use gas::prelude::*;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use pegboard::ops::runner::update_alloc_idx::Action;
use rivet_guard_core::{
	WebSocketHandle, custom_serve::CustomServeTrait, proxy_service::ResponseBody,
	request_context::RequestContext,
};
use rivet_runner_protocol as protocol;
use std::time::Duration;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use universalpubsub::PublishOpts;
use vbare::OwnedVersionedData;

mod client_to_pubsub_task;
mod conn;
mod errors;
mod ping_task;
mod pubsub_to_client_task;
mod utils;

const UPDATE_PING_INTERVAL: Duration = Duration::from_secs(3);

pub struct PegboardRunnerWsCustomServe {
	ctx: StandaloneCtx,
}

impl PegboardRunnerWsCustomServe {
	pub fn new(ctx: StandaloneCtx) -> Self {
		let service = Self { ctx: ctx.clone() };

		service
	}
}

#[async_trait]
impl CustomServeTrait for PegboardRunnerWsCustomServe {
	async fn handle_request(
		&self,
		_req: hyper::Request<http_body_util::Full<hyper::body::Bytes>>,
		_request_context: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
		// Pegboard runner ws doesn't handle regular HTTP requests
		// Return a simple status response
		let response = Response::builder()
			.status(StatusCode::OK)
			.header("Content-Type", "text/plain")
			.body(ResponseBody::Full(Full::new(Bytes::from(
				"pegboard-runner WebSocket endpoint",
			))))?;

		Ok(response)
	}

	async fn handle_websocket(
		&self,
		ws_handle: WebSocketHandle,
		_headers: &hyper::HeaderMap,
		path: &str,
		_request_context: &mut RequestContext,
		_unique_request_id: Uuid,
	) -> Result<()> {
		// Get UPS
		let ups = self.ctx.ups().context("failed to get UPS instance")?;

		// Parse URL to extract parameters
		let url = url::Url::parse(&format!("ws://placeholder/{path}"))
			.context("failed to parse WebSocket URL")?;
		let url_data = utils::UrlData::parse_url(url)
			.map_err(|err| errors::WsError::InvalidUrl(err.to_string()).build())?;

		tracing::debug!(?path, "tunnel ws connection established");

		// Create connection
		let conn = conn::init_conn(&self.ctx, ws_handle.clone(), url_data)
			.await
			.context("failed to initialize runner connection")?;

		// Subscribe to pubsub topic for this runner before accepting the client websocket so
		// that failures can be retried by the proxy.
		let topic =
			pegboard::pubsub_subjects::RunnerReceiverSubject::new(conn.runner_id).to_string();
		tracing::debug!(%topic, "subscribing to runner receiver topic");
		let sub = ups
			.subscribe(&topic)
			.await
			.with_context(|| format!("failed to subscribe to runner receiver topic: {}", topic))?;

		// Forward pubsub -> WebSocket
		let mut pubsub_to_client = tokio::spawn(pubsub_to_client_task::task(conn.clone(), sub));

		// Forward WebSocket -> pubsub
		let mut client_to_pubsub = tokio::spawn(client_to_pubsub_task::task(
			self.ctx.clone(),
			conn.clone(),
			ws_handle.recv(),
		));

		// Update pings
		let mut ping = tokio::spawn(ping_task::task(self.ctx.clone(), conn.clone()));

		// Wait for either task to complete
		let lifecycle_res = tokio::select! {
			res = &mut pubsub_to_client => {
				let res = res?;
				tracing::debug!(?res, "pubsub to WebSocket task completed");
				res
			}
			res = &mut client_to_pubsub => {
				let res = res?;
				tracing::debug!(?res, "WebSocket to pubsub task completed");
				res
			}
			res = &mut ping => {
				let res = res?;
				tracing::debug!(?res, "ping task completed");
				res
			}
		};

		// Abort remaining tasks
		pubsub_to_client.abort();
		client_to_pubsub.abort();
		ping.abort();

		// Make runner immediately ineligible when it disconnects
		let update_alloc_res = self
			.ctx
			.op(pegboard::ops::runner::update_alloc_idx::Input {
				runners: vec![pegboard::ops::runner::update_alloc_idx::Runner {
					runner_id: conn.runner_id,
					action: Action::ClearIdx,
				}],
			})
			.await;
		if let Err(err) = update_alloc_res {
			tracing::error!(
				runner_id=?conn.runner_id,
				?err,
				"critical: failed to evict runner from allocation index during disconnect"
			);
		}

		// Send WebSocket close messages to all remaining active requests
		let active_requests = conn.tunnel_active_requests.lock().await;
		for (request_id, req) in &*active_requests {
			let (close_code, close_reason) = if lifecycle_res.is_ok() {
				(CloseCode::Normal.into(), None)
			} else {
				(CloseCode::Error.into(), Some("ws.upstream_closed".into()))
			};

			let close_message = protocol::ToServerTunnelMessage {
				request_id: request_id.clone(),
				message_id: Uuid::new_v4().into_bytes(),
				message_kind: protocol::ToServerTunnelMessageKind::ToServerWebSocketClose(
					protocol::ToServerWebSocketClose {
						code: Some(close_code),
						reason: close_reason,
					},
				),
			};

			let msg_serialized = protocol::versioned::ToGateway::latest(protocol::ToGateway {
				message: close_message.clone(),
			})
			.serialize_with_embedded_version(protocol::PROTOCOL_VERSION)
			.context("failed to serialize tunnel message for gateway")?;

			// Publish message to UPS
			let res = self
				.ctx
				.ups()
				.context("failed to get UPS instance for tunnel message")?
				.publish(&req.gateway_reply_to, &msg_serialized, PublishOpts::one())
				.await;

			if let Err(err) = res {
				tracing::warn!(
					?err,
					%req.gateway_reply_to,
					"error sending close message to remaining active requests"
				);
			}
		}

		// This will determine the close frame sent back to the runner websocket
		lifecycle_res
	}
}
