use anyhow::*;
use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
pub enum SubCommand {
	/// Enable the Pyroscope CPU profiler (broadcast to all engine pods in the datacenter)
	Enable {
		/// Sampling frequency in Hz. Falls back to the configured rate when omitted.
		#[clap(short, long)]
		sample_rate: Option<u32>,

		/// API peer endpoint
		#[clap(long, default_value = "http://localhost:6421")]
		endpoint: String,
	},
	/// Disable the Pyroscope CPU profiler (broadcast to all engine pods in the datacenter)
	Disable {
		/// API peer endpoint
		#[clap(long, default_value = "http://localhost:6421")]
		endpoint: String,
	},
}

#[derive(Serialize, Deserialize)]
struct SetProfileConfigRequest {
	pub enabled: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub sample_rate: Option<u32>,
}

impl SubCommand {
	pub async fn execute(self, _config: rivet_config::Config) -> Result<()> {
		let (enabled, sample_rate, endpoint) = match self {
			Self::Enable {
				sample_rate,
				endpoint,
			} => (true, sample_rate, endpoint),
			Self::Disable { endpoint } => (false, None, endpoint),
		};

		let request = SetProfileConfigRequest {
			enabled,
			sample_rate,
		};

		let client = rivet_pools::reqwest::client().await?;
		let url = format!("{}/debug/profile/config", endpoint);

		let response = client
			.put(&url)
			.json(&request)
			.send()
			.await
			.context("failed to send request")?;

		if response.status().is_success() {
			if enabled {
				println!("Profiler enabled");
			} else {
				println!("Profiler disabled");
			}
			Ok(())
		} else {
			let status = response.status();
			let body = response.text().await.unwrap_or_default();
			bail!(
				"Failed to update profile configuration: {} - {}",
				status,
				body
			);
		}
	}
}
