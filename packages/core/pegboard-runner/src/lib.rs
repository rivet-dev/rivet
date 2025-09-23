use anyhow::Context;
use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use gas::prelude::*;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use hyper_tungstenite::tungstenite::Message;
use pegboard::ops::runner::update_alloc_idx::Action;
use rivet_guard_core::{
	WebSocketHandle, custom_serve::CustomServeTrait, proxy_service::ResponseBody,
	request_context::RequestContext,
};
use std::time::Duration;

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
	) -> Result<()> {
		// Get UPS
		let ups = self.ctx.ups().context("failed to get UPS instance")?;

		// Parse URL to extract parameters
		let url = url::Url::parse(&format!("ws://placeholder/{path}"))
			.context("failed to parse WebSocket URL")?;
		let url_data =
			utils::UrlData::parse_url(url).context("failed to extract URL parameters")?;

		tracing::info!(?path, "tunnel ws connection established");

		// Accept WS
		let mut ws_rx = ws_handle
			.accept()
			.await
			.context("failed to accept WebSocket connection")?;

		// Create connection
		let conn = conn::init_conn(&self.ctx, ws_handle.clone(), &mut ws_rx, url_data)
			.await
			.context("failed to initialize runner connection")?;

		// Subscribe to pubsub topic for this runner before accepting the client websocket so
		// that failures can be retried by the proxy.
		let topic =
			pegboard::pubsub_subjects::RunnerReceiverSubject::new(conn.runner_id).to_string();
		tracing::info!(%topic, "subscribing to runner receiver topic");
		let sub = ups
			.subscribe(&topic)
			.await
			.with_context(|| format!("failed to subscribe to runner receiver topic: {}", topic))?;

		// Forward pubsub -> WebSocket
		let mut pubsub_to_client = tokio::spawn(pubsub_to_client_task::task(
			self.ctx.clone(),
			conn.clone(),
			sub,
		));

		// Forward WebSocket -> pubsub
		let mut client_to_pubsub = tokio::spawn(client_to_pubsub_task::task(
			self.ctx.clone(),
			conn.clone(),
			ws_rx,
		));

		// Update pings
		let mut ping = tokio::spawn(ping_task::task(self.ctx.clone(), conn.clone()));

		// Wait for either task to complete
		tokio::select! {
			_ = &mut pubsub_to_client => {
				tracing::info!("pubsub to WebSocket task completed");
			}
			_ = &mut client_to_pubsub => {
				tracing::info!("WebSocket to pubsub task completed");
			}
			_ = &mut ping => {
				tracing::info!("ping task completed");
			}
		}

		// Abort remaining tasks
		pubsub_to_client.abort();
		client_to_pubsub.abort();
		ping.abort();

		// Make runner immediately ineligible when it disconnects
		self.ctx
			.op(pegboard::ops::runner::update_alloc_idx::Input {
				runners: vec![pegboard::ops::runner::update_alloc_idx::Runner {
					runner_id: conn.runner_id,
					action: Action::ClearIdx,
				}],
			})
			.await
			.map_err(|err| {
				// Log the error with full context but continue cleanup
				tracing::error!(
					?conn.runner_id,
					?err,
					"critical: failed to evict runner from allocation index during disconnect"
				);
				err
			})
			.ok();

		// Clean up
		tracing::info!(?conn.runner_id, "connection closed");

		Ok(())
	}
}
