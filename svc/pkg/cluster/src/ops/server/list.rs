use std::{convert::TryInto, net::IpAddr};

use chirp_workflow::prelude::*;

use super::get::ServerRow;
use crate::types::{Filter, Server};

#[derive(Debug)]
pub struct Input {
	pub filter: Filter,
	pub include_destroyed: bool,
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
			s.pool_type2,
			s.vlan_ip,
			s.public_ip,
			s.cloud_destroy_ts
		FROM db_cluster.servers AS s
		JOIN db_cluster.datacenters AS d
		ON s.datacenter_id = d.datacenter_id
		WHERE
			($1 OR s.cloud_destroy_ts IS NULL)
			AND ($2 IS NULL OR s.server_id = ANY($2))
			AND ($3 IS NULL OR s.datacenter_id = ANY($3))
			AND ($4 IS NULL OR d.cluster_id = ANY($4))
			AND ($5 IS NULL OR s.pool_type2 = ANY($5::JSONB[]))
			AND ($6 IS NULL OR s.public_ip = ANY($6))
		",
		input.include_destroyed,
		&input.filter.server_ids,
		&input.filter.datacenter_ids,
		&input.filter.cluster_ids,
		input.filter.pool_types
			.as_ref()
			.map(|x| x.iter()
				.map(serde_json::to_string)
				.collect::<Result<Vec<_>, _>>()
			).transpose()?,
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
