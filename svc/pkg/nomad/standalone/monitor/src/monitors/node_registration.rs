use proto::backend::pkg::*;
use rivet_operation::prelude::*;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NodeRegistration {
	node: nomad_client::models::Node,
}

pub async fn handle(
	client: chirp_client::Client,
	NodeRegistration { node }: &NodeRegistration,
) -> GlobalResult<()> {
	let node_id = unwrap_ref!(node.ID);
	let meta = unwrap_ref!(node.meta, "no metadata on node");
	let server_id = util::uuid::parse(unwrap!(meta.get("server-id"), "no server-id in metadata"))?;

	msg!([client] nomad::msg::monitor_node_registered(server_id) {
		server_id: Some(server_id.into()),
		node_id: node_id.to_owned(),
	})
	.await?;

	Ok(())
}
