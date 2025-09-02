use std::time::Duration;

use anyhow::Result;
use gas::prelude::*;
use hyper::header::HeaderName;
use rivet_guard_core::proxy_service::{RouteConfig, RouteTarget, RoutingOutput, RoutingTimeout};
use udb_util::{SERIALIZABLE, TxnExt};

use crate::errors;

const ACTOR_READY_TIMEOUT: Duration = Duration::from_secs(10);
pub const X_RIVET_ACTOR: HeaderName = HeaderName::from_static("x-rivet-actor");
pub const X_RIVET_PORT: HeaderName = HeaderName::from_static("x-rivet-port");

/// Route requests to actor services based on hostname and path
#[tracing::instrument(skip_all)]
pub async fn route_request(
	ctx: &StandaloneCtx,
	target: &str,
	_host: &str,
	path: &str,
	headers: &hyper::HeaderMap,
) -> Result<Option<RoutingOutput>> {
	// Check target
	if target != "actor" {
		return Ok(None);
	}

	// Find actor to route to
	let actor_id_str = headers.get(X_RIVET_ACTOR).ok_or_else(|| {
		crate::errors::MissingHeader {
			header: X_RIVET_ACTOR.to_string(),
		}
		.build()
	})?;
	let actor_id = Id::parse(actor_id_str.to_str()?)?;

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
					.guard_url
					.host()
					.context("peer dc guard_url has no host")?
					.to_string(),
				port: peer_dc
					.guard_url
					.port()
					.context("peer dc guard_url has no port")?,
				path: path.to_owned(),
			}],
			timeout: RoutingTimeout {
				routing_timeout: 10,
			},
		})));
	}

	let port_name = headers.get(X_RIVET_PORT).ok_or_else(|| {
		crate::errors::MissingHeader {
			header: X_RIVET_PORT.to_string(),
		}
		.build()
	})?;
	let port_name = port_name.to_str()?;

	// Lookup actor
	find_actor(ctx, actor_id, port_name, path).await
}

struct FoundActor {
	workflow_id: Id,
	sleeping: bool,
	destroyed: bool,
}

/// Find an actor by actor_id and port_name
#[tracing::instrument(skip_all, fields(%actor_id, %port_name, %path))]
async fn find_actor(
	ctx: &StandaloneCtx,
	actor_id: Id,
	port_name: &str,
	path: &str,
) -> Result<Option<RoutingOutput>> {
	// TODO: Optimize this down to a single FDB call

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

	let actor_res = tokio::time::timeout(
		Duration::from_secs(5),
		ctx.udb()?
			.run(|tx, _mc| async move {
				let txs = tx.subspace(pegboard::keys::subspace());

				let workflow_id_key = pegboard::keys::actor::WorkflowIdKey::new(actor_id);
				let sleep_ts_key = pegboard::keys::actor::SleepTsKey::new(actor_id);
				let destroy_ts_key = pegboard::keys::actor::DestroyTsKey::new(actor_id);

				let (workflow_id_entry, sleeping, destroyed) = tokio::try_join!(
					txs.read_opt(&workflow_id_key, SERIALIZABLE),
					txs.exists(&sleep_ts_key, SERIALIZABLE),
					txs.exists(&destroy_ts_key, SERIALIZABLE),
				)?;

				let Some(workflow_id) = workflow_id_entry else {
					return Ok(None);
				};

				Ok(Some(FoundActor {
					workflow_id,
					sleeping,
					destroyed,
				}))
			})
			.custom_instrument(tracing::info_span!("actor_exists_tx")),
	)
	.await??;

	let Some(actor) = actor_res else {
		return Err(errors::ActorNotFound {
			actor_id,
			port_name: port_name.to_string(),
		}
		.build());
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

	// Check if actor is connectable and get runner_id
	let runner_info = {
		let get_runner_fut = ctx.op(pegboard::ops::actor::get_runner::Input {
			actor_ids: vec![actor_id],
		});
		let output = tokio::time::timeout(Duration::from_secs(5), get_runner_fut).await??;
		output.actors.into_iter().find(|a| a.actor_id == actor_id)
	};

	let Some(runner_info) = runner_info else {
		return Err(errors::ActorNotFound {
			actor_id,
			port_name: port_name.to_string(),
		}
		.build());
	};

	if !runner_info.is_connectable {
		tracing::info!(?actor_id, "waiting for actor to become ready");

		// Wait for ready, fail, or destroy
		tokio::select! {
			res = ready_sub.next() => { res?; },
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

		// TODO: Is this needed? Can't we just re-check the actor exists if it fails to connect?
		// Verify actor is connectable again
		let runner_info = {
			let get_runner_fut = ctx.op(pegboard::ops::actor::get_runner::Input {
				actor_ids: vec![actor_id],
			});
			let output = tokio::time::timeout(Duration::from_secs(5), get_runner_fut).await??;
			output.actors.into_iter().find(|a| a.actor_id == actor_id)
		};

		let Some(runner_info) = runner_info else {
			return Err(errors::ActorNotFound {
				actor_id,
				port_name: port_name.to_string(),
			}
			.build());
		};

		if !runner_info.is_connectable {
			return Err(errors::ActorNotFound {
				actor_id,
				port_name: port_name.to_string(),
			}
			.build());
		};
	}

	tracing::debug!(?actor_id, runner_id = ?runner_info.runner_id, "actor ready");

	// Return pegboard-gateway instance
	let gateway = pegboard_gateway::PegboardGateway::new(
		ctx.clone(),
		actor_id,
		runner_info.runner_id,
		port_name.to_string(),
	);
	Ok(Some(RoutingOutput::CustomServe(std::sync::Arc::new(
		gateway,
	))))
}
