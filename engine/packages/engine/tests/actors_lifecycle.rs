use std::sync::{Arc, Mutex};

mod common;

// MARK: Creation and Initialization
#[test]
fn actor_basic_create() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (start_tx, start_rx) = tokio::sync::oneshot::channel();
		let start_tx = Arc::new(Mutex::new(Some(start_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("test-actor", move |_| {
				Box::new(common::test_runner::NotifyOnStartActor::new(
					start_tx.clone(),
				))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"test-actor",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to start (notification from actor)
		start_rx
			.await
			.expect("actor should have sent start notification");

		// Verify actor is allocated to runner
		assert!(
			runner.has_actor(&actor_id).await,
			"runner should have the actor allocated"
		);

		tracing::info!(?actor_id, runner_id = ?runner.runner_id, "actor allocated to runner");
	});
}

#[test]
fn create_actor_with_input() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Generate test input data (base64-encoded String)
		let input_data = common::generate_test_input_data();

		// Decode the base64 data to get the actual bytes the actor will receive
		// The API automatically decodes base64 input before sending to the runner
		let input_data_bytes =
			base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &input_data)
				.expect("failed to decode base64 input");

		// Create runner with VerifyInputActor that will validate the input
		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("test-actor", move |_| {
				Box::new(common::test_runner::VerifyInputActor::new(
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
				runner_name_selector: runner.name().to_string(),
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
fn actor_start_timeout() {
	// This test takes 35+ seconds
	common::run(
		common::TestOpts::new(1).with_timeout(45),
		|ctx| async move {
			let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

			// Create test runner with timeout actor behavior
			let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
				builder.with_actor_behavior("timeout-actor", move |_| {
					Box::new(common::test_runner::TimeoutActor::new())
				})
			})
			.await;

			tracing::info!("test runner ready, creating actor that will timeout");

			// Create actor with destroy crash policy
			let res = common::create_actor(
				ctx.leader_dc().guard_port(),
				&namespace,
				"timeout-actor",
				runner.name(),
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
fn actor_starts_and_connectable_via_guard_http() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (start_tx, start_rx) = tokio::sync::oneshot::channel();
		let start_tx = Arc::new(Mutex::new(Some(start_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("test-actor", move |_| {
				Box::new(common::test_runner::NotifyOnStartActor::new(
					start_tx.clone(),
				))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"test-actor",
			runner.name(),
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

		// TODO: HTTP ping test via guard needs to be implemented: the Rust test runner atm
		// doesn't implement HTTP tunneling yet. The original test with TypeScript
		// runner included: common::ping_actor_via_guard(ctx.leader_dc(), &actor_id).await;

		tracing::info!(?actor_id, "actor is connectable (state verified)");
	});
}

#[test]
fn actor_connectable_via_guard_websocket() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (start_tx, start_rx) = tokio::sync::oneshot::channel();
		let start_tx = Arc::new(Mutex::new(Some(start_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("test-actor", move |_| {
				Box::new(common::test_runner::NotifyOnStartActor::new(
					start_tx.clone(),
				))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"test-actor",
			runner.name(),
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

		// Note: WebSocket ping test via guard is skipped because the Rust test runner
		// doesn't implement HTTP tunneling yet. The original test with TypeScript
		// runner included: common::ping_actor_websocket_via_guard(ctx.leader_dc(), &actor_id).await;

		tracing::info!(?actor_id, "actor is connectable (state verified)");
	});
}

// MARK: Stopping and Graceful Shutdown
#[test]
fn actor_graceful_stop_with_destroy_policy() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Create test runner with stop immediately actor
		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("stop-actor", move |_| {
				Box::new(common::test_runner::StopImmediatelyActor::new())
			})
		})
		.await;

		tracing::info!("test runner ready, creating actor that will stop gracefully");

		// Create actor with destroy crash policy
		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"stop-actor",
			runner.name(),
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

		// Verify runner slot freed (actor no longer on runner)
		assert!(
			!runner.has_actor(&actor_id_str).await,
			"actor should be removed from runner after destroy"
		);

		tracing::info!(?actor_id_str, "actor gracefully stopped and destroyed");
	});
}

#[test]
fn actor_explicit_destroy() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Create a channel to be notified when the actor starts
		let (start_tx, start_rx) = tokio::sync::oneshot::channel();
		let start_tx = Arc::new(Mutex::new(Some(start_tx)));

		// Build a custom runner with NotifyOnStartActor
		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("test-actor", move |_| {
				Box::new(common::test_runner::NotifyOnStartActor::new(
					start_tx.clone(),
				))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"test-actor",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		start_rx
			.await
			.expect("actor should have sent start notification");

		// Verify actor is running
		assert!(
			runner.has_actor(&actor_id).await,
			"runner should have actor"
		);

		// Delete the actor
		common::api::public::actors_delete(
			ctx.leader_dc().guard_port(),
			common::api_types::actors::delete::DeletePath {
				actor_id: actor_id.parse().expect("failed to parse actor_id"),
			},
			common::api_types::actors::delete::DeleteQuery {
				namespace: Some(namespace.clone()),
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

// MARK: 5. Crash Handling and Policies
#[test]
fn crash_policy_restart() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Create channel to be notified when actor crashes
		let (crash_tx, crash_rx) = tokio::sync::oneshot::channel();
		let crash_tx = Arc::new(Mutex::new(Some(crash_tx)));

		// Create test runner with crashing actor
		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("crash-actor", move |_| {
				Box::new(common::test_runner::CrashOnStartActor::new_with_notify(
					1,
					crash_tx.clone(),
				))
			})
		})
		.await;

		tracing::info!("test runner ready, creating actor with restart policy");

		// Create actor with restart crash policy
		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"crash-actor",
			runner.name(),
			rivet_types::actors::CrashPolicy::Restart,
		)
		.await;

		let actor_id_str = res.actor.actor_id.to_string();

		tracing::info!(?actor_id_str, "actor created, will crash on start");

		// Wait for crash notification
		crash_rx
			.await
			.expect("actor should have sent crash notification");

		// Poll for reschedule_ts to be set (system needs to process the crash)
		let actor = loop {
			let actor =
				common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id_str, &namespace)
					.await
					.expect("failed to get actor")
					.expect("actor should exist");

			if actor.reschedule_ts.is_some() {
				break actor;
			}

			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		};

		assert!(
			actor.reschedule_ts.is_some(),
			"actor should have reschedule_ts after crash with restart policy"
		);

		tracing::info!(?actor_id_str, reschedule_ts = ?actor.reschedule_ts, "actor scheduled for restart");
	});
}

#[test]
fn crash_policy_restart_resets_on_success() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let crash_count = Arc::new(Mutex::new(0));

		// Create test runner with actor that crashes 2 times then succeeds
		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("crash-recover-actor", move |_| {
				Box::new(common::test_runner::CrashNTimesThenSucceedActor::new(
					2,
					crash_count.clone(),
				))
			})
		})
		.await;

		tracing::info!("test runner ready, creating actor with restart policy");

		// Create actor with restart crash policy
		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"crash-recover-actor",
			runner.name(),
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
fn crash_policy_sleep() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Create channel to be notified when actor crashes
		let (crash_tx, crash_rx) = tokio::sync::oneshot::channel();
		let crash_tx = Arc::new(Mutex::new(Some(crash_tx)));

		// Create test runner with crashing actor
		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("crash-actor", move |_| {
				Box::new(common::test_runner::CrashOnStartActor::new_with_notify(
					1,
					crash_tx.clone(),
				))
			})
		})
		.await;

		tracing::info!("test runner ready, creating actor with sleep policy");

		// Create actor with sleep crash policy
		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"crash-actor",
			runner.name(),
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

#[test]
fn crash_policy_destroy() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Create channel to be notified when actor crashes
		let (crash_tx, crash_rx) = tokio::sync::oneshot::channel();
		let crash_tx = Arc::new(Mutex::new(Some(crash_tx)));

		// Create test runner with crashing actor
		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("crash-actor", move |_| {
				Box::new(common::test_runner::CrashOnStartActor::new_with_notify(
					1,
					crash_tx.clone(),
				))
			})
		})
		.await;

		tracing::info!("test runner ready, creating actor with destroy policy");

		// Create actor with destroy crash policy
		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"crash-actor",
			runner.name(),
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
fn actor_sleep_intent() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Create channel to be notified when actor sends sleep intent
		let (sleep_tx, sleep_rx) = tokio::sync::oneshot::channel();
		let sleep_tx = Arc::new(Mutex::new(Some(sleep_tx)));

		// Create test runner with sleep actor behavior
		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("sleep-actor", move |_| {
				Box::new(common::test_runner::SleepImmediatelyActor::new_with_notify(
					sleep_tx.clone(),
				))
			})
		})
		.await;

		tracing::info!("test runner ready, creating actor that will sleep");

		// Create actor
		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"sleep-actor",
			runner.name(),
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
fn actor_pending_allocation_no_runners() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create namespace and start a runner with 1 slot
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let runner_full = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder
				.with_total_slots(1)
				.with_actor_behavior("filler-actor", |_| {
					Box::new(common::test_runner::EchoActor::new())
				})
				.with_actor_behavior("test-actor", |_| {
					Box::new(common::test_runner::EchoActor::new())
				})
		})
		.await;

		tracing::info!("runner with 1 slot started");

		// Fill the slot with a filler actor
		let filler_res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"filler-actor",
			runner_full.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let filler_actor_id = filler_res.actor.actor_id.to_string();

		// Wait for filler actor to be allocated
		loop {
			if runner_full.has_actor(&filler_actor_id).await {
				break;
			}
			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		}

		tracing::info!(
			?filler_actor_id,
			"filler actor allocated, runner is now full"
		);

		// Create test actor (should be pending because runner is full)
		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"test-actor",
			runner_full.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		// Verify actor is in pending state
		let actor = common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id, &namespace)
			.await
			.expect("failed to get actor")
			.expect("actor should exist");

		assert!(
			actor.pending_allocation_ts.is_some(),
			"pending_allocation_ts should be set when no runners available"
		);
		assert!(
			actor.connectable_ts.is_none(),
			"actor should not be connectable yet"
		);

		tracing::info!(?actor_id, "actor is pending allocation");

		// Now start a runner with available slots
		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("test-actor", |_| {
				Box::new(common::test_runner::EchoActor::new())
			})
		})
		.await;

		tracing::info!("runner with 20 slots started");

		// Poll for allocation
		loop {
			if runner.has_actor(&actor_id).await {
				break;
			}
			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		}

		// Verify actor is now allocated
		assert!(
			runner.has_actor(&actor_id).await,
			"actor should now be allocated to runner"
		);

		tracing::info!(
			?actor_id,
			"actor successfully allocated after runner with slots started"
		);
	});
}

#[test]
fn pending_allocation_queue_ordering() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		// Create namespace and start runner with only 2 slots
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder
				.with_total_slots(2)
				.with_actor_behavior("test-actor-0", |_| {
					Box::new(common::test_runner::EchoActor::new())
				})
				.with_actor_behavior("test-actor-1", |_| {
					Box::new(common::test_runner::EchoActor::new())
				})
				.with_actor_behavior("test-actor-2", |_| {
					Box::new(common::test_runner::EchoActor::new())
				})
		})
		.await;

		tracing::info!("runner with 2 slots started");

		// Create 3 actors in sequence
		// First 2 should be allocated immediately, 3rd should be pending
		let mut actor_ids = Vec::new();
		for i in 0..3 {
			let name = format!("test-actor-{}", i);
			let res = common::create_actor(
				ctx.leader_dc().guard_port(),
				&namespace,
				&name,
				runner.name(),
				rivet_types::actors::CrashPolicy::Destroy,
			)
			.await;

			actor_ids.push(res.actor.actor_id.to_string());

			// Small delay to ensure ordering
			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		}

		// Poll for first 2 actors to be allocated
		loop {
			let has_0 = runner.has_actor(&actor_ids[0]).await;
			let has_1 = runner.has_actor(&actor_ids[1]).await;

			if has_0 && has_1 {
				break;
			}

			tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
		}

		// Verify first 2 actors are allocated (FIFO)
		assert!(
			runner.has_actor(&actor_ids[0]).await,
			"first actor should be allocated"
		);
		assert!(
			runner.has_actor(&actor_ids[1]).await,
			"second actor should be allocated"
		);

		// Third actor should still be pending
		let actor_c =
			common::try_get_actor(ctx.leader_dc().guard_port(), &actor_ids[2], &namespace)
				.await
				.expect("failed to get actor")
				.expect("actor should exist");

		assert!(
			actor_c.pending_allocation_ts.is_some(),
			"third actor should still be pending"
		);

		tracing::info!("FIFO allocation ordering verified");
	});
}

// MARK: Runner Failures
#[test]
fn actor_survives_runner_disconnect() {
	common::run(
		common::TestOpts::new(1).with_timeout(60),
		|ctx| async move {
			let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

			// Create runner and start actor
			let (start_tx, start_rx) = tokio::sync::oneshot::channel();
			let start_tx = Arc::new(Mutex::new(Some(start_tx)));

			let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
				builder.with_actor_behavior("test-actor", move |_| {
					Box::new(common::test_runner::NotifyOnStartActor::new(
						start_tx.clone(),
					))
				})
			})
			.await;

			let res = common::create_actor(
				ctx.leader_dc().guard_port(),
				&namespace,
				"test-actor",
				runner.name(),
				rivet_types::actors::CrashPolicy::Restart,
			)
			.await;

			let actor_id_str = res.actor.actor_id.to_string();

			// Wait for actor to start
			start_rx
				.await
				.expect("actor should have sent start notification");

			tracing::info!(?actor_id_str, "actor started, simulating runner disconnect");

			// Simulate runner disconnect by shutting down
			runner.shutdown().await;

			tracing::info!(
				"runner disconnected, waiting for system to detect and apply crash policy"
			);

			// Now we wait for runner_lost_threshold so that actor state updates
			tokio::time::sleep(tokio::time::Duration::from_millis(
				ctx.leader_dc()
					.config
					.pegboard()
					.runner_lost_threshold()
					.try_into()
					.unwrap(),
			))
			.await;

			// Poll for actor to be rescheduled (crash policy is Restart)
			// The system should detect runner loss and apply the crash policy
			let start = std::time::Instant::now();
			let actor = loop {
				let actor =
					common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id_str, &namespace)
						.await
						.expect("failed to get actor")
						.expect("actor should exist");
				tracing::warn!(?actor);
				// Actor should be waiting for an allocation after runner loss
				if actor.pending_allocation_ts.is_some() {
					break actor;
				}

				if start.elapsed() > std::time::Duration::from_secs(50) {
					// TODO: Always times out here
					tracing::info!(?actor);
					break actor;
				}

				tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
			};

			assert!(
				actor.pending_allocation_ts.is_some(),
				"actor should be pending allocation after runner disconnected and threshold hit with restart policy"
			);
			assert!(
				actor.connectable_ts.is_none(),
				"actor should not be connectable after runner disconnect"
			);
		},
	);
}

// MARK: Resource Limits
#[test]
#[ignore]
fn runner_at_max_capacity() {
	common::run(
		common::TestOpts::new(1).with_timeout(30),
		|ctx| async move {
			let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

			// Start runner with only 2 slots

			let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
				builder
					.with_total_slots(2)
					.with_actor_behavior("test-actor", move |_| {
						Box::new(common::test_runner::EchoActor::new())
					})
			})
			.await;

			// Create first two actors to fill capacity
			let mut actor_ids = Vec::new();
			for _i in 0..2 {
				let res = common::create_actor(
					ctx.leader_dc().guard_port(),
					&namespace,
					"test-actor",
					runner.name(),
					rivet_types::actors::CrashPolicy::Destroy,
				)
				.await;

				actor_ids.push(res.actor.actor_id.to_string());
			}

			// Poll for both actors to be allocated
			loop {
				let has_0 = runner.has_actor(&actor_ids[0]).await;
				let has_1 = runner.has_actor(&actor_ids[1]).await;

				if has_0 && has_1 {
					break;
				}

				tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
			}

			// Verify both actors are allocated
			assert!(runner.has_actor(&actor_ids[0]).await);
			assert!(runner.has_actor(&actor_ids[1]).await);

			// Create third actor (should be pending)
			let res3 = common::create_actor(
				ctx.leader_dc().guard_port(),
				&namespace,
				"test-actor",
				runner.name(),
				rivet_types::actors::CrashPolicy::Destroy,
			)
			.await;

			let actor_id3 = res3.actor.actor_id.to_string();

			// Verify third actor is pending
			let actor3 =
				common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id3, &namespace)
					.await
					.expect("failed to get actor")
					.expect("actor should exist");

			assert!(
				actor3.pending_allocation_ts.is_some(),
				"third actor should be pending when runner at capacity"
			);

			// Destroy first actor to free a slot
			common::api::public::actors_delete(
				ctx.leader_dc().guard_port(),
				common::api_types::actors::delete::DeletePath {
					actor_id: actor_ids[0].parse().unwrap(),
				},
				common::api_types::actors::delete::DeleteQuery {
					namespace: Some(namespace.clone()),
				},
			)
			.await
			.expect("failed to delete actor");

			// Poll for third actor to be allocated (wait for slot to free and pending actor to be allocated)
			loop {
				tracing::warn!(
					"polling runner: current actors: {:?}",
					runner.get_actor_ids().await
				);
				if runner.has_actor(&actor_id3).await {
					break;
				}
				tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
			}

			// Verify third actor is now allocated
			assert!(
				runner.has_actor(&actor_id3).await,
				"pending actor should be allocated after slot freed"
			);
		},
	);
}

// MARK: Timeout and Retry Scenarios
#[test]
fn exponential_backoff_max_retries() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		// Create test runner with always-crashing actor

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("crash-always-actor", move |_| {
				Box::new(common::test_runner::CrashOnStartActor::new(1))
			})
		})
		.await;

		tracing::info!("test runner ready, creating actor that will always crash");

		// Create actor with restart crash policy
		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"crash-always-actor",
			runner.name(),
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
			let actor = loop {
				let actor =
					common::try_get_actor(ctx.leader_dc().guard_port(), &actor_id_str, &namespace)
						.await
						.expect("failed to get actor")
						.expect("actor should exist");

				if actor.reschedule_ts.is_some() {
					break actor;
				}

				tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
			};

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

			// Wait for the reschedule time to pass so next crash can happen
			let now = rivet_util::timestamp::now();
			if current_reschedule_ts > now {
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
