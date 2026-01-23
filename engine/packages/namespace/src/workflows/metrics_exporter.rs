use futures_util::{FutureExt, StreamExt, TryStreamExt};
use gas::prelude::*;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use universaldb::prelude::*;

use crate::keys;

const USAGE_METRICS_TOPIC: &str = "usage_metrics.v1";
const EXPORT_INTERVAL_MS: i64 = util::duration::minutes(1);
const EARLY_TXN_TIMEOUT: Duration = Duration::from_millis(2500);

#[derive(Debug, Deserialize, Serialize)]
pub struct Input {
	pub namespace_id: Id,
}

#[workflow]
pub async fn namespace_metrics_exporter(ctx: &mut WorkflowCtx, input: &Input) -> Result<()> {
	ctx.repeat(|ctx| {
		let namespace_id = input.namespace_id;
		async move {
			// Export before sleeping so the initial export is immediate
			ctx.activity(ExportMetricsInput { namespace_id }).await?;

			// Jitter sleep to prevent stampeding herds
			let jitter = { rand::thread_rng().gen_range(0..EXPORT_INTERVAL_MS / 10) };
			ctx.sleep(EXPORT_INTERVAL_MS + jitter).await?;

			Ok(Loop::<()>::Continue)
		}
		.boxed()
	})
	.await?;

	Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
struct ExportMetricsInput {
	namespace_id: Id,
}

#[activity(ExportMetrics)]
async fn export_metrics(ctx: &ActivityCtx, input: &ExportMetricsInput) -> Result<()> {
	let mut metrics = Vec::new();
	let mut last_key = Vec::new();

	loop {
		let (new_metrics, new_last_key) = ctx
			.udb()?
			.run(|tx| {
				let last_key = &last_key;
				async move {
					let start = Instant::now();
					let tx = tx.with_subspace(keys::subspace());
					let mut new_metrics = Vec::new();
					let mut new_last_key = Vec::new();

					let ns_metrics_subspace = keys::subspace()
						.subspace(&keys::metric::MetricKey::subspace(input.namespace_id));
					let range = ns_metrics_subspace.range();

					let range_start = if last_key.is_empty() {
						&range.0
					} else {
						&last_key
					};
					let range_end = &ns_metrics_subspace.range().1;

					let mut stream = tx.get_ranges_keyvalues(
						universaldb::RangeOption {
							mode: StreamingMode::WantAll,
							..(range_start.as_slice(), range_end.as_slice()).into()
						},
						Snapshot,
					);

					loop {
						if start.elapsed() > EARLY_TXN_TIMEOUT {
							tracing::warn!("timed out processing pending actors metrics");
							break;
						}

						let Some(entry) = stream.try_next().await? else {
							new_last_key = Vec::new();
							break;
						};

						let (metric_key, metric_value) =
							tx.read_entry::<keys::metric::MetricKey>(&entry)?;

						new_metrics.push((metric_key.metric, metric_value));

						new_last_key = [entry.key(), &[0xff]].concat();
					}

					Ok((new_metrics, new_last_key))
				}
			})
			.custom_instrument(tracing::info_span!("export_metrics_tx"))
			.await?;

		metrics.extend(new_metrics);
		last_key = new_last_key;

		if last_key.is_empty() {
			break;
		}
	}

	if metrics.is_empty() {
		return Ok(());
	}

	// Chunk metrics into 1024 rows
	let now = util::timestamp::now();
	let dc_name = ctx.config().dc_name()?;
	let mut payloads = vec![String::new()];
	let mut count = 0;
	for (metric, value) in metrics {
		let payload = payloads.last_mut().unwrap();

		let row = UsageMetricsRow {
			project: "TODO".to_string(),
			datacenter: dc_name.to_string(),
			timestamp: now,
			namespace_id: input.namespace_id.to_string(),
			metric_name: metric.variant().to_string(),
			metric_attributes: metric.attributes(),
			value,
		};
		payload.push_str(&row.serialize_compact()?);
		payload.push_str("\n");

		count += 1;
		if count == 1024 {
			payloads.push(String::new());
			count = 0;
		}
	}

	let kafka = ctx.kafka()?;
	futures_util::stream::iter(payloads)
		.map(|payload| {
			let kafka = kafka.clone();
			async move {
				kafka
					.send(
						rdkafka::producer::FutureRecord::to(USAGE_METRICS_TOPIC).payload(&payload),
						Duration::from_secs(5),
					)
					.await
					.map_err(|(err, _)| anyhow::Error::from(err))
			}
		})
		.buffer_unordered(1024)
		.try_collect::<Vec<_>>()
		.await?;

	Ok(())
}

pub struct UsageMetricsRow {
	pub project: String,
	pub datacenter: String,
	/// Milliseconds.
	pub timestamp: i64,
	pub namespace_id: String,
	pub metric_name: String,
	pub metric_attributes: HashMap<String, String>,
	pub value: i64,
}

impl UsageMetricsRow {
	/// See https://clickhouse.com/docs/interfaces/formats/JSONCompactEachRow
	pub fn serialize_compact(&self) -> Result<String> {
		Ok(format!(
			"[{},{},{},{},{},{},{}]",
			serde_json::to_string(&self.project)?,
			serde_json::to_string(&self.datacenter)?,
			serde_json::to_string(&self.timestamp)?,
			serde_json::to_string(&self.namespace_id)?,
			serde_json::to_string(&self.metric_name)?,
			serde_json::to_string(&self.metric_attributes)?,
			serde_json::to_string(&self.value)?,
		))
	}
}
