use chirp_worker::prelude::*;
use futures_util::FutureExt;
use proto::backend::{self, pkg::*};

#[derive(sqlx::FromRow)]
struct Server {
	datacenter_id: Uuid,
	pool_type: i64,
	provider_server_id: Option<String>,
}

#[worker(name = "cluster-server-destroy")]
async fn worker(ctx: &OperationContext<cluster::msg::server_destroy::Message>) -> GlobalResult<()> {
	// TODO: RVTEE-75
	rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| inner(ctx.clone(), tx).boxed()).await?;

	Ok(())
}

async fn inner(
	ctx: OperationContext<cluster::msg::server_destroy::Message>,
	_tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> GlobalResult<()> {
	let server_id = unwrap_ref!(ctx.server_id).as_uuid();

	// NOTE: We do not check if `cloud_destroy_ts` is set because of the destroy fn in
	// `cluster-server-provision`
	let server = sql_fetch_one!(
		[ctx, Server]
		"
		SELECT datacenter_id, pool_type, provider_server_id
		FROM db_cluster.servers
		WHERE server_id = $1
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
				datacenter_id: Some(server.datacenter_id.into()),
			})
			.await?;
		}
	}

	// Delete DNS record
	let pool_type = unwrap!(backend::cluster::PoolType::from_i32(
		server.pool_type as i32
	));
	if let backend::cluster::PoolType::Gg = pool_type {
		// Update db record
		sql_execute!(
			[ctx]
			"
			UPDATE db_cluster.servers_cloudflare
			SET destroy_ts = $2
			WHERE
				server_id = $1 AND
				destroy_ts IS NULL
			",
			server_id,
			util::timestamp::now(),
		)
		.await?;

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
