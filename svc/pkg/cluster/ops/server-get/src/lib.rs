use std::net::IpAddr;

use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Server {
	server_id: Uuid,
	datacenter_id: Uuid,
	pool_type: i64,
	vlan_ip: Option<IpAddr>,
	public_ip: Option<IpAddr>,
	cloud_destroy_ts: Option<i64>,
}

impl From<Server> for backend::cluster::Server {
	fn from(value: Server) -> Self {
		backend::cluster::Server {
			server_id: Some(value.server_id.into()),
			datacenter_id: Some(value.datacenter_id.into()),
			pool_type: value.pool_type as i32,
			vlan_ip: value.vlan_ip.map(|ip| ip.to_string()),
			public_ip: value.public_ip.map(|ip| ip.to_string()),
			cloud_destroy_ts: value.cloud_destroy_ts,
		}
	}
}

#[operation(name = "cluster-server-get")]
pub async fn handle(
	ctx: OperationContext<cluster::server_get::Request>,
) -> GlobalResult<cluster::server_get::Response> {
	let server_ids = ctx
		.server_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let servers = sql_fetch_all!(
		[ctx, Server]
		"
		SELECT
			server_id,
			datacenter_id,
			pool_type,
			vlan_ip,
			public_ip,
			cloud_destroy_ts
		FROM db_cluster.servers
		WHERE server_id = ANY($1)
		",
		server_ids
	)
	.await?;

	Ok(cluster::server_get::Response {
		servers: servers.into_iter().map(Into::into).collect::<Vec<_>>(),
	})
}
