use chirp_workflow::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Hash)]
pub struct InsertClickHouseInput {
	pub actor_id: Uuid,
}

/// Row to be inserted in to ClickHouse
#[derive(Serialize)]
pub struct ActorClickHouseRow {
	actor_id: String,
	project_id: Uuid,
	env_id: Uuid,
	datacenter_id: Uuid,
	tags: HashMap<String, String>,
	/// Alias of image_id
	build_id: Uuid,
	build_kind: i64,
	build_compression: i64,
	network_mode: i64,
	network_ports: HashMap<String, ActorClickHouseRowPort>,
	network_ports_ingress: HashMap<String, ActorClickHouseRowPortIngress>,
	network_ports_host: HashMap<String, ActorClickHouseRowPortHost>,
	network_ports_proxied: HashMap<String, ActorClickHouseRowPortProxied>,
	client_id: Uuid,
	client_wan_hostname: String,
	selected_cpu_millicores: u32,
	selected_memory_mib: u32,
	root_user_enabled: bool,
	env_vars: i64,
	env_var_bytes: i64,
	args: i64,
	args_bytes: i64,
	durable: bool,
	kill_timeout: i64,
	cpu_millicores: i64,
	memory_mib: i64,
	/// Used in ORDER BY for replacing the key so this must never change.
	created_at: i64,
	/// This will not be set until after the actor is destroyed because we only insert in to
	/// ClickHouse after start & destroy.
	///
	/// 0 = not set
	started_at: i64,
	/// See `started_at`.
	connectable_at: i64,
	/// See `started_at`.
	finished_at: i64,
	/// This column is used for configuring the TTL of the actor.
	///
	/// 0 = not set
	destroyed_at: i64,
	row_updated_at: i64,
}

#[derive(Serialize)]
pub struct ActorClickHouseRowPort {
	/// Will be 0 if not configured
	internal_port: u16,
	routing_guard: bool,
	routing_host: bool,
	routing_guard_protocol: i64,
	routing_host_protocol: i64,
}

#[derive(Serialize)]
pub struct ActorClickHouseRowPortIngress {
	port_number: u16,
	ingress_port_number: u16,
	protocol: i64,
}

#[derive(Serialize)]
pub struct ActorClickHouseRowPortHost {
	port_number: u16,
	protocol: i64,
}

#[derive(Serialize)]
pub struct ActorClickHouseRowPortProxied {
	ip: String,
	source: i64,
}

/// State row to select from SQLite
#[derive(sqlx::FromRow)]
struct StateRow {
	project_id: Uuid,
	env_id: Uuid,
	tags: sqlx::types::Json<HashMap<String, String>>,
	resources_cpu_millicores: i64,
	resources_memory_mib: i64,
	selected_resources_cpu_millicores: Option<i64>,
	selected_resources_memory_mib: Option<i64>,
	client_id: Option<Uuid>,
	client_workflow_id: Option<Uuid>,
	client_wan_hostname: Option<String>,
	lifecycle_kill_timeout_ms: i64,
	lifecycle_durable: bool,
	create_ts: i64,
	start_ts: Option<i64>,
	connectable_ts: Option<i64>,
	finish_ts: Option<i64>,
	destroy_ts: Option<i64>,
	image_id: Uuid,
	build_kind: i64,
	build_compression: i64,
	root_user_enabled: bool,
	args: sqlx::types::Json<Vec<String>>,
	network_mode: i64,
	environment: sqlx::types::Json<HashMap<String, String>>,
}

/// This activity is idempotent and will upsert the actor row. If we want to change the data in
/// ClickHouse, we need to use this. This gets inserted in to a ReplacingMergeTree so it's safe to
/// update frequently.
#[activity(InsertClickHouse)]
pub async fn insert_clickhouse(
	ctx: &ActivityCtx,
	input: &InsertClickHouseInput,
) -> GlobalResult<()> {
	let Ok(inserter) = ctx.clickhouse_inserter().await else {
		return Ok(());
	};

	let dc_id = ctx.config().server()?.rivet.edge()?.datacenter_id;

	// Read extra information
	let pool = ctx.sqlite().await?;

	// Read state
	let state_row = sql_fetch_one!(
		[ctx, StateRow, &pool]
		"
		SELECT 
			project_id,
			env_id,
			json(tags) AS tags,
			resources_cpu_millicores,
			resources_memory_mib,
			selected_resources_cpu_millicores,
			selected_resources_memory_mib,
			client_id,
			client_workflow_id,
			client_wan_hostname,
			lifecycle_kill_timeout_ms,
			lifecycle_durable,
			create_ts,
			start_ts,
			connectable_ts,
			finish_ts,
			destroy_ts,
			image_id,
			build_kind,
			build_compression,
			root_user_enabled,
			json(args) AS args,
			network_mode,
			json(environment) AS environment
		FROM state
		",
	)
	.await?;

	// Read network ports from SQLite tables
	let network_ports_data = sql_fetch_all!(
		[ctx, (String, Option<i64>, i64, String), &pool]
		"
		SELECT port_name, port_number, protocol, 'ingress' as routing_type FROM ports_ingress
		UNION ALL
		SELECT port_name, port_number, protocol, 'host' as routing_type FROM ports_host
		",
	)
	.await?;

	let network_ports: HashMap<String, ActorClickHouseRowPort> = network_ports_data
		.into_iter()
		.map(|(name, port_number, protocol, routing_type)| {
			let (routing_guard, routing_host, routing_guard_protocol, routing_host_protocol) =
				match routing_type.as_str() {
					"ingress" => (true, false, protocol as i64, 0),
					"host" => (false, true, 0, protocol as i64),
					_ => (false, false, 0, 0),
				};

			(
				name,
				ActorClickHouseRowPort {
					internal_port: port_number.unwrap_or_default() as u16,
					routing_guard,
					routing_host,
					routing_guard_protocol,
					routing_host_protocol,
				},
			)
		})
		.collect();

	// Read ingress ports
	let ingress_ports = sql_fetch_all!(
		[ctx, (String, Option<i64>, i64, i64), &pool]
		"SELECT port_name, port_number, ingress_port_number, protocol FROM ports_ingress",
	)
	.await?
	.into_iter()
	.map(|(name, port_number, ingress_port_number, protocol)| {
		(
			name,
			ActorClickHouseRowPortIngress {
				port_number: port_number.unwrap_or_default() as u16,
				ingress_port_number: ingress_port_number as u16,
				protocol: protocol as i64,
			},
		)
	})
	.collect::<HashMap<String, ActorClickHouseRowPortIngress>>();

	// Read host ports
	let host_ports = sql_fetch_all!(
		[ctx, (String, Option<i64>, i64), &pool]
		"SELECT port_name, port_number, protocol FROM ports_host",
	)
	.await?
	.into_iter()
	.map(|(name, port_number, protocol)| {
		(
			name,
			ActorClickHouseRowPortHost {
				port_number: port_number.unwrap_or_default() as u16,
				protocol: protocol as i64,
			},
		)
	})
	.collect::<HashMap<String, ActorClickHouseRowPortHost>>();

	// Read proxied ports
	let proxied_ports = sql_fetch_all!(
		[ctx, (String, String, i64), &pool]
		"SELECT port_name, ip, source FROM ports_proxied",
	)
	.await?
	.into_iter()
	.map(|(name, ip, source)| {
		(
			name,
			ActorClickHouseRowPortProxied {
				ip,
				source: source as i64,
			},
		)
	})
	.collect::<HashMap<String, ActorClickHouseRowPortProxied>>();

	inserter.insert(
		"db_pegboard_analytics",
		"actors",
		ActorClickHouseRow {
			actor_id: input.actor_id.to_string(),
			project_id: state_row.project_id,
			env_id: state_row.env_id,
			datacenter_id: dc_id,
			tags: state_row.tags.0,
			build_id: state_row.image_id,
			build_kind: state_row.build_kind,
			build_compression: state_row.build_compression,
			network_mode: state_row.network_mode as i64,
			network_ports,
			network_ports_ingress: ingress_ports,
			network_ports_host: host_ports,
			network_ports_proxied: proxied_ports,
			client_id: state_row.client_id.unwrap_or_default(),
			client_wan_hostname: state_row.client_wan_hostname.unwrap_or_default(),
			selected_cpu_millicores: state_row
				.selected_resources_cpu_millicores
				.unwrap_or_default() as u32,
			selected_memory_mib: state_row.selected_resources_memory_mib.unwrap_or_default() as u32,
			root_user_enabled: state_row.root_user_enabled,
			env_vars: state_row.environment.len() as i64,
			env_var_bytes: state_row.environment
				.iter()
				.map(|(k, v)| k.len() + v.len())
				.sum::<usize>() as i64,
			args: state_row.args.len() as i64,
			args_bytes: state_row.args.iter().map(|arg| arg.len()).sum::<usize>() as i64,
			durable: state_row.lifecycle_durable,
			kill_timeout: state_row.lifecycle_kill_timeout_ms,
			cpu_millicores: state_row.resources_cpu_millicores,
			memory_mib: state_row.resources_memory_mib,
			created_at: state_row.create_ts * 1_000_000, // Convert ms to ns for ClickHouse DateTime64(9)
			started_at: state_row
				.start_ts
				.map(|ts| ts * 1_000_000)
				.unwrap_or_default(),
			connectable_at: state_row
				.connectable_ts
				.map(|ts| ts * 1_000_000)
				.unwrap_or_default(),
			finished_at: state_row
				.finish_ts
				.map(|ts| ts * 1_000_000)
				.unwrap_or_default(),
			destroyed_at: state_row
				.destroy_ts
				.map(|ts| ts * 1_000_000)
				.unwrap_or_default(),
			row_updated_at: util::timestamp::now() * 1_000_000, // Convert ms to ns for ClickHouse DateTime64(9)
		},
	)?;

	Ok(())
}
