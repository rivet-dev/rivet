use anyhow::Context;
use async_trait::async_trait;
use bytes::Bytes;
use gas::prelude::*;
use http_body_util::Full;
use hyper::{Response, StatusCode};
use rivet_error::RivetError;
use rivet_guard_core::{
	ResponseBody, WebSocketHandle, custom_serve::CustomServeTrait, request_context::RequestContext,
};
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::protocol::frame::CloseFrame;
use universalpubsub::PublishOpts;

mod actor_event_demuxer;
mod actor_kv_task;
mod actor_lifecycle;
mod actor_remote_sqlite_task;
mod actor_sqlite_page_task;
mod conn;
mod control_task;
mod errors;
mod hibernating_requests;
pub mod metrics;
mod ping_task;
pub mod sqlite_runtime;
mod tunnel_message_task;
mod tunnel_to_ws_task;
mod utils;
mod ws_to_tunnel_task;

#[derive(Debug)]
enum LifecycleResult {
	Closed {
		incoming_close_code: Option<u16>,
		incoming_close_reason: Option<String>,
	},
	Aborted,
	Evicted,
}

pub struct PegboardEnvoyWs {
	ctx: StandaloneCtx,
}

impl PegboardEnvoyWs {
	pub fn new(ctx: StandaloneCtx) -> Self {
		metrics::prepopulate();

		let service = Self { ctx: ctx.clone() };

		service
	}
}

#[async_trait]
impl CustomServeTrait for PegboardEnvoyWs {
	#[tracing::instrument(skip_all)]
	async fn handle_request(
		&self,
		_req: hyper::Request<http_body_util::Full<hyper::body::Bytes>>,
		_req_ctx: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
		// Pegboard envoy ws doesn't handle regular HTTP requests
		// Return a simple status response
		let response = Response::builder()
			.status(StatusCode::OK)
			.header("Content-Type", "text/plain")
			.body(ResponseBody::Full(Full::new(Bytes::from(
				"pegboard-envoy WebSocket endpoint",
			))))?;

		Ok(response)
	}

	#[tracing::instrument(skip_all, fields(ray_id=?req_ctx.ray_id(), req_id=?req_ctx.req_id(), namespace_id=tracing::field::Empty, pool_name=tracing::field::Empty, envoy_key=tracing::field::Empty, protocol_version=tracing::field::Empty))]
	async fn handle_websocket(
		&self,
		req_ctx: &mut RequestContext,
		ws_handle: WebSocketHandle,
		_after_hibernation: bool,
	) -> Result<Option<CloseFrame>> {
		let ray_id = req_ctx.ray_id();
		let ctx = self.ctx.with_ray(ray_id, req_ctx.req_id())?;

		// Get UPS
		let ups = ctx.ups().context("failed to get UPS instance")?;

		// Parse URL to extract parameters
		let url = url::Url::parse(&format!("ws://placeholder/{}", req_ctx.path()))
			.context("failed to parse WebSocket URL")?;
		let url_data = utils::UrlData::parse_url(url)?;

		tracing::debug!(path=%req_ctx.path(), "tunnel ws connection established");

		let namespace_name = url_data.namespace.clone();
		let namespace = ctx
			.op(namespace::ops::resolve_for_name_global::Input {
				name: namespace_name.clone(),
			})
			.await
			.with_context(|| format!("failed to resolve namespace: {}", namespace_name))?
			.ok_or_else(|| namespace::errors::Namespace::NotFound.build())
			.with_context(|| format!("namespace not found: {}", namespace_name))?;

		let span = tracing::Span::current();
		span.record("namespace_id", namespace.namespace_id.to_string());
		span.record("pool_name", &url_data.pool_name);
		span.record("envoy_key", &url_data.envoy_key);
		span.record("protocol_version", url_data.protocol_version.to_string());

		// Subscribe before inserting the envoy in the load balancer. Pending actors can retry
		// as soon as the envoy is eligible, so subscribing after init_conn can miss a live start command.
		let topic = pegboard::pubsub_subjects::EnvoyReceiverSubject::new(
			namespace.namespace_id,
			url_data.envoy_key.clone(),
		);
		let eviction_topic = pegboard::pubsub_subjects::EnvoyEvictionSubject::new(
			namespace.namespace_id,
			url_data.envoy_key.clone(),
		);

		tracing::debug!(%topic, %eviction_topic, "subscribing to envoy topics");
		let sub = ups
			.subscribe(&topic)
			.await
			.with_context(|| format!("failed to subscribe to envoy receiver topic: {}", topic))?;
		let mut eviction_sub = ups.subscribe(&eviction_topic).await.with_context(|| {
			format!(
				"failed to subscribe to envoy eviction topic: {}",
				eviction_topic
			)
		})?;
		tracing::trace!(%topic, %eviction_topic, "subscribed to envoy topics");

		// Create the connection.
		let conn = conn::init_conn(&ctx, ws_handle.clone(), url_data)
			.await
			.context("failed to initialize envoy connection")?;

		// Publish eviction message to evict any currently connected envoys with the same key. This happens
		// after subscribing to prevent race conditions.
		ups.publish(&eviction_topic, &[], PublishOpts::broadcast())
			.await?;
		// Because we will receive our own message, skip the first message in the sub
		eviction_sub.next().await?;

		metrics::CONNECTION_ACTIVE
			.with_label_values(&[
				conn.namespace_id.to_string().as_str(),
				&conn.pool_name,
				conn.protocol_version.to_string().as_str(),
			])
			.inc();
		metrics::ENVOY_CONNECTED
			.with_label_values(&[conn.namespace_id.to_string().as_str(), &conn.pool_name])
			.inc();
		tracing::info!(
			namespace_id = %conn.namespace_id,
			pool_name = %conn.pool_name,
			envoy_key = %conn.envoy_key,
			protocol_version = conn.protocol_version,
			%topic,
			"envoy websocket connected"
		);

		let (tunnel_to_ws_abort_tx, tunnel_to_ws_abort_rx) = watch::channel(());
		let (ws_to_tunnel_abort_tx, ws_to_tunnel_abort_rx) = watch::channel(());
		let (ping_abort_tx, ping_abort_rx) = watch::channel(());

		let tunnel_to_ws = tokio::spawn(
			tunnel_to_ws_task::task(
				ctx.clone(),
				conn.clone(),
				sub,
				eviction_sub,
				tunnel_to_ws_abort_rx,
			)
			.in_current_span(),
		);
		let ws_to_tunnel = tokio::spawn(
			ws_to_tunnel_task::task(
				ctx.clone(),
				conn.clone(),
				ws_handle.recv(),
				ws_to_tunnel_abort_rx,
			)
			.in_current_span(),
		);
		let hard_abort_ws_to_tunnel = ws_to_tunnel.abort_handle();
		let ping = tokio::spawn(
			ping_task::task(ctx.clone(), conn.clone(), ping_abort_rx).in_current_span(),
		);

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
				let res = match ws_to_tunnel.await {
					Err(err) if err.is_cancelled() => Ok(LifecycleResult::Aborted),
					res => res?,
				};

				// Abort others if not aborted
				if !matches!(res, Ok(LifecycleResult::Aborted)) {
					tracing::debug!(?res, "ws to tunnel task completed, aborting others");

					let _ = ping_abort_tx.send(());
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

					let _ = ws_to_tunnel_abort_tx.send(());
					let _ = tunnel_to_ws_abort_tx.send(());
				} else {
					tracing::debug!(?res, "ping task completed");
				}

				// Any error of the ping task must result in a hard abort of ws_to_tunnel. This stops all in
				// flight kv requests from being completed immediately. This guarantees the invariant that an
				// actor's KV is only being accessed from one place at a time.
				if res.is_err() {
					tracing::warn!(?res, "ping task failed, aborting ws_to_tunnel");
					hard_abort_ws_to_tunnel.abort();
				}

				res
			}
		);

		// Determine single result from all tasks
		let mut lifecycle_res = match (tunnel_to_ws_res, ws_to_tunnel_res, ping_res) {
			// Prefer error
			(Err(err), _, _) => Err(err),
			(_, Err(err), _) => Err(err),
			(_, _, Err(err)) => Err(err),
			// Prefer non aborted result
			(Ok(res), Ok(LifecycleResult::Aborted), _) => Ok(res),
			(Ok(LifecycleResult::Aborted), Ok(res), _) => Ok(res),
			// Unlikely case
			(res, _, _) => res,
		};

		if let Ok(LifecycleResult::Evicted) = &lifecycle_res {
			lifecycle_res = Err(errors::WsError::Eviction.build());
		}
		// Evict envoy if lifecycle res is not evicted. Eviction means another envoy connected with the same
		// key so we need to keep it in the idx
		else {
			// Make envoy immediately ineligible when it disconnects
			let expire_res = self
				.ctx
				.op(pegboard::ops::envoy::expire::Input {
					namespace_id: conn.namespace_id,
					envoy_key: conn.envoy_key.to_string(),
					skip_if_fresh: false,
				})
				.await;
			if let Err(err) = expire_res {
				tracing::error!(
					namespace_id=?conn.namespace_id,
					envoy_key=?conn.envoy_key,
					?err,
					"failed to expire envoy during disconnect"
				);
			}
		}

		actor_lifecycle::shutdown_conn_actors(&conn).await;

		// Classify the disconnect so the log line carries the actual close-frame code+reason on both
		// sides of the wire. `incoming_*` is what the envoy sent us (only set when the envoy
		// initiated the close); `outgoing_*` is what we would send back, derived from the lifecycle
		// result. Mirrors `rivet_guard_core::utils::err_to_close_frame` (including the `#{ray_id}`
		// suffix) so the strings match what the envoy actually receives.
		let (
			lifecycle_kind,
			incoming_close_code,
			incoming_close_reason,
			outgoing_close_code,
			outgoing_close_reason,
			err_str,
		) = match &lifecycle_res {
			Ok(LifecycleResult::Closed {
				incoming_close_code,
				incoming_close_reason,
			}) => (
				"closed",
				*incoming_close_code,
				incoming_close_reason.clone(),
				Some(1000u16),
				Some("ws.closed".to_owned()),
				None,
			),
			Ok(LifecycleResult::Aborted) => (
				"aborted",
				None,
				None,
				Some(1000u16),
				Some("ws.closed".to_owned()),
				None,
			),
			Ok(LifecycleResult::Evicted) => (
				"evicted",
				None,
				None,
				Some(1000u16),
				Some(format!("ws.eviction#{}", ray_id)),
				None,
			),
			Err(err) => {
				let rivet_err = err
					.chain()
					.find_map(|x| x.downcast_ref::<RivetError>())
					.cloned();
				let (group, code) = rivet_err
					.as_ref()
					.map(|e| (e.group().to_owned(), e.code().to_owned()))
					.unwrap_or_else(|| ("internal".to_owned(), "internal_error".to_owned()));
				let close_code: u16 = match (group.as_str(), code.as_str()) {
					("ws", "connection_closed") | ("ws", "eviction") => 1000,
					_ => 1011,
				};
				let close_reason = format!("{}.{}#{}", group, code, ray_id);
				(
					"err",
					None,
					None,
					Some(close_code),
					Some(close_reason),
					Some(format!("{:#}", err)),
				)
			}
		};

		tracing::info!(
			namespace_id = %conn.namespace_id,
			pool_name = %conn.pool_name,
			envoy_key = %conn.envoy_key,
			protocol_version = conn.protocol_version,
			%topic,
			lifetime_seconds = conn.connected_at.elapsed().as_secs_f64(),
			lifecycle_kind,
			incoming_close_code = ?incoming_close_code,
			incoming_close_reason = ?incoming_close_reason,
			outgoing_close_code = ?outgoing_close_code,
			outgoing_close_reason = ?outgoing_close_reason,
			err = ?err_str,
			"envoy websocket closed"
		);

		metrics::CONNECTION_ACTIVE
			.with_label_values(&[
				conn.namespace_id.to_string().as_str(),
				&conn.pool_name,
				conn.protocol_version.to_string().as_str(),
			])
			.dec();
		metrics::ENVOY_CONNECTED
			.with_label_values(&[conn.namespace_id.to_string().as_str(), &conn.pool_name])
			.dec();
		metrics::ENVOY_LIFETIME_SECONDS
			.with_label_values(&[conn.namespace_id.to_string().as_str(), &conn.pool_name])
			.observe(conn.connected_at.elapsed().as_secs_f64());

		// This will determine the close frame sent back to the envoy websocket
		lifecycle_res.map(|_| None)
	}
}
