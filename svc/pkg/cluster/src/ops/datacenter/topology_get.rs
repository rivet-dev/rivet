use std::{
	collections::{HashMap, HashSet},
	convert::{TryFrom, TryInto},
};

use chirp_workflow::prelude::*;
use nomad_client::apis::{allocations_api, configuration::Configuration, nodes_api};
use server_spec::types::ServerSpec;

use crate::types::PoolType;

lazy_static::lazy_static! {
	static ref NOMAD_CONFIG: Configuration = nomad_util::new_config_from_env().unwrap();
}

#[derive(sqlx::FromRow)]
struct ServerRow {
	server_id: Uuid,
	datacenter_id: Uuid,
	pool_type: i64,
	provider_hardware: Option<String>,
	nomad_node_id: Option<String>,
	pegboard_client_id: Option<Uuid>,
}

struct ServerRowStructured {
	server_id: Uuid,
	datacenter_id: Uuid,
	pool_type: PoolType,
	provider_hardware: Option<String>,
	runtime: Runtime,
}

impl TryFrom<ServerRow> for ServerRowStructured {
	type Error = GlobalError;

	fn try_from(value: ServerRow) -> GlobalResult<Self> {
		let pool_type = unwrap!(PoolType::from_repr(value.pool_type.try_into()?));

		Ok(ServerRowStructured {
			server_id: value.server_id,
			datacenter_id: value.datacenter_id,
			pool_type,
			provider_hardware: value.provider_hardware,
			runtime: if let Some(nomad_node_id) = value.nomad_node_id {
				Runtime::Nomad(nomad_node_id)
			} else if let Some(pegboard_client_id) = value.pegboard_client_id {
				if let PoolType::Pegboard = pool_type {
					Runtime::Pegboard(pegboard_client_id)
				} else {
					// Pegboard isolate
					Runtime::None
				}
			} else {
				Runtime::None
			},
		})
	}
}

#[derive(Debug)]
enum Runtime {
	Nomad(String),
	// Does not include pegboard isolate
	Pegboard(Uuid),
	// Other pool types
	None,
}

#[derive(Debug)]
pub struct Input {
	pub datacenter_ids: Vec<Uuid>,
}

#[derive(Debug)]
pub struct Output {
	pub datacenters: Vec<Datacenter>,
	pub prometheus_fetched: bool,
}

#[derive(Debug)]
pub struct Datacenter {
	pub datacenter_id: Uuid,
	pub servers: Vec<Server>,
}

#[derive(Debug)]
pub struct Server {
	pub server_id: Uuid,
	pub pool_type: PoolType,
	pub usage: Stats,
	pub limits: Stats,
}

#[derive(Clone, Debug, Default)]
pub struct Stats {
	/// MHz
	pub cpu: u32,
	/// MiB
	pub memory: u32,
	/// MiB
	pub disk: u32,
	/// Kibps
	pub bandwidth: u32,
}

#[operation]
pub async fn cluster_datacenter_topology_get(
	ctx: &OperationCtx,
	input: &Input,
) -> GlobalResult<Output> {
	let servers = sql_fetch_all!(
		[ctx, ServerRow]
		"
		SELECT
			server_id, datacenter_id, pool_type, provider_hardware, nomad_node_id, pegboard_client_id
		FROM db_cluster.servers
		WHERE
			datacenter_id = ANY($1) AND
			cloud_destroy_ts IS NULL AND
			taint_ts IS NULL
		",
		&input.datacenter_ids,
	)
	.await?
	.into_iter()
	.map(TryInto::<ServerRowStructured>::try_into)
	.collect::<GlobalResult<Vec<_>>>()?;

	// Fetch batch data from nomad
	let (
		datacenters_res,
		pb_client_usage_res,
		(hardware_specs, prometheus_metrics),
		allocation_info,
		node_info,
	) = tokio::try_join!(
		ctx.op(crate::ops::datacenter::get::Input {
			datacenter_ids: input.datacenter_ids.clone(),
		}),
		ctx.op(pegboard::ops::client::usage_get::Input {
			client_ids: servers
				.iter()
				.filter_map(
					|s| if let Runtime::Pegboard(pegboard_client_id) = s.runtime {
						Some(pegboard_client_id)
					} else {
						None
					}
				)
				.collect(),
		}),
		async {
			// Fetch hardware for each server
			let instance_types_res = ctx
				.op(linode::ops::instance_type_get::Input {
					hardware_ids: servers
						.iter()
						.filter_map(|s| s.provider_hardware.clone())
						.collect::<HashSet<_>>()
						.into_iter()
						.collect::<Vec<_>>(),
				})
				.await?;

			// Make the hardware data agnostic and put it into a hashmap for better reads
			let hardware_specs = instance_types_res
				.instance_types
				.iter()
				.map(|instance_type| {
					(
						instance_type.hardware_id.clone(),
						ServerSpec::from_linode(instance_type),
					)
				})
				.collect::<HashMap<_, _>>();

			// Gracefully fetch prometheus metrics
			let servers = servers
				.iter()
				.filter(|server| {
					matches!(
						server.pool_type,
						PoolType::Gg | PoolType::Ats | PoolType::PegboardIsolate
					)
				})
				.collect::<Vec<_>>();
			let prometheus_metrics = fetch_server_metrics(&servers, &hardware_specs)
				.await
				.map_or_else(
					|err| {
						tracing::error!(?err, "failed to fetch prometheus metrics");
						None
					},
					Some,
				);

			Ok((hardware_specs, prometheus_metrics))
		},
		async {
			// Request is not paginated
			allocations_api::get_allocations(
				&NOMAD_CONFIG,
				None,
				None,
				None,
				None,
				None,
				None,
				None,
				None,
				None,
				Some(true),
				None,
			)
			.await
			.map_err(Into::<GlobalError>::into)
		},
		async {
			// Request is not paginated
			nodes_api::get_nodes(
				&NOMAD_CONFIG,
				None,
				None,
				None,
				None,
				None,
				None,
				None,
				None,
				None,
				Some(true),
			)
			.await
			.map_err(Into::<GlobalError>::into)
		},
	)?;

	// Preempt datacenters
	let mut datacenters = input
		.datacenter_ids
		.iter()
		.map(|datacenter_id| {
			(
				*datacenter_id,
				Datacenter {
					datacenter_id: *datacenter_id,
					servers: Vec::new(),
				},
			)
		})
		.collect::<HashMap<_, _>>();

	for server in servers {
		let datacenter = unwrap!(datacenters_res
			.datacenters
			.iter()
			.find(|dc| dc.datacenter_id == server.datacenter_id));
		let pool = unwrap!(datacenter
			.pools
			.iter()
			.find(|pool| pool.pool_type == server.pool_type));

		// Get default (first) hardware specs
		let default_provider_hardware = &unwrap!(pool.hardware.first()).provider_hardware;

		// We assume a server has the default memory
		// amount (memory of the first hardware in the list) if it is not provisioned yet

		let (usage, limits) = match &server.runtime {
			Runtime::Nomad(nomad_node_id) => {
				// Gracefully handle if node does not exist in API response
				if let Some(node) = node_info.iter().find(|node| {
					node.ID
						.as_ref()
						.map_or(false, |node_id| node_id == nomad_node_id)
				}) {
					// TODO: Bandwidth usage
					let mut usage = Stats::default();

					// Aggregate all allocated resources for this node
					for alloc in &allocation_info {
						let alloc_node_id = unwrap_ref!(alloc.node_id);

						if alloc_node_id == nomad_node_id {
							let resources = unwrap_ref!(alloc.allocated_resources);
							let shared_resources = unwrap_ref!(resources.shared);

							// Task states don't exist until a task starts
							if let Some(task_states) = &alloc.task_states {
								let tasks = unwrap_ref!(resources.tasks);

								for (task_name, task) in tasks {
									let task_state = unwrap!(task_states.get(task_name));
									let state = unwrap_ref!(task_state.state);

									// Only count pending, running, or failed tasks. In a "failed" allocation, all of the
									// tasks are have a "dead" state
									if state != "pending" && state != "running" && state != "failed"
									{
										continue;
									}

									let cpu = unwrap_ref!(task.cpu);
									let memory = unwrap_ref!(task.memory);

									usage.cpu += unwrap!(cpu.cpu_shares) as u32;
									// MB to MiB
									usage.memory += unwrap!(memory.memory_mb) as u32 * 1000 / 1024
										* 1000 / 1024;
								}
							}

							// MB to MiB
							usage.disk += unwrap!(shared_resources.disk_mb) as u32 * 1000 / 1024
								* 1000 / 1024;
						}
					}

					// Get node resource limits
					let resources = unwrap_ref!(node.node_resources);
					let limits = Stats {
						cpu: unwrap!(unwrap_ref!(resources.cpu).cpu_shares) as u32,
						// MB to MiB
						memory: unwrap!(unwrap_ref!(resources.memory).memory_mb) as u32 * 1000
							/ 1024 * 1000 / 1024,
						// MB to MiB
						disk: unwrap!(unwrap_ref!(resources.disk).disk_mb) as u32 * 1000 / 1024
							* 1000 / 1024,
						bandwidth: 0, // TODO:
					};

					(usage, limits)
				} else {
					tracing::error!(%nomad_node_id, "node not found in nomad response");

					(Stats::default(), Stats::default())
				}
			}
			Runtime::Pegboard(pegboard_client_id) => {
				// Gracefully handle if client usage exists
				let usage = if let Some(client) = pb_client_usage_res
					.clients
					.iter()
					.find(|client| &client.client_id == pegboard_client_id)
				{
					Stats {
						cpu: client.usage.cpu,
						memory: client.usage.memory,
						disk: client.usage.disk,
						bandwidth: 0, // TODO:
					}
				} else {
					tracing::error!(%pegboard_client_id, "pegboard client not found in response");

					Stats::default()
				};

				(
					usage,
					get_hardware_specs_or_default(
						&hardware_specs,
						server.provider_hardware.as_deref(),
						&default_provider_hardware,
					)?,
				)
			}
			Runtime::None => {
				// Gracefully handle if prometheus metrics exist
				let usage = if let Some(server_metrics) = prometheus_metrics
					.as_ref()
					.and_then(|x| x.get(&server.server_id))
				{
					server_metrics.clone()
				} else {
					tracing::warn!(server_id=%server.server_id, "no prometheus metrics for server");

					Stats::default()
				};

				(
					usage,
					get_hardware_specs_or_default(
						&hardware_specs,
						server.provider_hardware.as_deref(),
						&default_provider_hardware,
					)?,
				)
			}
		};

		let datacenter = unwrap!(datacenters.get_mut(&server.datacenter_id));
		datacenter.servers.push(Server {
			server_id: server.server_id,
			pool_type: server.pool_type,
			usage,
			limits,
		});
	}

	Ok(Output {
		datacenters: datacenters.into_values().collect(),
		prometheus_fetched: prometheus_metrics.is_some(),
	})
}

// Gracefully get hardware specs or default specs
fn get_hardware_specs_or_default(
	hardware_specs: &HashMap<String, ServerSpec>,
	provider_hardware: Option<&str>,
	default_provider_hardware: &str,
) -> GlobalResult<Stats> {
	let hardware_specs = unwrap!(provider_hardware
		.and_then(|provider_hardware| hardware_specs.get(provider_hardware))
		.or(hardware_specs.get(default_provider_hardware)));

	Ok(Stats {
		cpu: hardware_specs.cpu,
		memory: hardware_specs.memory,
		disk: hardware_specs.disk,
		bandwidth: hardware_specs.bandwidth,
	})
}

// Fetches cpu and memory data for specified servers
async fn fetch_server_metrics(
	servers: &[&ServerRowStructured],
	hardware_specs: &HashMap<String, ServerSpec>,
) -> GlobalResult<HashMap<Uuid, Stats>> {
	let prom_res = handle_request(
		&PROMETHEUS_URL,
		formatdoc!(
			r#"
			label_replace(
				# Selects average CPU across all cores of a server
				avg by (datacenter_id, pool_type, server_id) (
					max by (datacenter_id, pool_type, server_id, cpu) (
						sum by (datacenter_id, pool_type, server_id, cpu) (
							last_over_time(
								irate(
									node_cpu_seconds_total{{
										server_id=~"({server_ids})",

										mode!="idle",
										mode!="iowait",
										mode!="steal"
									}}
									[5m]
								)
								[15m:15s]
							) * 100
						)
					)
				),
				"metric", "cpu", "", ""
			)
			OR
			label_replace(
				# Selects the percent memory usage of a server
				max by (datacenter_id, pool_type, server_id) (
					node_memory_Active_bytes{{
						server_id=~"({server_ids})",
					}}
					/
					node_memory_MemTotal_bytes{{
						server_id=~"({server_ids})",
					}} *
					100
				),
				"metric", "mem", "", ""
			)
			OR
			label_replace(
				# Selects the bandwidth usage of a server
				sum by (datacenter_id, pool_type, server_id) (
					last_over_time((
						irate(
							node_network_transmit_bytes_total{{
								server_id=~"({server_ids})",
								device=~"(eth0|eth1)"
							}}[1m]
						)
					) [1m:15s])
				)
				# Convert from B/s to Kb/s
				* 8 / 1000,
				"metric", "bandwidth", "", ""
			)
			"#,
			server_ids = servers
				.iter()
				.map(|server| server.server_id.to_string())
				.collect::<Vec<_>>()
				.join("|"),
		)
		.to_string(),
	)
	.await?;

	let mut stats_by_server_id = HashMap::new();

	// Aggregate rows into hashmap
	for row in prom_res {
		// Only include server in this dc
		let Some(server) = servers.iter().find(|s| s.server_id == row.labels.server_id) else {
			continue;
		};

		let server_entry = stats_by_server_id
			.entry(server.server_id)
			.or_insert_with(|| Stats {
				cpu: 0,
				memory: 0,
				disk: 0,
				bandwidth: 0,
			});

		// Aggregate data
		if let Some((_, value)) = row.value {
			match row.labels.metric {
				Metric::Cpu => {
					server_entry.cpu += value.parse::<f64>()? as u32;
				}
				Metric::Memory => {
					server_entry.memory += value.parse::<f64>()? as u32;
				}
				Metric::Bandwidth => {
					let bandwidth = if let Some(provider_hardware) = &server.provider_hardware {
						let hardware_specs = unwrap!(hardware_specs.get(provider_hardware));

						// Normalize bandwidth
						value.parse::<f64>()? as u32 * 100 / hardware_specs.bandwidth
					} else {
						tracing::error!(server_id=%server.server_id, "server with metrics has no hardware");

						0
					};

					server_entry.bandwidth += bandwidth as u32;
				}
			}
		} else {
			tracing::warn!(?row, "no value from metric");
		}
	}

	Ok(stats_by_server_id)
}

#[derive(Debug, Deserialize)]
struct PrometheusResponse {
	data: PrometheusData,
}

#[derive(Debug, Deserialize)]
struct PrometheusData {
	#[serde(rename = "resultType")]
	_result_type: String,
	result: Vec<PrometheusResult>,
}

#[derive(Debug, Deserialize)]
struct PrometheusResult {
	#[serde(rename = "metric")]
	labels: PrometheusLabels,
	value: Option<(f64, String)>,
}

#[derive(Debug, Deserialize)]
struct PrometheusLabels {
	server_id: Uuid,
	metric: Metric,
}

#[derive(Debug, Deserialize)]
enum Metric {
	#[serde(rename = "cpu")]
	Cpu,
	#[serde(rename = "mem")]
	Memory,
	#[serde(rename = "bandwidth")]
	Bandwidth,
}

lazy_static::lazy_static! {
	static ref PROMETHEUS_URL: String = util::env::var("PROMETHEUS_URL").unwrap();
}

async fn handle_request(url: &String, query: String) -> GlobalResult<Vec<PrometheusResult>> {
	let query_pairs = vec![("query", query), ("timeout", "2500ms".to_owned())];

	let query_string = serde_urlencoded::to_string(query_pairs)?;
	let req_url = format!("{}/api/v1/query?{}", url, query_string);

	// Query prometheus
	tracing::info!("querying prometheus");
	let res = reqwest::Client::new().get(req_url).send().await?;

	if !res.status().is_success() {
		let status = res.status();
		let text = res.text().await?;

		bail!("failed prometheus request: ({}) {}", status, text);
	}

	let body = res.json::<PrometheusResponse>().await?;

	Ok(body.data.result)
}
