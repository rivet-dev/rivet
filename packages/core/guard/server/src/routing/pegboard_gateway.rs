use std::time::Duration;

use anyhow::Result;
use gas::prelude::*;
use hyper::header::HeaderName;
use rivet_guard_core::proxy_service::{RouteConfig, RouteTarget, RoutingOutput, RoutingTimeout};

use super::SEC_WEBSOCKET_PROTOCOL;
use crate::{errors, shared_state::SharedState};

const ACTOR_READY_TIMEOUT: Duration = Duration::from_secs(10);
pub const X_RIVET_ACTOR: HeaderName = HeaderName::from_static("x-rivet-actor");
const WS_PROTOCOL_ACTOR: &str = "rivet_actor.";

/// Route requests to actor services based on hostname and path
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

	// Extract actor ID from WebSocket protocol or HTTP header
	let actor_id_str = if is_websocket {
		// For WebSocket, parse the sec-websocket-protocol header
		headers
			.get(SEC_WEBSOCKET_PROTOCOL)
			.and_then(|protocols| protocols.to_str().ok())
			.and_then(|protocols| {
				// Parse protocols to find actor.{id}
				protocols
					.split(',')
					.map(|p| p.trim())
					.find_map(|p| p.strip_prefix(WS_PROTOCOL_ACTOR))
			})
			.ok_or_else(|| {
				crate::errors::MissingHeader {
					header: "`rivet_actor.*` protocol in sec-websocket-protocol".to_string(),
				}
				.build()
			})?
	} else {
		// For HTTP, use the x-rivet-actor header
		headers
			.get(X_RIVET_ACTOR)
			.and_then(|x| x.to_str().ok())
			.ok_or_else(|| {
				crate::errors::MissingHeader {
					header: X_RIVET_ACTOR.to_string(),
				}
				.build()
			})?
	};

	// Find actor to route to
	let actor_id = Id::parse(actor_id_str)?;

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
					.public_url
					.host()
					.context("peer dc public_url has no host")?
					.to_string(),
				port: peer_dc
					.public_url
					.port()
					.context("peer dc public_url has no port")?,
				path: path.to_owned(),
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
		return Err(errors::ActorNotFound { actor_id }.build());
	};

	if actor.destroyed {
		return Err(errors::ActorDestroyed { actor_id }.build());
	}

	// Wake actor if sleeping
	if actor.sleeping {
		ctx.signal(pegboard::workflows::actor::Wake {})
			.to_workflow_id(actor.workflow_id)
			.send()
			.await?;
	}

	let runner_id = if let (Some(runner_id), true) = (actor.runner_id, actor.connectable) {
		runner_id
	} else {
		tracing::debug!(?actor_id, "waiting for actor to become ready");

		// Wait for ready, fail, or destroy
		tokio::select! {
			res = ready_sub.next() => { res?.runner_id },
			res = fail_sub.next() => {
				let msg = res?;
				return Err(msg.error.clone().build());
			}
			res = destroy_sub.next() => {
				res?;
				return Err(pegboard::errors::Actor::DestroyedWhileWaitingForReady.build());
			}
			// Ready timeout
			_ = tokio::time::sleep(ACTOR_READY_TIMEOUT) => {
				return Err(errors::ActorReadyTimeout { actor_id }.build());
			}
		}
	};

	tracing::debug!(?actor_id, ?runner_id, "actor ready");

	// Return pegboard-gateway instance
	let gateway = pegboard_gateway::PegboardGateway::new(
		shared_state.pegboard_gateway.clone(),
		runner_id,
		actor_id,
	);
	Ok(Some(RoutingOutput::CustomServe(std::sync::Arc::new(
		gateway,
	))))
}
