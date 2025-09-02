use anyhow::*;
use axum::body::Bytes;
use epoxy_protocol::{protocol, versioned};
use rivet_api_builder::prelude::*;
use versioned_data_util::OwnedVersionedData;

#[derive(Deserialize)]
pub struct VersionedPath {
	pub version: u16,
}

pub fn mount_routes(
	router: axum::Router<rivet_api_builder::GlobalApiCtx>,
) -> axum::Router<rivet_api_builder::GlobalApiCtx> {
	router.route("/v{version}/epoxy/message", bin::post(message))
}

pub async fn message(ctx: ApiCtx, path: VersionedPath, _query: (), body: Bytes) -> Result<Vec<u8>> {
	let request = versioned::Request::deserialize(&body, path.version)?;

	// Assert that the request is intended for this replica
	let current_replica_id = ctx.config().epoxy_replica_id();
	ensure!(
		request.to_replica_id == current_replica_id,
		"request intended for replica {} but received by replica {}",
		request.to_replica_id,
		current_replica_id
	);

	// Process message directly using ops
	let response =
		crate::replica::message_request::message_request(&ctx, current_replica_id, request).await?;

	versioned::Response::latest(response).serialize(path.version)
}
