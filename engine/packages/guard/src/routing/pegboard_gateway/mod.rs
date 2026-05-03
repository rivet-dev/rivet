mod cors;
mod resolve_actor_query;

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use gas::{ctx::message::SubscriptionHandle, prelude::*};
use hyper::header::HeaderName;
use rivet_guard_core::{RouteConfig, RouteTarget, RoutingOutput, request_context::RequestContext};

use super::{
	SEC_WEBSOCKET_PROTOCOL, WS_PROTOCOL_ACTOR, WS_PROTOCOL_BYPASS_CONNECTABLE, WS_PROTOCOL_TOKEN,
	X_RIVET_BYPASS_CONNECTABLE, X_RIVET_TOKEN, actor_path::ParsedActorPath,
};
use crate::{
	errors,
	routing::{
		actor_path::parse_actor_path,
		pegboard_gateway::resolve_actor_query::ResolveQueryActorResult,
	},
	shared_state::SharedState,
};
use cors::{CorsPreflight, set_non_preflight_cors};
use resolve_actor_query::resolve_query;

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
	req_ctx: &mut RequestContext,
) -> Result<Option<RoutingOutput>> {
	let res = route_request_path_based_inner(ctx, shared_state, req_ctx).await;

	match &res {
		Ok(Some(_)) | Err(_) => {
			// Attach CORS headers to the actual (non-OPTIONS) response so both the
			// actor response and any early error are readable by the browser.
			set_non_preflight_cors(req_ctx);
		}
		_ => {}
	}

	res
}

pub async fn route_request_path_based_inner(
	ctx: &StandaloneCtx,
	shared_state: &SharedState,
	req_ctx: &mut RequestContext,
) -> Result<Option<RoutingOutput>> {
	let Some(actor_path) = parse_actor_path(req_ctx.path())? else {
		return Ok(None);
	};

	if req_ctx.method() == hyper::Method::OPTIONS {
		return Ok(Some(RoutingOutput::CustomServe(Arc::new(CorsPreflight))));
	}

	tracing::debug!(?actor_path, "routing using path-based actor routing");

	let (actor_id, token, stripped_path, bypass_connectable) = match actor_path {
		ParsedActorPath::Direct(path) => (
			Id::parse(&path.actor_id).context("invalid actor id in path")?,
			read_gateway_token_for_path_based(req_ctx, path.token.as_deref())?
				.map(ToOwned::to_owned),
			path.stripped_path.clone(),
			// TODO:
			false,
		),
		ParsedActorPath::Query(path) => match resolve_query(ctx, &path.query).await? {
			ResolveQueryActorResult::Found { actor_id } => (
				actor_id,
				read_gateway_token_for_path_based(req_ctx, path.token.as_deref())?
					.map(ToOwned::to_owned),
				path.stripped_path.clone(),
				path.query.bypass_connectable(),
			),
			ResolveQueryActorResult::Forward { dc_label } => {
				let peer_dc = ctx
					.config()
					.dc_for_label(dc_label)
					.ok_or_else(|| rivet_api_util::errors::Datacenter::NotFound.build())?;

				return Ok(Some(RoutingOutput::Route(RouteConfig {
					targets: vec![RouteTarget {
						host: peer_dc
							.proxy_url_host()
							.context("bad peer dc proxy url host")?
							.to_string(),
						port: peer_dc
							.proxy_url_port()
							.context("bad peer dc proxy url port")?,
						path: req_ctx.path().to_owned(),
					}],
				})));
			}
		},
	};

	route_request_inner(
		ctx,
		shared_state,
		req_ctx,
		actor_id,
		&stripped_path,
		token.as_deref(),
		bypass_connectable,
	)
	.await
	.map(Some)
}

/// Route requests to actor services based on headers
#[tracing::instrument(skip_all)]
pub async fn route_request(
	ctx: &StandaloneCtx,
	shared_state: &SharedState,
	req_ctx: &mut RequestContext,
	target: &str,
) -> Result<Option<RoutingOutput>> {
	// Check target
	if target != "actor" {
		return Ok(None);
	}

	if req_ctx.method() == hyper::Method::OPTIONS {
		return Ok(Some(RoutingOutput::CustomServe(Arc::new(CorsPreflight))));
	}

	if !req_ctx.is_websocket() && !is_actor_http_request_path(req_ctx.path()) {
		return Ok(None);
	}

	// Attach CORS headers to the actual (non-OPTIONS) response so both the
	// actor response and any early error are readable by the browser.
	set_non_preflight_cors(req_ctx);

	// Extract actor ID and token from WebSocket protocol or HTTP headers
	let (actor_id_str, token, bypass_connectable) = if req_ctx.is_websocket() {
		// For WebSocket, parse the sec-websocket-protocol header
		let protocols_header = req_ctx
			.headers()
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
			.find_map(|p| p.strip_prefix(WS_PROTOCOL_TOKEN))
			.map(ToOwned::to_owned);

		let bypass_connectable = protocols
			.iter()
			.any(|p| p == &WS_PROTOCOL_BYPASS_CONNECTABLE);

		(actor_id, token, bypass_connectable)
	} else {
		// For HTTP, use headers
		let actor_id = req_ctx
			.headers()
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

		let token = req_ctx
			.headers()
			.get(X_RIVET_TOKEN)
			.map(|x| x.to_str())
			.transpose()
			.context("invalid x-rivet-token header")?
			.map(ToOwned::to_owned);

		let bypass_connectable = req_ctx.headers().contains_key(X_RIVET_BYPASS_CONNECTABLE);

		(actor_id.to_string(), token, bypass_connectable)
	};

	// Find actor to route to
	let actor_id = Id::parse(&actor_id_str).context("invalid x-rivet-actor header")?;
	let stripped_path = req_ctx.path().to_owned();

	route_request_inner(
		ctx,
		shared_state,
		req_ctx,
		actor_id,
		&stripped_path,
		token.as_deref(),
		bypass_connectable,
	)
	.await
	.map(Some)
}

fn is_actor_http_request_path(path: &str) -> bool {
	let Some(stripped) = path.strip_prefix("/request") else {
		return false;
	};

	stripped.is_empty() || matches!(stripped.as_bytes().first(), Some(b'/') | Some(b'?'))
}

async fn route_request_inner(
	ctx: &StandaloneCtx,
	shared_state: &SharedState,
	req_ctx: &mut RequestContext,
	actor_id: Id,
	stripped_path: &str,
	_token: Option<&str>,
	bypass_connectable: bool,
) -> Result<RoutingOutput> {
	// NOTE: Token validation implemented in EE

	// Route to peer dc where the actor lives
	if actor_id.label() != ctx.config().dc_label() {
		tracing::debug!(peer_dc_label=?actor_id.label(), "re-routing actor to peer dc");

		let peer_dc = ctx
			.config()
			.dc_for_label(actor_id.label())
			.ok_or_else(|| rivet_api_util::errors::Datacenter::NotFound.build())?;

		return Ok(RoutingOutput::Route(RouteConfig {
			targets: vec![RouteTarget {
				host: peer_dc
					.proxy_url_host()
					.context("bad peer dc proxy url host")?
					.to_string(),
				port: peer_dc
					.proxy_url_port()
					.context("bad peer dc proxy url port")?,
				path: req_ctx.path().to_owned(),
			}],
		}));
	}

	// Create subs before checking if actor exists/is not destroyed
	let (
		ready_sub,
		stopped_sub,
		fail_sub,
		destroy_sub,
		migrate_sub,
		ready_sub2,
		stopped_sub2,
		fail_sub2,
		destroy_sub2,
	) = tokio::try_join!(
		ctx.subscribe::<pegboard::workflows::actor::Ready>(("actor_id", actor_id)),
		ctx.subscribe::<pegboard::workflows::actor::Stopped>(("actor_id", actor_id)),
		ctx.subscribe::<pegboard::workflows::actor::Failed>(("actor_id", actor_id)),
		ctx.subscribe::<pegboard::workflows::actor::DestroyStarted>(("actor_id", actor_id)),
		ctx.subscribe::<pegboard::workflows::actor::MigratedToV2>(("actor_id", actor_id)),
		ctx.subscribe::<pegboard::workflows::actor2::Ready>(("actor_id", actor_id)),
		ctx.subscribe::<pegboard::workflows::actor2::Stopped>(("actor_id", actor_id)),
		ctx.subscribe::<pegboard::workflows::actor2::Failed>(("actor_id", actor_id)),
		ctx.subscribe::<pegboard::workflows::actor2::DestroyStarted>(("actor_id", actor_id)),
	)?;

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

	match actor.version {
		2 => {
			drop(ready_sub);
			drop(stopped_sub);
			drop(fail_sub);
			drop(destroy_sub);
			drop(migrate_sub);

			handle_actor_v2(
				ctx,
				shared_state,
				actor_id,
				actor,
				stripped_path,
				bypass_connectable,
				ready_sub2,
				stopped_sub2,
				fail_sub2,
				destroy_sub2,
			)
			.await
		}
		1 => {
			handle_actor_v1(
				ctx,
				shared_state,
				actor_id,
				actor,
				stripped_path,
				bypass_connectable,
				ready_sub,
				stopped_sub,
				fail_sub,
				destroy_sub,
				migrate_sub,
				ready_sub2,
				stopped_sub2,
				fail_sub2,
				destroy_sub2,
			)
			.await
		}
		_ => bail!("unknown actor version"),
	}
}

async fn handle_actor_v2(
	ctx: &StandaloneCtx,
	shared_state: &SharedState,
	actor_id: Id,
	actor: pegboard::ops::actor::get_for_gateway::Output,
	stripped_path: &str,
	bypass_connectable: bool,
	mut ready_sub: SubscriptionHandle<pegboard::workflows::actor2::Ready>,
	mut stopped_sub: SubscriptionHandle<pegboard::workflows::actor2::Stopped>,
	mut fail_sub: SubscriptionHandle<pegboard::workflows::actor2::Failed>,
	mut destroy_sub: SubscriptionHandle<pegboard::workflows::actor2::DestroyStarted>,
) -> Result<RoutingOutput> {
	// Wake actor if sleeping
	if actor.sleeping {
		tracing::debug!(?actor_id, "actor sleeping, waking");

		ctx.signal(pegboard::workflows::actor2::Wake {})
			.to_workflow_id(actor.workflow_id)
			.send()
			.await?;
	}

	let envoy_key = if let (Some(envoy_key), true) =
		(actor.envoy_key, actor.connectable || bypass_connectable)
	{
		envoy_key
	} else {
		tracing::debug!(?actor_id, "waiting for actor to become ready");

		let mut wake_retries = 0;

		// Create pool error check future
		let pool_error_check_fut = check_runner_pool_error_loop(
			ctx,
			actor.namespace_id,
			actor.runner_name_selector.as_deref(),
		);
		tokio::pin!(pool_error_check_fut);

		// Wait for ready, fail, or destroy
		loop {
			tokio::select! {
				res = ready_sub.next() => break res?.into_body().envoy_key,
				res = stopped_sub.next() => {
					res?;

					if wake_retries < 8 {
						tracing::debug!(?actor_id, ?wake_retries, "actor stopped while we were waiting for it to become ready, attempting rewake");
						wake_retries += 1;

						let res = ctx.signal(pegboard::workflows::actor2::Wake {})
						.to_workflow_id(actor.workflow_id)
						.graceful_not_found()
						.send()
						.await?;

						if res.is_none() {
							tracing::warn!(
								?actor_id,
								"actor workflow not found for rewake"
							);
							return Err(pegboard::errors::Actor::NotFound.build());
						}
					} else {
						tracing::warn!("actor retried waking 8 times, has not yet started");
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
				_ = tokio::time::sleep(ctx.config().guard().actor_ready_timeout()) => {
					return Err(errors::ActorReadyTimeout { actor_id }.build());
				}
			}
		}
	};

	tracing::debug!(?actor_id, %envoy_key, "actor ready");

	// Return pegboard-gateway2 instance with path
	let gateway = pegboard_gateway2::PegboardGateway2::new(
		ctx.clone(),
		shared_state.pegboard_gateway2.clone(),
		actor.namespace_id,
		envoy_key,
		actor_id,
		stripped_path.to_string(),
	);
	Ok(RoutingOutput::CustomServe(std::sync::Arc::new(gateway)))
}

async fn handle_actor_v1(
	ctx: &StandaloneCtx,
	shared_state: &SharedState,
	actor_id: Id,
	actor: pegboard::ops::actor::get_for_gateway::Output,
	stripped_path: &str,
	bypass_connectable: bool,
	mut ready_sub: SubscriptionHandle<pegboard::workflows::actor::Ready>,
	mut stopped_sub: SubscriptionHandle<pegboard::workflows::actor::Stopped>,
	mut fail_sub: SubscriptionHandle<pegboard::workflows::actor::Failed>,
	mut destroy_sub: SubscriptionHandle<pegboard::workflows::actor::DestroyStarted>,
	mut migrate_sub: SubscriptionHandle<pegboard::workflows::actor::MigratedToV2>,
	ready_sub2: SubscriptionHandle<pegboard::workflows::actor2::Ready>,
	stopped_sub2: SubscriptionHandle<pegboard::workflows::actor2::Stopped>,
	fail_sub2: SubscriptionHandle<pegboard::workflows::actor2::Failed>,
	destroy_sub2: SubscriptionHandle<pegboard::workflows::actor2::DestroyStarted>,
) -> Result<RoutingOutput> {
	// Wake actor if sleeping
	if actor.sleeping {
		tracing::debug!(?actor_id, "actor sleeping, waking");

		ctx.signal(pegboard::workflows::actor::Wake {
			allocation_override: pegboard::workflows::actor::AllocationOverride::DontSleep {
				pending_timeout: Some(ctx.config().guard().actor_force_wake_pending_timeout()),
			},
		})
		.to_workflow_id(actor.workflow_id)
		.send()
		.await?;
	}

	let runner_id = if let (Some(runner_id), true) =
		(actor.runner_id, actor.connectable || bypass_connectable)
	{
		runner_id
	} else {
		tracing::debug!(?actor_id, "waiting for actor to become ready");

		let mut wake_retries = 0;

		// Create pool error check future
		let runner_name_selector = actor.runner_name_selector.clone();
		let pool_error_check_fut =
			check_runner_pool_error_loop(ctx, actor.namespace_id, runner_name_selector.as_deref());
		tokio::pin!(pool_error_check_fut);

		// Wait for ready, fail, or destroy
		loop {
			tokio::select! {
				res = ready_sub.next() => break res?.runner_id,
				res = stopped_sub.next() => {
					res?;

					if wake_retries < 8 {
						tracing::debug!(?actor_id, ?wake_retries, "actor stopped while we were waiting for it to become ready, attempting rewake");
						wake_retries += 1;

						let res = ctx.signal(pegboard::workflows::actor::Wake {
							allocation_override: pegboard::workflows::actor::AllocationOverride::DontSleep {
								pending_timeout: Some(
									ctx.config().guard().actor_force_wake_pending_timeout(),
								),
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
							return Err(pegboard::errors::Actor::NotFound.build());
						}
					} else {
						tracing::warn!("actor retried waking 8 times, has not yet started");
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
				res = migrate_sub.next() => {
					res?;
					return handle_actor_v2(
						ctx,
						shared_state,
						actor_id,
						actor,
						stripped_path,
						bypass_connectable,
						ready_sub2,
						stopped_sub2,
						fail_sub2,
						destroy_sub2,
					).await;
				}
				res = &mut pool_error_check_fut => {
					if res? {
						return Err(errors::ActorRunnerFailed { actor_id }.build());
					}
				}
				// Ready timeout
				_ = tokio::time::sleep(ctx.config().guard().actor_ready_timeout()) => {
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
	Ok(RoutingOutput::CustomServe(std::sync::Arc::new(gateway)))
}

fn read_gateway_token_for_path_based<'a>(
	req_ctx: &'a RequestContext,
	token_from_path: Option<&'a str>,
) -> Result<Option<&'a str>> {
	if let Some(token) = token_from_path {
		return Ok(Some(token));
	}

	if req_ctx.is_websocket() {
		let protocols_header = req_ctx
			.headers()
			.get(SEC_WEBSOCKET_PROTOCOL)
			.and_then(|protocols| protocols.to_str().ok())
			.ok_or_else(|| {
				crate::errors::MissingHeader {
					header: "sec-websocket-protocol".to_string(),
				}
				.build()
			})?;

		let protocols = protocols_header
			.split(',')
			.map(|p| p.trim())
			.collect::<Vec<&str>>();

		Ok(protocols
			.iter()
			.find_map(|p| p.strip_prefix(WS_PROTOCOL_TOKEN)))
	} else {
		req_ctx
			.headers()
			.get(X_RIVET_TOKEN)
			.map(|x| x.to_str())
			.transpose()
			.context("invalid x-rivet-token header")
	}
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
	runner_name: Option<&str>,
) -> Result<bool> {
	// Skip pool error check for actors that have not backfilled yet
	let Some(runner_name) = runner_name else {
		std::future::pending::<()>().await;
		unreachable!()
	};

	tokio::time::sleep(RUNNER_POOL_ERROR_CHECK_DELAY).await;

	loop {
		let errors = ctx
			.op(pegboard::ops::runner_config::get_error::Input {
				runners: vec![(namespace_id, runner_name.to_string())],
			})
			.await?;

		if let Some(entry) = errors.into_iter().next() {
			tracing::warn!(
				%namespace_id,
				%runner_name,
				error = ?entry.error,
				"runner pool has active error, fast-failing request"
			);
			return Ok(true);
		}

		// Wait before next check
		tokio::time::sleep(RUNNER_POOL_ERROR_CHECK_INTERVAL).await;
	}
}
