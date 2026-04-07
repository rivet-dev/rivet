use anyhow::{Context, Result};
use async_stream::stream;
use axum::{
	Router,
	body::Bytes,
	extract::State,
	response::{
		IntoResponse,
		Json,
		Sse,
		sse::{Event, KeepAlive},
	},
	routing::{get, post},
};
use rivet_envoy_protocol as protocol;
use serde_json::json;
use std::{convert::Infallible, sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::Mutex};
use tracing_subscriber::EnvFilter;

use crate::{EchoActor, Envoy, EnvoyBuilder, EnvoyConfig};

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
			pool_name: std::env::var("RIVET_POOL_NAME").unwrap_or_else(|_| "test-envoy".to_string()),
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
	envoy: Arc<Mutex<Option<Arc<Envoy>>>>,
}

pub async fn run_from_env() -> Result<()> {
	init_tracing();

	let settings = Settings::from_env();
	let state = AppState {
		settings: settings.clone(),
		envoy: Arc::new(Mutex::new(None)),
	};

	if settings.autostart_envoy {
		let envoy = start_envoy(&settings).await?;
		*state.envoy.lock().await = Some(envoy);
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

	tracing::info!(port = state.settings.internal_server_port, "internal http server listening");

	axum::serve(listener, app).await.context("http server failed")
}

async fn health() -> &'static str {
	"ok"
}

async fn shutdown(State(state): State<AppState>) -> &'static str {
	if let Some(envoy) = state.envoy.lock().await.clone() {
		let _ = envoy.shutdown().await;
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

async fn start_serverless(
	State(state): State<AppState>,
	body: Bytes,
) -> impl IntoResponse {
	tracing::info!("received serverless start request");

	let envoy = match start_envoy(&state.settings).await {
		Ok(envoy) => envoy,
		Err(err) => {
			tracing::error!(?err, "failed to start serverless envoy");
			return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
		}
	};

	if let Err(err) = envoy.start_serverless_actor(body.as_ref()).await {
		tracing::error!(?err, "failed to inject serverless start payload");
		return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response();
	}

	*state.envoy.lock().await = Some(envoy.clone());

	let stream = stream! {
		let mut interval = tokio::time::interval(Duration::from_secs(1));
		loop {
			interval.tick().await;
			yield Ok::<Event, Infallible>(Event::default().event("ping").data(""));
		}
	};

	Sse::new(stream)
		.keep_alive(KeepAlive::default())
		.into_response()
}

async fn start_envoy(settings: &Settings) -> Result<Arc<Envoy>> {
	let config = EnvoyConfig::builder()
		.endpoint(&settings.endpoint)
		.token(&settings.token)
		.namespace(&settings.namespace)
		.pool_name(&settings.pool_name)
		.version(settings.envoy_version)
		.build()?;

	let envoy = EnvoyBuilder::new(config)
		.with_default_actor_behavior(|_config| Box::new(EchoActor::new()))
		.build()?;
	let envoy = Arc::new(envoy);

	envoy.start().await?;
	envoy.wait_ready().await;

	Ok(envoy)
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
