use anyhow::*;
use async_trait::async_trait;
use common::test_runner::*;
use rivet_runner_protocol::mk2 as rp;
use std::sync::{Arc, Mutex};

mod common;

// MARK: Helper Functions

/// Convert string to KV key format (Vec<u8>)
fn make_key(s: &str) -> Vec<u8> {
	s.as_bytes().to_vec()
}

/// Convert string to KV value format (Vec<u8>)
fn make_value(s: &str) -> Vec<u8> {
	s.as_bytes().to_vec()
}

/// Result of KV test operations
#[derive(Debug, Clone)]
enum KvTestResult {
	Success,
	Failure(String),
}

// MARK: Actor Behaviors

/// Actor that tests drop clearing all data
struct DropClearsAllActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl DropClearsAllActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for DropClearsAllActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "drop clears all actor starting");

		let result = async {
			// Put 10 key-value pairs
			let mut keys = Vec::new();
			let mut values = Vec::new();
			for i in 0..10 {
				keys.push(make_key(&format!("drop-key-{}", i)));
				values.push(make_value(&format!("drop-value-{}", i)));
			}

			config
				.send_kv_put(keys, values)
				.await
				.context("failed to put keys")?;

			tracing::info!("put 10 keys");

			// Verify keys exist with listAll
			let response1 = config
				.send_kv_list(rp::KvListQuery::KvListAllQuery, None, None)
				.await
				.context("failed to list all before drop")?;

			if response1.keys.len() != 10 {
				bail!("expected 10 keys before drop, got {}", response1.keys.len());
			}

			tracing::info!("verified 10 keys exist before drop");

			// Call drop
			config
				.send_kv_drop()
				.await
				.context("failed to drop kv store")?;

			tracing::info!("called drop");

			// Verify keys are cleared with listAll
			let response2 = config
				.send_kv_list(rp::KvListQuery::KvListAllQuery, None, None)
				.await
				.context("failed to list all after drop")?;

			if !response2.keys.is_empty() {
				bail!(
					"expected empty keys after drop, got {}",
					response2.keys.len()
				);
			}

			if !response2.values.is_empty() {
				bail!(
					"expected empty values after drop, got {}",
					response2.values.len()
				);
			}

			tracing::info!("verified all data cleared after drop");
			Result::Ok(KvTestResult::Success)
		}
		.await;

		let test_result = match result {
			Result::Ok(r) => r,
			Result::Err(e) => KvTestResult::Failure(e.to_string()),
		};

		if let Some(tx) = self.notify_tx.lock().unwrap().take() {
			let _ = tx.send(test_result);
		}

		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"DropClearsAllActor"
	}
}

/// Actor that tests drop on empty store
struct DropEmptyActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl DropEmptyActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for DropEmptyActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "drop empty actor starting");

		let result = async {
			// Call drop on fresh store
			config
				.send_kv_drop()
				.await
				.context("drop should succeed on empty store")?;

			tracing::info!("successfully dropped empty store (no error)");
			Ok(())
		}
		.await;

		let test_result = match result {
			Result::Ok(_) => KvTestResult::Success,
			Err(e) => KvTestResult::Failure(e.to_string()),
		};

		if let Some(tx) = self.notify_tx.lock().unwrap().take() {
			let _ = tx.send(test_result);
		}

		Ok(ActorStartResult::Running)
	}

	async fn on_stop(&mut self) -> Result<ActorStopResult> {
		Ok(ActorStopResult::Success)
	}

	fn name(&self) -> &str {
		"DropEmptyActor"
	}
}

// MARK: Tests

#[test]
fn kv_drop_clears_all_data() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-drop-clears", move |_| {
				Box::new(DropClearsAllActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-drop-clears",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "drop clears all test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("drop clears all test failed: {}", msg);
			}
		}
	});
}

#[test]
fn kv_drop_empty_store() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-drop-empty", move |_| {
				Box::new(DropEmptyActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-drop-empty",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "drop empty store test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("drop empty store test failed: {}", msg);
			}
		}
	});
}
