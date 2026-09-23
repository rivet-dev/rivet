use anyhow::{Result, ensure};
use axum::body::Bytes;
use epoxy_protocol::{protocol, versioned};
use rivet_api_builder::prelude::*;
use rivet_perf::{perf_finish, perf_start};

use crate::metrics;

#[derive(Deserialize)]
pub struct ProtocolPath {
	pub version: u16,
}

pub fn mount_routes(
	router: axum::Router<rivet_api_builder::GlobalApiCtx>,
) -> axum::Router<rivet_api_builder::GlobalApiCtx> {
	router
		.route("/v{version}/epoxy/message", bin::post(message))
		.route(
			"/v{version}/epoxy/changelog-read",
			bin::post(changelog_read),
		)
		// Deliberately unversioned: a peer probes this to find out which versions we speak, so it
		// has to stay reachable by a replica that shares no protocol version with us.
		.route("/epoxy/protocol-version", get(protocol_version))
}

/// Publishes the epoxy protocol version this datacenter agreed on, for peer replicas in other
/// datacenters to probe. Their heartbeats live in a different database, so this is the only way they
/// can learn what we accept.
pub async fn protocol_version(
	ctx: ApiCtx,
	_path: (),
	_query: (),
) -> Result<crate::protocol_version::ProtocolVersionResponse> {
	Ok(crate::protocol_version::ProtocolVersionResponse {
		protocol_version: ctx.config().protocols().epoxy.version(),
	})
}

pub async fn message(ctx: ApiCtx, path: ProtocolPath, _query: (), body: Bytes) -> Result<Vec<u8>> {
	let request = versioned::decode_request(&body, path.version)?;
	ensure!(
		!matches!(request.kind, protocol::RequestKind::ChangelogReadRequest(_)),
		"use /epoxy/changelog-read for changelog reads"
	);

	handle_request(ctx, request, path.version).await
}

pub async fn changelog_read(
	ctx: ApiCtx,
	path: ProtocolPath,
	_query: (),
	body: Bytes,
) -> Result<Vec<u8>> {
	let request = versioned::decode_request(&body, path.version)?;
	ensure!(
		matches!(request.kind, protocol::RequestKind::ChangelogReadRequest(_)),
		"/epoxy/changelog-read only accepts changelog read requests"
	);

	handle_request(ctx, request, path.version).await
}

fn request_kind_label(kind: &protocol::RequestKind) -> &'static str {
	match kind {
		protocol::RequestKind::UpdateConfigRequest(_) => "update_config",
		protocol::RequestKind::PrepareRequest(_) => "prepare",
		protocol::RequestKind::PreAcceptRequest(_) => "pre_accept",
		protocol::RequestKind::AcceptRequest(_) => "accept",
		protocol::RequestKind::CommitRequest(_) => "commit",
		protocol::RequestKind::ChangelogReadRequest(_) => "changelog_read",
		protocol::RequestKind::HealthCheckRequest => "health_check",
		protocol::RequestKind::CoordinatorUpdateReplicaStatusRequest(_) => {
			"coordinator_update_replica_status"
		}
		protocol::RequestKind::BeginLearningRequest(_) => "begin_learning",
		protocol::RequestKind::KvGetRequest(_) => "kv_get",
		protocol::RequestKind::KvReadStateRequest(_) => "kv_read_state",
		protocol::RequestKind::KvPurgeCacheRequest(_) => "kv_purge_cache",
	}
}

async fn handle_request(ctx: ApiCtx, request: protocol::Request, version: u16) -> Result<Vec<u8>> {
	let current_replica_id = ctx.config().epoxy_replica_id();
	ensure!(
		request.to_replica_id == current_replica_id,
		"request intended for replica {} but received by replica {}",
		request.to_replica_id,
		current_replica_id
	);

	let kind_label = request_kind_label(&request.kind);
	let measure = perf_start!(
		&metrics::REQUEST_DURATION,
		slow_ms = 1000,
		"epoxy_request",
		labels: { request_type = %kind_label },
		fields: {
			to_replica_id = %request.to_replica_id,
			current_replica_id = %current_replica_id,
		},
	);
	let res = crate::replica::message_request::message_request(&ctx, request).await;
	let result_label = if res.is_ok() { "ok" } else { "err" };
	metrics::record_request_result(kind_label, result_label);
	perf_finish!(measure, fields: { result = %result_label });

	versioned::encode_response(res?, version)
}
