use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};

#[derive(sqlx::FromRow)]
struct Server {
	datacenter_id: Uuid,
	pool_type: i64,
	provider_server_id: Option<String>,
}

#[worker(name = "cluster-server-destroy")]
async fn worker(ctx: &OperationContext<cluster::msg::server_destroy::Message>) -> GlobalResult<()> {
	let server_id = unwrap_ref!(ctx.server_id).as_uuid();
	let crdb = ctx.crdb().await?;

	let server = sql_fetch_one!(
		[ctx, Server, &crdb]
		"
		SELECT
			datacenter_id, pool_type, provider_server_id
		FROM db_cluster.servers AS s
		LEFT JOIN db_cluster.cloudflare_misc AS cf
		ON s.server_id = cf.server_id
		WHERE s.server_id = $1
		",
		&server_id,
		util::timestamp::now(),
	)
	.await?;
	if server.provider_server_id.is_none() && !ctx.force {
		bail!("server is not completely provisioned yet, retrying");
	}

	let datacenter_res = op!([ctx] cluster_datacenter_get {
		datacenter_ids: vec![server.datacenter_id.into()],
	})
	.await?;
	let datacenter = unwrap!(datacenter_res.datacenters.first());
	let provider = unwrap!(backend::cluster::Provider::from_i32(datacenter.provider));

	match provider {
		backend::cluster::Provider::Linode => {
			tracing::info!(?server_id, "destroying linode server");

			op!([ctx] linode_server_destroy {
				server_id: ctx.server_id,
			})
			.await?;
		}
	}

	// Delete DNS record
	let pool_type = unwrap!(backend::cluster::PoolType::from_i32(
		server.pool_type as i32
	));
	if let backend::cluster::PoolType::Gg = pool_type {
		msg!([ctx] cluster::msg::server_dns_delete(server_id) {
			server_id: ctx.server_id,
		})
		.await?;
	}

	msg!([ctx] cluster::msg::server_destroy_complete(server_id) {
		server_id: ctx.server_id,
	})
	.await?;

	Ok(())
}
