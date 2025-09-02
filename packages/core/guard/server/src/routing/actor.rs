//use std::time::Duration;
//
//use anyhow::Result;
//use gas::prelude::*;
//use hyper::header::HeaderName;
//use rivet_guard_core::proxy_service::{RouteConfig, RouteTarget, RoutingOutput, RoutingTimeout};
//use rivet_key_data::generated::pegboard_runner_address_v1::Data as AddressKeyData;
//use udb_util::{SERIALIZABLE, TxnExt};
//
//use crate::errors;
//
//const ACTOR_READY_TIMEOUT: Duration = Duration::from_secs(10);
//pub(crate) const X_RIVET_ACTOR: HeaderName = HeaderName::from_static("x-rivet-actor");
//pub(crate) const X_RIVET_ADDR: HeaderName = HeaderName::from_static("x-rivet-addr");
//
///// Route requests to actor services based on hostname and path
//#[tracing::instrument(skip_all)]
//pub async fn route_request(
//	ctx: &StandaloneCtx,
//	target: &str,
//	_host: &str,
//	path: &str,
//	headers: &hyper::HeaderMap,
//) -> Result<Option<RoutingOutput>> {
//	// Check target
//	if target != "actor" {
//		return Ok(None);
//	}
//
//	// Find actor to route to
//	let actor_id_str = headers.get(X_RIVET_ACTOR).ok_or_else(|| {
//		crate::errors::MissingHeader {
//			header: X_RIVET_ACTOR.to_string(),
//		}
//		.build()
//	})?;
//	let actor_id = Id::parse(actor_id_str.to_str()?)?;
//
//	// Route to peer dc where the actor lives
//	if actor_id.label() != ctx.config().dc_label() {
//		tracing::debug!(peer_dc_label=?actor_id.label(), "re-routing actor to peer dc");
//
//		let peer_dc = ctx
//			.config()
//			.dc_for_label(actor_id.label())
//			.context("dc with the given label not found")?;
//
//		return Ok(Some(RoutingOutput::Route(RouteConfig {
//			targets: vec![RouteTarget {
//				actor_id: Some(actor_id),
//				host: peer_dc
//					.guard_url
//					.host()
//					.context("peer dc guard_url has no host")?
//					.to_string(),
//				port: peer_dc
//					.guard_url
//					.port()
//					.context("peer dc guard_url has no port")?,
//				path: path.to_owned(),
//			}],
//			timeout: RoutingTimeout {
//				routing_timeout: 10,
//			},
//		})));
//	}
//
//	let addr_name = headers.get(X_RIVET_ADDR).ok_or_else(|| {
//		crate::errors::MissingHeader {
//			header: X_RIVET_ADDR.to_string(),
//		}
//		.build()
//	})?;
//	let addr_name = addr_name.to_str()?;
//
//	// Now that we have the actor_id and addr_name, lookup the actor
//	if let Some(target) = find_actor(ctx, actor_id, addr_name, path).await? {
//		Ok(Some(RoutingOutput::Route(RouteConfig {
//			targets: vec![target],
//			timeout: RoutingTimeout {
//				routing_timeout: 10,
//			},
//		})))
//	} else {
//		tracing::debug!(
//			?actor_id,
//			?addr_name,
//			"attempted to route to actor not found"
//		);
//
//		Err(errors::ActorNotFound {
//			actor_id,
//			addr: addr_name.to_string(),
//		}
//		.build())
//	}
//}
//
//struct FoundActor {
//	workflow_id: Id,
//	sleeping: bool,
//	destroyed: bool,
//}
//
///// Find an actor by actor_id and addr_name - this would call into the actor registry
//#[tracing::instrument(skip_all, fields(%actor_id, %addr_name, %path))]
//async fn find_actor(
//	ctx: &StandaloneCtx,
//	actor_id: Id,
//	addr_name: &str,
//	path: &str,
//) -> Result<Option<RouteTarget>> {
//	// TODO: Optimize this down to a single FDB call
//
//	// Create subs before checking if actor exists/is not destroyed
//	let mut ready_sub = ctx
//		.subscribe::<pegboard::workflows::actor::Ready>(("actor_id", actor_id))
//		.await?;
//	let mut fail_sub = ctx
//		.subscribe::<pegboard::workflows::actor::Failed>(("actor_id", actor_id))
//		.await?;
//	let mut destroy_sub = ctx
//		.subscribe::<pegboard::workflows::actor::DestroyStarted>(("actor_id", actor_id))
//		.await?;
//
//	let actor_res = tokio::time::timeout(
//		Duration::from_secs(5),
//		ctx.udb()?
//			.run(|tx, _mc| async move {
//				let txs = tx.subspace(pegboard::keys::subspace());
//
//				let workflow_id_key = pegboard::keys::actor::WorkflowIdKey::new(actor_id);
//				let sleep_ts_key = pegboard::keys::actor::SleepTsKey::new(actor_id);
//				let destroy_ts_key = pegboard::keys::actor::DestroyTsKey::new(actor_id);
//
//				let (workflow_id_entry, sleeping, destroyed) = tokio::try_join!(
//					txs.read_opt(&workflow_id_key, SERIALIZABLE),
//					txs.exists(&sleep_ts_key, SERIALIZABLE),
//					txs.exists(&destroy_ts_key, SERIALIZABLE),
//				)?;
//
//				let Some(workflow_id) = workflow_id_entry else {
//					return Ok(None);
//				};
//
//				Ok(Some(FoundActor {
//					workflow_id,
//					sleeping,
//					destroyed,
//				}))
//			})
//			.custom_instrument(tracing::info_span!("actor_exists_tx")),
//	)
//	.await??;
//
//	let Some(actor) = actor_res else {
//		return Err(errors::ActorNotFound {
//			actor_id,
//			addr: addr_name.to_string(),
//		}
//		.build());
//	};
//
//	if actor.destroyed {
//		return Err(errors::ActorDestroyed { actor_id }.build());
//	}
//
//	// Wake actor if sleeping
//	if actor.sleeping {
//		ctx.signal(pegboard::workflows::actor::Wake {})
//			.to_workflow_id(actor.workflow_id)
//			.send()
//			.await?;
//	}
//
//	// Fetch address. Will return None if actor is not ready yet.
//	let addr = if let Some(addr) = fetch_addr(ctx, actor_id, addr_name).await? {
//		addr
//	} else {
//		tracing::info!(?actor_id, "waiting for actor to become ready");
//
//		// Wait for ready, fail, or destroy
//		tokio::select! {
//			res = ready_sub.next() => { res?; },
//			res = fail_sub.next() => {
//				let msg = res?;
//				return Err(msg.error.clone().build());
//			}
//			res = destroy_sub.next() => {
//				res?;
//				return Err(pegboard::errors::Actor::DestroyedWhileWaitingForReady.build());
//			}
//			// Ready timeout
//				_ = tokio::time::sleep(ACTOR_READY_TIMEOUT) => {
//				return Err(errors::ActorReadyTimeout { actor_id }.build());
//			}
//		}
//
//		// Fetch address again after ready
//		let Some(addr) = fetch_addr(ctx, actor_id, addr_name).await? else {
//			return Err(errors::ActorNotFound {
//				actor_id,
//				addr: addr_name.to_string(),
//			}
//			.build());
//		};
//
//		addr
//	};
//
//	tracing::debug!(?actor_id, ?addr, "actor ready");
//
//	// Validate addr type
//	let AddressKeyData::Http(addr) = addr else {
//		return Err(crate::errors::WrongAddrProtocol {
//			addr_name: addr_name.into(),
//			expected: "http",
//			received: match addr {
//				AddressKeyData::Http(_) => unreachable!(),
//				AddressKeyData::Tcp(_) => "tcp",
//				AddressKeyData::Udp(_) => "udp",
//			},
//		}
//		.build());
//	};
//
//	Ok(Some(RouteTarget {
//		actor_id: Some(actor_id),
//		host: addr.hostname,
//		port: addr.port,
//		path: path.to_owned(),
//	}))
//}
//
//#[tracing::instrument(skip_all, fields(?actor_id))]
//async fn fetch_addr(
//	ctx: &StandaloneCtx,
//	actor_id: Id,
//	addr_name: &str,
//) -> Result<Option<AddressKeyData>> {
//	let get_address_fut = ctx.op(pegboard::ops::actor::get_address::Input {
//		actor_id,
//		address_name: addr_name.into(),
//	});
//	let get_address = tokio::time::timeout(Duration::from_secs(5), get_address_fut).await??;
//	Ok(get_address)
//}
