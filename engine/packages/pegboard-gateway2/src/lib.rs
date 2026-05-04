use anyhow::{Result, anyhow};
use async_trait::async_trait;
use bytes::Bytes;
use gas::prelude::*;
use http_body_util::{BodyExt, Full};
use hyper::{Request, Response, StatusCode, body::Body};
use rivet_envoy_protocol as protocol;
use rivet_error::*;
use rivet_guard_core::{
	ResponseBody, WebSocketHandle,
	custom_serve::{CustomServeTrait, HibernationResult},
	errors::{ServiceUnavailable, WebSocketServiceUnavailable},
	request_context::RequestContext,
	utils::is_ws_hibernate,
};
use rivet_util::serde::HashableMap;
use std::{
	sync::{Arc, atomic::AtomicU64},
	time::Duration,
};
use tokio::sync::watch;
use tokio_tungstenite::tungstenite::protocol::frame::{CloseFrame, coding::CloseCode};
use universaldb::utils::IsolationLevel::*;

use crate::shared_state::{InFlightRequestCtx, SharedState};

mod hibernation_task;
mod keepalive_task;
mod metrics;
mod metrics_task;
mod ping_task;
pub mod shared_state;
mod tunnel_to_ws_task;
mod ws_to_tunnel_task;

#[derive(RivetError, Serialize, Deserialize)]
#[error(
	"guard",
	"websocket_pending_limit_reached",
	"Reached limit on pending websocket messages, aborting connection."
)]
pub struct WebsocketPendingLimitReached;

const UPDATE_METRICS_INTERVAL: Duration = Duration::from_secs(15);

#[derive(Debug)]
enum LifecycleResult {
	ServerClose(protocol::ToRivetWebSocketClose),
	ClientClose(Option<CloseFrame>),
	Aborted,
}

#[derive(Debug)]
enum HibernationLifecycleResult {
	Continue,
	Close,
	Aborted,
}

pub struct PegboardGateway2 {
	ctx: StandaloneCtx,
	shared_state: SharedState,
	namespace_id: Id,
	envoy_key: String,
	actor_id: Id,
	path: String,
}

impl PegboardGateway2 {
	#[tracing::instrument(skip_all, fields(?actor_id, ?namespace_id, %envoy_key, ?path))]
	pub fn new(
		ctx: StandaloneCtx,
		shared_state: SharedState,
		namespace_id: Id,
		envoy_key: String,
		actor_id: Id,
		path: String,
	) -> Self {
		Self {
			ctx,
			shared_state,
			namespace_id,
			envoy_key,
			actor_id,
			path,
		}
	}
}

impl PegboardGateway2 {
	async fn handle_request_inner(
		&self,
		ctx: &StandaloneCtx,
		req: Request<Full<Bytes>>,
		req_ctx: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
		// Use the actor ID from the gateway instance
		let actor_id = self.actor_id.to_string();
		let request_id = req_ctx.in_flight_request_id()?;

		// Extract request parts
		let headers = req
			.headers()
			.iter()
			.filter_map(|(name, value)| {
				value
					.to_str()
					.ok()
					.map(|value_str| (name.to_string(), value_str.to_string()))
			})
			.collect::<HashableMap<_, _>>();

		// NOTE: Size constraints have already been applied by guard
		let body_bytes = req
			.into_body()
			.collect()
			.await
			.context("failed to read body")?
			.to_bytes();

		let mut stopped_sub = ctx
			.subscribe::<pegboard::workflows::actor2::Stopped>(("actor_id", self.actor_id))
			.await?;

		// Build subject to publish to
		let tunnel_subject = pegboard::pubsub_subjects::EnvoyReceiverSubject::new(
			self.namespace_id,
			self.envoy_key.clone(),
		)
		.to_string();

		// Start listening for request responses
		let InFlightRequestCtx {
			mut msg_rx,
			mut drop_rx,
			handle: in_flight_req,
		} = self
			.shared_state
			.create_or_wake_in_flight_request(tunnel_subject, request_id, false)
			.await?;

		// Start request
		let message = protocol::ToEnvoyTunnelMessageKind::ToEnvoyRequestStart(
			protocol::ToEnvoyRequestStart {
				actor_id: actor_id.clone(),
				method: req_ctx.method().to_string(),
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
		in_flight_req.send_message(message).await?;

		// Wait for response
		tracing::debug!("gateway waiting for response from tunnel");
		let fut = async {
			loop {
				tokio::select! {
					res = msg_rx.recv() => {
						if let Some(msg) = res {
							match msg {
								protocol::ToRivetTunnelMessageKind::ToRivetResponseStart(
									response_start,
								) => {
									return anyhow::Ok(response_start);
								}
								protocol::ToRivetTunnelMessageKind::ToRivetResponseAbort => {
									tracing::warn!("request aborted");
									return Err(ServiceUnavailable.build());
								}
								_ => {
									tracing::warn!("received non-response message from pubsub");
								}
							}
						} else {
							tracing::warn!(
								request_id=%protocol::util::id_to_string(&request_id),
								"received no message response during request init",
							);
							break;
						}
					}
					_ = stopped_sub.next() => {
						tracing::debug!("actor stopped while waiting for request response");
						return Err(ServiceUnavailable.build());
					}
					_ = drop_rx.changed() => {
						tracing::warn!(reason=?drop_rx.borrow(), "tunnel message timeout");
						return Err(ServiceUnavailable.build());
					}
				}
			}

			Err(ServiceUnavailable.build())
		}
		.instrument(tracing::info_span!("wait_for_tunnel_response"));
		let response_start_timeout = Duration::from_millis(
			self.ctx
				.config()
				.pegboard()
				.gateway_response_start_timeout_ms(),
		);
		let response_start = tokio::time::timeout(response_start_timeout, fut)
			.await
			.map_err(|_| {
				tracing::warn!("timed out waiting for response start from envoy");

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

		// Add body
		let body = response_start.body.unwrap_or_default();
		let response = response_builder.body(ResponseBody::Full(Full::new(Bytes::from(body))))?;

		in_flight_req.stop().await;

		Ok(response)
	}

	async fn handle_websocket_inner(
		&self,
		ctx: &StandaloneCtx,
		req_ctx: &mut RequestContext,
		client_ws: WebSocketHandle,
		after_hibernation: bool,
	) -> Result<Option<CloseFrame>> {
		let request_id = req_ctx.in_flight_request_id()?;

		// Extract headers
		let mut request_headers = HashableMap::new();
		for (name, value) in req_ctx.headers() {
			if let Result::Ok(value_str) = value.to_str() {
				request_headers.insert(name.to_string(), value_str.to_string());
			}
		}

		let mut stopped_sub = ctx
			.subscribe::<pegboard::workflows::actor2::Stopped>(("actor_id", self.actor_id))
			.await?;

		// Build subject to publish to
		let tunnel_subject = pegboard::pubsub_subjects::EnvoyReceiverSubject::new(
			self.namespace_id,
			self.envoy_key.clone(),
		)
		.to_string();

		// Start listening for WebSocket messages
		let InFlightRequestCtx {
			mut msg_rx,
			mut drop_rx,
			handle: in_flight_req,
		} = self
			.shared_state
			.create_or_wake_in_flight_request(tunnel_subject.clone(), request_id, after_hibernation)
			.await?;

		// If we are reconnecting after hibernation, don't send an open message
		let can_hibernate = if after_hibernation {
			true
		} else {
			// Send WebSocket open message
			let open_message = protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketOpen(
				protocol::ToEnvoyWebSocketOpen {
					actor_id: self.actor_id.to_string(),
					path: self.path.clone(),
					headers: request_headers,
				},
			);

			in_flight_req.send_message(open_message).await?;

			tracing::debug!("gateway waiting for websocket open from tunnel");

			// Wait for WebSocket open acknowledgment
			let fut = async {
				loop {
					tokio::select! {
						res = msg_rx.recv() => {
							if let Some(msg) = res {
								match msg {
									protocol::ToRivetTunnelMessageKind::ToRivetWebSocketOpen(msg) => {
										return anyhow::Ok(msg);
									}
									protocol::ToRivetTunnelMessageKind::ToRivetWebSocketClose(close) => {
										tracing::warn!(?close, "websocket closed before opening");
										return Err(WebSocketServiceUnavailable.build());
									}
									_ => {
										tracing::warn!(
											"received unexpected message while waiting for websocket open"
										);
									}
								}
							} else {
								tracing::warn!(
									request_id=%protocol::util::id_to_string(&request_id),
									"received no message response during ws init",
								);
								break;
							}
						}
						_ = stopped_sub.next() => {
							tracing::debug!("actor stopped while waiting for websocket open");
							return Err(WebSocketServiceUnavailable.build());
						}
						_ = drop_rx.changed() => {
							tracing::warn!(reason=?drop_rx.borrow(), "websocket open timeout");
							return Err(WebSocketServiceUnavailable.build());
						}
					}
				}

				Err(WebSocketServiceUnavailable.build())
			};

			let websocket_open_timeout = Duration::from_millis(
				self.ctx
					.config()
					.pegboard()
					.gateway_websocket_open_timeout_ms(),
			);
			let open_msg = tokio::time::timeout(websocket_open_timeout, fut)
				.await
				.map_err(|_| {
					tracing::warn!("timed out waiting for websocket open from envoy");

					WebSocketServiceUnavailable.build()
				})??;

			in_flight_req
				.toggle_hibernatable(open_msg.can_hibernate)
				.await?;

			open_msg.can_hibernate
		};

		let ingress_bytes = Arc::new(AtomicU64::new(0));
		let egress_bytes = Arc::new(AtomicU64::new(0));

		// Send pending messages
		in_flight_req.resend_pending_websocket_messages().await?;

		let ws_rx = client_ws.recv();

		let (tunnel_to_ws_abort_tx, tunnel_to_ws_abort_rx) = watch::channel(());
		let (ws_to_tunnel_abort_tx, ws_to_tunnel_abort_rx) = watch::channel(());
		let (ping_abort_tx, ping_abort_rx) = watch::channel(());
		let (keepalive_abort_tx, keepalive_abort_rx) = watch::channel(());
		let (metrics_abort_tx, metrics_abort_rx) = watch::channel(());

		let tunnel_to_ws = tokio::spawn(tunnel_to_ws_task::task(
			in_flight_req.clone(),
			client_ws,
			stopped_sub,
			msg_rx,
			drop_rx,
			can_hibernate,
			egress_bytes.clone(),
			tunnel_to_ws_abort_rx,
		));
		let ws_to_tunnel = tokio::spawn(ws_to_tunnel_task::task(
			in_flight_req.clone(),
			ws_rx,
			ingress_bytes.clone(),
			ws_to_tunnel_abort_rx,
		));
		let update_ping_interval = Duration::from_millis(
			self.ctx
				.config()
				.pegboard()
				.gateway_update_ping_interval_ms(),
		);
		let ping = tokio::spawn(ping_task::task(
			in_flight_req.clone(),
			ping_abort_rx,
			update_ping_interval,
		));
		let metrics = tokio::spawn(metrics_task::task(
			ctx.clone(),
			self.actor_id,
			self.namespace_id,
			ingress_bytes,
			egress_bytes,
			metrics_abort_rx,
		));
		let keepalive = if can_hibernate {
			Some(tokio::spawn(keepalive_task::task(
				in_flight_req.clone(),
				ctx.clone(),
				self.actor_id,
				self.shared_state.gateway_id(),
				request_id,
				keepalive_abort_rx,
			)))
		} else {
			None
		};

		// Wait for all tasks to complete
		let (tunnel_to_ws_res, ws_to_tunnel_res, ping_res, metrics_res, keepalive_res) = tokio::join!(
			async {
				let res = tunnel_to_ws.await?;

				// Abort other if not aborted
				if !matches!(res, Ok(LifecycleResult::Aborted)) {
					tracing::debug!(?res, "tunnel to ws task completed, aborting others");

					let _ = ping_abort_tx.send(());
					let _ = ws_to_tunnel_abort_tx.send(());
					let _ = keepalive_abort_tx.send(());
					let _ = metrics_abort_tx.send(());
				} else {
					tracing::debug!(?res, "tunnel to ws task completed");
				}

				res
			},
			async {
				let res = ws_to_tunnel.await?;

				// Abort other if not aborted
				if !matches!(res, Ok(LifecycleResult::Aborted)) {
					tracing::debug!(?res, "ws to tunnel task completed, aborting others");

					let _ = ping_abort_tx.send(());
					let _ = tunnel_to_ws_abort_tx.send(());
					let _ = keepalive_abort_tx.send(());
					let _ = metrics_abort_tx.send(());
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
					let _ = keepalive_abort_tx.send(());
					let _ = metrics_abort_tx.send(());
				} else {
					tracing::debug!(?res, "ping task completed");
				}

				res
			},
			async {
				let res = metrics.await?;

				// Abort others if not aborted
				if !matches!(res, Ok(LifecycleResult::Aborted)) {
					tracing::debug!(?res, "metrics task completed, aborting others");

					let _ = ws_to_tunnel_abort_tx.send(());
					let _ = tunnel_to_ws_abort_tx.send(());
					let _ = ping_abort_tx.send(());
					let _ = keepalive_abort_tx.send(());
				} else {
					tracing::debug!(?res, "metrics task completed");
				}

				res
			},
			async {
				let Some(keepalive) = keepalive else {
					return Ok(LifecycleResult::Aborted);
				};

				let res = keepalive.await?;

				// Abort others if not aborted
				if !matches!(res, Ok(LifecycleResult::Aborted)) {
					tracing::debug!(?res, "keepalive task completed, aborting others");

					let _ = ws_to_tunnel_abort_tx.send(());
					let _ = tunnel_to_ws_abort_tx.send(());
					let _ = ping_abort_tx.send(());
					let _ = metrics_abort_tx.send(());
				} else {
					tracing::debug!(?res, "keepalive task completed");
				}

				res
			},
		);

		// Determine single result from all tasks
		let mut lifecycle_res = match (
			tunnel_to_ws_res,
			ws_to_tunnel_res,
			ping_res,
			metrics_res,
			keepalive_res,
		) {
			// Prefer error
			(Err(err), _, _, _, _) => Err(err),
			(_, Err(err), _, _, _) => Err(err),
			(_, _, Err(err), _, _) => Err(err),
			(_, _, _, Err(err), _) => Err(err),
			(_, _, _, _, Err(err)) => Err(err),
			// Prefer non aborted result if all succeed
			(Ok(res), Ok(LifecycleResult::Aborted), _, _, _) => Ok(res),
			(Ok(LifecycleResult::Aborted), Ok(res), _, _, _) => Ok(res),
			// Unlikely case
			(res, _, _, _, _) => res,
		};

		// Send close frame to envoy if not hibernating
		if !&lifecycle_res
			.as_ref()
			.map_or_else(is_ws_hibernate, |_| false)
		{
			let (close_code, close_reason) = match &mut lifecycle_res {
				// Taking here because it won't be used again
				Ok(LifecycleResult::ClientClose(Some(close))) => {
					(close.code, Some(std::mem::take(&mut close.reason)))
				}
				Ok(_) => (CloseCode::Normal.into(), None),
				Err(_) => (CloseCode::Error.into(), Some("ws.downstream_closed".into())),
			};
			let close_message = protocol::ToEnvoyTunnelMessageKind::ToEnvoyWebSocketClose(
				protocol::ToEnvoyWebSocketClose {
					code: Some(close_code.into()),
					reason: close_reason.map(|x| x.as_str().to_string()),
				},
			);

			if let Err(err) = in_flight_req.send_message(close_message).await {
				tracing::error!(?err, "error sending close message");
			}

			in_flight_req.stop().await;
		} else {
			in_flight_req.start_hibernation().await?;
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

#[async_trait]
impl CustomServeTrait for PegboardGateway2 {
	#[tracing::instrument(skip_all, fields(actor_id=?self.actor_id, namespace_id=?self.namespace_id, envoy_key=%self.envoy_key))]
	async fn handle_request(
		&self,
		req: Request<Full<Bytes>>,
		req_ctx: &mut RequestContext,
	) -> Result<Response<ResponseBody>> {
		let ctx = self.ctx.with_ray(req_ctx.ray_id(), req_ctx.req_id())?;
		let req_body_size_hint = req.body().size_hint();

		let (res, metrics_res) = tokio::join!(
			self.handle_request_inner(&ctx, req, req_ctx),
			record_req_metrics(
				&ctx,
				self.actor_id,
				self.namespace_id,
				Metric::HttpIngress(
					req_body_size_hint
						.upper()
						.unwrap_or(req_body_size_hint.lower()) as usize
				),
			),
		);

		let response_size = match &res {
			Ok(res) => res.size_hint().upper().unwrap_or(res.size_hint().lower()),
			Err(_) => 0,
		};

		if let Err(err) = metrics_res {
			tracing::error!(?err, "http req ingress metrics failed");
		} else {
			let actor_id = self.actor_id;
			let namespace_id = self.namespace_id;
			let envoy_key = self.envoy_key.clone();
			tokio::spawn(async move {
				if let Err(err) = record_req_metrics(
					&ctx,
					actor_id,
					namespace_id,
					Metric::HttpEgress(response_size as usize),
				)
				.await
				{
					tracing::error!(
						?err,
						?namespace_id,
						%envoy_key,
						"http req egress metrics failed, likely corrupt now",
					);
				}
			});
		}

		res
	}

	#[tracing::instrument(skip_all, fields(actor_id=?self.actor_id, namespace_id=?self.namespace_id, envoy_key=%self.envoy_key))]
	async fn handle_websocket(
		&self,
		req_ctx: &mut RequestContext,
		client_ws: WebSocketHandle,
		after_hibernation: bool,
	) -> Result<Option<CloseFrame>> {
		let ctx = self.ctx.with_ray(req_ctx.ray_id(), req_ctx.req_id())?;
		let (res, metrics_res) = tokio::join!(
			self.handle_websocket_inner(&ctx, req_ctx, client_ws, after_hibernation),
			record_req_metrics(
				&ctx,
				self.actor_id,
				self.namespace_id,
				Metric::WebsocketOpen
			),
		);

		if let Err(err) = metrics_res {
			tracing::error!(?err, "ws open metrics failed");
		} else {
			let actor_id = self.actor_id;
			let namespace_id = self.namespace_id;
			let envoy_key = self.envoy_key.clone();
			tokio::spawn(async move {
				if let Err(err) =
					record_req_metrics(&ctx, actor_id, namespace_id, Metric::WebsocketClose).await
				{
					tracing::error!(
						?err,
						?namespace_id,
						%envoy_key,
						"ws close metrics failed, likely corrupt now",
					);
				}
			});
		}

		res
	}

	#[tracing::instrument(skip_all, fields(actor_id=?self.actor_id))]
	async fn handle_websocket_hibernation(
		&self,
		req_ctx: &mut RequestContext,
		client_ws: WebSocketHandle,
	) -> Result<HibernationResult> {
		let ctx = self.ctx.with_ray(req_ctx.ray_id(), req_ctx.req_id())?;
		let request_id = req_ctx.in_flight_request_id()?;
		let InFlightRequestCtx {
			msg_rx,
			drop_rx,
			handle: in_flight_req,
		} = self
			.shared_state
			.get_hibernating_in_flight_request(request_id)
			.await?;

		// Immediately rewake if we have pending ws messages from the client
		if in_flight_req.has_pending_websocket_messages().await? {
			tracing::debug!("exiting hibernation due to pending ws messages");

			return Ok(HibernationResult::Continue);
		}

		// Unused during hibernation
		let ingress_bytes = Arc::new(AtomicU64::new(0));
		let egress_bytes = Arc::new(AtomicU64::new(0));

		let (hibernation_abort_tx, hibernation_abort_rx) = watch::channel(());
		let (keepalive_abort_tx, keepalive_abort_rx) = watch::channel(());
		let (metrics_abort_tx, metrics_abort_rx) = watch::channel(());

		let hibernation = tokio::spawn(hibernation_task::task(
			client_ws,
			in_flight_req.clone(),
			ctx.clone(),
			self.actor_id,
			msg_rx,
			drop_rx,
			egress_bytes.clone(),
			hibernation_abort_rx,
		));
		let metrics = tokio::spawn(metrics_task::task(
			ctx.clone(),
			self.actor_id,
			self.namespace_id,
			ingress_bytes,
			egress_bytes,
			metrics_abort_rx,
		));
		let keepalive = tokio::spawn(keepalive_task::task(
			in_flight_req.clone(),
			ctx.clone(),
			self.actor_id,
			self.shared_state.gateway_id(),
			request_id,
			keepalive_abort_rx,
		));

		// Wait for all tasks to complete or ws msg from client
		let (record_start_res, hibernation_res, metrics_res, keepalive_res) = tokio::join!(
			record_req_metrics(
				&ctx,
				self.actor_id,
				self.namespace_id,
				Metric::WebsocketHibernate
			),
			async {
				let res = hibernation.await?;

				// Abort other if not aborted
				if !matches!(res, Ok(HibernationLifecycleResult::Aborted)) {
					tracing::debug!(?res, "hibernation completed, aborting others");

					let _ = keepalive_abort_tx.send(());
					let _ = metrics_abort_tx.send(());
				} else {
					tracing::debug!(?res, "hibernation completed");
				}

				res
			},
			async {
				let res = metrics.await?;

				// Abort others if not aborted
				if !matches!(res, Ok(LifecycleResult::Aborted)) {
					tracing::debug!(?res, "metrics task completed, aborting others");

					let _ = hibernation_abort_tx.send(());
					let _ = keepalive_abort_tx.send(());
				} else {
					tracing::debug!(?res, "metrics task completed");
				}

				res
			},
			async {
				let res = keepalive.await?;

				// Abort others if not aborted
				if !matches!(res, Ok(LifecycleResult::Aborted)) {
					tracing::debug!(?res, "keepalive task completed, aborting others");

					let _ = hibernation_abort_tx.send(());
					let _ = metrics_abort_tx.send(());
				} else {
					tracing::debug!(?res, "keepalive task completed");
				}

				res
			},
		);

		// Determine single result from all tasks
		let res = match (hibernation_res, metrics_res, keepalive_res) {
			// Prefer error
			(Err(err), _, _) => Err(err),
			(_, Err(err), _) => Err(err),
			(_, _, Err(err)) => Err(err),
			// Prefer hibernation result
			(Ok(res), _, _) => match res {
				HibernationLifecycleResult::Continue => Ok(HibernationResult::Continue),
				HibernationLifecycleResult::Close => Ok(HibernationResult::Close),
				// Should be unreachable
				HibernationLifecycleResult::Aborted => Err(anyhow!("hibernation aborted")),
			},
		};

		let (delete_res, _) = tokio::join!(
			async {
				match &res {
					Ok(HibernationResult::Continue) => {}
					Ok(HibernationResult::Close) | Err(_) => {
						in_flight_req.stop().await;

						// No longer an active hibernating request, delete entry
						ctx.op(pegboard::ops::actor::hibernating_request::delete::Input {
							actor_id: self.actor_id,
							gateway_id: self.shared_state.gateway_id(),
							request_id,
						})
						.await?;
					}
				}

				anyhow::Ok(())
			},
			async {
				if let Err(err) = record_start_res {
					tracing::error!(?err, "ws start hibernate metrics failed");
				} else {
					if let Err(err) = record_req_metrics(
						&ctx,
						self.actor_id,
						self.namespace_id,
						Metric::WebsocketStopHibernate,
					)
					.await
					{
						tracing::error!(
							?err,
							namespace_id=?self.namespace_id,
							envoy_key=%self.envoy_key,
							"ws stop hibernate metrics failed, likely corrupt now"
						);
					}
				}
			},
		);

		delete_res?;

		res
	}
}

#[derive(Debug)]
enum Metric {
	HttpIngress(usize),
	HttpEgress(usize),
	WebsocketOpen,
	// Ingress, Egress
	WebsocketTransfer(usize, usize),
	WebsocketClose,
	WebsocketHibernate,
	WebsocketStopHibernate,
}

#[tracing::instrument(skip_all, fields(?actor_id, ?metric))]
async fn record_req_metrics(
	ctx: &StandaloneCtx,
	actor_id: Id,
	namespace_id: Id,
	metric: Metric,
) -> Result<()> {
	let metric = &metric;
	ctx.udb()?
		.run(|tx| async move {
			let tx = tx.with_subspace(pegboard::keys::subspace());

			let actor_name = tx
				.read(&pegboard::keys::actor::NameKey::new(actor_id), Serializable)
				.await?;

			metric_inc(&tx, namespace_id, &actor_name, metric);

			Ok(())
		})
		.instrument(tracing::info_span!("record_req_metrics_tx"))
		.await?;

	Ok(())
}

fn metric_inc(tx: &universaldb::Transaction, namespace_id: Id, name: &str, metric: &Metric) {
	match metric {
		Metric::HttpIngress(size) => {
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::GatewayIngress(
					name.to_string(),
					"http".to_string(),
				),
				(*size).try_into().unwrap_or_default(),
			);
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::Requests(name.to_string(), "http".to_string()),
				1,
			);
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::ActiveRequests(
					name.to_string(),
					"http".to_string(),
				),
				1,
			);
		}
		Metric::HttpEgress(size) => {
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::GatewayEgress(
					name.to_string(),
					"http".to_string(),
				),
				(*size).try_into().unwrap_or_default(),
			);
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::ActiveRequests(
					name.to_string(),
					"http".to_string(),
				),
				-1,
			);
		}
		Metric::WebsocketOpen => {
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::Requests(name.to_string(), "ws".to_string()),
				1,
			);
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::ActiveRequests(name.to_string(), "ws".to_string()),
				1,
			);
		}
		Metric::WebsocketTransfer(ingress_size, egress_size) => {
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::GatewayIngress(name.to_string(), "ws".to_string()),
				(*ingress_size).try_into().unwrap_or_default(),
			);
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::GatewayEgress(name.to_string(), "ws".to_string()),
				(*egress_size).try_into().unwrap_or_default(),
			);
		}
		Metric::WebsocketClose => {
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::ActiveRequests(name.to_string(), "ws".to_string()),
				-1,
			);
		}
		Metric::WebsocketHibernate => {
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::Requests(name.to_string(), "hws".to_string()),
				1,
			);
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::ActiveRequests(
					name.to_string(),
					"hws".to_string(),
				),
				1,
			);
		}
		Metric::WebsocketStopHibernate => {
			namespace::keys::metric::inc(
				&tx.with_subspace(namespace::keys::subspace()),
				namespace_id,
				namespace::keys::metric::Metric::ActiveRequests(
					name.to_string(),
					"hws".to_string(),
				),
				-1,
			);
		}
	}
}
