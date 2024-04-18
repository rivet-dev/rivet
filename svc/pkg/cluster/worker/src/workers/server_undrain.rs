use chirp_worker::prelude::*;
use futures_util::FutureExt;
use nomad_client::{
	apis::{configuration::Configuration, nodes_api},
	models,
};
use proto::backend::{self, pkg::*};

lazy_static::lazy_static! {
	static ref NOMAD_CONFIG: Configuration = nomad_util::new_config_from_env().unwrap();
}

#[derive(sqlx::FromRow)]
struct Server {
	datacenter_id: Uuid,
	pool_type: i64,
	nomad_node_id: Option<String>,
	is_draining: bool,
}

#[worker(name = "cluster-server-undrain")]
async fn worker(ctx: &OperationContext<cluster::msg::server_undrain::Message>) -> GlobalResult<()> {
	rivet_pools::utils::crdb::tx_no_retry(&ctx.crdb().await?, |tx| inner(ctx.clone(), tx).boxed())
		.await?;

	Ok(())
}

async fn inner(
	ctx: OperationContext<cluster::msg::server_undrain::Message>,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> GlobalResult<()> {
	let server_id = unwrap_ref!(ctx.server_id).as_uuid();

	// NOTE: `drain_ts` will already be set to null before this worker is called
	let server = sql_fetch_one!(
		[ctx, Server, @tx tx]
		"
		SELECT
			datacenter_id,
			pool_type,
			nomad_node_id,
			(drain_ts IS NOT NULL) AS is_draining
		FROM db_cluster.servers
		WHERE server_id = $1
		FOR UPDATE
		",
		server_id,
	)
	.await?;

	if server.is_draining {
		tracing::error!("attempting to undrain server that was not set as undraining");
		return Ok(());
	}

	let pool_type = unwrap!(backend::cluster::PoolType::from_i32(
		server.pool_type as i32
	));
	match pool_type {
		backend::cluster::PoolType::Job => {
			// This worker will never be called if the server has no nomad instance running. This should be an
			// unreachable log.
			let Some(nomad_node_id) = server.nomad_node_id else {
				tracing::error!("server does not have nomad running, cannot undrain");
				return Ok(());
			};

			nodes_api::update_node_drain(
				&NOMAD_CONFIG,
				&nomad_node_id,
				models::NodeUpdateDrainRequest {
					drain_spec: None,
					mark_eligible: Some(true),
					meta: None,
					node_id: Some(nomad_node_id.clone()),
				},
				None,
				None,
				None,
				None,
				None,
				None,
				None,
				None,
				None,
			)
			.await?;

			// Allow new matchmaker requests to the node running on this server
			msg!([ctx] mm::msg::nomad_node_closed_set(&nomad_node_id) {
				datacenter_id: Some(server.datacenter_id.into()),
				nomad_node_id: nomad_node_id.clone(),
				is_closed: false,
			})
			.await?;
		}
		backend::cluster::PoolType::Gg => {
			// Recreate DNS record
			msg!([ctx] cluster::msg::server_dns_create(server_id) {
				server_id: Some(server_id.into()),
			})
			.await?;
		}
		_ => {
			// Gracefully fail
			tracing::error!("cannot undrain this pool type: {:?}", pool_type);
		}
	}

	Ok(())
}
