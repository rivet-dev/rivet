use std::net::Ipv4Addr;

use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use rivet_api::models;
use rivet_operation::prelude::*;

use crate::auth::Auth;

// MARK: GET /servers/{}/info
pub async fn info(
	ctx: Ctx<Auth>,
	public_ip: Ipv4Addr,
	_watch_index: WatchIndexQuery,
) -> GlobalResult<models::ProvisionServersGetInfoResponse> {
	ctx.auth().server()?;

	// Find server based on public ip
	let servers_res = ctx
		.op(cluster::ops::server::resolve_for_ip::Input {
			ips: vec![public_ip],
			// We include destroyed because this request can only ever come from a running server. This means
			// the server was marked as destroyed, but is still provisioning.
			include_destroyed: true,
		})
		.await?;
	let server = unwrap!(servers_res.servers.first(), "server with ip not found");

	// Get server info
	let server_res = ctx
		.op(cluster::ops::server::get::Input {
			server_ids: vec![server.server_id],
		})
		.await?;
	let server = unwrap!(server_res.servers.first(), "server not found");

	// Get datacenter info
	let datacenter_res = ctx
		.op(cluster::ops::datacenter::get::Input {
			datacenter_ids: vec![server.datacenter_id],
		})
		.await?;
	let datacenter = unwrap!(datacenter_res.datacenters.first());

	let name = cluster::util::server_name(
		&datacenter.provider_datacenter_id,
		server.pool_type,
		server.server_id,
	);

	Ok(models::ProvisionServersGetInfoResponse {
		name,
		server_id: server.server_id,
		datacenter_id: server.datacenter_id,
		cluster_id: datacenter.cluster_id,
		vlan_ip: unwrap_ref!(server.vlan_ip, "server should have vlan ip by now").to_string(),
	})
}
