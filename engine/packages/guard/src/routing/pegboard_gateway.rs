use std::time::Duration;

use anyhow::Result;
use gas::prelude::*;
use hyper::header::HeaderName;
use rivet_guard_core::proxy_service::{RouteConfig, RouteTarget, RoutingOutput, RoutingTimeout};

use super::{SEC_WEBSOCKET_PROTOCOL, WS_PROTOCOL_ACTOR, WS_PROTOCOL_TOKEN, X_RIVET_TOKEN};
use crate::{errors, shared_state::SharedState};

const ACTOR_FORCE_WAKE_PENDING_TIMEOUT: i64 = util::duration::seconds(60);
const ACTOR_READY_TIMEOUT: Duration = Duration::from_secs(10);

/// Time to wait before starting pool error checks
const RUNNER_POOL_ERROR_CHECK_DELAY: Duration = Duration::from_secs(1);
/// Interval between pool error checks
const RUNNER_POOL_ERROR_CHECK_INTERVAL: Duration = Duration::from_secs(2);

pub const X_RIVET_ACTOR: HeaderName = HeaderName::from_static("x-rivet-actor");

/// Route requests to actor services using path-based routing
#[tracing::instrument(skip_all)]
pub async fn route_request_path_based(
	ctx: &StandaloneCtx,
	shared_state: &SharedState,
	actor_id_str: &str,
	token: Option<&str>,
	original_path: &str,
	stripped_path: &str,
	_headers: &hyper::HeaderMap,
	_is_websocket: bool,
) -> Result<Option<RoutingOutput>> {
	// Parse actor ID
	let actor_id = Id::parse(actor_id_str).context("invalid actor id in path")?;

	route_request_inner(
		ctx,
		shared_state,
		actor_id,
		original_path,
		stripped_path,
		token,
	)
	.await
}

/// Route requests to actor services based on headers
#[tracing::instrument(skip_all)]
pub async fn route_request(
	ctx: &StandaloneCtx,
	shared_state: &SharedState,
	target: &str,
	_host: &str,
	path: &str,
	headers: &hyper::HeaderMap,
	is_websocket: bool,
) -> Result<Option<RoutingOutput>> {
	// Check target
	if target != "actor" {
		return Ok(None);
	}

	// Extract actor ID and token from WebSocket protocol or HTTP headers
	let (actor_id_str, token) = if is_websocket {
		// For WebSocket, parse the sec-websocket-protocol header
		let protocols_header = headers
			.get(SEC_WEBSOCKET_PROTOCOL)
			.and_then(|protocols| protocols.to_str().ok())
			.ok_or_else(|| {
				crate::errors::MissingHeader {
					header: "sec-websocket-protocol".to_string(),
				}
				.build()
			})?;

		let protocols: Vec<&str> = protocols_header.split(',').map(|p| p.trim()).collect();

		let actor_id_raw = protocols
			.iter()
			.find_map(|p| p.strip_prefix(WS_PROTOCOL_ACTOR))
			.ok_or_else(|| {
				crate::errors::MissingHeader {
					header: "`rivet_actor.*` protocol in sec-websocket-protocol".to_string(),
				}
				.build()
			})?;

		let actor_id = urlencoding::decode(actor_id_raw)
			.context("invalid url encoding in actor id")?
			.to_string();

		let token = protocols
			.iter()
			.find_map(|p| p.strip_prefix(WS_PROTOCOL_TOKEN));

		(actor_id, token)
	} else {
		// For HTTP, use headers
		let actor_id = headers
			.get(X_RIVET_ACTOR)
			.map(|x| x.to_str())
			.transpose()
			.context("invalid x-rivet-actor header")?
			.ok_or_else(|| {
				crate::errors::MissingHeader {
					header: X_RIVET_ACTOR.to_string(),
				}
				.build()
			})?;

		let token = headers
			.get(X_RIVET_TOKEN)
			.map(|x| x.to_str())
			.transpose()
			.context("invalid x-rivet-token header")?;

		(actor_id.to_string(), token)
	};

	// Find actor to route to
	let actor_id = Id::parse(&actor_id_str).context("invalid x-rivet-actor header")?;

	route_request_inner(ctx, shared_state, actor_id, path, path, token).await
}

async fn route_request_inner(
	ctx: &StandaloneCtx,
	shared_state: &SharedState,
	actor_id: Id,
	original_path: &str,
	stripped_path: &str,
	_token: Option<&str>,
) -> Result<Option<RoutingOutput>> {
	// NOTE: Token validation implemented in EE

	// Route to peer dc where the actor lives
	if actor_id.label() != ctx.config().dc_label() {
		tracing::debug!(peer_dc_label=?actor_id.label(), "re-routing actor to peer dc");

		let peer_dc = ctx
			.config()
			.dc_for_label(actor_id.label())
			.context("dc with the given label not found")?;

		return Ok(Some(RoutingOutput::Route(RouteConfig {
			targets: vec![RouteTarget {
				actor_id: Some(actor_id),
				host: peer_dc
					.proxy_url_host()
					.context("bad peer dc proxy url host")?
					.to_string(),
				port: peer_dc
					.proxy_url_port()
					.context("bad peer dc proxy url port")?,
				path: original_path.to_owned(),
			}],
			timeout: RoutingTimeout {
				routing_timeout: 10,
			},
		})));
	}

	// Create subs before checking if actor exists/is not destroyed
	let mut ready_sub = ctx
		.subscribe::<pegboard::workflows::actor::Ready>(("actor_id", actor_id))
		.await?;
	let mut stopped_sub = ctx
		.subscribe::<pegboard::workflows::actor::Stopped>(("actor_id", actor_id))
		.await?;
	let mut fail_sub = ctx
		.subscribe::<pegboard::workflows::actor::Failed>(("actor_id", actor_id))
		.await?;
	let mut destroy_sub = ctx
		.subscribe::<pegboard::workflows::actor::DestroyStarted>(("actor_id", actor_id))
		.await?;

	// Fetch actor info
	let Some(actor) = ctx
		.op(pegboard::ops::actor::get_for_gateway::Input { actor_id })
		.await?
	else {
		return Err(pegboard::errors::Actor::NotFound.build());
	};

	if actor.destroyed {
		return Err(pegboard::errors::Actor::NotFound.build());
	}

	// Wake actor if sleeping
	if actor.sleeping {
		tracing::debug!(?actor_id, "actor sleeping, waking");

		ctx.signal(pegboard::workflows::actor::Wake {
			allocation_override: pegboard::workflows::actor::AllocationOverride::DontSleep {
				pending_timeout: Some(ACTOR_FORCE_WAKE_PENDING_TIMEOUT),
			},
		})
		.to_workflow_id(actor.workflow_id)
		.send()
		.await?;
	}

	let runner_id = if let (Some(runner_id), true) = (actor.runner_id, actor.connectable) {
		runner_id
	} else {
		tracing::debug!(?actor_id, "waiting for actor to become ready");

		let mut wake_retries = 0;

		// Create pool error check future
		let pool_error_check_fut =
			check_runner_pool_error_loop(ctx, actor.namespace_id, &actor.runner_name_selector);
		tokio::pin!(pool_error_check_fut);

		// Wait for ready, fail, or destroy
		loop {
			tokio::select! {
				res = ready_sub.next() => break res?.runner_id,
				res = stopped_sub.next() => {
					res?;

					if wake_retries < 16 {
						tracing::debug!(?actor_id, ?wake_retries, "actor stopped while we were waiting for it to become ready, attempting rewake");
						wake_retries += 1;

						let res = ctx.signal(pegboard::workflows::actor::Wake {
							allocation_override: pegboard::workflows::actor::AllocationOverride::DontSleep {
								pending_timeout: Some(ACTOR_FORCE_WAKE_PENDING_TIMEOUT),
							},
						})
						.to_workflow_id(actor.workflow_id)
						.graceful_not_found()
						.send()
						.await?;

						if res.is_none() {
							tracing::warn!(
								?actor_id,
								"actor workflow not found for rewake"
							);
						}
					} else {
						tracing::warn!("actor retried waking 16 times, has not yet started");
						return Err(rivet_guard_core::errors::ServiceUnavailable.build());
					}
				}
				res = fail_sub.next() => {
					let msg = res?;
					return Err(msg.error.clone().build());
				}
				res = destroy_sub.next() => {
					res?;
					return Err(pegboard::errors::Actor::DestroyedWhileWaitingForReady.build());
				}
				res = &mut pool_error_check_fut => {
					if res? {
						return Err(errors::ActorRunnerFailed { actor_id }.build());
					}
				}
				// Ready timeout
				_ = tokio::time::sleep(ACTOR_READY_TIMEOUT) => {
					return Err(errors::ActorReadyTimeout { actor_id }.build());
				}
			}
		}
	};

	tracing::debug!(?actor_id, ?runner_id, "actor ready");

	// Return pegboard-gateway instance with path
	let gateway = pegboard_gateway::PegboardGateway::new(
		ctx.clone(),
		shared_state.pegboard_gateway.clone(),
		runner_id,
		actor_id,
		stripped_path.to_string(),
	);
	Ok(Some(RoutingOutput::CustomServe(std::sync::Arc::new(
		gateway,
	))))
}

/// Waits for initial delay, then periodically checks for runner pool errors.
///
/// Returns `true` if the pool has an active error, `false` otherwise.
///
/// This is used to short circuit waiting for the actor to schedule by checking if the underlying
/// pool is unhealthy. The initial delay is intended to give the actor time to allocate cleanly in
/// case the pool status is flapping.
async fn check_runner_pool_error_loop(
	ctx: &StandaloneCtx,
	namespace_id: Id,
	runner_name: &str,
) -> Result<bool> {
	tokio::time::sleep(RUNNER_POOL_ERROR_CHECK_DELAY).await;

	loop {
		let errors = ctx
			.op(pegboard::ops::runner_config::get_error::Input {
				runners: vec![(namespace_id, runner_name.to_string())],
			})
			.await?;

		if let Some(entry) = errors.into_iter().next() {
			tracing::debug!(?entry.error, "runner pool has active error, failing fast");
			return Ok(true);
		}

		// Wait before next check
		tokio::time::sleep(RUNNER_POOL_ERROR_CHECK_INTERVAL).await;
	}
}
