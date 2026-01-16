use anyhow::Result;
use gas::prelude::*;
use serde::{Deserialize, Serialize};
use universalpubsub::NextOutput;

#[derive(Serialize, Deserialize)]
pub struct SetTracingConfigMessage {
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub filter: Option<Option<String>>,
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub sampler_ratio: Option<Option<f64>>,
}

#[tracing::instrument(skip_all)]
pub async fn start(_config: rivet_config::Config, pools: rivet_pools::Pools) -> Result<()> {
	// Subscribe to tracing config updates
	let ups = pools.ups()?;
	let subject = "rivet.debug.tracing.config";
	let mut sub = ups.subscribe(subject).await?;

	tracing::debug!(%subject, "subscribed to tracing config updates");

	// Process incoming messages
	while let Ok(NextOutput::Message(msg)) = sub.next().await {
		match serde_json::from_slice::<SetTracingConfigMessage>(&msg.payload) {
			Ok(update_msg) => {
				tracing::debug!(
					filter = ?update_msg.filter,
					sampler_ratio = ?update_msg.sampler_ratio,
					"received tracing config update"
				);

				// Apply the new log filter if provided
				match &update_msg.filter {
					Some(Some(filter)) => {
						// Set to specific value
						if let Err(err) = rivet_runtime::reload_log_filter(filter) {
							tracing::error!(?err, "failed to reload log filter");
						}
					}
					Some(None) => {
						// Reset to default (empty string)
						if let Err(err) = rivet_runtime::reload_log_filter("") {
							tracing::error!(?err, "failed to reload log filter to default");
						}
					}
					None => {
						// Not provided, no change
					}
				}

				// Apply the new sampler ratio if provided
				match update_msg.sampler_ratio {
					Some(Some(ratio)) => {
						// Set to specific value
						if let Err(err) = rivet_metrics::set_sampler_ratio(ratio) {
							tracing::error!(?err, "failed to reload sampler ratio");
						}
					}
					Some(None) => {
						// Reset to default (0.001)
						if let Err(err) = rivet_metrics::set_sampler_ratio(0.001) {
							tracing::error!(?err, "failed to reload sampler ratio to default");
						}
					}
					None => {
						// Not provided, no change
					}
				}
			}
			Err(err) => {
				tracing::error!(?err, "failed to deserialize tracing config update message");
			}
		}
	}

	Ok(())
}
