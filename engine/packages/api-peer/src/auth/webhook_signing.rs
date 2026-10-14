//! Exact-byte internal transport. No private keys or payloads enter workflow history.
use anyhow::Result;
use axum::{
	extract::Request,
	response::{IntoResponse, Response},
};
use futures_util::StreamExt;
use gas::prelude::*;
use rivet_api_builder::{
	ApiCtx, ApiError,
	extract::{Extension, Json, Path},
};
use rivet_api_types::auth::webhooks::SignResponse;
use rivet_auth_jwt::{ops::webhook as ops, webhook};
use rivet_auth_policy::errors::Signing;
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

// Reservations include both the upload and the contiguous Ed25519 signing message.
// Holding the permit in the blocking task also bounds cancelled/in-flight signing work.
static BUFFER_BUDGET: std::sync::LazyLock<Arc<Semaphore>> =
	std::sync::LazyLock::new(|| Arc::new(Semaphore::new(128)));
pub const DEADLINE_HEADER: &str = "x-rivet-signing-deadline-ms";

pub async fn read_body(request: Request) -> Result<(Vec<u8>, OwnedSemaphorePermit)> {
	if request
		.headers()
		.get(axum::http::header::CONTENT_TYPE)
		.and_then(|h| h.to_str().ok())
		!= Some("application/octet-stream")
		|| request
			.headers()
			.get(axum::http::header::CONTENT_ENCODING)
			.is_some_and(|h| h != "identity")
	{
		return Err(Signing::InvalidInput.build());
	}
	let permit = BUFFER_BUDGET
		.clone()
		.try_acquire_many_owned(64)
		.map_err(|_| Signing::Throttled.build())?;
	let body = tokio::time::timeout(std::time::Duration::from_secs(30), async move {
		let mut stream = request.into_body().into_data_stream();
		let mut bytes = Vec::new();
		while let Some(chunk) = stream.next().await {
			let chunk = chunk.map_err(|_| Signing::InvalidInput.build())?;
			if chunk.len() > webhook::MAX_PAYLOAD_BYTES.saturating_sub(bytes.len()) {
				return Err(Signing::PayloadTooLarge.build());
			}
			bytes.extend_from_slice(&chunk);
		}
		Ok(bytes)
	})
	.await
	.map_err(|_| Signing::Unavailable.build())??;
	Ok((body, permit))
}

#[tracing::instrument(skip_all)]
pub async fn sign(
	Extension(ctx): Extension<ApiCtx>,
	Path((namespace_id,)): Path<(Id,)>,
	request: Request,
) -> Response {
	let result = async {
		let deadline = request
			.headers()
			.get(DEADLINE_HEADER)
			.and_then(|h| h.to_str().ok())
			.and_then(|h| h.parse::<i64>().ok())
			.ok_or_else(|| Signing::InvalidInput.build())?;
		let (body, permit) = read_body(request).await?;
		sign_body(&ctx, namespace_id, deadline, body, permit).await
	}
	.await;
	let mut response = match result {
		Ok(body) => Json(body).into_response(),
		Err(error) => ApiError::from(error).into_response(),
	};
	response.headers_mut().insert(
		axum::http::header::CACHE_CONTROL,
		axum::http::HeaderValue::from_static("no-store"),
	);
	response
}

pub async fn sign_body(
	ctx: &StandaloneCtx,
	namespace_id: Id,
	deadline: i64,
	body: Vec<u8>,
	permit: OwnedSemaphorePermit,
) -> Result<SignResponse> {
	if !ctx.config().is_leader() || !ctx.config().auth_required()?.jwt.issuance_enabled() {
		return Err(Signing::Unavailable.build());
	}
	if !ctx
		.op(namespace::ops::exists_authoritative::Input { namespace_id })
		.await?
	{
		return Err(Signing::NotFound.build());
	}
	let snapshot = ctx
		.op(ops::get::Input { namespace_id })
		.await
		.map_err(super::webhooks::map_error)?
		.ok_or_else(|| Signing::NotFound.build())?;
	let config_generation = ctx.config().auth_required()?.jwt.issuer_generation();
	if snapshot.ring.keys.leader_datacenter_id != ctx.config().dc_label()
		|| snapshot.ring.keys.leader_config_generation != config_generation
	{
		return Err(Signing::Unavailable.build());
	}
	let read_started = snapshot.read_started;
	let response = tokio::task::spawn_blocking(move || {
		let _permit = permit;
		let now = rivet_util::timestamp::now();
		if now >= deadline || read_started.elapsed() >= rivet_auth_jwt::SIGNING_CACHE_LEASE {
			return Err(Signing::Unavailable.build());
		}
		let response =
			webhook::sign(&snapshot.ring, &body, now).map_err(|_| Signing::Unavailable.build())?;
		let after = rivet_util::timestamp::now();
		if after < now
			|| after >= deadline
			|| after >= snapshot.ring.keys.active.sign_until_ts
			|| read_started.elapsed() >= rivet_auth_jwt::SIGNING_CACHE_LEASE
		{
			return Err(Signing::Unavailable.build());
		}
		Ok(SignResponse {
			timestamp: response.timestamp,
			kid: response.kid,
			signature: response.signature,
		})
	})
	.await
	.map_err(|_| Signing::Unavailable.build())??;
	// Scheduling pauses cannot extend the lease before sending the reply.
	if read_started.elapsed() >= rivet_auth_jwt::SIGNING_CACHE_LEASE
		|| rivet_util::timestamp::now() >= deadline
	{
		return Err(Signing::Unavailable.build());
	}
	Ok(response)
}

#[cfg(test)]
#[path = "../../tests/unit/webhook_signing.rs"]
mod tests;
