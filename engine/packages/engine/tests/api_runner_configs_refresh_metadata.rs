#[path = "common/api/mod.rs"]
mod api;
#[path = "common/ctx.rs"]
mod ctx;

use axum::{
	Json, Router,
	body::Bytes,
	extract::State,
	http::StatusCode,
	response::{
		IntoResponse, Response, Sse,
		sse::{Event, KeepAlive},
	},
	routing::{get, post},
};
use futures_util::stream;
use serde_json::json;
use std::collections::HashMap;
use std::convert::Infallible;
use std::future::Future;
use std::sync::{
	Arc,
	atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tokio::sync::mpsc;

struct MockServerlessState {
	expose_protocol_version: AtomicBool,
	start_tx: mpsc::UnboundedSender<()>,
}

struct MockMetadataErrorState {
	body: String,
}

async fn metadata_handler(
	State(state): State<Arc<MockServerlessState>>,
) -> Json<serde_json::Value> {
	let mut response = json!({
		"runtime": "rivetkit",
		"version": "1",
	});

	if state.expose_protocol_version.load(Ordering::SeqCst) {
		response["envoyProtocolVersion"] = json!(rivet_envoy_protocol::PROTOCOL_VERSION);
	}

	Json(response)
}

async fn metadata_error_handler(State(state): State<Arc<MockMetadataErrorState>>) -> Response {
	(
		StatusCode::OK,
		[(axum::http::header::CONTENT_TYPE, "application/json")],
		state.body.clone(),
	)
		.into_response()
}

async fn start_handler(
	State(state): State<Arc<MockServerlessState>>,
	_body: Bytes,
) -> impl IntoResponse {
	let _ = state.start_tx.send(());
	let events =
		stream::once(async { Ok::<Event, Infallible>(Event::default().event("ping").data("")) });

	Sse::new(events)
		.keep_alive(KeepAlive::default())
		.into_response()
}

fn run<F, Fut>(opts: ctx::TestOpts, test_fn: F)
where
	F: FnOnce(ctx::TestCtx) -> Fut,
	Fut: Future<Output = ()>,
{
	let runtime = tokio::runtime::Runtime::new().expect("failed to build runtime");
	runtime.block_on(async {
		let timeout = Duration::from_secs(opts.timeout_secs);
		let ctx = ctx::TestCtx::new_with_opts(opts)
			.await
			.expect("failed to build test ctx");
		tokio::time::timeout(timeout, test_fn(ctx))
			.await
			.expect("test timed out");
	});
}

async fn setup_test_namespace(leader_dc: &ctx::TestDatacenter) -> (String, rivet_util::Id) {
	let random_suffix = rand::random::<u16>();
	let namespace_name = format!("test-{random_suffix}");
	let response = api::public::namespaces_create(
		leader_dc.guard_port(),
		rivet_api_peer::namespaces::CreateRequest {
			name: namespace_name,
			display_name: "Test Namespace".to_string(),
		},
	)
	.await
	.expect("failed to set up test namespace");

	(response.namespace.name, response.namespace.namespace_id)
}

#[test]
fn refresh_metadata_invalidates_protocol_cache_before_v2_dispatch() {
	run(
		ctx::TestOpts::new(1)
			.with_timeout(30)
			.with_pegboard_outbound(),
		|ctx| async move {
			let (namespace, namespace_id) = setup_test_namespace(ctx.leader_dc()).await;
			let runner_name = "metadata-refresh-v2-dispatch";

			let (start_tx, mut start_rx) = mpsc::unbounded_channel();
			let mock_state = Arc::new(MockServerlessState {
				expose_protocol_version: AtomicBool::new(false),
				start_tx,
			});
			let app = Router::new()
				.route("/metadata", get(metadata_handler))
				.route("/start", post(start_handler))
				.with_state(mock_state.clone());

			let mock_port = portpicker::pick_unused_port().expect("failed to pick port");
			let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{mock_port}"))
				.await
				.expect("failed to bind mock serverless endpoint");
			let server_handle = tokio::spawn(async move {
				axum::serve(listener, app).await.expect("server error");
			});

			let mut datacenters = HashMap::new();
			datacenters.insert(
				"dc-1".to_string(),
				rivet_api_types::namespaces::runner_configs::RunnerConfig {
					kind:
						rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Serverless {
							url: format!("http://127.0.0.1:{mock_port}"),
							headers: None,
							request_lifespan: 30,
							max_concurrent_actors: Some(10),
							drain_grace_period: None,
							slots_per_runner: 1,
							min_runners: Some(0),
							max_runners: 0,
							runners_margin: Some(0),
							metadata_poll_interval: None,
						},
					metadata: None,
					drain_on_version_upgrade: true,
				},
			);

			api::public::runner_configs_upsert(
				ctx.leader_dc().guard_port(),
				rivet_api_peer::runner_configs::UpsertPath {
					runner_name: runner_name.to_string(),
				},
				rivet_api_peer::runner_configs::UpsertQuery {
					namespace: namespace.clone(),
				},
				rivet_api_public::runner_configs::upsert::UpsertRequest { datacenters },
			)
			.await
			.expect("failed to upsert serverless runner config");

			let cached_before_refresh = ctx
				.leader_dc()
				.workflow_ctx
				.op(pegboard::ops::runner_config::get::Input {
					runners: vec![(namespace_id, runner_name.to_string())],
					bypass_cache: false,
				})
				.await
				.expect("failed to read cached runner config");
			assert_eq!(cached_before_refresh[0].protocol_version, None);

			mock_state
				.expose_protocol_version
				.store(true, Ordering::SeqCst);

			api::public::runner_configs_refresh_metadata(
				ctx.leader_dc().guard_port(),
				runner_name.to_string(),
				api::public::RefreshMetadataQuery {
					namespace: namespace.clone(),
				},
				api::public::RefreshMetadataRequest {},
			)
			.await
			.expect("failed to refresh metadata");

			tokio::time::timeout(Duration::from_millis(100), async {
				let cached_after_refresh = ctx
					.leader_dc()
					.workflow_ctx
					.op(pegboard::ops::runner_config::get::Input {
						runners: vec![(namespace_id, runner_name.to_string())],
						bypass_cache: false,
					})
					.await
					.expect("failed to read refreshed runner config");
				assert_eq!(
					cached_after_refresh[0].protocol_version,
					Some(rivet_envoy_protocol::PROTOCOL_VERSION)
				);
			})
			.await
			.expect("refreshed protocol version should bypass the old 5s cache TTL");

			api::public::actors_create(
				ctx.leader_dc().guard_port(),
				rivet_api_types::actors::create::CreateQuery {
					namespace: namespace.clone(),
				},
				rivet_api_types::actors::create::CreateRequest {
					datacenter: None,
					name: "test-actor".to_string(),
					key: Some(format!("key-{}", rand::random::<u64>())),
					input: None,
					runner_name_selector: runner_name.to_string(),
					crash_policy: rivet_types::actors::CrashPolicy::Sleep,
				},
			)
			.await
			.expect("failed to create actor after metadata refresh");

			tokio::time::timeout(Duration::from_secs(2), start_rx.recv())
				.await
				.expect("v2 serverless dispatch should start immediately after refresh")
				.expect("mock serverless start channel closed");

			server_handle.abort();
		},
	);
}

#[test]
fn refresh_metadata_surfaces_invalid_envoy_protocol_version_in_error_metadata() {
	run(ctx::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = setup_test_namespace(ctx.leader_dc()).await;
		let runner_name = "metadata-refresh-invalid-envoy-protocol";
		let invalid_version = rivet_envoy_protocol::PROTOCOL_VERSION + 1;

		let app = Router::new()
			.route("/metadata", get(metadata_error_handler))
			.with_state(Arc::new(MockMetadataErrorState {
				body: format!(
					r#"{{"runtime":"rivetkit","version":"1","envoyProtocolVersion":{invalid_version}}}"#
				),
			}));

		let mock_port = portpicker::pick_unused_port().expect("failed to pick port");
		let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{mock_port}"))
			.await
			.expect("failed to bind mock serverless endpoint");
		let server_handle = tokio::spawn(async move {
			axum::serve(listener, app).await.expect("server error");
		});

		let mut datacenters = HashMap::new();
		datacenters.insert(
			"dc-1".to_string(),
			rivet_api_types::namespaces::runner_configs::RunnerConfig {
				kind: rivet_api_types::namespaces::runner_configs::RunnerConfigKind::Serverless {
					url: format!("http://127.0.0.1:{mock_port}"),
					headers: None,
					request_lifespan: 30,
					max_concurrent_actors: Some(10),
					drain_grace_period: None,
					slots_per_runner: 1,
					min_runners: Some(0),
					max_runners: 0,
					runners_margin: Some(0),
					metadata_poll_interval: None,
				},
				metadata: None,
				drain_on_version_upgrade: true,
			},
		);

		api::public::runner_configs_upsert(
			ctx.leader_dc().guard_port(),
			rivet_api_peer::runner_configs::UpsertPath {
				runner_name: runner_name.to_string(),
			},
			rivet_api_peer::runner_configs::UpsertQuery {
				namespace: namespace.clone(),
			},
			rivet_api_public::runner_configs::upsert::UpsertRequest { datacenters },
		)
		.await
		.expect("failed to upsert serverless runner config");

		let response = api::public::build_runner_configs_refresh_metadata_request(
			ctx.leader_dc().guard_port(),
			runner_name.to_string(),
			api::public::RefreshMetadataQuery {
				namespace: namespace.clone(),
			},
			api::public::RefreshMetadataRequest {},
		)
		.await
		.expect("failed to build refresh request")
		.send()
		.await
		.expect("failed to send refresh request");

		server_handle.abort();

		assert!(
			!response.status().is_success(),
			"refresh metadata should fail for an unsupported envoy protocol version"
		);

		let body: serde_json::Value = response
			.json()
			.await
			.expect("failed to decode refresh-metadata error body");

		assert_eq!(body["group"], "serverless_runner_pool");
		assert_eq!(body["code"], "failed_to_fetch_metadata");
		assert_eq!(
			body["metadata"]["reason"]["metadata"]["envoy_protocol_version"],
			invalid_version
		);
		assert_eq!(
			body["metadata"]["reason"]["metadata"]["max_supported_envoy_protocol_version"],
			rivet_envoy_protocol::PROTOCOL_VERSION
		);
		assert!(
			body["metadata"]["reason"]["message"]
				.as_str()
				.unwrap_or("")
				.to_ascii_lowercase()
				.contains("envoy protocol"),
			"message should mention envoy protocol version, got {:?}",
			body["metadata"]["reason"]["message"]
		);
	});
}
