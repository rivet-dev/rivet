use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use redis::AsyncCommands;
use serde::Deserialize;

lazy_static::lazy_static! {
	static ref NOMAD_CONFIG: nomad_client::apis::configuration::Configuration =
		nomad_util::config_from_env().unwrap();
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PlanResult {
	allocation: nomad_client::models::Allocation,
}

#[derive(Debug, sqlx::FromRow)]
struct RunRow {
	run_id: Uuid,
	region_id: Uuid,
	alloc_plan_ts: Option<i64>,
}

#[derive(Debug, sqlx::FromRow)]
struct ProxiedPort {
	target_nomad_port_label: Option<String>,
	ingress_port: i64,
	ingress_hostnames: Vec<String>,
	proxy_protocol: i64,
	ssl_domain_mode: i64,
}

#[derive(Clone)]
struct RunData {
	job_id: String,
	alloc_id: String,
	nomad_node_id: String,
	nomad_node_name: String,
	nomad_node_public_ipv4: String,
	nomad_node_vlan_ipv4: String,
	run_networks: Vec<backend::job::Network>,
	ports: Vec<backend::job::Port>,
}

#[worker(name = "job-run-nomad-monitor-alloc-plan")]
async fn worker(
	ctx: &OperationContext<job_run::msg::nomad_monitor_alloc_plan::Message>,
) -> GlobalResult<()> {
	let crdb = ctx.crdb().await?;
	let mut redis_job = ctx.redis_job().await?;

	let PlanResult { allocation: alloc } = serde_json::from_str::<PlanResult>(&ctx.payload_json)?;

	let job_id = unwrap_ref!(alloc.job_id, "alloc has no job id");
	let alloc_id = unwrap_ref!(alloc.ID);
	let nomad_node_id = unwrap_ref!(alloc.node_id, "alloc has no node id");
	let nomad_node_name = unwrap_ref!(alloc.node_id, "alloc has no node name");

	if !util_job::is_nomad_job_run(job_id) {
		tracing::info!(%job_id, "disregarding event");
		return Ok(());
	}

	// Fetch node metadata
	let node = nomad_client::apis::nodes_api::get_node(
		&NOMAD_CONFIG,
		&nomad_node_id,
		None,
		None,
		None,
		None,
	)
	.await?;
	let mut meta = unwrap!(node.meta);

	// Read ports
	let mut run_networks = Vec::new();
	let mut ports = Vec::new();
	let alloc_resources = unwrap_ref!(alloc.resources);
	if let Some(networks) = &alloc_resources.networks {
		for network in networks {
			let network_mode = unwrap_ref!(network.mode);
			let network_ip = unwrap_ref!(network.IP);

			run_networks.push(backend::job::Network {
				mode: network_mode.clone(),
				ip: network_ip.clone(),
			});

			if let Some(dynamic_ports) = &network.dynamic_ports {
				for port in dynamic_ports {
					// Don't share connect proxy ports
					let label = unwrap_ref!(port.label);
					ports.push(backend::job::Port {
						label: label.clone(),
						source: *unwrap_ref!(port.value) as u32,
						target: *unwrap_ref!(port.to) as u32,
						ip: network_ip.clone(),
					});
				}
			}
		}
	} else {
		tracing::info!("no network returned");
	}

	// Fetch the run
	//
	// Backoff mitigates race condition with job-run-create not having inserted
	// the dispatched_job_id yet.
	let run_data = RunData {
		job_id: job_id.clone(),
		alloc_id: alloc_id.clone(),
		nomad_node_id: nomad_node_id.clone(),
		nomad_node_name: unwrap!(node.name),
		nomad_node_public_ipv4: unwrap!(meta.remove("network-public-ipv4")),
		nomad_node_vlan_ipv4: unwrap!(meta.remove("network-vlan-ipv4")),
		run_networks: run_networks.clone(),
		ports: ports.clone(),
	};
	let db_output = rivet_pools::utils::crdb::tx(&crdb, |tx| {
		let now = ctx.ts();
		let run_data = run_data.clone();
		Box::pin(async move { update_db(tx, now, run_data).await })
	})
	.await?;

	// Check if run found
	let Some(DbOutput {
		run_id,
		region_id,
		proxied_ports,
	}) = db_output
	else {
		if ctx.req_dt() > util::duration::minutes(5) {
			tracing::error!("discarding stale message");
			return Ok(());
		} else {
			retry_bail!("run not found, may be race condition with insertion");
		}
	};

	// Write the port to the cache
	{
		let msg = job::redis_job::RunProxiedPorts {
			run_id: Some(run_id.into()),
			proxied_ports: proxied_ports
				.iter()
				.filter_map(|pp| {
					ports
						.iter()
						.find(|p| Some(&p.label) == pp.target_nomad_port_label.as_ref())
						.map(|p| (p, pp))
				})
				.map(
					|(port, proxied_port)| job::redis_job::run_proxied_ports::ProxiedPort {
						ip: port.ip.clone(),
						source: port.source,
						target_nomad_port_label: proxied_port.target_nomad_port_label.clone(),
						ingress_port: proxied_port.ingress_port as u32,
						ingress_hostnames: proxied_port.ingress_hostnames.clone(),
						proxy_protocol: proxied_port.proxy_protocol as i32,
						ssl_domain_mode: proxied_port.ssl_domain_mode as i32,
					},
				)
				.collect(),
		};
		let mut buf = Vec::with_capacity(msg.encoded_len());
		msg.encode(&mut buf)?;

		let write_perf = ctx.perf().start("write-proxied-ports-redis").await;
		tracing::info!(proxied_ports = ?msg, "writing job run proxied ports to cache");
		redis_job
			.hset(
				util_job::key::proxied_ports(region_id),
				run_id.to_string(),
				buf,
			)
			.await?;
		write_perf.end();
	}

	tracing::info!(%job_id, %run_id, "updated run");
	msg!([ctx] job_run::msg::alloc_planned(run_id) {
		run_id: Some(run_id.into()),
		run_meta: Some(job_run::msg::alloc_planned::message::RunMeta::Nomad(job_run::msg::alloc_planned::message::Nomad {
			alloc_id: alloc_id.clone(),
			node_id: nomad_node_id.clone(),
		})),
	})
	.await?;
	msg!([ctx] job_run::msg::ports_resolved(run_id) {
		run_id: Some(run_id.into()),
	})
	.await?;

	Ok(())
}

#[derive(Debug)]
struct DbOutput {
	run_id: Uuid,
	region_id: Uuid,
	proxied_ports: Vec<ProxiedPort>,
}

/// Returns `None` if the run could not be found.
#[tracing::instrument(skip_all)]
async fn update_db(
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	now: i64,
	RunData {
		job_id,
		alloc_id,
		nomad_node_id,
		nomad_node_name,
		nomad_node_public_ipv4,
		nomad_node_vlan_ipv4,
		run_networks,
		ports,
	}: RunData,
) -> GlobalResult<Option<DbOutput>> {
	let run_row = sqlx::query_as::<_, RunRow>(indoc!(
		"
		SELECT runs.run_id, runs.region_id, run_meta_nomad.alloc_plan_ts
		FROM db_job_state.run_meta_nomad
		INNER JOIN db_job_state.runs ON runs.run_id = run_meta_nomad.run_id
		WHERE dispatched_job_id = $1
		FOR UPDATE OF run_meta_nomad
		"
	))
	.bind(&job_id)
	.fetch_optional(&mut **tx)
	.await?;

	// Check if run found
	let run_row = if let Some(run_row) = run_row {
		run_row
	} else {
		tracing::info!("caught race condition with job-run-create");
		return Ok(None);
	};
	let run_id = run_row.run_id;

	// Write run meta on first plan
	if run_row.alloc_plan_ts.is_none() {
		// Write alloc information
		sqlx::query(indoc!(
			"
			UPDATE db_job_state.run_meta_nomad
			SET alloc_id = $2, alloc_plan_ts = $3, node_id = $4, name = $5, public_ipv4 = $6, vlan_ipv4 = $7
			WHERE run_id = $1
			"
		))
		.bind(run_row.run_id)
		.bind(&alloc_id)
		.bind(now)
		.bind(&nomad_node_id)
		.bind(&nomad_node_name)
		.bind(&nomad_node_public_ipv4)
		.bind(&nomad_node_vlan_ipv4)
		.execute(&mut **tx)
		.await?;

		// Save the ports to the db
		for network in &run_networks {
			tracing::info!(%run_id, mode = %network.mode, ip = %network.ip, "inserting network");
			sqlx::query(indoc!(
				"
				INSERT INTO db_job_state.run_networks (run_id, mode, ip)
				VALUES ($1, $2, $3)
				"
			))
			.bind(run_id)
			.bind(&network.mode)
			.bind(&network.ip)
			.execute(&mut **tx)
			.await?;
		}

		// Save the ports to the db
		for port in &ports {
			tracing::info!(%run_id, label = %port.label, source = port.source, target = port.target, ip = %port.ip, "inserting port");
			sqlx::query(indoc!(
				"
				INSERT INTO db_job_state.run_ports (run_id, label, source, target, ip)
				VALUES ($1, $2, $3, $4, $5)
				"
			))
			.bind(run_id)
			.bind(&port.label)
			.bind(port.source as i64)
			.bind(port.target as i64)
			.bind(&port.ip)
			.execute(&mut **tx)
			.await?;
		}
	}

	// Update the run ports
	let proxied_ports = sqlx::query_as::<_, ProxiedPort>(indoc!(
		"
		SELECT target_nomad_port_label, ingress_port, ingress_hostnames, proxy_protocol, ssl_domain_mode
		FROM db_job_state.run_proxied_ports
		WHERE run_id = $1
		"
	))
	.bind(run_id)
	.fetch_all(&mut **tx)
	.await?;
	tracing::info!(?proxied_ports, "fetched proxied ports");

	// Validate ports match proxied ports
	for proxied_port in &proxied_ports {
		ensure!(
			ports
				.iter()
				.any(|port| Some(&port.label) == proxied_port.target_nomad_port_label.as_ref()),
			"no matching port with proxied target"
		);
	}

	Ok(Some(DbOutput {
		run_id,
		region_id: run_row.region_id,
		proxied_ports,
	}))
}
