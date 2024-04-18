use std::net::Ipv4Addr;

use api_helper::{anchor::WatchIndexQuery, ctx::Ctx};
use proto::backend;
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
	let servers_res = op!([ctx] cluster_server_resolve_for_ip {
		ips: vec![public_ip.to_string()],
	})
	.await?;

	let server = unwrap!(servers_res.servers.first(), "server not found");
	let server_id = unwrap!(server.server_id);

	// Get server info
	let server_res = op!([ctx] cluster_server_get {
		server_ids: vec![server_id],
	})
	.await?;
	let server = unwrap!(server_res.servers.first(), "server not found");

	// Get datacenter info
	let datacenter_id = unwrap!(server.datacenter_id);
	let datacenter_res = op!([ctx] cluster_datacenter_get {
		datacenter_ids: vec![datacenter_id],
	})
	.await?;
	let datacenter = unwrap!(datacenter_res.datacenters.first());

	let pool_type = unwrap!(backend::cluster::PoolType::from_i32(server.pool_type));
	let name = util_cluster::server_name(
		&datacenter.provider_datacenter_id,
		pool_type,
		server_id.as_uuid(),
	);

	Ok(models::ProvisionServersGetInfoResponse {
		name,
		server_id: server_id.as_uuid(),
		datacenter_id: datacenter_id.as_uuid(),
		cluster_id: unwrap_ref!(server.cluster_id).as_uuid(),
		vlan_ip: unwrap_ref!(server.vlan_ip, "server should have vlan ip by now").clone(),
	})
}
