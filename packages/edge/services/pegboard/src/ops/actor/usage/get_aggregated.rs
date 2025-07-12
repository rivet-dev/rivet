use chirp_workflow::prelude::*;
use clickhouse_user_query::{KeyPath, QueryExpr, UserDefinedQueryBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

use crate::schema::ACTOR_SCHEMA;

#[derive(Debug)]
pub struct Input {
	pub env_id: Uuid,
	pub start_ms: i64,
	pub end_ms: i64,
	pub group_by: Option<KeyPath>,
	pub user_query_expr: Option<QueryExpr>,
}

#[derive(Debug)]
pub struct Output {
	pub usage: Vec<UsageMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageMetrics {
	pub group: Option<HashMap<String, String>>,
	pub total_runtime_seconds: f64,
	pub total_cpu_core_seconds: f64,
	pub total_memory_gib_seconds: f64,
}

#[operation]
pub async fn pegboard_actor_usage_get_aggregated(
	ctx: &OperationCtx,
	input: &Input,
) -> GlobalResult<Output> {
	let server = ctx.config().server()?;
	let client = ctx.clickhouse().await?;

	// Build the base query
	let mut select_columns = vec![];
	let group_by_column: Option<String>;

	// Add group by column if requested
	if let Some(key_path) = &input.group_by {
		// Validate the column can be grouped by
		let prop = ACTOR_SCHEMA
			.get_property(&key_path.property)
			.ok_or_else(|| {
				err_code!(
					REQUEST_QUERY_INVALID_GROUP_BY_FIELD,
					field = &key_path.property
				)
			})?;

		if !prop.can_group_by {
			return Err(err_code!(
				REQUEST_QUERY_FIELD_NOT_GROUPABLE,
				field = &key_path.property,
				msg = "This field cannot be used in GROUP BY"
			));
		}

		// Build the column reference
		let column = if let Some(map_key) = &key_path.map_key {
			// For map properties with key
			if !prop.is_map {
				return Err(err_code!(
					REQUEST_QUERY_FIELD_NOT_MAP,
					field = &key_path.property,
					msg = "Property is not a map, cannot use map key"
				));
			}
			format!("{}['{}']", key_path.property, map_key.replace("'", "\\'"))
		} else {
			// For simple properties
			if prop.is_map {
				return Err(err_code!(
					REQUEST_QUERY_MAP_REQUIRES_KEY,
					field = &key_path.property,
					msg = "Map property requires a map key for GROUP BY"
				));
			}
			key_path.property.clone()
		};

		select_columns.push(format!("{} as group_value", column));
		group_by_column = Some(column);
	} else {
		group_by_column = None;
	}

	// Add aggregation columns
	select_columns.push("sum(multiIf(finished_at > 0, finished_at - started_at, now64(9) - started_at)) / 1000000000.0 as total_runtime_seconds".to_string());
	select_columns.push("sum(multiIf(finished_at > 0, (finished_at - started_at) * selected_cpu_millicores, (now64(9) - started_at) * selected_cpu_millicores)) / 1000000000000.0 as total_cpu_core_seconds".to_string());
	select_columns.push("sum(multiIf(finished_at > 0, (finished_at - started_at) * selected_memory_mib, (now64(9) - started_at) * selected_memory_mib)) / 1073741824000000000.0 as total_memory_gib_seconds".to_string());

	let select_clause = select_columns.join(", ");
	let group_by_clause = if let Some(column) = &group_by_column {
		format!(" GROUP BY {}", column)
	} else {
		"".to_string()
	};

	// Base query filters
	let mut where_conditions = vec![
		"namespace = ?".to_string(),
		"env_id = ?".to_string(),
		"started_at >= ?".to_string(),
		"started_at <= ?".to_string(),
	];

	// Add user query filter if provided
	let user_query_builder = if let Some(ref query_expr) = input.user_query_expr {
		let builder = UserDefinedQueryBuilder::new(&ACTOR_SCHEMA, query_expr)
			.map_err(Into::<GlobalError>::into)?;
		where_conditions.push(format!("({})", builder.where_expr()));
		Some(builder)
	} else {
		None
	};

	let where_clause = where_conditions.join(" AND ");

	let query_str = format!(
		"SELECT {} FROM actors WHERE {}{} SETTINGS date_time_output_format = 'iso'",
		select_clause, where_clause, group_by_clause
	);

	let mut query = client
		.query(&query_str)
		.bind(&server.rivet.namespace)
		.bind(input.env_id)
		.bind(input.start_ms * 1_000_000) // Convert milliseconds to nanoseconds
		.bind(input.end_ms * 1_000_000); // Convert milliseconds to nanoseconds

	// Bind user query parameters if present
	if let Some(builder) = user_query_builder {
		query = builder.bind_to(query);
	}

	// Define row structure based on whether we have group by
	if input.group_by.is_some() {
		#[derive(Debug, clickhouse::Row, Deserialize)]
		struct UsageRow {
			group_value: String,
			total_runtime_seconds: f64,
			total_cpu_core_seconds: f64,
			total_memory_gib_seconds: f64,
		}

		let rows = query.fetch_all::<UsageRow>().await?;

		let usage = rows
			.into_iter()
			.map(|row| {
				// Extract group value
				let mut group = HashMap::new();
				if let Some(key_path) = &input.group_by {
					// Use the full key path as the key in the result
					let key = if let Some(map_key) = &key_path.map_key {
						format!("{}.{}", key_path.property, map_key)
					} else {
						key_path.property.clone()
					};
					group.insert(key, row.group_value);
				}

				UsageMetrics {
					group: Some(group),
					total_runtime_seconds: row.total_runtime_seconds,
					total_cpu_core_seconds: row.total_cpu_core_seconds,
					total_memory_gib_seconds: row.total_memory_gib_seconds,
				}
			})
			.collect();

		Ok(Output { usage })
	} else {
		// Simple aggregation without group by
		#[derive(Debug, clickhouse::Row, Deserialize)]
		struct UsageRow {
			total_runtime_seconds: f64,
			total_cpu_core_seconds: f64,
			total_memory_gib_seconds: f64,
		}

		let mut cursor = query.fetch::<UsageRow>()?;

		if let Some(row) = cursor.next().await? {
			Ok(Output {
				usage: vec![UsageMetrics {
					group: None,
					total_runtime_seconds: row.total_runtime_seconds,
					total_cpu_core_seconds: row.total_cpu_core_seconds,
					total_memory_gib_seconds: row.total_memory_gib_seconds,
				}],
			})
		} else {
			Ok(Output {
				usage: vec![UsageMetrics {
					group: None,
					total_runtime_seconds: 0.0,
					total_cpu_core_seconds: 0.0,
					total_memory_gib_seconds: 0.0,
				}],
			})
		}
	}
}
