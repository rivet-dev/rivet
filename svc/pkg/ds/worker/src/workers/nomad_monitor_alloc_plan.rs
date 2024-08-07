use chirp_worker::prelude::*;
use proto::backend::{self, pkg::*};
use redis::AsyncCommands;
use serde::Deserialize;

use crate::workers::NEW_NOMAD_CONFIG;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct PlanResult {
	allocation: nomad_client::models::Allocation,
}

#[derive(Debug, sqlx::FromRow)]
struct RunRow {
	server_id: Uuid,
	datacenter_id: Uuid,
	stop_ts: Option<i64>,
	nomad_alloc_plan_ts: Option<i64>, // this was nomad_plan_ts
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
	ports: Vec<backend::job::Port>,
}

#[worker(name = "ds-nomad-monitor-alloc-plan")]
async fn worker(
	ctx: &OperationContext<nomad::msg::monitor_alloc_plan::Message>,
) -> GlobalResult<()> {
	let PlanResult { allocation: alloc } = serde_json::from_str::<PlanResult>(&ctx.payload_json)?;
	tracing::info!(?alloc, "from noamddd");

	let job_id = unwrap_ref!(alloc.job_id, "alloc has no job id");
	let alloc_id = unwrap_ref!(alloc.ID);
	let nomad_node_id = unwrap_ref!(alloc.node_id, "alloc has no node id");
	let _nomad_node_name = unwrap_ref!(alloc.node_id, "alloc has no node name");

	// Fetch node metadata
	let node = nomad_client::apis::nodes_api::get_node(
		&NEW_NOMAD_CONFIG,
		nomad_node_id,
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
	let mut meta = unwrap!(node.meta);

	// Read ports
	let mut ports = Vec::new();
	let alloc_resources = unwrap_ref!(alloc.resources);
	if let Some(networks) = &alloc_resources.networks {
		for network in networks {
			let network_ip = unwrap_ref!(network.IP);

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

	// This works
	tracing::info!(?ports, "found protsadf");

	// {"timestamp":"2024-06-28T01:43:24.930496Z","level":"INFO","fields":{"message":"found protsadf","ports":"[Port { label: \"game_testing2\", source: 20202, target: 0, ip: \"10.0.50.97\" }]"},"target":"ds_worker::workers::nomad_monitor_alloc_plan","spans":[{"ray_id":"1c8bfa81-3c80-4a2c-ab7c-2655f6c6a665","req_id":"a44227ad-4f1a-44b8-b4d0-7746dd8a622e","worker_name":"monolith-worker--ds-nomad-monitor-alloc-plan","name":"handle_req"},{"name":"ds-nomad-monitor-alloc-plan","tick_index":0,"name":"handle"}]}

	// Fetch the run
	//
	// Backoff mitigates race condition with job-run-create not having inserted
	// the dispatched_job_id yet.
	let run_data: RunData = RunData {
		job_id: job_id.clone(),
		alloc_id: alloc_id.clone(),
		nomad_node_id: nomad_node_id.clone(),
		nomad_node_name: unwrap!(node.name),
		nomad_node_public_ipv4: unwrap!(meta.remove("network-public-ipv4")),
		nomad_node_vlan_ipv4: unwrap!(meta.remove("network-vlan-ipv4")),
		ports: ports.clone(),
	};
	tokio::time::sleep(std::time::Duration::from_secs(3)).await;
	let db_output = rivet_pools::utils::crdb::tx(&ctx.crdb().await?, |tx| {
		let ctx = ctx.clone();
		let now = ctx.ts();
		let run_data = run_data.clone();
		Box::pin(update_db(ctx, tx, now, run_data))
	})
	.await?;

	// Check if run found
	let Some(DbOutput {
		server_id,
		datacenter_id,
		stop_ts,
	}) = db_output
	else {
		if ctx.req_dt() > util::duration::minutes(5) {
			tracing::error!("discarding stale message");
			return Ok(());
		} else {
			retry_bail!("run not found, may be race condition with insertion");
		}
	};

	tracing::info!(%job_id, %server_id, "updated run");

	Ok(())
}

#[derive(Debug)]
struct DbOutput {
	server_id: Uuid,
	datacenter_id: Uuid,
	stop_ts: Option<i64>,
}

/// Returns `None` if the run could not be found.
#[tracing::instrument(skip_all)]
async fn update_db(
	ctx: OperationContext<nomad::msg::monitor_alloc_plan::Message>,
	tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
	now: i64,
	RunData {
		job_id,
		alloc_id,
		nomad_node_id,
		nomad_node_name,
		nomad_node_public_ipv4,
		nomad_node_vlan_ipv4,
		ports,
	}: RunData,
) -> GlobalResult<Option<DbOutput>> {
	tracing::info!(?ports, "got the portdatda");

	let run_row = sql_fetch_optional!(
		[ctx, RunRow, @tx tx]
		"
		SELECT
			servers.server_id,
			servers.datacenter_id,
			servers.stop_ts,
			server_nomad.nomad_alloc_plan_ts
		FROM
			db_dynamic_servers.server_nomad
		INNER JOIN
			db_dynamic_servers.servers
		ON
			servers.server_id = server_nomad.server_id
		WHERE
			server_nomad.nomad_dispatched_job_id = $1
		FOR UPDATE OF
			server_nomad
		",
		&job_id,
	)
	.await?;
	tracing::info!(?job_id, "checking jobid");

	tracing::info!(?run_row, "ayy event2a");

	// Check if run found
	let run_row = if let Some(run_row) = run_row {
		run_row
	} else {
		tracing::info!("caught race condition with ds-server-create");
		return Ok(None);
	};
	let server_id = run_row.server_id;

	tracing::info!("ayy event2b");

	// Write run meta on first plan
	if run_row.nomad_alloc_plan_ts.is_none() {
		// Write alloc information
		sql_execute!(
			[ctx, @tx tx]
			"
			UPDATE
				db_dynamic_servers.server_nomad
			SET
				nomad_alloc_id = $2,
				nomad_alloc_plan_ts = $3,
				nomad_node_id = $4,
				nomad_node_name = $5,
				nomad_node_public_ipv4 = $6,
				nomad_node_vlan_ipv4 = $7
			WHERE
				server_id = $1
			",
			server_id,
			&alloc_id,
			now,
			&nomad_node_id,
			&nomad_node_name,
			&nomad_node_public_ipv4,
			&nomad_node_vlan_ipv4,
		)
		.await?;

		tracing::info!(?ports, "got ds ports");

		// Save the ports to the db
		for port in &ports {
			tracing::info!(%server_id, label = %port.label, source = port.source, target = port.target, ip = %port.ip, "inserting ds port");
			sql_execute!(
				[ctx, @tx tx]
				"
				INSERT INTO
					db_dynamic_servers.internal_ports (
						server_id,
						nomad_label,
						nomad_source,
						nomad_ip
					)
				VALUES
					($1, $2, $3, $4)
				",
				server_id,
				&port.label,
				port.source as i64,
				&port.ip,
			)
			.await?;
		}
	}

	tracing::info!("ayy event2c");

	Ok(Some(DbOutput {
		server_id,
		datacenter_id: run_row.datacenter_id,
		stop_ts: run_row.stop_ts,
	}))
}
