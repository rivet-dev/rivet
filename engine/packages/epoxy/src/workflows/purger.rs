use anyhow::*;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use epoxy_protocol::protocol;
use futures_util::FutureExt;
use gas::prelude::*;
use rivet_api_builder::ApiCtx;
use serde::{Deserialize, Serialize};

use crate::http_client;

#[derive(Debug, Deserialize, Serialize)]
pub struct Input {
	pub replica_id: protocol::ReplicaId,
}

// HACK: This workflow is a hack used to implement token revoking. It should be replaced with proper snapshot
// reads
#[workflow]
pub async fn epoxy_purger(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	ctx.repeat(|ctx| {
		let replica_id = input.replica_id;

		async move {
			let signals = ctx.listen_n::<Purge>(1024).await?;

			ctx.activity(PurgeInput {
				replica_id,
				keys: signals.into_iter().flat_map(|sig| sig.keys).collect(),
			})
			.await?;

			Ok(Loop::<()>::Continue)
		}
		.boxed()
	})
	.await?;

	Ok(())
}

#[signal("epoxy_purger_purge")]
pub struct Purge {
	/// Base64 encoded keys.
	pub keys: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Hash)]
struct PurgeInput {
	replica_id: protocol::ReplicaId,
	/// Base64 encoded keys.
	keys: Vec<String>,
}

#[activity(PurgeActivity)]
#[max_retries = 18_446_744_073_709_551_615] // Retry forever
async fn send_purge(ctx: &ActivityCtx, input: &PurgeInput) -> Result<()> {
	let config = ctx
		.op(crate::ops::read_cluster_config::Input {})
		.await?
		.config;

	http_client::send_message(
		&ApiCtx::new_from_activity(&ctx)?,
		&config,
		protocol::Request {
			from_replica_id: ctx.config().epoxy_replica_id(),
			to_replica_id: input.replica_id,
			kind: protocol::RequestKind::KvPurgeRequest(protocol::KvPurgeRequest {
				keys: input
					.keys
					.iter()
					.map(|key| BASE64.decode(key).context("invalid base64 key"))
					.collect::<Result<Vec<_>>>()?,
			}),
		},
	)
	.await?;

	Ok(())
}
