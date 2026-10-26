use rivet_error::RivetError;
use serde::{Deserialize, Serialize};

#[derive(RivetError, Debug, Clone, Deserialize, Serialize)]
#[error("engine")]
pub enum EngineProcessError {
	#[error(
		"binary_not_found",
		"Engine binary was not found.",
		"Engine binary was not found at '{path}'. Point RIVET_ENGINE_BINARY_PATH at an existing rivet-engine binary, or unset it so RivetKit downloads the Engine."
	)]
	BinaryNotFound { path: String },

	#[error(
		"binary_unavailable",
		"Engine binary is unavailable.",
		"No usable engine binary was found for version '{version}' and downloading it is turned off. Build `rivet-engine`, set `RIVET_ENGINE_BINARY_PATH`, turn downloads back on with `RIVETKIT_ENGINE_AUTO_DOWNLOAD=1` (or `engine_auto_download` in a Rust `ServeConfig`), or set `RIVET_ENDPOINT` to a remote Rivet Engine."
	)]
	BinaryUnavailable { version: String },

	#[error(
		"invalid_endpoint",
		"Engine endpoint is invalid.",
		"Engine endpoint '{endpoint}' is invalid: {reason}"
	)]
	InvalidEndpoint { endpoint: String, reason: String },

	#[error("missing_pid", "Engine process is missing a pid.")]
	MissingPid,

	#[error(
		"health_check_failed",
		"Engine health check failed.",
		"Engine health check failed after {attempts} attempts: {reason}"
	)]
	HealthCheckFailed { attempts: u32, reason: String },

	#[error(
		"port_occupied",
		"Engine port is occupied by a different runtime.",
		"Cannot start engine: endpoint '{endpoint}' is already serving runtime '{runtime}'. Stop that process and retry."
	)]
	PortOccupied { endpoint: String, runtime: String },
}
