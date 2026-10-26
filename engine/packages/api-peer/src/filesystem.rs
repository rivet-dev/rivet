//! Regional filesystem requests enter the ordinary local tablet routing path.
use anyhow::{Result, ensure};
use axum::{
	Router,
	body::{Body, to_bytes},
	extract::State,
	http::{Request, StatusCode},
	response::{IntoResponse, Response},
	routing::post,
};
use depot_client::regional::{MAX_MESSAGE_BYTES, PEER_PATH, REGION_HEADER, TIMEOUT_HEADER};
use rivet_data::versioned::{DepotClientRequestData, DepotClientResponseData};
use rivet_env::Clock;
use std::sync::Arc;
use std::time::Duration;
use vbare::Versioned;

/// Process-owned regional endpoint dependencies; constructing it starts no tasks.
#[derive(Clone)]
pub struct PeerService {
	/// Resolves the currently installed client, including after startup completes.
	pub client: Arc<dyn Fn() -> depot_client::Client + Send + Sync>,
	pub clock: Arc<dyn Clock>,
	pub region: u16,
	pub timeout_ms: u64,
	/// Reads fresh process/lease authority at admission and immediately before reply.
	pub may_serve: Arc<dyn Fn() -> bool + Send + Sync>,
}
pub fn router(service: PeerService) -> Router {
	Router::new()
		.route(PEER_PATH, post(request))
		.with_state(service)
}
async fn request(State(service): State<PeerService>, request: Request<Body>) -> Response {
	match dispatch(&service, request).await {
		Ok(bytes) => ([("content-type", "application/octet-stream")], bytes).into_response(),
		Err(error) => {
			tracing::warn!(%error, "regional filesystem request failed; sent commands may have completed");
			StatusCode::SERVICE_UNAVAILABLE.into_response()
		}
	}
}
async fn dispatch(service: &PeerService, request: Request<Body>) -> Result<Vec<u8>> {
	ensure!((service.may_serve)(), "regional peer is not serving");
	let region = request
		.headers()
		.get(REGION_HEADER)
		.and_then(|v| v.to_str().ok())
		.and_then(|v| v.parse::<u16>().ok())
		.ok_or_else(|| anyhow::anyhow!("missing destination region"))?;
	ensure!(region == service.region, "wrong destination region");
	let timeout_ms = request
		.headers()
		.get(TIMEOUT_HEADER)
		.and_then(|v| v.to_str().ok())
		.and_then(|v| v.parse::<u64>().ok())
		.ok_or_else(|| anyhow::anyhow!("missing request deadline"))?
		.min(service.timeout_ms);
	ensure!(timeout_ms > 0, "request deadline expired");
	let clock = service.clock.clone();
	let timeout = Duration::from_millis(timeout_ms);
	let deadline = rivet_env::deadline_after(clock.as_ref(), timeout)?;
	let response = rivet_env::timeout(clock.as_ref(), timeout, async {
		let bytes = to_bytes(request.into_body(), MAX_MESSAGE_BYTES).await?;
		let command = DepotClientRequestData::deserialize_with_embedded_version(&bytes)?;
		let response = (service.client)()
			.filesystem_peer_request(region, command, deadline)
			.await?;
		let bytes = DepotClientResponseData::serialize_with_embedded_version(
			&response,
			rivet_data::DEPOT_CLIENT_PROTOCOL_VERSION,
		)?;
		ensure!(
			bytes.len() <= MAX_MESSAGE_BYTES,
			"regional response too large"
		);
		Ok::<_, anyhow::Error>(bytes)
	})
	.await??;
	ensure!(
		(service.may_serve)(),
		"regional peer lost authority before reply"
	);
	Ok(response)
}
