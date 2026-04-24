use rivet_error::*;
use serde::{Deserialize, Serialize};

#[derive(RivetError, Debug, Clone, Deserialize, Serialize)]
#[error("actor")]
pub enum ActorLifecycle {
	#[error("starting", "Actor is starting.")]
	Starting,

	#[error("not_ready", "Actor is not ready.")]
	NotReady,

	#[error("stopping", "Actor is stopping.")]
	Stopping,

	#[error("destroying", "Actor is destroying.")]
	Destroying,

	#[error("shutdown_timeout", "Actor shutdown timed out.")]
	ShutdownTimeout,

	#[error("dropped_reply", "Actor reply channel was dropped without a response.")]
	DroppedReply,

	#[error(
		"overloaded",
		"Actor is overloaded.",
		"Actor channel '{channel}' is overloaded while attempting to {operation} (capacity {capacity})."
	)]
	Overloaded {
		channel: String,
		capacity: usize,
		operation: String,
	},
}

#[derive(RivetError, Debug, Clone, Deserialize, Serialize)]
#[error("actor")]
pub enum ActorRuntime {
	#[error(
		"not_configured",
		"Actor capability is not configured.",
		"Actor capability '{component}' is not configured."
	)]
	NotConfigured { component: String },

	#[error(
		"not_found",
		"Actor resource was not found.",
		"Actor {resource} '{id}' was not found."
	)]
	NotFound { resource: String, id: String },

	#[error(
		"not_registered",
		"Actor factory is not registered.",
		"Actor factory '{actor_name}' is not registered."
	)]
	NotRegistered { actor_name: String },

	#[error("missing_input", "Actor input is missing.")]
	MissingInput,

	#[error(
		"invalid_operation",
		"Actor operation is invalid.",
		"Actor operation '{operation}' is invalid: {reason}"
	)]
	InvalidOperation { operation: String, reason: String },

	#[error(
		"panicked",
		"Actor task panicked.",
		"Actor task panicked while running {operation}."
	)]
	Panicked { operation: String },
}

#[derive(RivetError, Debug, Clone, Deserialize, Serialize)]
#[error("protocol")]
pub(crate) enum ProtocolError {
	#[error(
		"invalid_http_request",
		"Invalid HTTP request.",
		"Invalid HTTP request {field}: {reason}"
	)]
	InvalidHttpRequest { field: String, reason: String },

	#[error(
		"invalid_http_response",
		"Invalid HTTP response.",
		"Invalid HTTP response {field}: {reason}"
	)]
	InvalidHttpResponse { field: String, reason: String },

	#[error(
		"invalid_actor_connect_request",
		"Invalid actor-connect request.",
		"Invalid actor-connect request {field}: {reason}"
	)]
	InvalidActorConnectRequest { field: String, reason: String },

	#[error(
		"invalid_persisted_data",
		"Invalid persisted actor data.",
		"Invalid persisted {label}: {reason}"
	)]
	InvalidPersistedData { label: String, reason: String },

	#[error(
		"unsupported_encoding",
		"Unsupported protocol encoding.",
		"Unsupported protocol encoding '{encoding}'."
	)]
	UnsupportedEncoding { encoding: String },
}

#[derive(RivetError, Debug, Clone, Deserialize, Serialize)]
#[error("sqlite")]
pub(crate) enum SqliteRuntimeError {
	#[error(
		"unavailable",
		"SQLite is unavailable.",
		"Actor database is not available because rivetkit-core was built without the sqlite feature."
	)]
	Unavailable,

	#[error("closed", "SQLite database is closed.")]
	Closed,

	#[error(
		"not_configured",
		"SQLite is not configured.",
		"SQLite {component} is not configured."
	)]
	NotConfigured { component: String },

	#[error(
		"invalid_bind_parameter",
		"Invalid SQLite bind parameter.",
		"Invalid SQLite bind parameter {name}: {reason}"
	)]
	InvalidBindParameter { name: String, reason: String },
}

#[derive(RivetError, Debug, Clone, Deserialize, Serialize)]
#[error("engine")]
pub(crate) enum EngineProcessError {
	#[error(
		"binary_not_found",
		"Engine binary was not found.",
		"Engine binary was not found at '{path}'."
	)]
	BinaryNotFound { path: String },

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
