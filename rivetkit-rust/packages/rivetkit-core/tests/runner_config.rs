use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Method, Request, StatusCode};
use axum::response::Response;
use reqwest::Client;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use super::{ServeConfig, ensure_local_normal_runner_config_with_retry};

#[derive(Debug)]
struct FakeEngineState {
	failed_upserts: usize,
	datacenter_requests: AtomicUsize,
	upsert_requests: AtomicUsize,
}

async fn fake_engine_handler(
	State(state): State<Arc<FakeEngineState>>,
	request: Request<Body>,
) -> Response {
	match (request.method(), request.uri().path()) {
		(&Method::GET, "/datacenters") => {
			state.datacenter_requests.fetch_add(1, Ordering::SeqCst);
			Response::builder()
				.status(StatusCode::OK)
				.header("content-type", "application/json")
				.body(Body::from(r#"{"datacenters":[{"name":"default"}]}"#))
				.expect("build datacenters response")
		}
		(&Method::PUT, "/runner-configs/default") => {
			let attempt = state.upsert_requests.fetch_add(1, Ordering::SeqCst);
			if attempt < state.failed_upserts {
				Response::builder()
					.status(StatusCode::BAD_REQUEST)
					.header("content-type", "application/json")
					.body(Body::from(r#"{"group":"namespace","code":"not_found"}"#))
					.expect("build not-found response")
			} else {
				Response::builder()
					.status(StatusCode::OK)
					.body(Body::empty())
					.expect("build upsert response")
			}
		}
		_ => Response::builder()
			.status(StatusCode::NOT_FOUND)
			.body(Body::empty())
			.expect("build fallback response"),
	}
}

async fn start_fake_engine(
	failed_upserts: usize,
) -> (
	String,
	Arc<FakeEngineState>,
	oneshot::Sender<()>,
	tokio::task::JoinHandle<()>,
) {
	let state = Arc::new(FakeEngineState {
		failed_upserts,
		datacenter_requests: AtomicUsize::new(0),
		upsert_requests: AtomicUsize::new(0),
	});
	let app = Router::new()
		.fallback(fake_engine_handler)
		.with_state(state.clone());
	let listener = TcpListener::bind("127.0.0.1:0")
		.await
		.expect("bind fake Engine");
	let address = listener.local_addr().expect("read fake Engine address");
	let (shutdown_tx, shutdown_rx) = oneshot::channel();
	let server = tokio::spawn(async move {
		axum::serve(listener, app)
			.with_graceful_shutdown(async move {
				let _ = shutdown_rx.await;
			})
			.await
			.expect("serve fake Engine");
	});

	(format!("http://{address}"), state, shutdown_tx, server)
}

fn config(endpoint: String) -> ServeConfig {
	ServeConfig {
		endpoint,
		token: Some("dev".to_owned()),
		namespace: "default".to_owned(),
		pool_name: "default".to_owned(),
		..ServeConfig::default()
	}
}

#[tokio::test]
async fn retries_namespace_not_found_until_runner_config_is_ready() {
	let (endpoint, state, shutdown, server) = start_fake_engine(1).await;

	ensure_local_normal_runner_config_with_retry(
		&Client::new(),
		&config(endpoint),
		2,
		Duration::ZERO,
	)
	.await
	.expect("runner config should become ready");

	assert_eq!(state.datacenter_requests.load(Ordering::SeqCst), 2);
	assert_eq!(state.upsert_requests.load(Ordering::SeqCst), 2);
	let _ = shutdown.send(());
	server.await.expect("join fake Engine");
}

#[tokio::test]
async fn stops_after_the_configured_retry_bound() {
	let (endpoint, state, shutdown, server) = start_fake_engine(usize::MAX).await;

	let error = ensure_local_normal_runner_config_with_retry(
		&Client::new(),
		&config(endpoint),
		3,
		Duration::ZERO,
	)
	.await
	.expect_err("runner config should exhaust its retry bound");

	assert!(error.to_string().contains("did not become ready after 3 attempts"));
	assert_eq!(state.datacenter_requests.load(Ordering::SeqCst), 3);
	assert_eq!(state.upsert_requests.load(Ordering::SeqCst), 3);
	let _ = shutdown.send(());
	server.await.expect("join fake Engine");
}
