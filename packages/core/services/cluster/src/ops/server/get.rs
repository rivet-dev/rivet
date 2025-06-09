use std::{
	convert::{TryFrom, TryInto},
	net::IpAddr,
};

use chirp_workflow::prelude::*;

use crate::types::{PoolType, Server, ServerState};

#[derive(Debug)]
pub struct Input {
	pub server_ids: Vec<Uuid>,
}

#[derive(Debug)]
pub struct Output {
	pub servers: Vec<Server>,
}

#[derive(sqlx::FromRow)]
pub(crate) struct ServerRow {
	server_id: Uuid,
	datacenter_id: Uuid,
	pool_type: i64,
	provider_server_id: Option<String>,
	vlan_ip: Option<IpAddr>,
	public_ip: Option<IpAddr>,
	create_ts: i64,
	cloud_destroy_ts: Option<i64>,
	state: i64,
}

impl TryFrom<ServerRow> for Server {
	type Error = GlobalError;

	fn try_from(value: ServerRow) -> GlobalResult<Self> {
		Ok(Server {
			server_id: value.server_id,
			datacenter_id: value.datacenter_id,
			pool_type: unwrap!(PoolType::from_repr(value.pool_type.try_into()?)),
			provider_server_id: value.provider_server_id,
			lan_ip: value.vlan_ip,
			wan_ip: value.public_ip,
			create_ts: value.create_ts,
			cloud_destroy_ts: value.cloud_destroy_ts,
			state: unwrap!(ServerState::from_repr(value.state.try_into()?)),
		})
	}
}

#[operation]
pub async fn cluster_server_get(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let servers = sql_fetch_all!(
		[ctx, ServerRow]
		"
		SELECT
			server_id,
			datacenter_id,
			pool_type,
			provider_server_id,
			vlan_ip,
			public_ip,
			create_ts,
			cloud_destroy_ts,
			CASE
				WHEN cloud_destroy_ts IS NOT NULL THEN 6  -- Destroyed
				WHEN taint_ts IS NOT NULL AND drain_ts IS NOT NULL THEN 5  -- TaintedDraining
				WHEN drain_ts IS NOT NULL THEN 4  -- Draining
				WHEN taint_ts IS NOT NULL THEN 3  -- Tainted
				WHEN install_complete_ts IS NOT NULL THEN 2  -- Running
				WHEN provision_complete_ts IS NOT NULL THEN 1  -- Installing
				ELSE 0  -- Provisioning
			END AS state
		FROM db_cluster.servers
		WHERE server_id = ANY($1)
		",
		&input.server_ids,
	)
	.await?
	.into_iter()
	.map(TryInto::try_into)
	.collect::<GlobalResult<Vec<_>>>()?;

	Ok(Output { servers })
}
