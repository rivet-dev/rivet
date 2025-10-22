use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Telemetry {
	pub enabled: bool,
}

impl Default for Telemetry {
	fn default() -> Self {
		// NOTE: Telemetry is opt-out
		Telemetry { enabled: true }
	}
}
