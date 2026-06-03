use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Pyroscope {
	/// Base URL of the Pyroscope server profiles are pushed to. Presence of this block means
	/// profiling is available; the profiler itself is toggled at runtime via the
	/// `profile enable`/`profile disable` CLI and starts off.
	pub server_url: String,
	/// Sampling frequency in Hz.
	#[serde(default)]
	pub sample_rate: Option<u32>,
}

impl Pyroscope {
	pub fn sample_rate(&self) -> u32 {
		self.sample_rate.unwrap_or(100)
	}
}
