use async_trait::async_trait;
use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use gas::prelude::*;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use hyper_tungstenite::{HyperWebsocket, tungstenite::Message};
use pegboard::ops::runner::update_alloc_idx::Action;
use rivet_guard_core::{
	custom_serve::CustomServeTrait, proxy_service::ResponseBody, request_context::RequestContext,
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
		client_ws: HyperWebsocket,
		_headers: &hyper::HeaderMap,
		path: &str,
		_request_context: &mut RequestContext,
	) -> Result<(), (HyperWebsocket, anyhow::Error)> {
		// TODO: Spawn ping thread
		// TODO: Spawn message thread
		// TODO: Create conn

		// Get UPS
		let ups = match self.ctx.ups() {
			Ok(x) => x,
			Err(err) => {
				tracing::warn!(?err, "could not get ups");
				return Err((client_ws, err));
			}
		};

		// Parse URL to extract parameters
		let url = match url::Url::parse(&format!("ws://placeholder/{path}")) {
			Result::Ok(u) => u,
			Result::Err(e) => return Err((client_ws, e.into())),
		};

		let url_data = match utils::UrlData::parse_url(url) {
			Result::Ok(x) => x,
			Result::Err(err) => {
				tracing::warn!(?err, "could not parse runner connection url");
				return Err((client_ws, err));
			}
		};

		tracing::info!(?path, "tunnel ws connection established");

		// Accept WS
		let ws_stream = match client_ws.await {
			Result::Ok(ws) => ws,
			Err(e) => {
				// Handshake already in progress; cannot retry safely here
				tracing::error!(error=?e, "client websocket await failed");
				return Result::<(), (HyperWebsocket, anyhow::Error)>::Ok(());
			}
		};
		let (ws_tx, mut ws_rx) = ws_stream.split();

		// Create connection
		let mut ws_tx = Some(ws_tx);
		let conn = match conn::init_conn(&self.ctx, &mut ws_tx, &mut ws_rx, url_data).await {
			Ok(x) => x,

			Err(err) => {
				tracing::warn!(?err, "failed to build connection");

				if let Some(mut tx) = ws_tx {
					let close_frame = utils::err_to_close_frame(err);

					if let Err(err) = tx.send(Message::Close(Some(close_frame))).await {
						tracing::error!(?err, "failed closing socket");
					}
				}

				return Ok(());
			}
		};

		// Subscribe to pubsub topic for this runner before accepting the client websocket so
		// that failures can be retried by the proxy.
		let topic =
			pegboard::pubsub_subjects::RunnerReceiverSubject::new(conn.runner_id).to_string();
		tracing::info!(%topic, "subscribing to runner receiver topic");
		let mut sub = match ups.subscribe(&topic).await {
			Result::Ok(s) => s,
			Err(err) => {
				// TODO: Handle this error correctly
				tracing::error!(?err, "failed to subscribe to runner receiver");
				return Ok(());
			}
		};

		// Forward pubsub -> WebSocket
		let pubsub_to_client = tokio::spawn(pubsub_to_client_task::task(
			self.ctx.clone(),
			conn.clone(),
			sub,
		));

		// Forward WebSocket -> pubsub
		let client_to_pubsub = tokio::spawn(client_to_pubsub_task::task(
			self.ctx.clone(),
			conn.clone(),
			ws_rx,
		));

		// Update pings
		let ping = tokio::spawn(ping_task::task(self.ctx.clone(), conn.clone()));

		// Wait for either task to complete
		tokio::select! {
			_ = pubsub_to_client => {
				tracing::info!("pubsub to WebSocket task completed");
			}
			_ = client_to_pubsub => {
				tracing::info!("WebSocket to pubsub task completed");
			}
			_ = ping => {
				tracing::info!("ping task completed");
			}
		}

		// Make runner immediately ineligible when it disconnects
		if let Err(err) = self
			.ctx
			.op(pegboard::ops::runner::update_alloc_idx::Input {
				runners: vec![pegboard::ops::runner::update_alloc_idx::Runner {
					runner_id: conn.runner_id,
					action: Action::ClearIdx,
				}],
			})
			.await
		{
			tracing::error!(?conn.runner_id, ?err, "failed evicting runner from alloc idx");
		}

		// TODO: Handle errors
		// // Close WS
		// let close_frame = utils::err_to_close_frame(err);
		// let mut tx = conn.ws_tx.lock().await;
		// if let Err(err) = tx.send(Message::Close(Some(close_frame))).await {
		// 	tracing::error!(?runner_id, ?err, "failed closing socket");
		// }

		// Clean up
		tracing::info!(?conn.runner_id, "connection closed");

		Result::<(), (HyperWebsocket, anyhow::Error)>::Ok(())
	}
}
