use chirp_worker::prelude::*;
use proto::backend::pkg::*;
use std::collections::HashSet;

#[worker(name = "cluster-server-taint")]
async fn worker(ctx: &OperationContext<cluster::msg::server_taint::Message>) -> GlobalResult<()> {
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

	// Taint server records. These will be incrementally drained and destroyed by `cluster-datacenter-scale`
	let updated_dc_ids = sql_fetch_all!(
		[ctx, (Uuid,)]
		"
		UPDATE db_cluster.servers AS s
		SET taint_ts = $1
		FROM db_cluster.datacenters AS d
		WHERE
			s.datacenter_id = d.datacenter_id
			AND s.taint_ts IS NULL
			AND ($2 IS NULL OR s.server_id = ANY($2))
			AND ($3 IS NULL OR d.cluster_id = ANY($3))
			AND ($4 IS NULL OR s.datacenter_id = ANY($4))
			AND ($5 IS NULL OR s.pool_type = ANY($5))
			AND ($6 IS NULL OR s.public_ip = ANY($6::inet[]))
		RETURNING s.datacenter_id
		",
		util::timestamp::now(),
		&server_ids,
		&cluster_ids,
		&datacenter_ids,
		&pool_types,
		&public_ips,
	)
	.await?;

	// Trigger rescale in affected datacenters
	let updated_dc_ids = updated_dc_ids
		.into_iter()
		.map(|x| x.0)
		.collect::<HashSet<Uuid>>();
	for dc_id in updated_dc_ids {
		msg!([ctx] cluster::msg::datacenter_scale(dc_id) {
			datacenter_id: Some(dc_id.into()),
		})
		.await?;
	}

	Ok(())
}
