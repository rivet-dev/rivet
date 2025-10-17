use anyhow::*;
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
pub enum SubCommand {
	/// Configure tracing settings (log filter and sampler ratio)
	Config {
		/// Log filter (e.g., "debug", "info", "rivet_api_peer=trace")
		/// Set to null to reset to defaults
		#[clap(short, long)]
		filter: Option<String>,

		/// OpenTelemetry sampler ratio (0.0-1.0)
		/// Set to null to reset to default
		#[clap(short, long)]
		sampler_ratio: Option<f64>,

		/// API peer endpoint
		#[clap(long, default_value = "http://localhost:6421")]
		endpoint: String,
	},
}

#[derive(Serialize, Deserialize)]
struct SetTracingConfigRequest {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub filter: Option<Option<String>>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sampler_ratio: Option<Option<f64>>,
}

impl SubCommand {
	pub async fn execute(self, _config: rivet_config::Config) -> Result<()> {
		match self {
			Self::Config {
				filter,
				sampler_ratio,
				endpoint,
			} => {
				// Build request body
				let request = SetTracingConfigRequest {
					filter: filter.map(|f| if f.is_empty() { None } else { Some(f) }),
					sampler_ratio: sampler_ratio.map(Some),
				};

				// Send HTTP request
				let client = rivet_pools::reqwest::client().await?;
				let url = format!("{}/debug/tracing/config", endpoint);

				let response = client
					.put(&url)
					.json(&request)
					.send()
					.await
					.context("failed to send request")?;

				if response.status().is_success() {
					println!("Tracing configuration updated successfully");

					if let Some(Some(f)) = &request.filter {
						println!("  Filter: {}", f);
					} else if let Some(None) = &request.filter {
						println!("  Filter: reset to default");
					}

					if let Some(Some(r)) = request.sampler_ratio {
						println!("  Sampler ratio: {}", r);
					} else if let Some(None) = request.sampler_ratio {
						println!("  Sampler ratio: reset to default (0.001)");
					}
				} else {
					let status = response.status();
					let body = response.text().await.unwrap_or_default();
					bail!(
						"Failed to update tracing configuration: {} - {}",
						status,
						body
					);
				}

				Ok(())
			}
		}
	}
}
