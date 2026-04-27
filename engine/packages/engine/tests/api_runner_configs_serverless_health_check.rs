#[path = "common/api/mod.rs"]
mod api;
#[path = "common/ctx.rs"]
mod ctx;

use axum::{
	Router,
	extract::State,
	http::StatusCode,
	response::{IntoResponse, Response},
	routing::get,
};
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

struct MockMetadataResponse {
	status: StatusCode,
	body: String,
}

async fn metadata_handler(State(state): State<Arc<MockMetadataResponse>>) -> Response {
	(
		state.status,
		[(axum::http::header::CONTENT_TYPE, "application/json")],
		state.body.clone(),
	)
		.into_response()
}

async fn spawn_mock_metadata_server(
	status: StatusCode,
	body: impl Into<String>,
) -> (u16, tokio::task::JoinHandle<()>) {
	let app = Router::new()
		.route("/metadata", get(metadata_handler))
		.with_state(Arc::new(MockMetadataResponse {
			status,
			body: body.into(),
		}));

	let port = portpicker::pick_unused_port().expect("failed to pick port");
	let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
		.await
		.expect("failed to bind mock metadata server");
	let handle = tokio::spawn(async move {
		axum::serve(listener, app).await.expect("server error");
	});

	(port, handle)
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
fn serverless_health_check_returns_invalid_response_json_for_malformed_body() {
	run(ctx::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = setup_test_namespace(ctx.leader_dc()).await;
		let (port, handle) =
			spawn_mock_metadata_server(StatusCode::OK, r#"{"runtime":"rivetkit","version":"1""#)
				.await;

		let response = api::public::runner_configs_serverless_health_check(
			ctx.leader_dc().guard_port(),
			api::public::ServerlessHealthCheckQuery {
				namespace: namespace.clone(),
			},
			api::public::ServerlessHealthCheckRequest {
				url: format!("http://127.0.0.1:{port}"),
				headers: Default::default(),
			},
		)
		.await
		.expect("health check request failed");

		handle.abort();

		match response {
			api::public::ServerlessHealthCheckResponse::Failure { error } => {
				assert_eq!(
					error
						.metadata
						.get("body")
						.and_then(serde_json::Value::as_str),
					Some(r#"{"runtime":"rivetkit","version":"1""#),
				);
				let parse_error = error
					.metadata
					.get("parse_error")
					.and_then(serde_json::Value::as_str)
					.expect("metadata should include parse_error");
				assert!(!parse_error.is_empty(), "parse_error should not be empty");
				assert!(
					error.message.to_ascii_lowercase().contains("json"),
					"message should mention JSON, got {message:?}",
					message = error.message,
				);
			}
			other => panic!("expected failure response, got {other:?}"),
		}
	});
}

#[test]
fn serverless_health_check_surfaces_invalid_envoy_protocol_version_in_metadata() {
	run(ctx::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = setup_test_namespace(ctx.leader_dc()).await;
		let invalid_version = rivet_envoy_protocol::PROTOCOL_VERSION + 1;
		let (port, handle) = spawn_mock_metadata_server(
			StatusCode::OK,
			format!(
				r#"{{"runtime":"rivetkit","version":"1","envoyProtocolVersion":{invalid_version}}}"#
			),
		)
		.await;

		let response = api::public::runner_configs_serverless_health_check(
			ctx.leader_dc().guard_port(),
			api::public::ServerlessHealthCheckQuery {
				namespace: namespace.clone(),
			},
			api::public::ServerlessHealthCheckRequest {
				url: format!("http://127.0.0.1:{port}"),
				headers: Default::default(),
			},
		)
		.await
		.expect("health check request failed");

		handle.abort();

		match response {
			api::public::ServerlessHealthCheckResponse::Failure { error } => {
				assert_eq!(
					error
						.metadata
						.get("envoy_protocol_version")
						.and_then(serde_json::Value::as_u64),
					Some(u64::from(invalid_version)),
				);
				assert_eq!(
					error
						.metadata
						.get("max_supported_envoy_protocol_version")
						.and_then(serde_json::Value::as_u64),
					Some(u64::from(rivet_envoy_protocol::PROTOCOL_VERSION)),
				);
				assert!(
					error
						.message
						.to_ascii_lowercase()
						.contains("envoy protocol"),
					"message should mention envoy protocol version, got {message:?}",
					message = error.message,
				);
			}
			other => panic!("expected failure response, got {other:?}"),
		}
	});
}
