pub mod bridge_actor;
pub mod database;
pub mod envoy_handle;
pub mod types;

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Once;

use napi_derive::napi;
use rivet_envoy_client::config::EnvoyConfig;
use rivet_envoy_client::envoy::start_envoy_sync;
use tokio::runtime::Runtime;

static INIT_TRACING: Once = Once::new();

fn init_tracing(log_level: Option<&str>) {
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

use crate::bridge_actor::{BridgeCallbacks, ResponseMap, WsSenderMap};
use crate::envoy_handle::JsEnvoyHandle;
use crate::types::JsEnvoyConfig;

/// Start the native envoy client synchronously.
///
/// Returns a handle immediately. The caller must call `await handle.started()`
/// to wait for the connection to be ready.
#[napi]
pub fn start_envoy_sync_js(
	config: JsEnvoyConfig,
	#[napi(ts_arg_type = "(event: any) => void")] event_callback: napi::JsFunction,
) -> napi::Result<JsEnvoyHandle> {
	init_tracing(config.log_level.as_deref());

	let runtime = Runtime::new()
		.map_err(|e| napi::Error::from_reason(format!("failed to create tokio runtime: {}", e)))?;
	let runtime = Arc::new(runtime);

	let response_map: ResponseMap = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
	let ws_sender_map: WsSenderMap = Arc::new(tokio::sync::Mutex::new(HashMap::new()));

	// Create threadsafe callback for bridging events to JS
	let tsfn: bridge_actor::EventCallback = event_callback.create_threadsafe_function(
		0,
		|ctx: napi::threadsafe_function::ThreadSafeCallContext<serde_json::Value>| {
			let env = ctx.env;
			let value = env.to_js_value(&ctx.value)?;
			Ok(vec![value])
		},
	)?;

	let callbacks = Arc::new(BridgeCallbacks::new(
		tsfn.clone(),
		response_map.clone(),
		ws_sender_map.clone(),
	));

	let metadata: Option<HashMap<String, String>> = config.metadata.and_then(|v| {
		if let serde_json::Value::Object(map) = v {
			Some(map.into_iter().map(|(k, v)| (k, v.to_string())).collect())
		} else {
			None
		}
	});

	let envoy_config = EnvoyConfig {
		version: config.version,
		endpoint: config.endpoint,
		token: Some(config.token),
		namespace: config.namespace,
		pool_name: config.pool_name,
		prepopulate_actor_names: HashMap::new(),
		metadata,
		not_global: config.not_global,
		debug_latency_ms: None,
		callbacks,
	};

	let _guard = runtime.enter();
	let handle = start_envoy_sync(envoy_config);

	Ok(JsEnvoyHandle::new(
		runtime,
		handle,
		response_map,
		ws_sender_map,
	))
}

/// Start the native envoy client asynchronously.
#[napi]
pub fn start_envoy_js(
	config: JsEnvoyConfig,
	#[napi(ts_arg_type = "(event: any) => void")] event_callback: napi::JsFunction,
) -> napi::Result<JsEnvoyHandle> {
	start_envoy_sync_js(config, event_callback)
}
