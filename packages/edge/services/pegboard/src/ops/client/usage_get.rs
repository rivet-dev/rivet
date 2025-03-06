use std::collections::HashMap;

use chirp_workflow::prelude::*;

#[derive(Debug)]
pub struct Input {
	pub client_ids: Vec<Uuid>,
}

#[derive(Debug)]
pub struct Output {
	pub clients: HashMap<Uuid, Stats>,
}

#[derive(Debug)]
pub struct Stats {
	/// Mhz
	pub cpu: u32,
	/// MiB
	pub memory: u32,
	/// MiB
	pub disk: u32,
}

#[operation]
pub async fn pegboard_client_usage_get(ctx: &OperationCtx, input: &Input) -> GlobalResult<Output> {
	if ctx.config().server()?.prometheus.is_none() {
		tracing::debug!("prometheus disabled");
		return Ok(Output {
			clients: HashMap::new(),
		});
	};

	let prom_res = handle_request(
		&ctx.config().server()?.prometheus()?.url.to_string(),
		formatdoc!(
			r#"
			label_replace(
				sum by (client_id) (
					last_over_time(
						rivet_pegboard_cpu_allocated{{
							client_id=~"({client_ids})",
						}}
						[15m:15s]
					)
				),
				"metric", "cpu", "", ""
			)
			OR
			label_replace(
				sum by (client_id) (
					last_over_time(
						rivet_pegboard_memory_allocated{{
							client_id=~"({client_ids})",
						}}
						[15m:15s]
					)
				),
				"metric", "mem", "", ""
			)
			"#,
			client_ids = input
				.client_ids
				.iter()
				.map(|client_id| client_id.to_string())
				.collect::<Vec<_>>()
				.join("|"),
		)
		.to_string(),
	)
	.await?;

	let mut stats_by_client_id = HashMap::new();

	// Aggregate rows into hashmap
	for row in prom_res {
		let server_entry = stats_by_client_id
			.entry(row.labels.client_id)
			.or_insert_with(|| Stats {
				cpu: 0,
				memory: 0,
				disk: 0,
			});

		// Aggregate data
		if let Some((_, value)) = row.value {
			match row.labels.metric {
				Metric::Cpu => {
					// MiB
					server_entry.cpu += value.parse::<f64>()? as u32;
				}
				Metric::Memory => {
					// MHz
					server_entry.memory +=
						value.parse::<f64>()? as u32 * server_spec::LINODE_CPU_PER_CORE / 1000;
				}
			}
		} else {
			tracing::warn!(?row, "no value from metric");
		}
	}

	Ok(Output {
		clients: stats_by_client_id,
	})
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
	client_id: Uuid,
	metric: Metric,
}

#[derive(Debug, Deserialize)]
enum Metric {
	#[serde(rename = "cpu")]
	Cpu,
	#[serde(rename = "mem")]
	Memory,
}

// TODO: Copied from topology_get
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
