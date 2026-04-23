pub mod actor_context;
pub mod actor_factory;
pub mod cancellation_token;
pub mod connection;
pub mod database;
pub mod kv;
pub mod napi_actor_events;
pub mod queue;
pub mod registry;
pub mod schedule;
pub mod types;
pub mod websocket;

use std::sync::Once;

use rivet_error::RivetError as RivetTransportError;

static INIT_TRACING: Once = Once::new();
pub(crate) const BRIDGE_RIVET_ERROR_PREFIX: &str = "__RIVET_ERROR_JSON__:";

#[derive(rivet_error::RivetError, serde::Serialize)]
#[error(
	"napi",
	"invalid_argument",
	"Invalid native argument",
	"Invalid native argument '{argument}': {reason}"
)]
pub(crate) struct NapiInvalidArgument {
	pub(crate) argument: String,
	pub(crate) reason: String,
}

#[derive(rivet_error::RivetError, serde::Serialize)]
#[error(
	"napi",
	"invalid_state",
	"Invalid native state",
	"Invalid native state '{state}': {reason}"
)]
pub(crate) struct NapiInvalidState {
	pub(crate) state: String,
	pub(crate) reason: String,
}

pub(crate) fn napi_anyhow_error(error: anyhow::Error) -> napi::Error {
	let bridge_context = error
		.chain()
		.find_map(|cause| cause.downcast_ref::<crate::actor_factory::BridgeRivetErrorContext>());
	let error = RivetTransportError::extract(&error);
	let public_ = bridge_context.and_then(|context| context.public_);
	let status_code = bridge_context.and_then(|context| context.status_code);
	let payload = serde_json::json!({
		"group": error.group(),
		"code": error.code(),
		"message": error.message(),
		"metadata": error.metadata(),
		"public": public_,
		"statusCode": status_code,
	});
	tracing::debug!(
		group = error.group(),
		code = error.code(),
		has_metadata = error.metadata().is_some(),
		?public_,
		?status_code,
		"encoded structured bridge error"
	);
	napi::Error::from_reason(format!("{BRIDGE_RIVET_ERROR_PREFIX}{}", payload))
}

pub(crate) fn init_tracing(log_level: Option<&str>) {
	INIT_TRACING.call_once(|| {
		// Priority: explicit config > RIVET_LOG_LEVEL > LOG_LEVEL > RUST_LOG > "warn"
		let filter = log_level
			.map(String::from)
			.or_else(|| std::env::var("RIVET_LOG_LEVEL").ok())
			.or_else(|| std::env::var("LOG_LEVEL").ok())
			.or_else(|| std::env::var("RUST_LOG").ok())
			.unwrap_or_else(|| "warn".to_string());

		tracing_subscriber::fmt()
			.with_env_filter(tracing_subscriber::EnvFilter::new(&filter))
			.with_target(true)
			.with_writer(std::io::stdout)
			.init();
	});
}
