use proto::backend::pkg::*;
use reqwest::StatusCode;
use rivet_operation::prelude::*;
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
enum Error {
	#[error("prometheus error: {0}")]
	PrometheusError(String),
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

#[derive(Debug, Clone, Deserialize)]
struct PrometheusResult {
	value: Option<(f64, String)>,
	values: Option<Vec<(u64, String)>>,
}

#[derive(Debug)]
struct QueryTiming {
	start: i64,
	end: i64,
	step: i64,
}

impl QueryTiming {
	fn new(start: i64, end: i64, step: i64) -> Self {
		QueryTiming { start, end, step }
	}
}

#[operation(name = "job-run-metrics-log")]
async fn handle(
	ctx: OperationContext<job_run::metrics_log::Request>,
) -> GlobalResult<job_run::metrics_log::Response> {
	let prometheus_url = std::env::var("PROMETHEUS_URL")?;

	let mut metrics = Vec::new();

	for metric in &ctx.metrics {
		let query_timing = Some(QueryTiming::new(ctx.start, ctx.end, ctx.step));

		// Get all queries at once
		//
		// If you need to add new metrics, explicitly add then to the `keep`
		// relabel action in the Kubernetes config.
		let (mem_allocated, cpu_usage, mem_usage, mem_max_usage) = tokio::try_join!(
			handle_request(
				&prometheus_url,
				None,
				format!("last_over_time(nomad_client_allocs_memory_allocated{{exported_job=\"{nomad_job_id}\",task=\"{task}\"}} [15m:15s]) or vector(0)",
					nomad_job_id = metric.job,
					task = metric.task
			)),
			handle_request(
				&prometheus_url,
				query_timing.as_ref(),
				format!("max(nomad_client_allocs_cpu_total_percent{{exported_job=\"{nomad_job_id}\",task=\"{task}\"}}) or vector(0)",
					nomad_job_id = metric.job,
					task = metric.task
			)),
			handle_request(
				&prometheus_url,
				query_timing.as_ref(),
				format!("max(nomad_client_allocs_memory_usage{{exported_job=\"{nomad_job_id}\",task=\"{task}\"}}) or vector(0)",
					nomad_job_id = metric.job,
					task = metric.task
			)),
			handle_request(
				&prometheus_url,
				query_timing.as_ref(),
				format!("max(nomad_client_allocs_memory_max_usage{{exported_job=\"{nomad_job_id}\",task=\"{task}\"}}) or vector(0)",
					nomad_job_id = metric.job,
					task = metric.task
			)),
		)?;

		let (_, mem_allocated) = internal_unwrap_owned!(mem_allocated.value);
		let cpu_usage = internal_unwrap_owned!(cpu_usage.values)
			.into_iter()
			.map(|(_, v)| v.parse::<f32>())
			.collect::<Result<Vec<_>, _>>()?;
		let mem_usage = internal_unwrap_owned!(mem_usage.values)
			.into_iter()
			.map(|(_, v)| v.parse::<u64>())
			.collect::<Result<Vec<_>, _>>()?;
		let mem_max_usage = internal_unwrap_owned!(mem_max_usage.values)
			.into_iter()
			.map(|(_, v)| v.parse::<u64>())
			.collect::<Result<Vec<_>, _>>()?;

		metrics.push(job_run::metrics_log::response::Metrics {
			job: metric.job.clone(),
			cpu: cpu_usage,
			memory: mem_usage,
			memory_max: mem_max_usage,
			allocated_memory: mem_allocated.parse::<u64>()?,
		})
	}

	Ok(job_run::metrics_log::Response { metrics })
}

async fn handle_request(
	url: &String,
	timing: Option<&QueryTiming>,
	query: String,
) -> GlobalResult<PrometheusResult> {
	// Start query string building
	let mut query_pairs = vec![("query", query), ("timeout", "2500ms".to_owned())];

	// Append timing queries
	if let Some(timing) = timing {
		query_pairs.push(("start", (timing.start / 1000).to_string()));
		query_pairs.push(("end", (timing.end / 1000).to_string()));
		query_pairs.push(("step", format!("{}ms", timing.step)));
	}

	let query_string = serde_urlencoded::to_string(query_pairs)?;
	let req_url = format!(
		"http://{}/api/v1/query{}?{}",
		url,
		if timing.is_some() { "_range" } else { "" },
		query_string
	);
	tracing::info!(?req_url, "prometheus query");

	// Query prometheus
	let res = reqwest::Client::new().get(req_url).send().await?;

	if res.status() != StatusCode::OK {
		let status = res.status();
		let text = res.text().await?;

		return Err(Error::PrometheusError(format!(
			"failed prometheus request: ({}) {}",
			status, text
		))
		.into());
	}

	Ok(internal_unwrap_owned!(res.json::<PrometheusResponse>().await?.data.result.first()).clone())
}
