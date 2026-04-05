use anyhow::*;
use axum::body::Bytes;
use epoxy_protocol::{PROTOCOL_VERSION, protocol};
use rivet_api_builder::prelude::*;
use std::time::Instant;

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
}

pub async fn message(ctx: ApiCtx, path: ProtocolPath, _query: (), body: Bytes) -> Result<Vec<u8>> {
	assert_protocol_version(path.version)?;
	let request = deserialize_request(&body)?;
	ensure!(
		!matches!(request.kind, protocol::RequestKind::ChangelogReadRequest(_)),
		"use /epoxy/changelog-read for changelog reads"
	);

	handle_request(ctx, request).await
}

pub async fn changelog_read(
	ctx: ApiCtx,
	path: ProtocolPath,
	_query: (),
	body: Bytes,
) -> Result<Vec<u8>> {
	assert_protocol_version(path.version)?;
	let request = deserialize_request(&body)?;
	ensure!(
		matches!(request.kind, protocol::RequestKind::ChangelogReadRequest(_)),
		"/epoxy/changelog-read only accepts changelog read requests"
	);

	handle_request(ctx, request).await
}

fn assert_protocol_version(version: u16) -> Result<()> {
	ensure!(
		version == PROTOCOL_VERSION,
		"unsupported epoxy protocol version: {version}"
	);
	Ok(())
}

fn deserialize_request(body: &[u8]) -> Result<protocol::Request> {
	serde_bare::from_slice(body).map_err(Into::into)
}

fn request_kind_label(kind: &protocol::RequestKind) -> &'static str {
	match kind {
		protocol::RequestKind::UpdateConfigRequest(_) => "update_config",
		protocol::RequestKind::PrepareRequest(_) => "prepare",
		protocol::RequestKind::AcceptRequest(_) => "accept",
		protocol::RequestKind::CommitRequest(_) => "commit",
		protocol::RequestKind::ChangelogReadRequest(_) => "changelog_read",
		protocol::RequestKind::HealthCheckRequest => "health_check",
		protocol::RequestKind::CoordinatorUpdateReplicaStatusRequest(_) => {
			"coordinator_update_replica_status"
		}
		protocol::RequestKind::BeginLearningRequest(_) => "begin_learning",
		protocol::RequestKind::KvGetRequest(_) => "kv_get",
		protocol::RequestKind::KvPurgeCacheRequest(_) => "kv_purge_cache",
	}
}

async fn handle_request(ctx: ApiCtx, request: protocol::Request) -> Result<Vec<u8>> {
	let current_replica_id = ctx.config().epoxy_replica_id();
	ensure!(
		request.to_replica_id == current_replica_id,
		"request intended for replica {} but received by replica {}",
		request.to_replica_id,
		current_replica_id
	);

	let kind_label = request_kind_label(&request.kind);
	let start = Instant::now();
	let res = crate::replica::message_request::message_request(&ctx, request).await;
	let result_label = if res.is_ok() { "ok" } else { "err" };
	metrics::record_request(kind_label, result_label, start.elapsed());

	serde_bare::to_vec(&res?).map_err(Into::into)
}
