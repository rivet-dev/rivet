use std::net::IpAddr;

use anyhow::{Result, ensure};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Configuration for the metrics service.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct Metrics {
	pub host: Option<IpAddr>,
	pub port: Option<u16>,
	/// Host hardware metrics, including a short disk probe of each data directory at startup.
	#[serde(default)]
	pub hardware: HardwareMetrics,
}

impl Metrics {
	pub fn host(&self) -> IpAddr {
		self.host.unwrap_or(crate::defaults::hosts::METRICS)
	}

	pub fn port(&self) -> u16 {
		self.port.unwrap_or(crate::defaults::ports::METRICS)
	}
}

/// Largest disk probe accepted, so a misconfiguration cannot fill a disk or stall startup I/O.
pub const MAX_DISK_PROBE_BYTES: u64 = 1024 * 1024 * 1024;

/// Configuration for host hardware metrics.
#[derive(Debug, Serialize, Deserialize, Clone, Default, JsonSchema)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct HardwareMetrics {
	/// Times sequential writes and fsyncs on each data directory once at startup, in the
	/// background. Defaults to true.
	pub disk_probe: Option<bool>,
	/// Bytes the probe writes sequentially to each data directory. The fsync phase writes a
	/// further 400 KiB. Defaults to 64 MiB; at most 1 GiB.
	pub disk_probe_bytes: Option<u64>,
}

impl HardwareMetrics {
	pub fn disk_probe(&self) -> bool {
		self.disk_probe.unwrap_or(true)
	}

	pub fn disk_probe_bytes(&self) -> u64 {
		self.disk_probe_bytes.unwrap_or(64 * 1024 * 1024)
	}

	pub fn validate(&self) -> Result<()> {
		let bytes = self.disk_probe_bytes();
		ensure!(
			bytes > 0 && bytes <= MAX_DISK_PROBE_BYTES,
			"metrics.hardware.disk_probe_bytes must be between 1 and {MAX_DISK_PROBE_BYTES}; set metrics.hardware.disk_probe to false to skip the probe"
		);
		Ok(())
	}
}
