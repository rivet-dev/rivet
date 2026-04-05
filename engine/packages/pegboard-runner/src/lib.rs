use anyhow::Context;
use async_trait::async_trait;
use bytes::Bytes;
use gas::prelude::*;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use pegboard::ops::runner::update_alloc_idx::Action;
use rivet_guard_core::{
	ResponseBody, WebSocketHandle, custom_serve::CustomServeTrait, request_context::RequestContext,
};
use std::time::Duration;
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::protocol::frame::CloseFrame;
use universalpubsub::PublishOpts;

mod actor_event_demuxer;
mod conn;
mod errors;
mod metrics;
mod ping_task;
mod tunnel_to_ws_task;
mod utils;
mod ws_to_tunnel_task;

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
	#[tracing::instrument(skip_all)]
	async fn handle_request(
		&self,
		_req: hyper::Request<http_body_util::Full<hyper::body::Bytes>>,
		_req_ctx: &mut RequestContext,
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

	#[tracing::instrument(skip_all)]
	async fn handle_websocket(
		&self,
		req_ctx: &mut RequestContext,
		ws_handle: WebSocketHandle,
		_after_hibernation: bool,
	) -> Result<Option<CloseFrame>> {
		let ctx = self.ctx.with_ray(req_ctx.ray_id(), req_ctx.req_id())?;

		// Get UPS
		let ups = ctx.ups().context("failed to get UPS instance")?;

		// Parse URL to extract parameters
		let url = url::Url::parse(&format!("ws://placeholder/{}", req_ctx.path()))
			.context("failed to parse WebSocket URL")?;
		let url_data = utils::UrlData::parse_url(url)
			.map_err(|err| errors::WsError::InvalidUrl(err.to_string()).build())?;

		tracing::debug!(path=%req_ctx.path(), "tunnel ws connection established");

		// Create connection
		let conn = conn::init_conn(&ctx, ws_handle.clone(), url_data)
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

		metrics::CONNECTION_ACTIVE
			.with_label_values(&[
				conn.namespace_id.to_string().as_str(),
				&conn.runner_name,
				conn.protocol_version.to_string().as_str(),
			])
			.inc();

		let (tunnel_to_ws_abort_tx, tunnel_to_ws_abort_rx) = watch::channel(());
		let (ws_to_tunnel_abort_tx, ws_to_tunnel_abort_rx) = watch::channel(());
		let (ping_abort_tx, ping_abort_rx) = watch::channel(());

		let tunnel_to_ws = tokio::spawn(tunnel_to_ws_task::task(
			ctx.clone(),
			conn.clone(),
			sub,
			eviction_sub,
			tunnel_to_ws_abort_rx,
		));

		let ws_to_tunnel = tokio::spawn(ws_to_tunnel_task::task(
			ctx.clone(),
			conn.clone(),
			ws_handle.recv(),
			eviction_sub2,
			ws_to_tunnel_abort_rx,
		));

		// Update pings
		let update_ping_interval =
			Duration::from_millis(ctx.config().pegboard().runner_update_ping_interval_ms());
		let ping = tokio::spawn(ping_task::task(
			ctx.clone(),
			conn.clone(),
			ping_abort_rx,
			update_ping_interval,
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

		tracing::debug!(%topic, "runner websocket closed");

		metrics::CONNECTION_ACTIVE
			.with_label_values(&[
				conn.namespace_id.to_string().as_str(),
				&conn.runner_name,
				conn.protocol_version.to_string().as_str(),
			])
			.dec();

		// This will determine the close frame sent back to the runner websocket
		lifecycle_res.map(|_| None)
	}
}
