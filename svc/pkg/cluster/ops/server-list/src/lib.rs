use std::collections::HashMap;

use proto::backend::pkg::*;
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Server {
	cluster_id: Uuid,
	server_id: Uuid,
}

#[operation(name = "cluster-server-list")]
pub async fn handle(
	ctx: OperationContext<cluster::server_list::Request>,
) -> GlobalResult<cluster::server_list::Response> {
	let cluster_ids = ctx
		.cluster_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let servers = if ctx.include_destroyed {
		sql_fetch_all!(
			[ctx, Server]
			"
			SELECT
				cluster_id,
				server_id
			FROM db_cluster.servers
			WHERE
				cluster_id = ANY($1) AND
				taint_ts IS NULL
			",
			&cluster_ids
		)
		.await?
	} else {
		sql_fetch_all!(
			[ctx, Server]
			"
			SELECT
				cluster_id,
				server_id
			FROM db_cluster.servers
			WHERE
				cluster_id = ANY($1) AND
				cloud_destroy_ts IS NULL AND
				taint_ts IS NULL
			",
			&cluster_ids
		)
		.await?
	};

	// Fill in empty clusters
	let mut dcs_by_cluster_id = cluster_ids
		.iter()
		.map(|cluster_id| (*cluster_id, Vec::new()))
		.collect::<HashMap<_, Vec<Uuid>>>();

	for dc in servers {
		dcs_by_cluster_id
			.entry(dc.cluster_id)
			.or_default()
			.push(dc.server_id);
	}

	Ok(cluster::server_list::Response {
		clusters: dcs_by_cluster_id
			.into_iter()
			.map(
				|(cluster_id, server_ids)| cluster::server_list::response::Cluster {
					cluster_id: Some(cluster_id.into()),
					server_ids: server_ids.into_iter().map(Into::into).collect::<Vec<_>>(),
				},
			)
			.collect::<Vec<_>>(),
	})
}
