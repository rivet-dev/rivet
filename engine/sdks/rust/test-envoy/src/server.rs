use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, Result};
use async_stream::stream;
use axum::{
	Router,
	body::Bytes,
	extract::State,
	response::{
		IntoResponse, Json, Sse,
		sse::{Event, KeepAlive},
	},
	routing::{get, post},
};
use rivet_envoy_protocol as protocol;
use serde_json::json;
use std::convert::Infallible;
use std::time::Duration;
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

use crate::behaviors::DefaultTestCallbacks;
use rivet_envoy_client::config::EnvoyConfig;
use rivet_envoy_client::envoy::start_envoy_sync;
use rivet_envoy_client::handle::EnvoyHandle;

#[derive(Clone)]
struct Settings {
	internal_server_port: u16,
	namespace: String,
	pool_name: String,
	envoy_version: u32,
	endpoint: String,
	token: String,
	autostart_server: bool,
	autostart_envoy: bool,
	autoconfigure_serverless: bool,
}

impl Settings {
	fn from_env() -> Self {
		Self {
			internal_server_port: std::env::var("INTERNAL_SERVER_PORT")
				.ok()
				.and_then(|value| value.parse().ok())
				.unwrap_or(5051),
			namespace: std::env::var("RIVET_NAMESPACE").unwrap_or_else(|_| "default".to_string()),
			pool_name: std::env::var("RIVET_POOL_NAME")
				.unwrap_or_else(|_| "test-envoy".to_string()),
			envoy_version: std::env::var("RIVET_ENVOY_VERSION")
				.ok()
				.and_then(|value| value.parse().ok())
				.unwrap_or(1),
			endpoint: std::env::var("RIVET_ENDPOINT")
				.unwrap_or_else(|_| "http://127.0.0.1:6420".to_string()),
			token: std::env::var("RIVET_TOKEN").unwrap_or_else(|_| "dev".to_string()),
			autostart_server: read_bool_env("AUTOSTART_SERVER", true),
			autostart_envoy: read_bool_env("AUTOSTART_ENVOY", false),
			autoconfigure_serverless: read_bool_env("AUTOCONFIGURE_SERVERLESS", true),
		}
	}
}

#[derive(Clone)]
struct AppState {
	settings: Settings,
	envoy_handle: Arc<tokio::sync::Mutex<Option<EnvoyHandle>>>,
}

pub async fn run_from_env() -> Result<()> {
	init_tracing();

	let settings = Settings::from_env();
	let state = AppState {
		settings: settings.clone(),
		envoy_handle: Arc::new(tokio::sync::Mutex::new(None)),
	};

	if settings.autostart_envoy {
		let (handle, _) = create_envoy(&settings);
		*state.envoy_handle.lock().await = Some(handle);
	} else if settings.autoconfigure_serverless {
		auto_configure_serverless(&settings).await?;
	}

	let server = if settings.autostart_server {
		Some(tokio::spawn(run_http_server(state.clone())))
	} else {
		None
	};

	install_signal_handlers();

	if let Some(server) = server {
		server.await.context("http server task failed")??;
	} else if settings.autostart_envoy {
		std::future::pending::<()>().await;
	}

	Ok(())
}

async fn run_http_server(state: AppState) -> Result<()> {
	let app = Router::new()
		.route("/health", get(health))
		.route("/shutdown", get(shutdown))
		.route("/api/rivet/start", post(start_serverless))
		.route("/api/rivet/metadata", get(metadata))
		.with_state(state.clone());

	let addr = format!("0.0.0.0:{}", state.settings.internal_server_port);
	let listener = TcpListener::bind(&addr)
		.await
		.with_context(|| format!("failed to bind {addr}"))?;

	tracing::info!(
		port = state.settings.internal_server_port,
		"internal http server listening"
	);

	axum::serve(listener, app)
		.await
		.context("http server failed")
}

async fn health() -> &'static str {
	"ok"
}

async fn shutdown(State(state): State<AppState>) -> &'static str {
	if let Some(handle) = state.envoy_handle.lock().await.as_ref() {
		handle.shutdown(false);
	}
	"ok"
}

async fn metadata() -> Json<serde_json::Value> {
	Json(json!({
		"runtime": "rivetkit",
		"version": "1",
		"envoyProtocolVersion": protocol::PROTOCOL_VERSION,
	}))
}

async fn start_serverless(State(state): State<AppState>, body: Bytes) -> impl IntoResponse {
	tracing::info!("received serverless start request");

	let (handle, is_shutdown) = create_envoy(&state.settings);

	// Inject the serverless start payload
	let handle_clone = handle.clone();
	tokio::spawn(async move {
		if let Err(err) = handle_clone.start_serverless_actor(&body).await {
			tracing::error!(?err, "failed to inject serverless start payload");
		}
	});

	*state.envoy_handle.lock().await = Some(handle);

	let stream = stream! {
		let mut interval = tokio::time::interval(Duration::from_secs(1));
		loop {
			interval.tick().await;

			if is_shutdown.load(Ordering::SeqCst) {
				tracing::debug!("envoy shutdown, aborting SSE");
				return;
			}

			yield Ok::<Event, Infallible>(Event::default().event("ping").data(""));
		}
	};

	Sse::new(stream)
		.keep_alive(KeepAlive::default())
		.into_response()
}

fn create_envoy(settings: &Settings) -> (EnvoyHandle, Arc<AtomicBool>) {
	let cbs = DefaultTestCallbacks::default();
	let is_shutdown = cbs.is_shutdown.clone();

	let config = EnvoyConfig {
		version: settings.envoy_version,
		endpoint: settings.endpoint.clone(),
		token: Some(settings.token.clone()),
		namespace: settings.namespace.clone(),
		pool_name: settings.pool_name.clone(),
		prepopulate_actor_names: std::collections::HashMap::new(),
		metadata: None,
		not_global: false,
		debug_latency_ms: None,
		callbacks: Arc::new(cbs),
	};

	(start_envoy_sync(config), is_shutdown)
}

async fn auto_configure_serverless(settings: &Settings) -> Result<()> {
	tracing::info!("configuring serverless");

	let client = reqwest::Client::new();
	let url = format!(
		"{}/runner-configs/{}?namespace={}",
		settings.endpoint.trim_end_matches('/'),
		settings.pool_name,
		settings.namespace,
	);
	let body = json!({
		"datacenters": {
			"default": {
				"serverless": {
					"url": format!("http://localhost:{}/api/rivet", settings.internal_server_port),
					"request_lifespan": 300,
					"max_concurrent_actors": 10000,
					"max_runners": 10000,
					"slots_per_runner": 1
				}
			}
		}
	});

	let response = client
		.put(url)
		.bearer_auth(&settings.token)
		.json(&body)
		.send()
		.await
		.context("failed to upsert serverless config")?;

	if !response.status().is_success() {
		let status = response.status();
		let text = response.text().await.unwrap_or_default();
		anyhow::bail!("serverless config request failed: {}: {}", status, text);
	}

	Ok(())
}

fn init_tracing() {
	let filter = EnvFilter::try_from_default_env()
		.unwrap_or_else(|_| EnvFilter::new("info,rivet_test_envoy=debug,rivet_envoy_client=debug"));

	let _ = tracing_subscriber::fmt()
		.with_env_filter(filter)
		.with_target(false)
		.with_ansi(true)
		.try_init();
}

fn install_signal_handlers() {
	tokio::spawn(async {
		if tokio::signal::ctrl_c().await.is_ok() {
			tracing::debug!("received stop signal, force exiting in 3s");
			tokio::time::sleep(Duration::from_secs(3)).await;
			std::process::exit(0);
		}
	});
}

fn read_bool_env(name: &str, default: bool) -> bool {
	match std::env::var(name) {
		Ok(value) => value == "1",
		Err(_) => default,
	}
}
