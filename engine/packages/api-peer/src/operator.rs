//! Operator routes that make a storage worker's data cold on demand, for honest cold-start
//! measurements on a running cluster. They are served on the internal API-peer listener only,
//! like every route of this crate, and reach the worker over its UPS volume command subject.
//!
//! - `POST /depot/operator/tier` asks the worker's maintainers to write applied data to cold now.
//! - `POST /depot/operator/drop-caches` closes the command host's idle volumes and begins the
//!   unload of quiet volumes, then empties the worker's SlateDB block and metadata caches and
//!   its disk-cached object parts.
//!
//! Query parameters: `worker` (a worker id, default this worker), `tablet` (one tablet id,
//! default every tablet the worker maintains), and for drop caches `quiet_ms` (default 0, raised
//! by the maintainer to its holder check interval) and
//! `limit` (unloads per tablet, default 1024). Both reply with what they started. Repeating a
//! request until it reports nothing left to tier or unload waits for the work.

use anyhow::{Context, Result};
use axum::{
	Json, Router,
	extract::{Query, State},
	http::StatusCode,
	response::{IntoResponse, Response},
	routing::post,
};
use depot_types::keys::pubsub::VolumeCommandSubject;
use rivet_data::{
	DEPOT_CLIENT_PROTOCOL_VERSION,
	generated::depot_client_protocol_v1::{
		ClientRequest, ClientResponse, OperatorCachesOutcome, OperatorDropCachesRequest,
		OperatorTabletOutcome, OperatorTierRequest,
	},
	versioned::{DepotClientRequestData, DepotClientResponseData},
};
use rivet_engine_context::EngineContext;
use rivet_topology::worker::WorkerId;
use serde::Deserialize;
use serde_json::{Value, json};
use std::time::Duration;
use vbare::Versioned;

/// How long one operator request may take. Dropping a large disk cache removes many files.
const OPERATOR_TIMEOUT: Duration = Duration::from_secs(60);

/// Unloads one drop-caches request may begin per tablet when the caller names no limit.
const DEFAULT_UNLOAD_LIMIT: u32 = 1024;

#[derive(Debug, Default, Deserialize)]
struct OperatorQuery {
	worker: Option<String>,
	tablet: Option<String>,
	quiet_ms: Option<u64>,
	limit: Option<u32>,
}

pub fn router(ctx: EngineContext) -> Router {
	Router::new()
		.route("/depot/operator/tier", post(tier))
		.route("/depot/operator/drop-caches", post(drop_caches))
		.with_state(ctx)
}

async fn tier(State(ctx): State<EngineContext>, Query(query): Query<OperatorQuery>) -> Response {
	let request = parse_tablet(&query)
		.map(|tablet| ClientRequest::OperatorTierRequest(OperatorTierRequest { tablet }));
	respond(&ctx, &query, request).await
}

async fn drop_caches(
	State(ctx): State<EngineContext>,
	Query(query): Query<OperatorQuery>,
) -> Response {
	let request = parse_tablet(&query).map(|tablet| {
		ClientRequest::OperatorDropCachesRequest(OperatorDropCachesRequest {
			tablet,
			quiet_ms: query.quiet_ms.unwrap_or(0),
			limit: query.limit.unwrap_or(DEFAULT_UNLOAD_LIMIT),
		})
	});
	respond(&ctx, &query, request).await
}

async fn respond(
	ctx: &EngineContext,
	query: &OperatorQuery,
	request: Result<ClientRequest>,
) -> Response {
	let result = async {
		let worker = match &query.worker {
			Some(worker) => WorkerId::from_bytes(*parse_id(worker, "worker id")?.as_bytes()),
			None => ctx.worker_identity().worker_id.clone(),
		};
		send(ctx, &worker, request?).await
	}
	.await;
	match result {
		Ok(report) => Json(report).into_response(),
		Err(error) => {
			// An invalid query never left this worker; any other failure may have started work.
			let status = match error.downcast_ref::<InvalidQuery>() {
				Some(_) => StatusCode::BAD_REQUEST,
				None => StatusCode::BAD_GATEWAY,
			};
			tracing::warn!(
				?error,
				"depot operator request failed; the worker may have started part of it"
			);
			(status, Json(json!({ "error": format!("{error:#}") }))).into_response()
		}
	}
}

/// Sends one operator request to `worker` and reports its reply as JSON.
async fn send(ctx: &EngineContext, worker: &WorkerId, request: ClientRequest) -> Result<Value> {
	let payload = DepotClientRequestData::serialize_with_embedded_version(
		&request,
		DEPOT_CLIENT_PROTOCOL_VERSION,
	)?;
	let reply = ctx
		.ups()?
		.request_with_timeout(
			universalpubsub::RequestClass::DepotVolume,
			worker,
			VolumeCommandSubject::new(worker.clone()),
			&payload,
			OPERATOR_TIMEOUT,
		)
		.await
		.context("worker did not answer the operator request")?;
	let response = DepotClientResponseData::deserialize_with_embedded_version(&reply)?;
	let ClientResponse::OperatorResponse(response) = response else {
		anyhow::bail!("worker answered an operator request with {response:?}");
	};

	let tablets = response
		.tablets
		.into_iter()
		.map(|outcome| match outcome {
			OperatorTabletOutcome::OperatorTabletReport(report) => json!({
				"tablet": uuid::Uuid::from_bytes(report.tablet_id).to_string(),
				"tierRequested": report.tier_requested,
				"unloadsStarted": report.unloads_started,
				"loadedVolumes": report.loaded_volumes,
				"unloadingVolumes": report.unloading_volumes,
			}),
			OperatorTabletOutcome::OperatorTabletRejected(rejected) => json!({
				"tablet": uuid::Uuid::from_bytes(rejected.tablet_id).to_string(),
				"rejected": format!("{:?}", rejected.reason),
			}),
		})
		.collect::<Vec<_>>();
	let caches = response.caches.map(|caches| match caches {
		OperatorCachesOutcome::OperatorDroppedCaches(dropped) => json!({
			"memoryBytes": dropped.memory_bytes,
			"diskFiles": dropped.disk_files,
			"diskBytes": dropped.disk_bytes,
		}),
		OperatorCachesOutcome::OperatorCachesFailed(failed) => json!({ "error": failed.message }),
	});
	Ok(json!({
		"worker": uuid::Uuid::from_bytes(*worker.as_bytes()).to_string(),
		"tablets": tablets,
		"caches": caches,
	}))
}

fn parse_tablet(query: &OperatorQuery) -> Result<Option<[u8; 16]>> {
	query
		.tablet
		.as_deref()
		.map(|tablet| parse_id(tablet, "tablet id").map(|id| *id.as_bytes()))
		.transpose()
}

/// A query parameter that does not parse; the request is not sent.
#[derive(Debug)]
struct InvalidQuery(&'static str);

impl std::fmt::Display for InvalidQuery {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "invalid {}", self.0)
	}
}

impl std::error::Error for InvalidQuery {}

fn parse_id(value: &str, what: &'static str) -> Result<uuid::Uuid> {
	uuid::Uuid::parse_str(value).map_err(|_| InvalidQuery(what).into())
}
