use chirp_workflow::prelude::*;
use clickhouse_user_query::{KeyPath, QueryExpr, UserDefinedQueryBuilder};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

use crate::schema::ACTOR_SCHEMA;

#[derive(Debug)]
pub struct Input {
	pub env_id: Uuid,
	pub start_ms: i64,
	pub end_ms: i64,
	pub interval_ms: i64,
	pub group_by: Option<KeyPath>,
	pub user_query_expr: Option<QueryExpr>,
}

#[derive(Debug)]
pub struct Output {
	pub metric_names: Vec<String>,
	pub metric_attributes: Vec<BTreeMap<String, String>>,
	pub metric_types: Vec<String>,
	pub metric_values: Vec<Vec<f64>>,
}

#[operation]
pub async fn pegboard_actor_usage_get(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let server = ctx.config().server()?;
	let client = ctx.clickhouse().await?;

	// Validate group_by field if provided
	let group_by_column = if let Some(key_path) = &input.group_by {
		let property = ACTOR_SCHEMA
			.get_property(&key_path.property)
			.ok_or_else(|| {
				err_code!(
					REQUEST_QUERY_INVALID_GROUP_BY_FIELD,
					field = &key_path.property
				)
			})?;

		if !property.can_group_by {
			return Err(err_code!(
				REQUEST_QUERY_FIELD_NOT_GROUPABLE,
				field = &key_path.property,
				msg = "This field cannot be used in GROUP BY"
			));
		}

		// Build the column reference
		let column = if let Some(map_key) = &key_path.map_key {
			// For map properties with key
			if !property.is_map {
				return Err(err_code!(
					REQUEST_QUERY_FIELD_NOT_MAP,
					field = &key_path.property,
					msg = "Property is not a map, cannot use map key"
				));
			}
			format!("{}['{}']", key_path.property, map_key.replace("'", "\\'"))
		} else {
			// For simple properties
			if property.is_map {
				return Err(err_code!(
					REQUEST_QUERY_MAP_REQUIRES_KEY,
					field = &key_path.property,
					msg = "Map property requires a map key for GROUP BY"
				));
			}
			key_path.property.clone()
		};
		Some(column)
	} else {
		None
	};

	// Build user query filter if provided
	let (user_query_where, user_query_builder) = if let Some(ref query_expr) = input.user_query_expr
	{
		let builder = UserDefinedQueryBuilder::new(&ACTOR_SCHEMA, query_expr)
			.map_err(|e| GlobalError::raw(e))?;
		let where_clause = format!("AND ({})", builder.where_expr());
		(where_clause, Some(builder))
	} else {
		(String::new(), None)
	};

	// Build GROUP BY clause
	let group_by_clause = if let Some(column) = &group_by_column {
		format!(", {}", column)
	} else {
		String::new()
	};

	// Build SELECT clause for grouped fields
	let select_grouped_fields = if let Some(column) = &group_by_column {
		format!(", {} as group_value", column)
	} else {
		String::new()
	};

	// Build the query for time series data
	// Convert interval from milliseconds to seconds for ClickHouse INTERVAL
	let interval_seconds = input.interval_ms / 1000;
	let query = formatdoc!(
		"
		SELECT
			toUnixTimestamp64Milli(toStartOfInterval(started_at, INTERVAL ? SECOND)) as time_bucket,
			count() as actor_count,
			sum(multiIf(finished_at > 0, finished_at - started_at, now64(9) - started_at)) / 1000000000.0 as total_runtime_seconds,
			sum(multiIf(finished_at > 0, (finished_at - started_at) * selected_cpu_millicores, (now64(9) - started_at) * selected_cpu_millicores)) / 1000000000000.0 as total_cpu_core_seconds,
			sum(multiIf(finished_at > 0, (finished_at - started_at) * selected_memory_mib, (now64(9) - started_at) * selected_memory_mib)) / 1073741824000000000.0 as total_memory_gib_seconds
			{select_grouped_fields}
		FROM
			db_pegboard_analytics.actors
		WHERE
			namespace = ?
			AND env_id = ?
			AND started_at >= fromUnixTimestamp64Nano(? * 1000000)
			AND started_at < fromUnixTimestamp64Nano(? * 1000000)
			{user_query_where}
		GROUP BY time_bucket{group_by_clause}
		ORDER BY time_bucket ASC
		"
	);

	// Build and execute query
	let mut query_builder = client
		.query(&query)
		.bind(interval_seconds)
		.bind(&server.rivet.namespace)
		.bind(input.env_id)
		.bind(input.start_ms)
		.bind(input.end_ms);

	// Bind user query parameters if present
	if let Some(builder) = user_query_builder {
		query_builder = builder.bind_to(query_builder);
	}

	// Execute the query
	let rows = query_builder
		.fetch_all::<TimeSeriesRow>()
		.await
		.map_err(|err| GlobalError::from(err))?;

	// Transform results into time series format
	let mut time_buckets = Vec::new();
	let mut grouped_metrics: HashMap<BTreeMap<String, String>, TimeSeriesMetrics> = HashMap::new();

	for row in rows {
		// Create attributes from grouped field
		let mut attributes = BTreeMap::new();
		if let Some(key_path) = &input.group_by {
			// Use the full key path as the key in the result
			let key = if let Some(map_key) = &key_path.map_key {
				format!("{}.{}", key_path.property, map_key)
			} else {
				key_path.property.clone()
			};

			// The group_value field contains the actual value
			if let Some(value) = &row.group_value {
				attributes.insert(key, value.clone());
			}
		}

		// Store time bucket
		if !time_buckets.contains(&row.time_bucket) {
			time_buckets.push(row.time_bucket);
		}

		// Store metrics
		let metrics = grouped_metrics
			.entry(attributes)
			.or_insert_with(|| TimeSeriesMetrics::new(time_buckets.len()));

		let bucket_index = time_buckets
			.iter()
			.position(|&t| t == row.time_bucket)
			.unwrap();
		metrics.ensure_capacity(bucket_index + 1);

		metrics.actor_count[bucket_index] = row.actor_count as f64;
		metrics.total_runtime_seconds[bucket_index] = row.total_runtime_seconds;
		metrics.total_cpu_core_seconds[bucket_index] = row.total_cpu_core_seconds;
		metrics.total_memory_gib_seconds[bucket_index] = row.total_memory_gib_seconds;
	}

	// Build output
	let metric_names = vec![
		"actor_count".to_string(),
		"total_runtime_seconds".to_string(),
		"total_cpu_core_seconds".to_string(),
		"total_memory_gib_seconds".to_string(),
	];

	let mut metric_attributes = Vec::new();
	let mut metric_types = Vec::new();
	let mut metric_values = Vec::new();

	// Add metrics for each group
	for (attributes, metrics) in grouped_metrics {
		// Add each metric type
		for metric_name in &metric_names {
			metric_attributes.push(attributes.clone());
			metric_types.push(metric_name.clone());

			let values = match metric_name.as_str() {
				"actor_count" => metrics.actor_count.clone(),
				"total_runtime_seconds" => metrics.total_runtime_seconds.clone(),
				"total_cpu_core_seconds" => metrics.total_cpu_core_seconds.clone(),
				"total_memory_gib_seconds" => metrics.total_memory_gib_seconds.clone(),
				_ => unreachable!(),
			};

			metric_values.push(values);
		}
	}

	Ok(Output {
		metric_names,
		metric_attributes,
		metric_types,
		metric_values,
	})
}

#[derive(Debug, clickhouse::Row, Deserialize)]
struct TimeSeriesRow {
	time_bucket: i64,
	actor_count: i64,
	total_runtime_seconds: f64,
	total_cpu_core_seconds: f64,
	total_memory_gib_seconds: f64,
	#[serde(skip_serializing_if = "Option::is_none")]
	group_value: Option<String>,
}

#[derive(Debug)]
struct TimeSeriesMetrics {
	actor_count: Vec<f64>,
	total_runtime_seconds: Vec<f64>,
	total_cpu_core_seconds: Vec<f64>,
	total_memory_gib_seconds: Vec<f64>,
}

impl TimeSeriesMetrics {
	fn new(initial_capacity: usize) -> Self {
		Self {
			actor_count: vec![0.0; initial_capacity],
			total_runtime_seconds: vec![0.0; initial_capacity],
			total_cpu_core_seconds: vec![0.0; initial_capacity],
			total_memory_gib_seconds: vec![0.0; initial_capacity],
		}
	}

	fn ensure_capacity(&mut self, size: usize) {
		if self.actor_count.len() < size {
			self.actor_count.resize(size, 0.0);
			self.total_runtime_seconds.resize(size, 0.0);
			self.total_cpu_core_seconds.resize(size, 0.0);
			self.total_memory_gib_seconds.resize(size, 0.0);
		}
	}
}
