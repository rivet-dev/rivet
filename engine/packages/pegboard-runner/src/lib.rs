use anyhow::Context;
use async_trait::async_trait;
use bytes::Bytes;
use gas::prelude::*;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use pegboard::ops::runner::update_alloc_idx::Action;
use pegboard::tunnel::id::RequestId;
use rivet_guard_core::{
	WebSocketHandle, custom_serve::CustomServeTrait, proxy_service::ResponseBody,
	request_context::RequestContext,
};
use std::time::Duration;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::protocol::frame::CloseFrame;
use universalpubsub::PublishOpts;

mod conn;
mod errors;
mod ping_task;
mod tunnel_to_ws_task;
mod utils;
mod ws_to_tunnel_task;

const UPDATE_PING_INTERVAL: Duration = Duration::from_secs(3);

#[derive(Debug)]
enum LifecycleResult {
	Closed,
	Aborted,
}

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
		_request_id: RequestId,
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
		_unique_request_id: pegboard::tunnel::id::RequestId,
		_after_hibernation: bool,
	) -> Result<Option<CloseFrame>> {
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

		// Subscribe before accepting the client websocket so that failures can be retried by the proxy.
		let topic =
			pegboard::pubsub_subjects::RunnerReceiverSubject::new(conn.runner_id).to_string();
		let eviction_topic =
			pegboard::pubsub_subjects::RunnerEvictionByIdSubject::new(conn.runner_id).to_string();
		let eviction_topic2 = pegboard::pubsub_subjects::RunnerEvictionByNameSubject::new(
			conn.namespace_id,
			&conn.runner_name,
			&conn.runner_key,
		)
		.to_string();

		tracing::debug!(%topic, %eviction_topic, %eviction_topic2, "subscribing to runner topics");
		let sub = ups
			.subscribe(&topic)
			.await
			.with_context(|| format!("failed to subscribe to runner receiver topic: {}", topic))?;
		let mut eviction_sub = ups.subscribe(&eviction_topic).await.with_context(|| {
			format!(
				"failed to subscribe to runner eviction topic: {}",
				eviction_topic
			)
		})?;
		let mut eviction_sub2 = ups.subscribe(&eviction_topic2).await.with_context(|| {
			format!(
				"failed to subscribe to runner eviction topic: {}",
				eviction_topic2
			)
		})?;

		// Publish eviction message to evict any currently connected runners with the same id or ns id +
		// runner name + runner key. This happens after subscribing to prevent race conditions.
		tokio::try_join!(
			async {
				ups.publish(&eviction_topic, &[], PublishOpts::broadcast())
					.await?;
				// Because we will receive our own message, skip the first message in the sub
				eviction_sub.next().await
			},
			async {
				ups.publish(&eviction_topic2, &[], PublishOpts::broadcast())
					.await?;
				eviction_sub2.next().await
			},
		)?;

		let (tunnel_to_ws_abort_tx, tunnel_to_ws_abort_rx) = watch::channel(());
		let (ws_to_tunnel_abort_tx, ws_to_tunnel_abort_rx) = watch::channel(());
		let (ping_abort_tx, ping_abort_rx) = watch::channel(());

		let tunnel_to_ws = tokio::spawn(tunnel_to_ws_task::task(
			self.ctx.clone(),
			conn.clone(),
			sub,
			eviction_sub,
			tunnel_to_ws_abort_rx,
		));

		let ws_to_tunnel = tokio::spawn(ws_to_tunnel_task::task(
			self.ctx.clone(),
			conn.clone(),
			ws_handle.recv(),
			eviction_sub2,
			ws_to_tunnel_abort_rx,
		));

		// Update pings
		let ping = tokio::spawn(ping_task::task(
			self.ctx.clone(),
			conn.clone(),
			ping_abort_rx,
		));
		let tunnel_to_ws_abort_tx2 = tunnel_to_ws_abort_tx.clone();
		let ws_to_tunnel_abort_tx2 = ws_to_tunnel_abort_tx.clone();
		let ping_abort_tx2 = ping_abort_tx.clone();

		// Wait for all tasks to complete
		let (tunnel_to_ws_res, ws_to_tunnel_res, ping_res) = tokio::join!(
			async {
				let res = tunnel_to_ws.await?;

				// Abort others if not aborted
				if !matches!(res, Ok(LifecycleResult::Aborted)) {
					tracing::debug!(?res, "tunnel to ws task completed, aborting others");

					let _ = ping_abort_tx.send(());
					let _ = ws_to_tunnel_abort_tx.send(());
				} else {
					tracing::debug!(?res, "tunnel to ws task completed");
				}

				res
			},
			async {
				let res = ws_to_tunnel.await?;

				// Abort others if not aborted
				if !matches!(res, Ok(LifecycleResult::Aborted)) {
					tracing::debug!(?res, "ws to tunnel task completed, aborting others");

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
			}
		);

		// Determine single result from all tasks
		let lifecycle_res = match (tunnel_to_ws_res, ws_to_tunnel_res, ping_res) {
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

		// This will determine the close frame sent back to the runner websocket
		lifecycle_res.map(|_| None)
	}
}
