mod common;

use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

async fn start_mock_serverless(status_code: u16) -> (SocketAddr, tokio::task::JoinHandle<()>) {
	use axum::{Router, routing::any};

	let router = Router::new().fallback(any(move || async move {
		(
			axum::http::StatusCode::from_u16(status_code).unwrap(),
			format!("mock error response with status {}", status_code),
		)
	}));

	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();

	let handle = tokio::spawn(async move {
		axum::serve(listener, router).await.unwrap();
	});

	(addr, handle)
}

/// Starts a mock serverless that returns an empty SSE stream on first connection
/// (causing StreamEndedEarly error), then fails with an HTTP error on subsequent connections.
/// Returns the address, handle, and a counter of how many connections have been made.
async fn start_mock_serverless_stream_then_fail(
	fail_status_code: u16,
) -> (SocketAddr, tokio::task::JoinHandle<()>, Arc<AtomicU32>) {
	use axum::{
		Router,
		body::Body,
		http::{Response, StatusCode, header},
		routing::get,
	};

	let connection_count = Arc::new(AtomicU32::new(0));
	let connection_count_clone = connection_count.clone();

	let router = Router::new().route(
		"/start",
		get(move || {
			let connection_count = connection_count_clone.clone();
			async move {
				let count = connection_count.fetch_add(1, Ordering::SeqCst);

				if count == 0 {
					// First connection: return valid SSE stream that closes immediately
					// This simulates a runner that connects but crashes before init
					// This triggers "StreamEndedEarly" error
					Response::builder()
						.status(StatusCode::OK)
						.header(header::CONTENT_TYPE, "text/event-stream")
						.header(header::CACHE_CONTROL, "no-cache")
						.body(Body::empty())
						.unwrap()
				} else {
					// Subsequent connections: return HTTP error
					Response::builder()
						.status(StatusCode::from_u16(fail_status_code).unwrap())
						.body(Body::from(format!(
							"mock error response with status {}",
							fail_status_code
						)))
						.unwrap()
				}
			}
		}),
	);

	let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
	let addr = listener.local_addr().unwrap();

	let handle = tokio::spawn(async move {
		axum::serve(listener, router).await.unwrap();
	});

	(addr, handle, connection_count)
}

async fn create_serverless_runner_config(
	guard_port: u16,
	namespace: &str,
	runner_name: &str,
	serverless_url: &str,
) {
	let client = reqwest::Client::new();
	let response = client
		.put(format!(
			"http://127.0.0.1:{}/runner-configs/{}?namespace={}",
			guard_port, runner_name, namespace
		))
		.json(&json!({
			"datacenters": {
				"dc-1": {
					"serverless": {
						"url": serverless_url,
						"max_runners": 1,
						"slots_per_runner": 1,
						"request_lifespan": 15,
					}
				}
			}
		}))
		.send()
		.await
		.unwrap();

	if !response.status().is_success() {
		let text = response.text().await.unwrap();
		panic!("failed to create runner config: {}", text);
	}
}

/// Creates an actor and returns the actor_id if successful, or the error response body if failed.
async fn create_actor(
	guard_port: u16,
	namespace: &str,
	runner_name: &str,
) -> Result<String, serde_json::Value> {
	let client = reqwest::Client::new();
	let response = client
		.post(format!(
			"http://127.0.0.1:{}/actors?namespace={}",
			guard_port, namespace
		))
		.json(&json!({
			"name": "test",
			"key": "key",
			"crash_policy": "destroy",
			"runner_name_selector": runner_name,
		}))
		.send()
		.await
		.unwrap();

	let success = response.status().is_success();
	let body: serde_json::Value = response.json().await.unwrap();

	if success {
		Ok(body["actor"]["actor_id"].as_str().unwrap().to_string())
	} else {
		Err(body)
	}
}

/// Spawns a task that makes a request to an actor via guard.
/// Returns a JoinHandle that resolves to the response.
fn request_guard(
	guard_port: u16,
	actor_id: String,
	timeout_secs: u64,
) -> tokio::task::JoinHandle<Result<reqwest::Response, reqwest::Error>> {
	tokio::spawn(async move {
		let client = reqwest::Client::builder()
			.timeout(Duration::from_secs(timeout_secs))
			.build()
			.unwrap();

		client
			.get(format!("http://127.0.0.1:{}/ping", guard_port))
			.header("X-Rivet-Target", "actor")
			.header("X-Rivet-Actor", &actor_id)
			.send()
			.await
	})
}

/// Fetches the actor error from the API.
/// Returns the error object if present, or None if not.
async fn get_actor_error(
	guard_port: u16,
	namespace: &str,
	actor_id: &str,
) -> Option<rivet_types::actor::ActorError> {
	let response = common::api::public::actors_list(
		guard_port,
		common::api_types::actors::list::ListQuery {
			actor_ids: Some(actor_id.to_string()),
			actor_id: vec![],
			namespace: namespace.to_string(),
			name: None,
			key: None,
			include_destroyed: Some(true),
			limit: None,
			cursor: None,
		},
	)
	.await
	.unwrap();

	let actor = response.actors.first().unwrap();

	actor.error.clone()
}

#[test]
fn no_runners_available_error() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let guard_port = ctx.leader_dc().guard_port();
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let error = create_actor(guard_port, &namespace, "nonexistent")
			.await
			.expect_err("actor creation should fail");

		assert_eq!(error["code"], "no_runners_available");
		assert_eq!(error["group"], "actor");
	});
}

#[test]
fn serverless_http_404_error() {
	common::run(
		common::TestOpts::new(1).with_timeout(30),
		|ctx| async move {
			let guard_port = ctx.leader_dc().guard_port();
			let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

			let (mock_addr, _mock_handle) = start_mock_serverless(404).await;
			let serverless_url = format!("http://{}", mock_addr);

			let runner_name = format!("serverless-404-{}", rand::random::<u16>());
			create_serverless_runner_config(guard_port, &namespace, &runner_name, &serverless_url)
				.await;

			let actor_id = create_actor(guard_port, &namespace, &runner_name)
				.await
				.expect("actor creation should succeed initially");

			// Make a request to the actor via guard - this should timeout with actor_ready_timeout
			let guard_response = request_guard(guard_port, actor_id.clone(), 25)
				.await
				.expect("guard request task panicked")
				.expect("guard request failed");

			assert!(
				!guard_response.status().is_success(),
				"guard request should have failed"
			);

			let guard_body: serde_json::Value = guard_response.json().await.unwrap();
			assert_eq!(
				guard_body["code"], "actor_ready_timeout",
				"expected actor_ready_timeout error, got: {:?}",
				guard_body
			);

			// Now fetch the actor error from the API
			let error = get_actor_error(guard_port, &namespace, &actor_id)
				.await
				.expect("actor should have error after guard timeout");

			match error {
				rivet_types::actor::ActorError::RunnerPoolError(
					rivet_types::actor::RunnerPoolError::ServerlessHttpError {
						status_code,
						body: _,
					},
				) => {
					assert_eq!(status_code, 404, "expected HTTP 404 error");
				}
				other => panic!("expected PoolError::ServerlessHttpError, got: {:?}", other),
			}
		},
	);
}

#[test]
fn runner_disconnect_error() {
	common::run(
		common::TestOpts::new(1).with_timeout(30),
		|ctx| async move {
			let guard_port = ctx.leader_dc().guard_port();
			let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

			// 1. Connect a runner with key "test-runner"
			let runner_key = "test-runner";
			let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
				builder
					.with_runner_key(runner_key)
					.with_version(1)
					.with_total_slots(10)
					.with_actor_behavior("test", |_config| Box::new(common::EchoActor::new()))
			})
			.await;
			let runner_id = runner.wait_ready().await;
			tracing::info!(?runner_id, "runner connected");

			// 2. Create an actor on the runner (runner_name_selector must match runner key)
			let actor_id = create_actor(guard_port, &namespace, runner_key)
				.await
				.expect("actor creation should succeed");
			tracing::info!(%actor_id, "actor created");

			// Wait for actor to be allocated to runner
			tokio::time::sleep(Duration::from_millis(500)).await;

			// Verify actor is on the runner
			assert!(
				runner.has_actor(&actor_id).await,
				"actor should be on the runner"
			);

			// 3. Stop the runner (graceful shutdown)
			runner.shutdown().await;
			tracing::info!("runner shutdown");

			// Wait for runner to be detected as gone
			tokio::time::sleep(Duration::from_millis(1000)).await;

			// 4. Make a request to the actor via guard
			let guard_request_task = request_guard(guard_port, actor_id.clone(), 20);

			// 5. Wait for guard request to fail
			// When a runner shuts down gracefully, it destroys its actors.
			// Guard returns "not_found" because the actor was destroyed.
			let guard_response = guard_request_task
				.await
				.expect("guard request task panicked")
				.expect("guard request failed");

			assert!(
				!guard_response.status().is_success(),
				"guard request should have failed"
			);

			let guard_body: serde_json::Value = guard_response.json().await.unwrap();
			tracing::info!(?guard_body, "guard response");

			// When runner shuts down gracefully, actors are destroyed
			assert_eq!(
				guard_body["code"], "not_found",
				"expected not_found error (actor destroyed with runner), got: {:?}",
				guard_body
			);

			// 6. Verify actor is destroyed via /actors API
			let response = common::api::public::actors_list(
				guard_port,
				common::api_types::actors::list::ListQuery {
					actor_ids: Some(actor_id.clone()),
					actor_id: vec![],
					namespace: namespace.clone(),
					name: None,
					key: None,
					include_destroyed: Some(true),
					limit: None,
					cursor: None,
				},
			)
			.await
			.unwrap();

			let actor = response.actors.first().unwrap();
			tracing::info!(?actor, "actor from API");

			// Actor should be destroyed
			assert!(
				actor.destroy_ts.is_some(),
				"actor should have destroy_ts set after runner shutdown"
			);
		},
	);
}

#[test]
fn serverless_stream_ended_then_http_error() {
	common::run(
		common::TestOpts::new(1).with_timeout(30),
		|ctx| async move {
			let guard_port = ctx.leader_dc().guard_port();
			let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

			// Start mock that returns empty SSE stream on first connection (StreamEndedEarly),
			// then returns 503 on subsequent connections
			let (mock_addr, _mock_handle, connection_count) =
				start_mock_serverless_stream_then_fail(503).await;
			let serverless_url = format!("http://{}", mock_addr);

			let runner_name = format!("serverless-stream-{}", rand::random::<u16>());
			create_serverless_runner_config(guard_port, &namespace, &runner_name, &serverless_url)
				.await;

			let actor_id = create_actor(guard_port, &namespace, &runner_name)
				.await
				.expect("actor creation should succeed");

			// Make a request to the actor via guard - this should timeout with actor_ready_timeout
			let guard_response = request_guard(guard_port, actor_id.clone(), 25)
				.await
				.expect("guard request task panicked")
				.expect("guard request failed");

			assert!(
				!guard_response.status().is_success(),
				"guard request should have failed"
			);

			let guard_body: serde_json::Value = guard_response.json().await.unwrap();
			assert_eq!(
				guard_body["code"], "actor_ready_timeout",
				"expected actor_ready_timeout error, got: {:?}",
				guard_body
			);

			// Verify multiple connections were made (retry after StreamEndedEarly)
			assert!(
				connection_count.load(Ordering::SeqCst) >= 2,
				"expected at least 2 connections, got {}",
				connection_count.load(Ordering::SeqCst)
			);

			// Now fetch the actor error from the API - should show the most recent error (503)
			let error = get_actor_error(guard_port, &namespace, &actor_id)
				.await
				.expect("actor should have error after guard timeout");

			match error {
				rivet_types::actor::ActorError::RunnerPoolError(
					rivet_types::actor::RunnerPoolError::ServerlessHttpError {
						status_code,
						body: _,
					},
				) => {
					assert_eq!(status_code, 503, "expected HTTP 503 error");
				}
				other => panic!("expected PoolError::ServerlessHttpError, got: {:?}", other),
			}
		},
	);
}

/// Fetches runner config from the API and returns the pool_error if present.
async fn get_runner_config_pool_error(
	guard_port: u16,
	namespace: &str,
	runner_name: &str,
) -> Option<rivet_types::actor::RunnerPoolError> {
	let client = reqwest::Client::new();
	let response = client
		.get(format!(
			"http://127.0.0.1:{}/runner-configs?namespace={}&runner_name={}",
			guard_port, namespace, runner_name
		))
		.send()
		.await
		.unwrap();

	if !response.status().is_success() {
		panic!(
			"failed to get runner config: {}",
			response.text().await.unwrap()
		);
	}

	let body: serde_json::Value = response.json().await.unwrap();
	let runner_configs = body["runner_configs"].as_object().unwrap();
	let config = runner_configs.get(runner_name)?;

	// Parse pool_error if present
	config["runner_pool_error"]
		.as_object()
		.map(|_| serde_json::from_value(config["runner_pool_error"].clone()).unwrap())
}

/// Tests that the runner configs API returns pool errors for serverless configs.
#[test]
fn runner_config_returns_pool_error() {
	common::run(
		common::TestOpts::new(1).with_timeout(30),
		|ctx| async move {
			let guard_port = ctx.leader_dc().guard_port();
			let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

			// Start mock that always returns 500 error
			let (mock_addr, _mock_handle) = start_mock_serverless(500).await;
			let serverless_url = format!("http://{}", mock_addr);

			let runner_name = format!("serverless-poolerror-{}", rand::random::<u16>());
			create_serverless_runner_config(guard_port, &namespace, &runner_name, &serverless_url)
				.await;

			// Initially, there should be no pool error
			let error = get_runner_config_pool_error(guard_port, &namespace, &runner_name).await;
			assert!(error.is_none(), "should have no pool error initially");

			// Create actor to trigger a serverless connection attempt
			let _actor_id = create_actor(guard_port, &namespace, &runner_name)
				.await
				.expect("actor creation should succeed");

			// Wait for the error tracker to be populated
			tokio::time::sleep(Duration::from_millis(1000)).await;

			// Now there should be a pool error
			let error = get_runner_config_pool_error(guard_port, &namespace, &runner_name).await;
			assert!(
				error.is_some(),
				"should have pool error after failed connection"
			);

			match error.unwrap() {
				rivet_types::actor::RunnerPoolError::ServerlessHttpError {
					status_code,
					body: _,
				} => {
					assert_eq!(status_code, 500, "expected HTTP 500 error");
				}
				other => panic!("expected ServerlessHttpError, got: {:?}", other),
			}
		},
	);
}

/// Tests that the guard returns `actor_runner_failed` quickly when the serverless pool has errors.
/// This verifies the fail-fast behavior - guard should detect pool errors and fail within a few
/// seconds instead of waiting for the full timeout.
#[test]
fn guard_fails_fast_on_pool_error() {
	common::run(
		common::TestOpts::new(1).with_timeout(30),
		|ctx| async move {
			let guard_port = ctx.leader_dc().guard_port();
			let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

			// Start mock that always returns 500 error
			let (mock_addr, _mock_handle) = start_mock_serverless(500).await;
			let serverless_url = format!("http://{}", mock_addr);

			let runner_name = format!("serverless-failfast-{}", rand::random::<u16>());
			create_serverless_runner_config(guard_port, &namespace, &runner_name, &serverless_url)
				.await;

			// Create actor - this triggers the first serverless connection attempt which will fail
			// and populate the error tracker
			let actor_id = create_actor(guard_port, &namespace, &runner_name)
				.await
				.expect("actor creation should succeed initially");

			// Wait a bit for the error tracker to be populated
			tokio::time::sleep(Duration::from_millis(500)).await;

			// Make a request to the actor via guard
			// This should fail fast with actor_runner_failed (within ~3 seconds)
			// instead of waiting for the full 10 second timeout
			let start = std::time::Instant::now();
			let guard_response = request_guard(guard_port, actor_id.clone(), 15)
				.await
				.expect("guard request task panicked")
				.expect("guard request failed");
			let elapsed = start.elapsed();

			assert!(
				!guard_response.status().is_success(),
				"guard request should have failed"
			);

			let guard_body: serde_json::Value = guard_response.json().await.unwrap();

			// Should be actor_runner_failed (fail-fast) not actor_ready_timeout
			assert_eq!(
				guard_body["code"], "actor_runner_failed",
				"expected actor_runner_failed error (fail-fast), got: {:?}",
				guard_body
			);

			// Should fail fast - within ~3 seconds (1s delay + 1 check cycle + buffer)
			// Definitely should not take the full 10 second timeout
			assert!(
				elapsed.as_secs() < 5,
				"guard should fail fast, but took {:?}",
				elapsed
			);

			tracing::info!(?elapsed, "guard failed fast as expected");
		},
	);
}
