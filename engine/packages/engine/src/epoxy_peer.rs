//! Routes regional Epoxy requests to one elected KV owner, without opening engines on proxy workers.

use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result, bail, ensure};
use axum::{
	Router,
	body::{Body, to_bytes},
	extract::State,
	http::{Request, StatusCode, header},
	response::{IntoResponse, Response},
	routing::{any, get},
};
use rivet_engine_context::EngineContext;
use rivet_singleton_coordinator::{
	singleton_coordinator_machine::{executor::LeaseStore, types::LeaseRead},
	store::ObjectLeaseStore,
};
use tower::ServiceExt;

use crate::epoxy_services::EpoxyServices;

#[derive(Clone)]
struct Peer {
	ctx: EngineContext,
	host: Option<Arc<EpoxyServices>>,
}

pub(crate) fn router(ctx: EngineContext, host: Option<Arc<EpoxyServices>>) -> Router {
	Router::new()
		.route("/epoxy/protocol-version", get(version))
		.route("/epoxy/{*path}", any(request))
		.route("/v{version}/epoxy/message", any(request))
		.with_state(Peer { ctx, host })
}

/// Discovery does not require membership or a tablet; bootstrap peers can always negotiate.
async fn version(State(peer): State<Peer>) -> impl IntoResponse {
	axum::Json(serde_json::json!({
		"protocol_version": peer.ctx.config().protocols().epoxy.version()
	}))
}

#[tracing::instrument(skip_all)]
async fn request(State(peer): State<Peer>, request: Request<Body>) -> Response {
	let clock = peer.ctx.env().clock();
	let result = rivet_env::timeout(
		clock.as_ref(),
		Duration::from_millis(peer.ctx.config().depot().volume_request_timeout_ms()),
		dispatch(&peer, request),
	)
	.await;
	match result {
		Ok(Ok(response)) => response,
		failure => {
			tracing::warn!(
				?failure,
				"Epoxy regional owner unavailable; sent mutations may have completed"
			);
			StatusCode::SERVICE_UNAVAILABLE.into_response()
		}
	}
}

/// A remote lease is only a routing hint. The target requires its live local authority
/// at admission and reply. Forward once: an uncertain mutation is never resubmitted here.
async fn dispatch(peer: &Peer, mut request: Request<Body>) -> Result<Response> {
	ensure!(
		peer.ctx.topology().may_admit()?,
		"peer worker is not serving"
	);
	let serving = peer
		.host
		.as_ref()
		.and_then(|host| host.serving.borrow().clone());
	if let Some((authority, client)) = serving
		&& authority.permits()
	{
		// Routing starts anew at the owner. Outer wildcard parameters must not be
		// combined with the protocol router's key or version parameters.
		request.extensions_mut().clear();
		let response = epoxy::http::router(peer.ctx.config().clone(), client)?
			.oneshot(request)
			.await?;
		ensure!(authority.permits(), "Epoxy tenure ended before reply");
		return Ok(response);
	}

	// A stale destination must reject, never bounce a mutation among successive owners.
	ensure!(
		!request.headers().contains_key("x-rivet-epoxy-forwarded"),
		"Epoxy route changed"
	);
	let store = ObjectLeaseStore::new(peer.ctx.object_stores().lease.clone());
	let owner = match store.read().await {
		LeaseRead::Found(lease) if !lease.released => lease.tenure.owner,
		LeaseRead::Found(_) | LeaseRead::Missing | LeaseRead::Unknown => bail!("no Epoxy owner"),
	};
	ensure!(
		owner != *peer.ctx.worker_identity(),
		"Epoxy owner is not ready"
	);
	let endpoint = peer
		.ctx
		.topology()
		.resolve_worker(&owner.worker_id)
		.await?
		.context("Epoxy owner endpoint unavailable")?;
	ensure!(
		endpoint.instance_id == owner.instance_id,
		"Epoxy owner instance changed"
	);

	let (mut parts, body) = request.into_parts();
	let path = parts
		.uri
		.path_and_query()
		.context("missing peer request path")?;
	parts.uri = format!("{}{path}", endpoint.api_peer_address.trim_end_matches('/')).parse()?;
	parts.headers.remove(header::HOST);
	parts.headers.remove(header::CONNECTION);
	parts.headers.remove(header::TRANSFER_ENCODING);
	parts
		.headers
		.insert("x-rivet-epoxy-forwarded", "1".parse()?);
	// Preserve the peer router's default request body bound and the transport's response bound.
	let body = to_bytes(body, 2 * 1024 * 1024).await?;
	let response = peer
		.ctx
		.http()
		.execute(Request::from_parts(parts, body), 64 * 1024 * 1024)
		.await?;
	ensure!(
		peer.ctx.topology().may_reply()?,
		"proxy worker lost authority before reply"
	);
	let (mut parts, body) = response.into_parts();
	parts.headers.remove(header::CONNECTION);
	parts.headers.remove(header::TRANSFER_ENCODING);
	Ok(Response::from_parts(parts, Body::from(body)))
}
