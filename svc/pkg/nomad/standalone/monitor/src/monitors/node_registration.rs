use chirp_workflow::prelude::*;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct NodeRegistration {
	node: nomad_client::models::Node,
}

pub async fn handle(
	ctx: StandaloneCtx,
	NodeRegistration { node }: &NodeRegistration,
) -> GlobalResult<()> {
	let node_id = unwrap_ref!(node.ID);
	let meta = unwrap_ref!(node.meta, "no metadata on node");
	let server_id = util::uuid::parse(unwrap!(meta.get("server-id"), "no server-id in metadata"))?;

	ctx.signal(cluster::workflows::server::NomadRegistered {
		node_id: node_id.to_owned(),
	})
	.tag("server_id", server_id)
	.send()
	.await?;

	Ok(())
}
