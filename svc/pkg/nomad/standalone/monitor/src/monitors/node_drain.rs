use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NodeDrain {
	node: nomad_client::models::Node,
}

pub async fn handle(
	client: chirp_client::Client,
	NodeDrain { node }: &NodeDrain,
) -> GlobalResult<()> {
	let node_id = unwrap_ref!(node.ID);
	let meta = unwrap_ref!(node.meta, "no metadata on node");
	let server_id = util::uuid::parse(unwrap!(meta.get("server-id"), "no server-id in metadata"))?;

	if let Some(events) = &node.events {
		// Check if the last message in the node events is a drain complete message
		let is_last_drain_complete_message = events
			.last()
			.filter(|event| event.details.is_none())
			.and_then(|event| event.message.as_ref())
			.map(|msg| msg == "Node drain complete")
			.unwrap_or_default();

		if is_last_drain_complete_message {
			msg!([client] nomad::msg::monitor_node_drain_complete(server_id) {
				server_id: Some(server_id.into()),
				node_id: node_id.to_owned(),
			})
			.await?;
		}
	}

	Ok(())
}
