use std::net::{IpAddr, Ipv4Addr};

use chirp_worker::prelude::*;
use futures_util::FutureExt;
use proto::backend::{self, cluster::PoolType, pkg::*};
use rand::Rng;
use util_cluster::metrics;

struct ProvisionResponse {
	provider_server_id: String,
	provider_hardware: String,
	public_ip: String,
	already_installed: bool,
}

// More than the timeout in linode-server-provision
#[worker(name = "cluster-server-provision", timeout = 300)]
async fn worker(
	ctx: &OperationContext<cluster::msg::server_provision::Message>,
) -> GlobalResult<()> {
	// TODO: RVTEE-75
	rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| inner(ctx.clone(), tx).boxed()).await?;

	Ok(())
}

async fn inner(
	ctx: OperationContext<cluster::msg::server_provision::Message>,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
) -> GlobalResult<()> {
	let datacenter_id = unwrap!(ctx.datacenter_id);
	let server_id = unwrap_ref!(ctx.server_id).as_uuid();
	let pool_type = unwrap!(backend::cluster::PoolType::from_i32(ctx.pool_type));
	let provider = unwrap!(backend::cluster::Provider::from_i32(ctx.provider));

	// Check if server is already provisioned
	// NOTE: sql record already exists before this worker is called
	let (provider_server_id,) = sql_fetch_one!(
		[ctx, (Option<String>,)]
		"
		SELECT
			provider_server_id
		FROM db_cluster.servers
		WHERE server_id = $1
		",
		server_id,
	)
	.await?;
	if let Some(provider_server_id) = provider_server_id {
		tracing::warn!(
			?server_id,
			?provider_server_id,
			"server is already provisioned"
		);
		return Ok(());
	}

	// Fetch datacenter config
	let datacenter_res = op!([ctx] cluster_datacenter_get {
		datacenter_ids: vec![datacenter_id],
	})
	.await?;
	let datacenter = unwrap!(datacenter_res.datacenters.first());
	let pool = unwrap!(
		datacenter
			.pools
			.iter()
			.find(|p| p.pool_type == ctx.pool_type),
		"datacenter does not have this type of pool configured"
	);

	// Get a new vlan ip
	let vlan_ip = get_vlan_ip(&ctx, tx, server_id, pool_type).await?;

	sql_execute!(
		[ctx]
		"
		UPDATE db_cluster.servers
		SET vlan_ip = $2
		WHERE server_id = $1
		",
		server_id,
		IpAddr::V4(vlan_ip),
	)
	.await?;

	// Iterate through list of hardware and attempt to schedule a server. Goes to the next
	// hardware if an error happens during provisioning
	let mut hardware_list = pool.hardware.iter();
	let provision_res = loop {
		// List exhausted
		let Some(hardware) = hardware_list.next() else {
			break None;
		};

		tracing::info!(
			"attempting to provision hardware: {}",
			hardware.provider_hardware
		);

		match provider {
			backend::cluster::Provider::Linode => {
				let res = op!([ctx] linode_server_provision {
					datacenter_id: ctx.datacenter_id,
					server_id: ctx.server_id,
					provider_datacenter_id: datacenter.provider_datacenter_id.clone(),
					hardware: Some(hardware.clone()),
					pool_type: ctx.pool_type,
					vlan_ip: vlan_ip.to_string(),
					tags: ctx.tags.clone(),
					use_prebakes: datacenter.prebakes_enabled,
				})
				.await;

				match res {
					Ok(res) => {
						break Some(ProvisionResponse {
							provider_server_id: res.provider_server_id.clone(),
							provider_hardware: hardware.provider_hardware.clone(),
							public_ip: res.public_ip.clone(),
							already_installed: res.already_installed,
						})
					}
					Err(err) => {
						tracing::error!(
							?err,
							?server_id,
							"failed to provision server, cleaning up"
						);

						cleanup(&ctx, server_id).await?;
					}
				}
			}
		}
	};

	if let Some(provision_res) = provision_res {
		let provision_complete_ts = util::timestamp::now();

		let (create_ts,) = sql_fetch_one!(
			[ctx, (i64,)]
			"
			UPDATE db_cluster.servers
			SET
				provider_server_id = $2,
				provider_hardware = $3,
				public_ip = $4,
				provision_complete_ts = $5,
				install_complete_ts = $6
			WHERE server_id = $1
			RETURNING create_ts
			",
			server_id,
			&provision_res.provider_server_id,
			&provision_res.provider_hardware,
			&provision_res.public_ip,
			provision_complete_ts,
			if provision_res.already_installed {
				Some(provision_complete_ts)
			} else {
				None
			},
		)
		.await?;

		if provision_res.already_installed {
			// Create DNS record because the server is already installed
			if let backend::cluster::PoolType::Gg = pool_type {
				// Source of truth record
				sql_execute!(
					[ctx]
					"
					INSERT INTO db_cluster.servers_cloudflare (server_id)
					VALUES ($1)
					",
					server_id,
				)
				.await?;

				msg!([ctx] cluster::msg::server_dns_create(server_id) {
					server_id: ctx.server_id,
				})
				.await?;
			}
		}
		// Install components on server
		else {
			let request_id = Uuid::new_v4();

			msg!([ctx] cluster::msg::server_install(request_id) {
				request_id: Some(request_id.into()),
				public_ip: provision_res.public_ip,
				datacenter_id: ctx.datacenter_id,
				server_id: ctx.server_id,
				pool_type: ctx.pool_type,
				provider: ctx.provider,
				initialize_immediately: true,
			})
			.await?;
		}

		insert_metrics(datacenter, &pool_type, provision_complete_ts, create_ts).await?;
	} else {
		tracing::error!(?server_id, hardware_options=?pool.hardware.len(), "failed all attempts to provision server");
		bail!("failed all attempts to provision server");
	}

	Ok(())
}

async fn get_vlan_ip(
	ctx: &OperationContext<cluster::msg::server_provision::Message>,
	_tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	server_id: Uuid,
	pool_type: backend::cluster::PoolType,
) -> GlobalResult<Ipv4Addr> {
	// Find next available vlan index
	let mut vlan_addr_range = match pool_type {
		PoolType::Job => util::net::job::vlan_addr_range(),
		PoolType::Gg => util::net::gg::vlan_addr_range(),
		PoolType::Ats => util::net::ats::vlan_addr_range(),
	};
	let max_idx = vlan_addr_range.count() as i64;
	let (network_idx,) = sql_fetch_one!(
		[ctx, (i64,)]
		"
		WITH
			get_next_network_idx AS (
				SELECT mod(idx + $1, $2) AS idx
				FROM generate_series(0, $2) AS s(idx)
				WHERE NOT EXISTS (
					SELECT 1
					FROM db_cluster.servers
					WHERE
						pool_type = $3 AND
						network_idx = mod(idx + $1, $2) AND
						cloud_destroy_ts IS NULL
				)
				LIMIT 1
			),
			update_network_idx AS (
				UPDATE db_cluster.servers
				SET network_idx = (SELECT idx FROM get_next_network_idx) 
				WHERE server_id = $4
				RETURNING 1
			)
		SELECT idx FROM get_next_network_idx
		",
		// Choose a random index to start from for better index spread
		rand::thread_rng().gen_range(0i64..max_idx),
		max_idx,
		pool_type as i64,
		server_id
	)
	.await?;

	let vlan_ip = unwrap!(vlan_addr_range.nth(network_idx as usize));

	Ok(vlan_ip)
}

// This function is used to destroy leftovers from a failed partial provision.
async fn cleanup(
	ctx: &OperationContext<cluster::msg::server_provision::Message>,
	server_id: Uuid,
) -> GlobalResult<()> {
	// NOTE: Usually before publishing this message we would set `cloud_destroy_ts`. We do not set it here
	// because this message will be retried with the same server id

	// Wait for server to complete destroying so we don't get a primary key conflict (the same server id
	// will be used to try and provision the next hardware option)
	msg!([ctx] cluster::msg::server_destroy(server_id) -> cluster::msg::server_destroy_complete {
		server_id: Some(server_id.into()),
		// We force destroy because the provision process failed
		force: true,
	})
	.await?;

	Ok(())
}

async fn insert_metrics(
	dc: &backend::cluster::Datacenter,
	pool_type: &backend::cluster::PoolType,
	provision_complete_ts: i64,
	create_ts: i64,
) -> GlobalResult<()> {
	let datacenter_id = unwrap_ref!(dc.datacenter_id).as_uuid().to_string();
	let cluster_id = unwrap_ref!(dc.cluster_id).as_uuid().to_string();
	let dt = (provision_complete_ts - create_ts) as f64 / 1000.0;

	metrics::PROVISION_DURATION
		.with_label_values(&[
			cluster_id.as_str(),
			datacenter_id.as_str(),
			&dc.provider_datacenter_id,
			&dc.name_id,
			match pool_type {
				backend::cluster::PoolType::Job => "job",
				backend::cluster::PoolType::Gg => "gg",
				backend::cluster::PoolType::Ats => "ats",
			},
		])
		.observe(dt);

	Ok(())
}
