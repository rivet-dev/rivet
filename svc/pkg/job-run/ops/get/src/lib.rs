use proto::backend::{self, pkg::*};
use rivet_operation::prelude::*;

#[derive(sqlx::FromRow)]
struct Run {
	run_id: Uuid,
	region_id: Uuid,
	create_ts: i64,
	start_ts: Option<i64>,
	stop_ts: Option<i64>,
	cleanup_ts: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct RunNetwork {
	run_id: Uuid,
	ip: String,
	mode: String,
}

#[derive(sqlx::FromRow)]
struct RunPort {
	run_id: Uuid,
	label: String,
	ip: String,
	source: i64,
	target: i64,
}

#[derive(sqlx::FromRow)]
struct RunMetaNomad {
	run_id: Uuid,
	dispatched_job_id: Option<String>,
	alloc_id: Option<String>,
	node_id: Option<String>,
	alloc_state: Option<serde_json::Value>,
}

#[derive(sqlx::FromRow)]
struct RunProxiedPort {
	run_id: Uuid,
	target_nomad_port_label: Option<String>,
	ingress_port: i64,
	ingress_hostnames: Vec<String>,
	proxy_protocol: i64,
	ssl_domain_mode: i64,
}

#[operation(name = "job-run-get")]
async fn handle(
	ctx: OperationContext<job_run::get::Request>,
) -> GlobalResult<job_run::get::Response> {
	let run_ids = ctx
		.run_ids
		.iter()
		.map(common::Uuid::as_uuid)
		.collect::<Vec<_>>();

	let crdb = ctx.crdb().await?;

	// Query the run data
	let (runs, run_networks, run_ports, run_meta_nomad, run_proxied_ports) = tokio::try_join!(
		// runs
		async {
			GlobalResult::Ok(
				sqlx::query_as::<_, Run>(indoc!(
					"
					SELECT run_id, region_id, create_ts, start_ts, stop_ts, cleanup_ts
					FROM db_job_state.runs
					WHERE run_id = ANY($1)
					"
				))
				.bind(&run_ids)
				.fetch_all(&crdb)
				.await?,
			)
		},
		// run_networks
		async {
			GlobalResult::Ok(
				sqlx::query_as::<_, RunNetwork>(indoc!(
					"
					SELECT run_id, ip, mode
					FROM db_job_state.run_networks
					WHERE run_id = ANY($1)
					"
				))
				.bind(&run_ids)
				.fetch_all(&crdb)
				.await?,
			)
		},
		// run_ports
		async {
			GlobalResult::Ok(
				sqlx::query_as::<_, RunPort>(indoc!(
					"
					SELECT run_id, label, ip, source, target
					FROM db_job_state.run_ports
					WHERE run_id = ANY($1)
					"
				))
				.bind(&run_ids)
				.fetch_all(&crdb)
				.await?,
			)
		},
		// run_meta_nomad
		async {
			GlobalResult::Ok(
				sqlx::query_as::<_, RunMetaNomad>("SELECT run_id, dispatched_job_id, alloc_id, node_id, alloc_state FROM db_job_state.run_meta_nomad WHERE run_id = ANY($1)")
					.bind(&run_ids)
					.fetch_all(&crdb)
					.await?
			)
		},
		// run_proxied_ports
		async {
			GlobalResult::Ok(
				sqlx::query_as::<_, RunProxiedPort>(indoc!(
					"
					SELECT run_id, target_nomad_port_label, ingress_port, ingress_hostnames, proxy_protocol, ssl_domain_mode
					FROM db_job_state.run_proxied_ports
					WHERE run_id = ANY($1)
					"
				))
				.bind(&run_ids)
				.fetch_all(&crdb)
				.await?,
			)
		},
	)?;

	// Build the runs
	let runs_proto = run_ids
		.iter()
		.cloned()
		.map(|run_id| {
			let run = if let Some(x) = runs.iter().find(|x| x.run_id == run_id) {
				x
			} else {
				tracing::warn!(%run_id, "run not found");
				return Ok(None);
			};

			// Match run meta
			let run_meta_kind = if let Some(run_meta_nomad) =
				run_meta_nomad.iter().find(|x| x.run_id == run_id)
			{
				let task_state = if let Some(alloc_state) = run_meta_nomad.alloc_state.as_ref() {
					derive_nomad_task_state(alloc_state)?
				} else {
					NomadTaskState::default()
				};

				backend::job::run_meta::Kind::Nomad(backend::job::run_meta::Nomad {
					dispatched_job_id: run_meta_nomad.dispatched_job_id.clone(),
					alloc_id: run_meta_nomad.alloc_id.clone(),
					node_id: run_meta_nomad.node_id.clone(),
					failed: task_state.failed,
					exit_code: task_state.exit_code,
				})
			} else {
				tracing::warn!(%run_id, "could not find run meta");
				return Ok(None);
			};
			let run_meta = backend::job::RunMeta {
				kind: Some(run_meta_kind),
			};

			let run = backend::job::Run {
				run_id: Some(run.run_id.into()),
				region_id: Some(run.region_id.into()),
				create_ts: run.create_ts,
				networks: run_networks
					.iter()
					.filter(|x| x.run_id == run_id)
					.map(|network| backend::job::Network {
						mode: network.mode.clone(),
						ip: network.ip.clone(),
					})
					.collect(),
				ports: run_ports
					.iter()
					.filter(|x| x.run_id == run_id)
					.map(|port| backend::job::Port {
						label: port.label.clone(),
						source: port.source as u32,
						target: port.target as u32,
						ip: port.ip.clone(),
					})
					.collect(),
				run_meta: Some(run_meta),
				proxied_ports: run_proxied_ports
					.iter()
					.filter(|x| x.run_id == run_id)
					.map(|port| backend::job::ProxiedPort {
						target_nomad_port_label: port.target_nomad_port_label.clone(),
						ingress_port: port.ingress_port as u32,
						ingress_hostnames: port.ingress_hostnames.clone(),
						proxy_protocol: port.proxy_protocol as i32,
						ssl_domain_mode: port.ssl_domain_mode as i32,
					})
					.collect(),
				start_ts: run.start_ts,
				stop_ts: run.stop_ts,
				cleanup_ts: run.cleanup_ts,
			};

			GlobalResult::Ok(Some(run))
		})
		.collect::<GlobalResult<Vec<Option<backend::job::Run>>>>()?
		.into_iter()
		.flatten()
		.collect::<Vec<_>>();

	Ok(job_run::get::Response { runs: runs_proto })
}

#[derive(Default)]
struct NomadTaskState {
	failed: Option<bool>,
	exit_code: Option<u32>,
}

fn derive_nomad_task_state(alloc_state_json: &serde_json::Value) -> GlobalResult<NomadTaskState> {
	let alloc =
		serde_json::from_value::<nomad_client::models::Allocation>(alloc_state_json.clone())?;
	let task_states = unwrap_ref!(alloc.task_states);

	// Get the main task by finding the task that is not the run cleanup task
	let main_task = task_states
		.iter()
		.filter(|(k, _)| k.as_str() == util_job::RUN_MAIN_TASK_NAME)
		.map(|(_, v)| v)
		.next();
	let main_task = unwrap!(main_task, "could not find main task");
	let main_task_state = unwrap_ref!(main_task.state);

	if main_task_state == "dead" {
		if main_task.failed == Some(true) {
			Ok(NomadTaskState {
				failed: Some(true),
				exit_code: Some(
					main_task
						.events
						.as_ref()
						.and_then(|events| {
							events.iter().filter_map(|x| x.exit_code).find(|x| *x != 0)
						})
						.unwrap_or(0) as u32,
				),
			})
		} else {
			Ok(NomadTaskState {
				failed: Some(false),
				exit_code: Some(0),
			})
		}
	} else {
		Ok(NomadTaskState {
			failed: None,
			exit_code: None,
		})
	}
}
