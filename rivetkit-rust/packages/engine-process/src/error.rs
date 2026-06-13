use rivet_error::RivetError;
use serde::{Deserialize, Serialize};

#[derive(RivetError, Debug, Clone, Deserialize, Serialize)]
#[error("engine")]
pub enum EngineProcessError {
	#[error(
		"binary_not_found",
		"Engine binary was not found.",
		"Engine binary was not found at '{path}'."
	)]
	BinaryNotFound { path: String },

	#[error(
		"binary_unavailable",
		"Engine binary is unavailable.",
		"No usable engine binary was found for version '{version}'. Build `rivet-engine`, set `RIVET_ENGINE_BINARY_PATH`, or enable `RIVETKIT_ENGINE_AUTO_DOWNLOAD=1`."
	)]
	BinaryUnavailable { version: String },

	#[error(
		"download_failed",
		"Engine binary download failed.",
		"Engine binary download failed for '{url}': {reason}"
	)]
	DownloadFailed { url: String, reason: String },

	#[error(
		"checksum_mismatch",
		"Engine binary checksum mismatch.",
		"Engine binary checksum mismatch for '{artifact}': expected {expected}, received {received}."
	)]
	ChecksumMismatch {
		artifact: String,
		expected: String,
		received: String,
	},

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
