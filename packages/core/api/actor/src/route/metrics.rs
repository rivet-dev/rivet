use api_helper::{
	anchor::{WatchIndexQuery, WatchResponse},
	ctx::Ctx,
};
use rivet_api::models;
use rivet_operation::prelude::*;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};

use crate::{
	assert,
	auth::{Auth, CheckOpts, CheckOutput},
	utils::build_global_query_compat,
};

use super::GlobalQuery;

#[derive(Debug, Deserialize)]
pub struct GetActorMetricsQuery {
	#[serde(flatten)]
	pub global: GlobalQuery,
	pub start: i64,
	pub end: i64,
	pub interval: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MetricType {
	Counter,
	Gauge,
}

impl MetricType {
	pub fn as_str(&self) -> &'static str {
		match self {
			MetricType::Counter => "counter",
			MetricType::Gauge => "gauge",
		}
	}
}

#[derive(Debug, clickhouse::Row, serde::Deserialize)]
pub struct MetricRow {
	pub time_bucket_index: u32,
	pub metric_name: String,
	pub value: f64,
	pub tcp_state: String,
	pub udp_state: String,
	pub device: String,
	pub failure_type: String,
	pub scope: String,
	pub task_state: String,
	pub interface: String,
}

#[derive(Debug)]
pub struct ProcessedMetricRow {
	pub row: MetricRow,
	pub metric_type: MetricType,
}

#[tracing::instrument(skip_all)]
pub async fn get_metrics(
	ctx: Ctx<Auth>,
	actor_id: Uuid,
	watch_index: WatchIndexQuery,
	query: GetActorMetricsQuery,
) -> GlobalResult<models::ActorsGetActorMetricsResponse> {
	let CheckOutput { game_id, env_id } = ctx
		.auth()
		.check(
			ctx.op_ctx(),
			CheckOpts {
				query: &query.global,
				allow_service_token: false,
				opt_auth: false,
			},
		)
		.await?;

	// Validate the actor belongs to this game/env
	assert::actor_for_env(&ctx, &[actor_id], game_id, env_id, None).await?;

	let clickhouse = ctx.clickhouse().await?;

	// Convert milliseconds to seconds for ClickHouse
	let start_seconds = query.start / 1000;
	let end_seconds = query.end / 1000;
	let interval_seconds = query.interval / 1000;

	if interval_seconds == 0 {
		bail_with!(ACTOR_METRICS_INVALID_INTERVAL);
	}

	let actor_prefix = format!("/system.slice/pegboard-actor-{}-", actor_id);

	// Available gauge metrics:
	// - container_cpu_load_average_10s
	// - container_file_descriptors
	// - container_last_seen
	// - container_memory_usage_bytes
	// - container_memory_working_set_bytes
	// - container_memory_cache
	// - container_memory_rss
	// - container_memory_swap
	// - container_memory_mapped_file
	// - container_memory_max_usage_bytes
	// - container_network_tcp_usage_total
	// - container_network_tcp6_usage_total
	// - container_network_udp_usage_total
	// - container_network_udp6_usage_total
	// - container_sockets
	// - container_spec_cpu_period
	// - container_spec_cpu_shares
	// - container_spec_memory_limit_bytes
	// - container_spec_memory_reservation_limit_bytes
	// - container_spec_memory_swap_limit_bytes
	// - container_start_time_seconds
	// - container_tasks_state
	// - container_threads
	// - container_threads_max
	// - container_processes

	// Query gauge metrics (current values)
	let gauge_query = indoc! {"
		SELECT
			toUInt32((toUnixTimestamp(toStartOfInterval(TimeUnix, INTERVAL ? second)) - ?) / ?) as time_bucket_index,
			MetricName as metric_name,
			max(Value) as value,
			COALESCE(Attributes['tcp_state'], '') as tcp_state,
			COALESCE(Attributes['udp_state'], '') as udp_state,
			COALESCE(Attributes['device'], '') as device,
			COALESCE(Attributes['failure_type'], '') as failure_type,
			COALESCE(Attributes['scope'], '') as scope,
			COALESCE(Attributes['state'], '') as task_state,
			COALESCE(Attributes['interface'], '') as interface
		FROM otel.otel_metrics_gauge
		WHERE
			TimeUnix >= fromUnixTimestamp(?)
			AND TimeUnix <= fromUnixTimestamp(?)
			AND MetricName IN [
				'container_memory_usage_bytes'
			]
			AND has(Attributes, 'id')
			AND startsWith(Attributes['id'], ?)
		GROUP BY time_bucket_index, metric_name, tcp_state, udp_state, device, failure_type, scope, task_state, interface
		ORDER BY time_bucket_index ASC, metric_name
	"};

	let gauge_future = clickhouse
		.query(&gauge_query)
		.bind(interval_seconds)
		.bind(start_seconds)
		.bind(interval_seconds)
		.bind(start_seconds)
		.bind(end_seconds)
		.bind(&actor_prefix)
		.fetch_all::<MetricRow>();

	// Available sum metrics:
	// - container_cpu_schedstat_run_periods_total
	// - container_cpu_schedstat_run_seconds_total
	// - container_cpu_schedstat_runqueue_seconds_total
	// - container_cpu_system_seconds_total
	// - container_cpu_user_seconds_total
	// - container_cpu_usage_seconds_total
	// - container_memory_failcnt
	// - container_memory_failures_total
	// - container_fs_reads_bytes_total
	// - container_fs_writes_bytes_total
	// - container_network_receive_bytes_total
	// - container_network_receive_errors_total
	// - container_network_receive_packets_dropped_total
	// - container_network_receive_packets_total
	// - container_network_transmit_bytes_total
	// - container_network_transmit_errors_total
	// - container_network_transmit_packets_dropped_total
	// - container_network_transmit_packets_total

	// Query sum metrics (rates/counters)
	let sum_query = indoc! {"
		SELECT
			toUInt32((toUnixTimestamp(toStartOfInterval(TimeUnix, INTERVAL ? second)) - ?) / ?) as time_bucket_index,
			MetricName as metric_name,
			max(Value) as value,
			COALESCE(Attributes['tcp_state'], '') as tcp_state,
			COALESCE(Attributes['udp_state'], '') as udp_state,
			COALESCE(Attributes['device'], '') as device,
			COALESCE(Attributes['failure_type'], '') as failure_type,
			COALESCE(Attributes['scope'], '') as scope,
			COALESCE(Attributes['state'], '') as task_state,
			COALESCE(Attributes['interface'], '') as interface
		FROM otel.otel_metrics_sum
		WHERE
			TimeUnix >= fromUnixTimestamp(?)
			AND TimeUnix <= fromUnixTimestamp(?)
			AND MetricName IN [
				'container_cpu_usage_seconds_total'
			]
			AND has(Attributes, 'id')
			AND startsWith(Attributes['id'], ?)
		GROUP BY time_bucket_index, metric_name, tcp_state, udp_state, device, failure_type, scope, task_state, interface
		ORDER BY time_bucket_index ASC, metric_name
	"};

	let sum_future = clickhouse
		.query(&sum_query)
		.bind(interval_seconds)
		.bind(start_seconds)
		.bind(interval_seconds)
		.bind(start_seconds)
		.bind(end_seconds)
		.bind(&actor_prefix)
		.fetch_all::<MetricRow>();

	let (gauge_rows, sum_rows) =
		tokio::try_join!(gauge_future, sum_future).map_err(|err| GlobalError::from(err))?;

	// Map metric types based on query source
	let gauge_rows: Vec<ProcessedMetricRow> = gauge_rows
		.into_iter()
		.map(|row| ProcessedMetricRow {
			row,
			metric_type: MetricType::Gauge,
		})
		.collect();

	let sum_rows: Vec<ProcessedMetricRow> = sum_rows
		.into_iter()
		.map(|row| ProcessedMetricRow {
			row,
			metric_type: MetricType::Counter,
		})
		.collect();

	// Combine both result sets
	let mut rows = gauge_rows;
	rows.extend(sum_rows);

	// Calculate the number of time buckets we expect
	let num_buckets = ((end_seconds - start_seconds) / interval_seconds + 1) as usize;

	// Use HashMap to store metrics with their attributes and types
	let mut metrics: HashMap<(String, BTreeMap<String, String>), (String, Vec<f64>)> =
		HashMap::new();

	// Process rows and organize by metric name + attributes
	for processed_row in rows {
		let row = &processed_row.row;
		if row.time_bucket_index >= num_buckets as u32 {
			continue;
		}
		let bucket_idx = row.time_bucket_index as usize;

		// Build attributes map for this row
		let mut attributes = BTreeMap::new();

		// Add non-empty attributes
		if !row.tcp_state.is_empty() {
			attributes.insert("tcp_state".to_string(), row.tcp_state.clone());
		}
		if !row.udp_state.is_empty() {
			attributes.insert("udp_state".to_string(), row.udp_state.clone());
		}
		if !row.device.is_empty() {
			attributes.insert("device".to_string(), row.device.clone());
		}
		if !row.failure_type.is_empty() {
			attributes.insert("failure_type".to_string(), row.failure_type.clone());
		}
		if !row.scope.is_empty() {
			attributes.insert("scope".to_string(), row.scope.clone());
		}
		if !row.task_state.is_empty() {
			attributes.insert("state".to_string(), row.task_state.clone());
		}
		if !row.interface.is_empty() {
			attributes.insert("interface".to_string(), row.interface.clone());
		}

		// Create metric key (metric name + attributes)
		let metric_key = (row.metric_name.clone(), attributes);

		// Initialize metric entry if it doesn't exist
		let (existing_type, metric_values) = metrics.entry(metric_key).or_insert_with(|| {
			(
				processed_row.metric_type.as_str().to_string(),
				vec![0.0; num_buckets],
			)
		});

		// Add or set the value based on metric type
		match processed_row.metric_type {
			MetricType::Counter => {
				metric_values[bucket_idx] += row.value;
			}
			MetricType::Gauge => {
				metric_values[bucket_idx] = row.value;
			}
		}
	}

	// Convert HashMap to ordered vectors for response
	let mut metric_names = Vec::new();
	let mut metric_attributes = Vec::new();
	let mut metric_types = Vec::new();
	let mut metric_values = Vec::new();

	// Sort metrics by name for consistent ordering
	let mut sorted_metrics: Vec<_> = metrics.into_iter().collect();
	sorted_metrics.sort_by(|a, b| a.0 .0.cmp(&b.0 .0));

	for ((name, attributes), (metric_type, values)) in sorted_metrics {
		metric_names.push(name);
		// Convert BTreeMap back to HashMap for the API response
		let attributes_hashmap: HashMap<String, String> = attributes.into_iter().collect();
		metric_attributes.push(attributes_hashmap);
		metric_types.push(metric_type);
		metric_values.push(values);
	}

	Ok(models::ActorsGetActorMetricsResponse {
		actor_ids: vec![actor_id.to_string()],
		metric_names,
		metric_attributes,
		metric_types,
		metric_values,
	})
}
