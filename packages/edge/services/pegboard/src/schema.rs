use clickhouse_user_query::{Property, PropertyType, Schema};
use once_cell::sync::Lazy;

/// Schema for the actors analytics table
///
/// Excludes namespace and env_id as they are automatically provided by the system:
/// - namespace: from ctx.config().server()?.rivet.namespace
/// - env_id: from the auth token
pub static ACTOR_SCHEMA: Lazy<Schema> = Lazy::new(|| {
	Schema::new(vec![
		Property::new("actor_id".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
		Property::new("project_id".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
		Property::new("datacenter_id".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
		Property::new("tags".to_string(), true, PropertyType::String).unwrap(), // Map type - is_map is true
		Property::new("build_id".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
		Property::new("build_kind".to_string(), false, PropertyType::Number)
			.unwrap()
			.with_group_by(true),
		Property::new("build_compression".to_string(), false, PropertyType::Number)
			.unwrap()
			.with_group_by(true),
		Property::new("network_mode".to_string(), false, PropertyType::Number)
			.unwrap()
			.with_group_by(true),
		Property::new("client_id".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
		Property::new(
			"client_wan_hostname".to_string(),
			false,
			PropertyType::String,
		)
		.unwrap(),
		Property::new(
			"selected_cpu_millicores".to_string(),
			false,
			PropertyType::Number,
		)
		.unwrap(),
		Property::new(
			"selected_memory_mib".to_string(),
			false,
			PropertyType::Number,
		)
		.unwrap(),
		Property::new("root_user_enabled".to_string(), false, PropertyType::Bool)
			.unwrap()
			.with_group_by(true),
		Property::new("env_vars".to_string(), false, PropertyType::Number).unwrap(),
		Property::new("env_var_bytes".to_string(), false, PropertyType::Number).unwrap(),
		Property::new("args".to_string(), false, PropertyType::Number).unwrap(),
		Property::new("args_bytes".to_string(), false, PropertyType::Number).unwrap(),
		Property::new("durable".to_string(), false, PropertyType::Bool)
			.unwrap()
			.with_group_by(true),
		Property::new("kill_timeout".to_string(), false, PropertyType::Number).unwrap(),
		Property::new("cpu_millicores".to_string(), false, PropertyType::Number).unwrap(),
		Property::new("memory_mib".to_string(), false, PropertyType::Number).unwrap(),
		Property::new("created_at".to_string(), false, PropertyType::Number).unwrap(),
		Property::new("started_at".to_string(), false, PropertyType::Number).unwrap(),
		Property::new("connectable_at".to_string(), false, PropertyType::Number).unwrap(),
		Property::new("finished_at".to_string(), false, PropertyType::Number).unwrap(),
		Property::new("destroyed_at".to_string(), false, PropertyType::Number).unwrap(),
	])
	.unwrap()
});

/// Schema for the actor_logs3_with_metadata table
///
/// Excludes namespace and env_id as they are automatically provided by the system:
/// - namespace: from ctx.config().server()?.rivet.namespace
/// - env_id: from the auth token
pub static ACTOR_LOGS_SCHEMA: Lazy<Schema> = Lazy::new(|| {
	Schema::new(vec![
		Property::new("actor_id".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
		Property::new("stream_type".to_string(), false, PropertyType::Number)
			.unwrap()
			.with_group_by(true),
		Property::new("message".to_string(), false, PropertyType::String).unwrap(),
		Property::new("datacenter_id".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
		Property::new("tags".to_string(), true, PropertyType::String).unwrap(), // Map type - is_map is true
		Property::new("build_id".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
		Property::new("client_id".to_string(), false, PropertyType::String)
			.unwrap()
			.with_group_by(true),
		Property::new("durable".to_string(), false, PropertyType::Bool)
			.unwrap()
			.with_group_by(true),
	])
	.unwrap()
});
