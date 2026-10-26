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
		"No usable engine binary was found for version '{version}'. Build `rivet-engine`, set `RIVET_ENGINE_BINARY_PATH`, or install the matching `@rivetkit/engine-cli` platform package."
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

	#[error(
		"incompatible_engine",
		"An incompatible engine is already running on the endpoint.",
		"Cannot use the engine at '{endpoint}': it is Rivet Engine {version} with architecture '{architecture}', but this RivetKit requires a {required} engine. Stop that engine process (it may have been started by an older RivetKit) and retry, or point RIVET_ENDPOINT at a {required} engine."
	)]
	IncompatibleEngine {
		endpoint: String,
		version: String,
		architecture: String,
		required: String,
	},

	#[error(
		"incompatible_binary",
		"The Engine binary is incompatible with this RivetKit.",
		"Cannot start Engine binary '{path}': {reason}; this RivetKit requires a {required} engine. Build the matching v3 Engine and configure its executable for local startup (RIVET_ENGINE_BINARY in JavaScript, RIVET_ENGINE_BINARY_PATH in Rust)."
	)]
	IncompatibleBinary {
		path: String,
		reason: String,
		required: String,
	},

	#[error(
		"not_ready",
		"Engine did not become ready.",
		"Engine answered health checks but could not serve actor requests after {attempts} attempts: {reason}"
	)]
	NotReady { attempts: u32, reason: String },
}
