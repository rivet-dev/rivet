use std::{
	convert::{TryFrom, TryInto},
	net::IpAddr,
};

use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Server {
	server_id: Uuid,
	cluster_id: Uuid,
	datacenter_id: Uuid,
	pool_type: i64,
	vlan_ip: Option<IpAddr>,
	public_ip: Option<IpAddr>,
	cloud_destroy_ts: Option<i64>,
}

impl TryFrom<Server> for backend::cluster::Server {
	type Error = GlobalError;

	fn try_from(value: Server) -> GlobalResult<Self> {
		Ok(backend::cluster::Server {
			server_id: Some(value.server_id.into()),
			cluster_id: Some(value.cluster_id.into()),
			datacenter_id: Some(value.datacenter_id.into()),
			pool_type: value.pool_type.try_into()?,
			vlan_ip: value.vlan_ip.map(|ip| ip.to_string()),
			public_ip: value.public_ip.map(|ip| ip.to_string()),
			cloud_destroy_ts: value.cloud_destroy_ts,
		})
	}
}

#[operation(name = "cluster-server-list")]
pub async fn handle(
	ctx: OperationContext<cluster::server_list::Request>,
) -> GlobalResult<cluster::server_list::Response> {
	let filter = unwrap_ref!(ctx.filter);

	let server_ids = if filter.filter_server_ids {
		Some(
			filter
				.server_ids
				.iter()
				.map(|&x| x.into())
				.collect::<Vec<Uuid>>(),
		)
	} else {
		None
	};
	let cluster_ids = if filter.filter_cluster_ids {
		Some(
			filter
				.cluster_ids
				.iter()
				.map(|&x| x.into())
				.collect::<Vec<Uuid>>(),
		)
	} else {
		None
	};
	let datacenter_ids = if filter.filter_datacenter_ids {
		Some(
			filter
				.datacenter_ids
				.iter()
				.map(|&x| x.into())
				.collect::<Vec<Uuid>>(),
		)
	} else {
		None
	};
	let pool_types = if filter.filter_pool_types {
		Some(&filter.pool_types)
	} else {
		None
	};
	let public_ips = if filter.filter_public_ips {
		Some(&filter.public_ips)
	} else {
		None
	};

	let servers = sql_fetch_all!(
		[ctx, Server]
		"
		SELECT
			s.server_id,
            d.cluster_id,
			s.datacenter_id,
			s.pool_type,
			s.vlan_ip,
			s.public_ip,
			s.cloud_destroy_ts
		FROM db_cluster.servers AS s
		JOIN db_cluster.datacenters AS d
		ON s.datacenter_id = d.datacenter_id
		WHERE
			($1 OR s.cloud_destroy_ts IS NULL)
			AND ($2 IS NULL OR s.server_id = ANY($2))
			AND ($3 IS NULL OR d.cluster_id = ANY($3))
			AND ($4 IS NULL OR s.datacenter_id = ANY($4))
			AND ($5 IS NULL OR s.pool_type = ANY($5))
			AND ($6 IS NULL OR s.public_ip = ANY($6::inet[]))
		",
		ctx.include_destroyed,
		&server_ids,
		&cluster_ids,
		&datacenter_ids,
		&pool_types,
		&public_ips,
	)
	.await?;

	Ok(cluster::server_list::Response {
		servers: servers
			.into_iter()
			.map(TryInto::try_into)
			.collect::<GlobalResult<Vec<_>>>()?,
	})
}
