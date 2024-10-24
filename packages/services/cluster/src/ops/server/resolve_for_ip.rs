use std::net::{IpAddr, Ipv4Addr};

use chirp_workflow::prelude::*;

#[derive(Debug)]
pub struct Input {
	pub ips: Vec<Ipv4Addr>,
	pub include_destroyed: bool,
}

#[derive(Debug)]
pub struct Output {
	pub servers: Vec<Server>,
}

#[derive(Debug, sqlx::FromRow)]
pub struct Server {
	pub server_id: Uuid,
	pub public_ip: IpAddr,
}

#[operation]
pub async fn cluster_server_resolve_for_ip(
	ctx: &OperationCtx,
	input: &Input,
) -> GlobalResult<Output> {
	let servers = sql_fetch_all!(
		[ctx, Server]
		"
		SELECT server_id, public_ip
		FROM db_cluster.servers
		WHERE
			($1 OR cloud_destroy_ts IS NULL) AND
			public_ip = ANY($2)
		-- When more than one record is returned per IP, sort by create_ts so that the first record is the
		-- latest server created
		ORDER BY create_ts DESC
		",
		input.include_destroyed,
		input.ips
			.iter()
			.cloned()
			.map(IpAddr::V4)
			.collect::<Vec<_>>(),
	)
	.await?;

	Ok(Output { servers })
}
