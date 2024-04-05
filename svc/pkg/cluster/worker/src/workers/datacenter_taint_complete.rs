use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

// We wait until a nomad node is registered before destroying tainted servers
#[worker(name = "cluster-datacenter-taint-complete")]
async fn worker(
	ctx: &OperationContext<nomad::msg::monitor_node_registered::Message>,
) -> GlobalResult<()> {
	let server_id = unwrap_ref!(ctx.server_id).as_uuid();

	// NOTE: This does not set the drain ts even though job and gg servers will be drained
	// Mark tainted servers of the same datacenter for destruction in db.
	let tainted_servers = sql_fetch_all!(
		[ctx, (Uuid, i64)]
		"
		UPDATE db_cluster.servers as s2
		SET cloud_destroy_ts = $2
		FROM db_cluster.servers as s1
		WHERE
			s1.datacenter_id = s2.datacenter_id AND
			s1.server_id = $1 AND
			s2.taint_ts IS NOT NULL AND
			s2.cloud_destroy_ts IS NULL
		RETURNING s2.server_id, s2.pool_type
		",
		&server_id,
		util::timestamp::now(),
	)
	.await?;

	// Destroy all tainted servers
	for (server_id, pool_type) in tainted_servers {
		let pool_type = unwrap!(backend::cluster::PoolType::from_i32(pool_type as i32));

		match pool_type {
			backend::cluster::PoolType::Gg | backend::cluster::PoolType::Job => {
				msg!([ctx] cluster::msg::server_drain(server_id) {
					server_id: Some(server_id.into()),
				})
				.await?;
			}
			backend::cluster::PoolType::Ats => {
				msg!([ctx] cluster::msg::server_destroy(server_id) {
					server_id: Some(server_id.into()),
					force: false,
				})
				.await?;
			}
		}
	}

	Ok(())
}
