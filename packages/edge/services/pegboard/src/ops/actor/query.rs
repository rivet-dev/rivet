use chirp_workflow::prelude::*;
use clickhouse_user_query::{QueryExpr, UserDefinedQueryBuilder};
use std::collections::HashMap;

use crate::schema::ACTOR_SCHEMA;

#[derive(Debug)]
pub struct Input {
	pub env_id: Uuid,
	pub user_query_expr: Option<QueryExpr>,
	pub cursor: Option<QueryCursor>,
	pub limit: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct QueryCursor {
	pub created_at: i64,
	pub actor_id: Uuid,
}

#[derive(Debug)]
pub struct Output {
	pub actors: Vec<ActorQueryResult>,
	pub cursor: Option<QueryCursor>,
}

#[derive(Debug)]
pub struct ActorQueryResult {
	pub actor_id: Uuid,
	pub datacenter_id: Uuid,
	pub tags: HashMap<String, String>,
	pub build_id: Uuid,
	pub build_kind: u8,
	pub build_compression: u8,
	pub network_mode: u8,
	pub network_ports: HashMap<String, NetworkPort>,
	pub network_ports_ingress: HashMap<String, NetworkPortIngress>,
	pub network_ports_host: HashMap<String, NetworkPortHost>,
	pub network_ports_proxied: HashMap<String, NetworkPortProxied>,
	pub selected_cpu_millicores: u32,
	pub selected_memory_mib: u32,
	pub durable: bool,
	pub kill_timeout: i64,
	pub created_at: i64,
	pub started_at: Option<i64>,
	pub connectable_at: Option<i64>,
	pub finished_at: Option<i64>,
	pub destroyed_at: Option<i64>,
}

#[operation]
pub async fn pegboard_actor_query(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let clickhouse = ctx.clickhouse().await?;

	// Build user query filter if provided
	let (user_query_where, user_query_builder) = if let Some(ref query_expr) = input.user_query_expr
	{
		let builder = UserDefinedQueryBuilder::new(&ACTOR_SCHEMA, Some(query_expr))
			.map_err(|e| GlobalError::raw(e))?;
		let where_clause = format!("AND ({})", builder.where_expr());
		(where_clause, Some(builder))
	} else {
		(String::new(), None)
	};

	// Build cursor condition if provided
	let cursor_condition = if let Some(cursor) = &input.cursor {
		format!(
			"AND (created_at < fromUnixTimestamp64Nano({}) OR (created_at = fromUnixTimestamp64Nano({}) AND actor_id < '{}'))",
			cursor.created_at,
			cursor.created_at,
			cursor.actor_id
		)
	} else {
		String::new()
	};

	// Build the query - ORDER BY is fixed to (created_at DESC, actor_id DESC)
	let query = formatdoc!(
		"
		SELECT
			actor_id,
			project_id,
			datacenter_id,
			tags,
			build_id,
			build_kind,
			build_compression,
			network_mode,
			network_ports,
			network_ports_ingress,
			network_ports_host,
			network_ports_proxied,
			client_id,
			client_wan_hostname,
			selected_cpu_millicores,
			selected_memory_mib,
			root_user_enabled,
			env_vars,
			env_var_bytes,
			args,
			args_bytes,
			durable,
			kill_timeout,
			cpu_millicores,
			memory_mib,
			created_at,
			started_at,
			connectable_at,
			finished_at,
			destroyed_at
		FROM
			db_pegboard_analytics.actors
		WHERE
			namespace = ?
			AND env_id = ?
			{cursor_condition}
			{user_query_where}
		ORDER BY created_at DESC, actor_id DESC
		LIMIT ?
		"
	);

	// Build and execute query
	let mut query_builder = clickhouse
		.query(&query)
		.bind(&ctx.config().server()?.rivet.namespace)
		.bind(input.env_id)
		.bind(input.limit);

	// Bind user query parameters if present
	if let Some(builder) = user_query_builder {
		query_builder = builder.bind_to(query_builder);
	}

	let rows = query_builder
		.fetch_all::<ActorRow>()
		.await
		.map_err(|err| GlobalError::from(err))?;

	// Convert rows to ActorQueryResult structs
	let actors = rows
		.into_iter()
		.map(|row| row.into_actor())
		.collect::<GlobalResult<Vec<_>>>()?;

	// Build cursor from last actor if we have results
	let cursor = actors.last().map(|actor| QueryCursor {
		created_at: actor.created_at,
		actor_id: actor.actor_id,
	});

	Ok(Output { actors, cursor })
}

#[derive(Debug, clickhouse::Row, serde::Deserialize)]
struct ActorRow {
	actor_id: String,
	project_id: uuid::Uuid,
	datacenter_id: uuid::Uuid,
	tags: std::collections::HashMap<String, String>,
	build_id: uuid::Uuid,
	build_kind: u8,
	build_compression: u8,
	network_mode: u8,
	network_ports: std::collections::HashMap<String, NetworkPort>,
	network_ports_ingress: std::collections::HashMap<String, NetworkPortIngress>,
	network_ports_host: std::collections::HashMap<String, NetworkPortHost>,
	network_ports_proxied: std::collections::HashMap<String, NetworkPortProxied>,
	client_id: uuid::Uuid,
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
	cpu_millicores: u32,
	memory_mib: u32,
	created_at: i64,
	started_at: i64,
	connectable_at: i64,
	finished_at: i64,
	destroyed_at: i64,
}

#[derive(Debug, clickhouse::Row, serde::Deserialize)]
pub struct NetworkPort {
	pub internal_port: u16,
	pub routing_guard: bool,
	pub routing_host: bool,
	pub routing_guard_protocol: u8,
	pub routing_host_protocol: u8,
}

#[derive(Debug, clickhouse::Row, serde::Deserialize)]
struct NetworkPortIngress {
	port_number: u16,
	ingress_port_number: u16,
	protocol: u8,
}

#[derive(Debug, clickhouse::Row, serde::Deserialize)]
struct NetworkPortHost {
	port_number: u16,
	protocol: u8,
}

#[derive(Debug, clickhouse::Row, serde::Deserialize)]
struct NetworkPortProxied {
	ip: String,
	source: u8,
}

impl ActorRow {
	fn into_actor(self) -> GlobalResult<ActorQueryResult> {
		Ok(ActorQueryResult {
			actor_id: unwrap!(Uuid::parse_str(&self.actor_id).ok(), "invalid actor uuid"),
			datacenter_id: self.datacenter_id,
			tags: self.tags,
			build_id: self.build_id,
			build_kind: self.build_kind,
			build_compression: self.build_compression,
			network_mode: self.network_mode,
			network_ports: self.network_ports,
			network_ports_ingress: self.network_ports_ingress,
			network_ports_host: self.network_ports_host,
			network_ports_proxied: self.network_ports_proxied,
			selected_cpu_millicores: self.selected_cpu_millicores,
			selected_memory_mib: self.selected_memory_mib,
			durable: self.durable,
			kill_timeout: self.kill_timeout,
			created_at: self.created_at,
			started_at: if self.started_at > 0 {
				Some(self.started_at)
			} else {
				None
			},
			connectable_at: if self.connectable_at > 0 {
				Some(self.connectable_at)
			} else {
				None
			},
			finished_at: if self.finished_at > 0 {
				Some(self.finished_at)
			} else {
				None
			},
			destroyed_at: if self.destroyed_at > 0 {
				Some(self.destroyed_at)
			} else {
				None
			},
		})
	}
}
