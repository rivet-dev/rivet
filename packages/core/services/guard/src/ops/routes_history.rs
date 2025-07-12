use chirp_workflow::prelude::*;
use clickhouse_user_query::{KeyPath, QueryExpr, UserDefinedQueryBuilder};
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

use crate::schema::HTTP_REQUESTS_SCHEMA;

#[derive(Debug)]
pub struct Input {
	pub namespace: String,
	pub user_query_expr: Option<QueryExpr>,
	pub group_by: Option<KeyPath>,
	pub start_ms: i64,
	pub end_ms: i64,
	pub interval_ms: i64,
}

#[derive(Debug)]
pub struct Output {
	pub metric_names: Vec<String>,
	pub metric_attributes: Vec<BTreeMap<String, String>>,
	pub metric_types: Vec<String>,
	pub metric_values: Vec<Vec<f64>>,
}

#[operation]
pub async fn guard_routes_history(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	let clickhouse = ctx.clickhouse().await?;

	// Validate group_by field if provided
	let group_by_column = if let Some(key_path) = &input.group_by {
		let property = HTTP_REQUESTS_SCHEMA
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
		let builder = UserDefinedQueryBuilder::new(&HTTP_REQUESTS_SCHEMA, Some(query_expr))
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
	let query = indoc::formatdoc!(
		"
		SELECT
			toUnixTimestamp64Milli(toStartOfInterval(guard_start_timestamp, INTERVAL ? SECOND)) as time_bucket,
			count() as request_count,
			avg(service_response_duration_ms) as avg_response_time_ms,
			sum(guard_response_body_bytes) as total_response_bytes,
			countIf(guard_response_status >= 500) as error_5xx_count,
			countIf(guard_response_status >= 400 AND guard_response_status < 500) as error_4xx_count
			{select_grouped_fields}
		FROM
			db_guard_analytics.http_requests
		WHERE
			namespace = ?
			AND guard_start_timestamp >= fromUnixTimestamp64Milli(?)
			AND guard_start_timestamp < fromUnixTimestamp64Milli(?)
			{user_query_where}
		GROUP BY time_bucket{group_by_clause}
		ORDER BY time_bucket ASC
		"
	);

	tracing::debug!(?query, "querying http_requests logs");

	// Build and execute query
	let mut query_builder = clickhouse
		.query(&query)
		.bind(interval_seconds)
		.bind(&input.namespace)
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

		metrics.request_count[bucket_index] = row.request_count as f64;
		metrics.avg_response_time_ms[bucket_index] = row.avg_response_time_ms;
		metrics.total_response_bytes[bucket_index] = row.total_response_bytes as f64;
		metrics.error_5xx_count[bucket_index] = row.error_5xx_count as f64;
		metrics.error_4xx_count[bucket_index] = row.error_4xx_count as f64;
	}

	// Build output
	let metric_names = vec![
		"request_count".to_string(),
		"avg_response_time_ms".to_string(),
		"total_response_bytes".to_string(),
		"error_5xx_count".to_string(),
		"error_4xx_count".to_string(),
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
				"request_count" => metrics.request_count.clone(),
				"avg_response_time_ms" => metrics.avg_response_time_ms.clone(),
				"total_response_bytes" => metrics.total_response_bytes.clone(),
				"error_5xx_count" => metrics.error_5xx_count.clone(),
				"error_4xx_count" => metrics.error_4xx_count.clone(),
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
	request_count: i64,
	avg_response_time_ms: f64,
	total_response_bytes: i64,
	error_5xx_count: i64,
	error_4xx_count: i64,
	#[serde(skip_serializing_if = "Option::is_none")]
	group_value: Option<String>,
}

#[derive(Debug)]
struct TimeSeriesMetrics {
	request_count: Vec<f64>,
	avg_response_time_ms: Vec<f64>,
	total_response_bytes: Vec<f64>,
	error_5xx_count: Vec<f64>,
	error_4xx_count: Vec<f64>,
}

impl TimeSeriesMetrics {
	fn new(initial_capacity: usize) -> Self {
		Self {
			request_count: vec![0.0; initial_capacity],
			avg_response_time_ms: vec![0.0; initial_capacity],
			total_response_bytes: vec![0.0; initial_capacity],
			error_5xx_count: vec![0.0; initial_capacity],
			error_4xx_count: vec![0.0; initial_capacity],
		}
	}

	fn ensure_capacity(&mut self, size: usize) {
		if self.request_count.len() < size {
			self.request_count.resize(size, 0.0);
			self.avg_response_time_ms.resize(size, 0.0);
			self.total_response_bytes.resize(size, 0.0);
			self.error_5xx_count.resize(size, 0.0);
			self.error_4xx_count.resize(size, 0.0);
		}
	}
}
