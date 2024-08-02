use chirp_workflow::prelude::*;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NodeDrain {
	node: nomad_client::models::Node,
}

pub async fn handle(ctx: StandaloneCtx, NodeDrain { node }: &NodeDrain) -> GlobalResult<()> {
	let node_id = unwrap_ref!(node.ID);
	let meta = unwrap_ref!(node.meta, "no metadata on node");
	let server_id = util::uuid::parse(unwrap!(meta.get("server-id"), "no server-id in metadata"))?;

	tracing::info!(%node_id, %server_id, "node message");

	if let Some(events) = &node.events {
		// Check if the last message in the node events is a drain complete message
		let is_last_drain_complete_message = events
			.last()
			.and_then(|event| event.message.as_ref())
			.map(|msg| msg == "Node drain complete")
			.unwrap_or_default();

		if is_last_drain_complete_message {
			ctx.tagged_signal(
				&json!({
					"server_id": server_id,
				}),
				cluster::workflows::server::NomadDrainComplete {},
			)
			.await?;
		}
	}

	Ok(())
}
