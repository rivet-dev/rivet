use std::{convert::TryInto, net::IpAddr};

use chirp_workflow::prelude::*;

use super::get::ServerRow;
use crate::types::{Filter, Server};

#[derive(Debug)]
pub struct Input {
	pub filter: Filter,
	pub include_destroyed: bool,
	pub exclude_draining: bool,
	pub exclude_no_vlan: bool,
}

#[derive(Debug)]
pub struct Output {
	pub servers: Vec<Server>,
}

#[operation]
pub async fn cluster_server_list(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let servers = sql_fetch_all!(
		[ctx, ServerRow]
		"
		SELECT
			s.server_id,
			s.datacenter_id,
			s.pool_type,
			s.provider_server_id,
			s.vlan_ip,
			s.public_ip,
			s.cloud_destroy_ts,
			CASE
				WHEN s.cloud_destroy_ts IS NOT NULL THEN 6  -- Destroyed
				WHEN s.taint_ts IS NOT NULL AND s.drain_ts IS NOT NULL THEN 5  -- TaintedDraining
				WHEN s.drain_ts IS NOT NULL THEN 4  -- Draining
				WHEN s.taint_ts IS NOT NULL THEN 3  -- Tainted
				WHEN s.install_complete_ts IS NOT NULL THEN 2  -- Running
				WHEN s.provision_complete_ts IS NOT NULL THEN 1  -- Installing
				ELSE 0  -- Provisioning
			END AS state
		FROM db_cluster.servers AS s
		JOIN db_cluster.datacenters AS d
		ON s.datacenter_id = d.datacenter_id
		WHERE
			($1 OR s.cloud_destroy_ts IS NULL) AND
			(NOT $2 OR s.drain_ts IS NULL) AND
			(NOT $3 OR s.vlan_ip IS NOT NULL) AND
			($4 IS NULL OR s.server_id = ANY($4)) AND
			($5 IS NULL OR s.datacenter_id = ANY($5)) AND
			($6 IS NULL OR d.cluster_id = ANY($6)) AND
			($7 IS NULL OR s.pool_type = ANY($7)) AND
			($8 IS NULL OR s.public_ip = ANY($8))
		",
		input.include_destroyed,
		input.exclude_draining,
		input.exclude_no_vlan,
		&input.filter.server_ids,
		&input.filter.datacenter_ids,
		&input.filter.cluster_ids,
		input.filter.pool_types
			.as_ref()
			.map(|x| x.iter()
				.map(|x| *x as i64)
				.collect::<Vec<_>>()
			),
		input.filter.public_ips
			.as_ref()
			.map(|x| x.iter()
				.cloned()
				.map(IpAddr::V4)
				.collect::<Vec<_>>()
			),
	)
	.await?
	.into_iter()
	.map(TryInto::try_into)
	.collect::<GlobalResult<Vec<_>>>()?;

	Ok(Output { servers })
}
