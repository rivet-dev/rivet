use std::sync::{
	Arc, Mutex,
	atomic::{AtomicUsize, Ordering},
};

use super::super::common;

async fn wait_for_envoy_actor(envoy: &common::test_envoy::TestEnvoy, actor_id: &str) {
	tokio::time::timeout(std::time::Duration::from_secs(5), async {
		loop {
			if envoy.has_actor(actor_id).await {
				break;
			}
			tokio::time::sleep(std::time::Duration::from_millis(50)).await;
		}
	})
	.await
	.expect("envoy should receive actor");
}

// MARK: Creation and Initialization
#[test]
fn envoy_actor_basic_create() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (start_tx, start_rx) = tokio::sync::oneshot::channel();
		let start_tx = Arc::new(Mutex::new(Some(start_tx)));

		let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("test-actor", move |_| {
				Box::new(common::test_envoy::NotifyOnStartActor::new(
					start_tx.clone(),
				))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"test-actor",
			envoy.pool_name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to start (notification from actor)
		start_rx
			.await
			.expect("actor should have sent start notification");

		// The actor sends its start notification before the test Envoy records it.
		tokio::time::timeout(tokio::time::Duration::from_secs(5), async {
			loop {
				if envoy.has_actor(&actor_id).await {
					break;
				}
				tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
			}
		})
		.await
		.expect("envoy should have the actor allocated");

		tracing::info!(?actor_id, envoy_key = ?envoy.envoy_key, "actor allocated to envoy");
	});
}

#[test]
fn envoy_create_actor_with_input() {
	common::run(common::TestOpts::new(1).with_timeout(30), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Generate test input data (base64-encoded String)
		let input_data = common::generate_test_input_data();

		// Decode the base64 data to get the actual bytes the actor will receive
		// The API automatically decodes base64 input before sending to the envoy
		let input_data_bytes =
			base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &input_data)
				.expect("failed to decode base64 input");

		// Create envoy with VerifyInputActor that will validate the input
		let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("test-actor", move |_| {
				Box::new(common::test_envoy::VerifyInputActor::new(
					input_data_bytes.clone(),
				))
			})
		})
		.await;

		// Create actor with input data
		let res = common::api::public::actors_create(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::create::CreateQuery {
				namespace: namespace.clone(),
			},
			common::api_types::actors::create::CreateRequest {
				datacenter: None,
				name: "test-actor".to_string(),
				key: None,
				input: Some(input_data.clone()),
				runner_name_selector: envoy.pool_name().to_string(),
				crash_policy: rivet_types::actors::CrashPolicy::Destroy,
			},
		)
		.await
		.expect("failed to create actor");

		let actor_id = res.actor.actor_id.to_string();

		// Poll for actor to become connectable
		// If input verification fails, the actor will crash and never become connectable
		let actor = loop {
			let actor = common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id, &namespace)
				.await
				.expect("failed to get actor")
				.expect("actor should exist");

			// Check if actor crashed (input verification failed)
			if actor.destroy_ts.is_some() {
				panic!(
					"actor crashed during input verification (input data was not received correctly)"
				);
			}

			// Check if actor is connectable (input verification succeeded)
			if actor.connectable_ts.is_some() {
				break actor;
			}

			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		};

		assert!(
			actor.connectable_ts.is_some(),
			"actor should be connectable after successful input verification"
		);

		tracing::info!(
			?actor_id,
			input_size = input_data.len(),
			"actor successfully verified input data"
		);
	});
}

#[test]
fn envoy_actor_start_timeout() {
	// This test takes 35+ seconds
	common::run(
		common::TestOpts::new(1).with_timeout(60),
		|ctx| async move {
			let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

			// Create envoy client with timeout actor behavior
			let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
				builder.with_actor_behavior("timeout-actor", move |_| {
					Box::new(common::test_envoy::TimeoutActor::new())
				})
			})
			.await;

			tracing::info!("envoy client ready, creating actor that will timeout");

			// Create actor with destroy crash policy
			let res = common::create_actor(
				ctx.leader_dc().guard_port(),
				&namespace,
				"timeout-actor",
				envoy.pool_name(),
				rivet_types::actors::CrashPolicy::Destroy,
			)
			.await;

			let actor_id_str = res.actor.actor_id.to_string();

			tracing::info!(?actor_id_str, "actor created, waiting for timeout");

			// Wait for the actor start timeout threshold (30s + buffer)
			tokio::time::sleep(tokio::time::Duration::from_secs(35)).await;

			// Verify actor was marked as destroyed due to timeout
			let actor =
				common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id_str, &namespace)
					.await
					.expect("failed to get actor")
					.expect("actor should exist");

			assert!(
				actor.destroy_ts.is_some(),
				"actor should be destroyed after start timeout"
			);

			tracing::info!(?actor_id_str, "actor correctly destroyed after timeout");
		},
	);
}

// MARK: Running State Management
#[test]
fn envoy_actor_starts_and_connectable_via_guard_http() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (start_tx, start_rx) = tokio::sync::oneshot::channel();
		let start_tx = Arc::new(Mutex::new(Some(start_tx)));

		let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("test-actor", move |_| {
				Box::new(common::test_envoy::NotifyOnStartActor::new(
					start_tx.clone(),
				))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"test-actor",
			envoy.pool_name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to start
		start_rx
			.await
			.expect("actor should have sent start notification");

		// Poll for connectable_ts to be set
		let actor = loop {
			let actor = common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id, &namespace)
				.await
				.expect("failed to get actor")
				.expect("actor should exist");

			if actor.connectable_ts.is_some() {
				break actor;
			}

			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		};

		assert!(
			actor.connectable_ts.is_some(),
			"actor should be connectable"
		);

		let response = common::ping_actor_via_guard(ctx.leader_dc(), &actor_id).await;
		assert_eq!(response["actorId"], actor_id);
		assert_eq!(response["status"], "ok");

		tracing::info!(?actor_id, "actor is connectable via guard HTTP");
	});
}

#[test]
fn envoy_http_tunnel_round_trips_request_and_errors() {
	common::run(common::TestOpts::new(1).with_timeout(20), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("test-actor", |_| {
				Box::new(common::test_envoy::EchoActor::new())
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"test-actor",
			envoy.pool_name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;
		let actor_id = res.actor.actor_id.to_string();
		wait_for_envoy_actor(&envoy, &actor_id).await;

		let client = reqwest::Client::new();
		let body = "hello over envoy".as_bytes().to_vec();
		let response = client
			.post(format!("http://127.0.0.1:{}/echo", ctx.leader_dc().guard_port()))
			.header("X-Rivet-Target", "actor")
			.header("X-Rivet-Actor", &actor_id)
			.header("X-Test-Header", "from-client")
			.body(body.clone())
			.send()
			.await
			.expect("failed to send HTTP tunnel request");

		assert_eq!(response.status(), reqwest::StatusCode::CREATED);
		assert_eq!(
			response
				.headers()
				.get("x-envoy-test")
				.and_then(|v| v.to_str().ok()),
			Some("ok")
		);
		let payload: serde_json::Value = response.json().await.expect("invalid echo response");
		assert_eq!(payload["actorId"], actor_id);
		assert_eq!(payload["method"], "POST");
		assert_eq!(payload["path"], "/echo");
		assert_eq!(payload["testHeader"], "from-client");
		assert_eq!(payload["body"], "hello over envoy");
		assert_eq!(payload["bodyLen"], body.len());

		let large_body = vec![b'x'; 128 * 1024];
		let large_response = client
			.put(format!("http://127.0.0.1:{}/echo", ctx.leader_dc().guard_port()))
			.header("X-Rivet-Target", "actor")
			.header("X-Rivet-Actor", &actor_id)
			.body(large_body.clone())
			.send()
			.await
			.expect("failed to send large HTTP tunnel request");
		assert_eq!(large_response.status(), reqwest::StatusCode::CREATED);
		let large_payload: serde_json::Value =
			large_response.json().await.expect("invalid large echo response");
		assert_eq!(large_payload["method"], "PUT");
		assert_eq!(large_payload["bodyLen"], large_body.len());

		let error_response = client
			.get(format!(
				"http://127.0.0.1:{}/actor-error",
				ctx.leader_dc().guard_port()
			))
			.header("X-Rivet-Target", "actor")
			.header("X-Rivet-Actor", &actor_id)
			.send()
			.await
			.expect("failed to send actor error request");
		assert!(
			!error_response.status().is_success(),
			"actor fetch error should map to an HTTP error"
		);
		assert_eq!(error_response.status(), reqwest::StatusCode::INTERNAL_SERVER_ERROR);
		assert_eq!(
			error_response
				.headers()
				.get("x-rivet-error")
				.and_then(|v| v.to_str().ok()),
			Some("envoy.fetch_failed")
		);
	});
}

#[test]
fn envoy_actor_connectable_via_guard_websocket() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (start_tx, start_rx) = tokio::sync::oneshot::channel();
		let start_tx = Arc::new(Mutex::new(Some(start_tx)));

		let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("test-actor", move |_| {
				Box::new(common::test_envoy::NotifyOnStartActor::new(
					start_tx.clone(),
				))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"test-actor",
			envoy.pool_name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to start
		start_rx
			.await
			.expect("actor should have sent start notification");

		// Poll for connectable_ts to be set
		let actor = loop {
			let actor = common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id, &namespace)
				.await
				.expect("failed to get actor")
				.expect("actor should exist");

			if actor.connectable_ts.is_some() {
				break actor;
			}

			tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
		};

		assert!(
			actor.connectable_ts.is_some(),
			"actor should be connectable"
		);

		let response = common::ping_actor_websocket_via_guard(ctx.leader_dc(), &actor_id).await;
		assert_eq!(response["status"], "ok");

		tracing::info!(?actor_id, "actor is connectable via guard WebSocket");
	});
}

#[test]
fn envoy_websocket_actor_close_round_trip() {
	common::run(common::TestOpts::new(1).with_timeout(20), |ctx| async move {
		use futures_util::{SinkExt, StreamExt};
		use tokio_tungstenite::{
			connect_async,
			tungstenite::{Message, client::IntoClientRequest},
		};

		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("test-actor", |_| {
				Box::new(common::test_envoy::EchoActor::new())
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"test-actor",
			envoy.pool_name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;
		let actor_id = res.actor.actor_id.to_string();
		wait_for_envoy_actor(&envoy, &actor_id).await;

		let mut request = format!("ws://127.0.0.1:{}/ws", ctx.leader_dc().guard_port())
			.into_client_request()
			.expect("failed to create WebSocket request");
		request.headers_mut().insert(
			"Sec-WebSocket-Protocol",
			format!(
				"rivet, rivet_target.actor, rivet_actor.{}",
				urlencoding::encode(&actor_id)
			)
			.parse()
			.unwrap(),
		);

		let (ws_stream, response) = connect_async(request)
			.await
			.expect("failed to connect WebSocket through guard");
		assert_eq!(response.status(), 101);
		let (mut write, mut read) = ws_stream.split();

		write
			.send(Message::Text("close-from-actor".to_string().into()))
			.await
			.expect("failed to send close request");

		let close = tokio::time::timeout(std::time::Duration::from_secs(5), read.next())
			.await
			.expect("timed out waiting for actor close")
			.expect("websocket should yield close frame")
			.expect("websocket close should not error");

		match close {
			Message::Close(Some(frame)) => {
				assert_eq!(u16::from(frame.code), 4001);
				assert_eq!(frame.reason, "actor.requested_close");
			}
			other => panic!("expected close frame, got {other:?}"),
		}
	});
}

// MARK: Stopping and Graceful Shutdown
#[test]
fn envoy_actor_graceful_stop_with_destroy_policy() {
	common::run(common::TestOpts::new(1).with_timeout(30), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Create envoy client with stop immediately actor
		let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("stop-actor", move |_| {
				Box::new(common::test_envoy::StopImmediatelyActor::new())
			})
		})
		.await;

		tracing::info!("envoy client ready, creating actor that will stop gracefully");

		// Create actor with destroy crash policy
		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"stop-actor",
			envoy.pool_name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id_str = res.actor.actor_id.to_string();

		tracing::info!(?actor_id_str, "actor created, will send stop intent");

		// Poll for actor to be destroyed after graceful stop
		let actor = loop {
			let actor =
				common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id_str, &namespace)
					.await
					.expect("failed to get actor")
					.expect("actor should exist");

			if actor.destroy_ts.is_some() {
				break actor;
			}

			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		};

		assert!(
			actor.destroy_ts.is_some(),
			"actor should be destroyed after graceful stop with destroy policy"
		);

		// Verify envoy slot freed (actor no longer on envoy)
		assert!(
			!envoy.has_actor(&actor_id_str).await,
			"actor should be removed from envoy after destroy"
		);

		tracing::info!(?actor_id_str, "actor gracefully stopped and destroyed");
	});
}

#[test]
fn envoy_actor_explicit_destroy() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Create a channel to be notified when the actor starts
		let (start_tx, start_rx) = tokio::sync::oneshot::channel();
		let start_tx = Arc::new(Mutex::new(Some(start_tx)));

		// Build a custom envoy with NotifyOnStartActor
		let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("test-actor", move |_| {
				Box::new(common::test_envoy::NotifyOnStartActor::new(
					start_tx.clone(),
				))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"test-actor",
			envoy.pool_name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		start_rx
			.await
			.expect("actor should have sent start notification");

		wait_for_envoy_actor(&envoy, &actor_id).await;

		// Delete the actor
		common::api::public::actors_delete(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::delete::DeletePath {
				actor_id: actor_id.parse().expect("failed to parse actor_id"),
			},
			common::api_types::actors::delete::DeleteQuery {
				namespace: namespace.clone(),
			},
		)
		.await
		.expect("failed to delete actor");

		// Poll for actor to be destroyed or timeout after 5s
		let start = std::time::Instant::now();
		let actor = loop {
			let actor = common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id, &namespace)
				.await
				.expect("failed to get actor")
				.expect("actor should still exist in database");

			if actor.destroy_ts.is_some() {
				break actor;
			}

			if start.elapsed() > std::time::Duration::from_secs(5) {
				panic!("actor deletion timed out after 5 seconds");
			}

			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		};

		assert!(
			actor.destroy_ts.is_some(),
			"destroy_ts should be set after deletion"
		);

		tracing::info!(?actor_id, "actor successfully destroyed");
	});
}

#[test]
fn envoy_reconnect_replays_pending_start_once() {
	common::run(common::TestOpts::new(1).with_timeout(20), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let start_count = Arc::new(AtomicUsize::new(0));
		let actor_start_count = start_count.clone();
		let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("replay-actor", move |_| {
				let actor_start_count = actor_start_count.clone();
				Box::new(
					common::test_envoy::CustomActorBuilder::new()
						.on_start(move |_| {
							let actor_start_count = actor_start_count.clone();
							Box::pin(async move {
								actor_start_count.fetch_add(1, Ordering::SeqCst);
								Ok(common::test_envoy::ActorStartResult::Running)
							})
						})
						.build(),
				)
			})
		})
		.await;
		envoy.shutdown().await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"replay-actor",
			envoy.pool_name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;
		let actor_id = res.actor.actor_id.to_string();

		tokio::time::timeout(std::time::Duration::from_secs(5), async {
			loop {
				let actor =
					common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id, &namespace)
						.await
						.expect("failed to get actor")
						.expect("actor should exist");
				if matches!(&actor.error, Some(rivet_types::actor::ActorError::NoEnvoys)) {
					break;
				}
				tokio::time::sleep(std::time::Duration::from_millis(50)).await;
			}
		})
		.await
		.expect("actor should wait for envoy while disconnected");

		envoy.start().await.expect("failed to restart envoy");
		envoy.wait_ready().await;
		wait_for_envoy_actor(&envoy, &actor_id).await;

		assert_eq!(
			start_count.load(Ordering::SeqCst),
			1,
			"reconnected envoy should receive the missed start exactly once"
		);
		tokio::time::sleep(std::time::Duration::from_millis(500)).await;
		assert_eq!(
			start_count.load(Ordering::SeqCst),
			1,
			"start command should not be replayed twice after reconnect"
		);
	});
}

#[test]
fn envoy_actor_stop_waits_for_completion_before_destroy() {
	common::run(common::TestOpts::new(1).with_timeout(20), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (stop_started_tx, stop_started_rx) = tokio::sync::oneshot::channel();
		let stop_started_tx = Arc::new(Mutex::new(Some(stop_started_tx)));
		let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("delayed-stop-actor", move |_| {
				let stop_started_tx = stop_started_tx.clone();
				Box::new(
					common::test_envoy::CustomActorBuilder::new()
						.on_stop(move || {
							let stop_started_tx = stop_started_tx.clone();
							Box::pin(async move {
								if let Some(tx) =
									stop_started_tx.lock().expect("stop tx lock").take()
								{
									let _ = tx.send(());
								}
								tokio::time::sleep(std::time::Duration::from_secs(3)).await;
								Ok(common::test_envoy::ActorStopResult::Success)
							})
						})
						.build(),
				)
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"delayed-stop-actor",
			envoy.pool_name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;
		let actor_id = res.actor.actor_id.to_string();
		wait_for_envoy_actor(&envoy, &actor_id).await;

		let guard_port = ctx.leader_dc().guard_port();
		let delete_actor_id = actor_id.clone();
		let delete_namespace = namespace.clone();
		let delete_task = tokio::spawn(async move {
			common::api::public::actors_delete(
				guard_port,
				common::api_types::actors::delete::DeletePath {
					actor_id: delete_actor_id.parse().expect("failed to parse actor_id"),
				},
				common::api_types::actors::delete::DeleteQuery {
					namespace: delete_namespace,
				},
			)
			.await
			.expect("failed to delete actor");
		});

		stop_started_rx
			.await
			.expect("envoy should begin graceful stop");

		let actor_during_stop =
			common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id, &namespace)
				.await
				.expect("failed to get actor")
				.expect("actor should exist during stop");
		assert!(
			actor_during_stop.destroy_ts.is_none(),
			"actor should not be destroyed before Envoy stop completion"
		);

		delete_task.await.expect("delete task should not panic");

		tokio::time::timeout(std::time::Duration::from_secs(5), async {
			loop {
				let actor =
					common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id, &namespace)
						.await
						.expect("failed to get actor")
						.expect("actor should exist");
				if actor.destroy_ts.is_some() {
					break;
				}
				tokio::time::sleep(std::time::Duration::from_millis(50)).await;
			}
		})
		.await
		.expect("actor should be destroyed after Envoy stop completion");
	});
}

// MARK: 5. Crash Handling and Policies
#[ignore = "non-sleep crash policies are not yet supported for envoys"]
#[test]
fn envoy_crash_policy_restart() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let crash_count = Arc::new(Mutex::new(0));

		// Create envoy client with actor that crashes once, then succeeds.
		let actor_crash_count = crash_count.clone();
		let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("crash-restart-actor", move |_| {
				Box::new(common::test_envoy::CrashNTimesThenSucceedActor::new(
					1,
					actor_crash_count.clone(),
				))
			})
		})
		.await;

		tracing::info!("envoy client ready, creating actor with restart policy");

		// Create actor with restart crash policy
		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"crash-restart-actor",
			envoy.pool_name(),
			rivet_types::actors::CrashPolicy::Restart,
		)
		.await;

		let actor_id_str = res.actor.actor_id.to_string();

		tracing::info!(?actor_id_str, "actor created, will crash on start");

		// Poll for the restarted actor to become connectable.
		let actor = loop {
			let actor =
				common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id_str, &namespace)
					.await
					.expect("failed to get actor")
					.expect("actor should exist");

			if actor.connectable_ts.is_some() {
				break actor;
			}

			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		};

		assert!(
			actor.connectable_ts.is_some(),
			"actor should become connectable after restart"
		);
		assert_eq!(
			*crash_count.lock().expect("crash count lock"),
			1,
			"actor should have crashed exactly once before restarting"
		);

		tracing::info!(?actor_id_str, "actor restarted successfully");
	});
}

#[ignore = "non-sleep crash policies are not yet supported for envoys"]
#[test]
fn envoy_crash_policy_restart_resets_on_success() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let crash_count = Arc::new(Mutex::new(0));

		// Create envoy client with actor that crashes 2 times then succeeds
		let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("crash-recover-actor", move |_| {
				Box::new(common::test_envoy::CrashNTimesThenSucceedActor::new(
					2,
					crash_count.clone(),
				))
			})
		})
		.await;

		tracing::info!("envoy client ready, creating actor with restart policy");

		// Create actor with restart crash policy
		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"crash-recover-actor",
			envoy.pool_name(),
			rivet_types::actors::CrashPolicy::Restart,
		)
		.await;

		let actor_id_str = res.actor.actor_id.to_string();

		tracing::info!(
			?actor_id_str,
			"actor created, will crash twice then succeed"
		);

		// Poll for actor to eventually become connectable after crashes and restarts
		// The actor should crash twice, reschedule, and eventually run successfully
		let actor = loop {
			let actor =
				common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id_str, &namespace)
					.await
					.expect("failed to get actor")
					.expect("actor should exist");

			// Actor successfully running after retries
			if actor.connectable_ts.is_some() {
				break actor;
			}

			tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
		};

		assert!(
			actor.connectable_ts.is_some(),
			"actor should eventually become connectable after crashes"
		);
		// actor.reschedule_ts is always Some(), not sure if this is intended
		assert!(
			actor.reschedule_ts.is_none()
				|| (actor.connectable_ts.unwrap() > actor.reschedule_ts.unwrap()),
			"actor should not be scheduled for retry when running successfully"
		);

		tracing::info!(?actor_id_str, "actor successfully recovered after crashes");
	});
}

#[test]
fn envoy_crash_policy_sleep() {
	common::run(common::TestOpts::new(1).with_timeout(75), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Create channel to be notified when actor crashes
		let (crash_tx, crash_rx) = tokio::sync::oneshot::channel();
		let crash_tx = Arc::new(Mutex::new(Some(crash_tx)));

		// Create envoy client with crashing actor
		let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("crash-actor", move |_| {
				Box::new(common::test_envoy::CrashOnStartActor::new_with_notify(
					1,
					crash_tx.clone(),
				))
			})
		})
		.await;

		tracing::info!("envoy client ready, creating actor with sleep policy");

		// Create actor with sleep crash policy
		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"crash-actor",
			envoy.pool_name(),
			rivet_types::actors::CrashPolicy::Sleep,
		)
		.await;

		let actor_id_str = res.actor.actor_id.to_string();

		tracing::info!(?actor_id_str, "actor created with sleep policy");

		// Wait for crash notification
		crash_rx
			.await
			.expect("actor should have sent crash notification");

		// Poll for sleep_ts to be set (system needs to process the crash)
		let actor = loop {
			let actor =
				common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id_str, &namespace)
					.await
					.expect("failed to get actor")
					.expect("actor should exist");

			if actor.sleep_ts.is_some() {
				break actor;
			}

			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		};

		assert!(
			actor.sleep_ts.is_some(),
			"actor should be sleeping after crash with sleep policy"
		);
		assert!(
			actor.connectable_ts.is_none(),
			"actor should not be connectable while sleeping"
		);

		tracing::info!(
			?actor_id_str,
			"actor correctly entered sleep state after crash"
		);
	});
}

#[ignore = "non-sleep crash policies are not yet supported for envoys"]
#[test]
fn envoy_crash_policy_destroy() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Create channel to be notified when actor crashes
		let (crash_tx, crash_rx) = tokio::sync::oneshot::channel();
		let crash_tx = Arc::new(Mutex::new(Some(crash_tx)));

		// Create envoy client with crashing actor
		let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("crash-actor", move |_| {
				Box::new(common::test_envoy::CrashOnStartActor::new_with_notify(
					1,
					crash_tx.clone(),
				))
			})
		})
		.await;

		tracing::info!("envoy client ready, creating actor with destroy policy");

		// Create actor with destroy crash policy
		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"crash-actor",
			envoy.pool_name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id_str = res.actor.actor_id.to_string();

		tracing::info!(?actor_id_str, "actor created with destroy policy");

		// Wait for crash notification
		crash_rx
			.await
			.expect("actor should have sent crash notification");

		// Poll for destroy_ts to be set (system needs to process the crash)
		let actor = loop {
			let actor =
				common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id_str, &namespace)
					.await
					.expect("failed to get actor")
					.expect("actor should exist");

			if actor.destroy_ts.is_some() {
				break actor;
			}

			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		};

		assert!(
			actor.destroy_ts.is_some(),
			"actor should be destroyed after crash with destroy policy"
		);

		tracing::info!(?actor_id_str, "actor correctly destroyed after crash");
	});
}

// MARK: 6. Sleep and Wake
#[test]
fn envoy_actor_sleep_intent() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Create channel to be notified when actor sends sleep intent
		let (sleep_tx, sleep_rx) = tokio::sync::oneshot::channel();
		let sleep_tx = Arc::new(Mutex::new(Some(sleep_tx)));

		// Create envoy client with sleep actor behavior
		let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("sleep-actor", move |_| {
				Box::new(common::test_envoy::SleepImmediatelyActor::new_with_notify(
					sleep_tx.clone(),
				))
			})
		})
		.await;

		tracing::info!("envoy client ready, creating actor that will sleep");

		// Create actor
		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"sleep-actor",
			envoy.pool_name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id_str = res.actor.actor_id.to_string();

		tracing::info!(?actor_id_str, "actor created, will send sleep intent");

		// Wait for sleep intent notification
		sleep_rx
			.await
			.expect("actor should have sent sleep intent notification");

		// Poll for sleep_ts to be set (system needs to process the sleep intent)
		let actor = loop {
			let actor =
				common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id_str, &namespace)
					.await
					.expect("failed to get actor")
					.expect("actor should exist");

			if actor.sleep_ts.is_some() {
				break actor;
			}

			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		};

		assert!(
			actor.sleep_ts.is_some(),
			"actor should have sleep_ts after sending sleep intent"
		);
		assert!(
			actor.connectable_ts.is_none(),
			"actor should not be connectable while sleeping"
		);

		tracing::info!(?actor_id_str, "actor correctly entered sleep state");
	});
}

// MARK: Pending Allocation Queue
#[test]
fn envoy_actor_pending_allocation_no_envoys() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;
		let pool_name = "pending-envoy";

		// Prime the pool's Envoy protocol version, then disconnect so the actor is
		// created as actor2 with no active envoys available.
		let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
			builder
				.with_pool_name(pool_name)
				.with_actor_behavior("test-actor", |_| {
					Box::new(common::test_envoy::EchoActor::new())
				})
		})
		.await;
		envoy.shutdown().await;

		// Create test actor (should be pending because no envoy is connected).
		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"test-actor",
			pool_name,
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		// Verify actor is in pending state. The no-envoy error is set by actor2
		// workflow allocation, so poll instead of reading the actor immediately after
		// create returns.
		let actor = tokio::time::timeout(std::time::Duration::from_secs(5), async {
			loop {
				let actor =
					common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id, &namespace)
						.await
						.expect("failed to get actor")
						.expect("actor should exist");

				if matches!(actor.error, Some(rivet_types::actor::ActorError::NoEnvoys)) {
					break actor;
				}

				assert!(
					actor.connectable_ts.is_none(),
					"actor should not be connectable before an envoy is available"
				);

				tokio::time::sleep(std::time::Duration::from_millis(50)).await;
			}
		})
		.await
		.expect("actor should report no connected envoys before allocation");

		assert!(
			actor.pending_allocation_ts.is_none(),
			"actor2 Envoy actors should not use the legacy runner pending_allocation_ts field"
		);
		assert!(
			actor.connectable_ts.is_none(),
			"actor should not be connectable yet"
		);
		assert!(
			matches!(
				&actor.error,
				Some(rivet_types::actor::ActorError::NoEnvoys)
			),
			"actor should report no connected envoys before allocation, got {:?}",
			actor.error
		);

		tracing::info!(?actor_id, "actor is pending allocation");

		// Now restart the envoy for that pool.
		envoy.start().await.expect("failed to restart envoy");
		envoy.wait_ready().await;

		tracing::info!("envoy started");

		// Poll for allocation
		loop {
			if envoy.has_actor(&actor_id).await {
				break;
			}
			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		}

		// Verify actor is now allocated
		assert!(
			envoy.has_actor(&actor_id).await,
			"actor should now be allocated to envoy"
		);

		tracing::info!(
			?actor_id,
			"actor successfully allocated after envoy with slots started"
		);
	});
}

#[test]
fn envoy_multiple_pending_allocations_start_after_envoy_reconnect() {
	common::run(common::TestOpts::new(1).with_timeout(45), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Prime the pool's Envoy protocol version, then disconnect so all actors are
		// created as actor2 with no active envoys available.
		let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
			builder
				.with_actor_behavior("test-actor-0", |_| {
					Box::new(common::test_envoy::EchoActor::new())
				})
				.with_actor_behavior("test-actor-1", |_| {
					Box::new(common::test_envoy::EchoActor::new())
				})
				.with_actor_behavior("test-actor-2", |_| {
					Box::new(common::test_envoy::EchoActor::new())
				})
		})
		.await;
		envoy.shutdown().await;

		tracing::info!("envoy protocol version primed, envoy disconnected");

		// Create 3 actors while no envoy is connected.
		let mut actor_ids = Vec::new();
		for i in 0..3 {
			let name = format!("test-actor-{}", i);
			let res = common::create_actor(
				ctx.leader_dc().guard_port(),
				&namespace,
				&name,
				envoy.pool_name(),
				rivet_types::actors::CrashPolicy::Destroy,
			)
			.await;

			let actor_id = res.actor.actor_id.to_string();
			tokio::time::timeout(tokio::time::Duration::from_secs(5), async {
				loop {
					let actor =
						common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id, &namespace)
							.await
							.expect("failed to get actor")
							.expect("actor should exist");

					assert!(
						actor.connectable_ts.is_none(),
						"actor should not be connectable before envoy reconnect"
					);
					if matches!(&actor.error, Some(rivet_types::actor::ActorError::NoEnvoys)) {
						break;
					}

					tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
				}
			})
			.await
			.expect("actor should report no connected envoys before allocation");

			actor_ids.push(actor_id);
		}

		envoy.start().await.expect("failed to restart envoy");
		envoy.wait_ready().await;

		// Poll for all pending actors to be allocated.
		loop {
			let mut all_allocated = true;
			for actor_id in &actor_ids {
				if !envoy.has_actor(actor_id).await {
					all_allocated = false;
					break;
				}
			}
			if all_allocated {
				break;
			}
			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		}

		tracing::info!("all pending actors allocated after envoy reconnect");
	});
}

// MARK: envoy Failures
#[test]
fn envoy_actor_survives_envoy_disconnect() {
	common::run(
		common::TestOpts::new(1).with_timeout(90),
		|ctx| async move {
			let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

			// Create envoy and start actor
			let (start_tx, start_rx) = tokio::sync::oneshot::channel();
			let start_tx = Arc::new(Mutex::new(Some(start_tx)));

			let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
				builder.with_actor_behavior("test-actor", move |_| {
					Box::new(common::test_envoy::NotifyOnStartActor::new(
						start_tx.clone(),
					))
				})
			})
			.await;

			let res = common::create_actor(
				ctx.leader_dc().guard_port(),
				&namespace,
				"test-actor",
				envoy.pool_name(),
				rivet_types::actors::CrashPolicy::Restart,
			)
			.await;

			let actor_id_str = res.actor.actor_id.to_string();

			// Wait for actor to start
			start_rx
				.await
				.expect("actor should have sent start notification");

			tracing::info!(?actor_id_str, "actor started, simulating envoy disconnect");

			// Simulate an ungraceful envoy disconnect. Graceful shutdown waits for actor
			// drain and exercises GoingAway instead of EnvoyConnectionLost.
			envoy.crash().await;

			tracing::info!(
				"envoy disconnected, waiting for system to detect and apply crash policy"
			);

			let start = std::time::Instant::now();
			loop {
				let actor =
					common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id_str, &namespace)
						.await
						.expect("failed to get actor")
						.expect("actor should exist");
				tracing::warn!(?actor);
				if actor.connectable_ts.is_none()
					&& matches!(
						&actor.error,
						Some(rivet_types::actor::ActorError::EnvoyNoResponse { .. })
							| Some(rivet_types::actor::ActorError::EnvoyConnectionLost { .. })
							| Some(rivet_types::actor::ActorError::NoEnvoys)
					) {
					break;
				}

				if start.elapsed() > std::time::Duration::from_secs(30) {
					panic!(
						"actor should become non-connectable after envoy disconnect; last actor: {:?}",
						actor
					);
				}

				tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
			}

			envoy.start().await.expect("failed to restart envoy");
			envoy.wait_ready().await;

			let start = std::time::Instant::now();
			loop {
				let actor =
					common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id_str, &namespace)
						.await
						.expect("failed to get actor")
						.expect("actor should exist");

				if actor.connectable_ts.is_some() && envoy.has_actor(&actor_id_str).await {
					break;
				}

				if start.elapsed() > std::time::Duration::from_secs(20) {
					panic!(
						"actor should reconnect after envoy restarts; last actor: {:?}",
						actor
					);
				}

				tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
			}
		},
	);
}

// MARK: Resource Limits
#[test]
fn envoy_normal_pool_does_not_apply_legacy_runner_slot_capacity() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("test-actor", move |_| {
				Box::new(common::test_envoy::EchoActor::new())
			})
		})
		.await;

		let mut actor_ids = Vec::new();
		for _i in 0..3 {
			let res = common::create_actor(
				ctx.leader_dc().guard_port(),
				&namespace,
				"test-actor",
				envoy.pool_name(),
				rivet_types::actors::CrashPolicy::Destroy,
			)
			.await;

			actor_ids.push(res.actor.actor_id.to_string());
		}

		let start = std::time::Instant::now();
		loop {
			let mut all_allocated = true;
			for actor_id in &actor_ids {
				if !envoy.has_actor(actor_id).await {
					all_allocated = false;
					break;
				}
				let actor =
					common::try_get_actor(ctx.leader_dc().guard_port(), actor_id, &namespace)
						.await
						.expect("failed to get actor")
						.expect("actor should exist");
				if actor.connectable_ts.is_none() {
					all_allocated = false;
					break;
				}
			}
			if all_allocated {
				break;
			}
			if start.elapsed() > std::time::Duration::from_secs(5) {
				panic!("all normal Envoy actors should become connectable");
			}

			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		}

		for actor_id in &actor_ids {
			let actor = common::try_get_actor(ctx.leader_dc().guard_port(), actor_id, &namespace)
				.await
				.expect("failed to get actor")
				.expect("actor should exist");

			assert!(
				actor.connectable_ts.is_some(),
				"normal Envoy actor should be connectable"
			);
			assert!(
				actor.pending_allocation_ts.is_none(),
				"actor2 Envoy actors should not use legacy runner pending_allocation_ts"
			);
		}
	});
}

// MARK: Timeout and Retry Scenarios
#[ignore = "non-sleep crash policies are not yet supported for envoys"]
#[test]
fn envoy_exponential_backoff_max_retries() {
	common::run(common::TestOpts::new(1).with_timeout(45), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Create envoy client with always-crashing actor

		let envoy = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("crash-always-actor", move |_| {
				Box::new(common::test_envoy::CrashOnStartActor::new(1))
			})
		})
		.await;

		tracing::info!("envoy client ready, creating actor that will always crash");

		// Create actor with restart crash policy
		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"crash-always-actor",
			envoy.pool_name(),
			rivet_types::actors::CrashPolicy::Restart,
		)
		.await;

		let actor_id_str = res.actor.actor_id.to_string();

		tracing::info!(?actor_id_str, "actor created, will crash repeatedly");

		// Track reschedule timestamps to verify backoff increases
		let mut previous_reschedule_ts: Option<i64> = None;
		let mut backoff_deltas = Vec::new();

		// Poll for multiple crashes and verify backoff increases
		for iteration in 0..5 {
			let actor = tokio::time::timeout(tokio::time::Duration::from_secs(20), async {
				loop {
					let actor =
						common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id_str, &namespace)
							.await
							.expect("failed to get actor")
							.expect("actor should exist");

					if let Some(reschedule_ts) = actor.reschedule_ts {
						if previous_reschedule_ts.map_or(true, |prev| reschedule_ts > prev) {
							break actor;
						}
					}

					tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
				}
			})
			.await
			.expect("timed out waiting for fresh reschedule_ts");

			let current_reschedule_ts = actor.reschedule_ts.expect("reschedule_ts should be set");

			tracing::info!(
				iteration,
				reschedule_ts = current_reschedule_ts,
				"actor has reschedule_ts after crash"
			);

			// Calculate backoff delta if we have a previous timestamp
			if let Some(prev_ts) = previous_reschedule_ts {
				let delta = current_reschedule_ts - prev_ts;
				backoff_deltas.push(delta);
				tracing::info!(
					iteration,
					delta_ms = delta,
					"backoff delta from previous reschedule"
				);
			}

			previous_reschedule_ts = Some(current_reschedule_ts);

			// Wait for the reschedule time to pass so next crash can happen.
			let now = rivet_util::timestamp::now();
			if iteration < 4 && current_reschedule_ts > now {
				let wait_duration = (current_reschedule_ts - now) as u64;
				tracing::info!(
					wait_duration_ms = wait_duration,
					"waiting for reschedule time"
				);
				tokio::time::sleep(tokio::time::Duration::from_millis(wait_duration + 100)).await;
			}
		}

		// Verify that backoff intervals generally increase (exponential backoff)
		// We expect each delta to be larger than or equal to the previous
		// (allowing some tolerance for system timing)
		for i in 1..backoff_deltas.len() {
			tracing::info!(
				iteration = i,
				current_delta = backoff_deltas[i],
				previous_delta = backoff_deltas[i - 1],
				"comparing backoff deltas"
			);

			// Allow some tolerance: current should be >= 80% of expected growth
			// (exponential backoff typically doubles, but we allow for some variance)
			assert!(
				backoff_deltas[i] >= backoff_deltas[i - 1] / 2,
				"backoff should not decrease significantly: iteration {}, prev={}, curr={}",
				i,
				backoff_deltas[i - 1],
				backoff_deltas[i]
			);
		}

		tracing::info!(
			?backoff_deltas,
			"exponential backoff verified across multiple crashes"
		);
	});
}
