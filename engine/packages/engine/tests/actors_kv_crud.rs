use anyhow::*;
use async_trait::async_trait;
use common::test_runner::*;
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

/// Actor that puts a key-value pair and then gets it to verify
struct PutAndGetActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl PutAndGetActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for PutAndGetActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "put and get actor starting");

		let result = async {
			// Put a key-value pair
			let key = make_key("test-key");
			let value = make_value("test-value");

			config
				.send_kv_put(vec![key.clone()], vec![value.clone()])
				.await
				.context("failed to put key-value")?;

			// Get the key back
			let response = config
				.send_kv_get(vec![key.clone()])
				.await
				.context("failed to get key")?;

			// Verify we got exactly one value
			if response.values.len() != 1 {
				bail!("expected 1 value, got {}", response.values.len());
			}

			// Verify the value matches
			let retrieved_value = response
				.values
				.first()
				.context("expected value to exist,  got null")?;

			if *retrieved_value != value {
				bail!(
					"value mismatch: expected {:?}, got {:?}",
					String::from_utf8_lossy(&value),
					String::from_utf8_lossy(retrieved_value)
				);
			}

			tracing::info!("value verified successfully");
			Result::Ok(KvTestResult::Success)
		}
		.await;

		// Notify test of result
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
		"PutAndGetActor"
	}
}

/// Actor that attempts to get a key that doesn't exist
struct GetNonexistentKeyActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl GetNonexistentKeyActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for GetNonexistentKeyActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "get nonexistent key actor starting");

		let result = async {
			// Try to get a key that was never put
			let key = make_key("nonexistent-key");

			let response = config
				.send_kv_get(vec![key.clone()])
				.await
				.context("failed to get key")?;

			tracing::info!(?response, "got response");

			// TODO: Engine returns empty arrays for nonexistent keys instead of array with null
			// Should return: keys: [key], values: [None]
			// Currently returns: keys: [], values: []
			if response.values.is_empty() {
				tracing::info!("verified nonexistent key returns empty array (engine behavior)");
			} else {
				// Verify we got exactly one entry
				if response.values.len() != 1 {
					bail!("expected 1 value entry, got {}", response.values.len());
				}

				// Verify the value is None (null)
				if response.values.first().is_some() {
					bail!(
						"expected null for nonexistent key, got value: {:?}",
						response.values.first()
					);
				}

				tracing::info!("verified nonexistent key returns null");
			}

			Result::Ok(KvTestResult::Success)
		}
		.await;

		// Notify test of result
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
		"GetNonexistentKeyActor"
	}
}

/// Actor that puts a key, then overwrites it with a new value
struct PutOverwriteActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl PutOverwriteActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for PutOverwriteActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "put overwrite actor starting");

		let result = async {
			let key = make_key("overwrite-key");
			let value1 = make_value("first-value");
			let value2 = make_value("second-value");

			// Put first value
			config
				.send_kv_put(vec![key.clone()], vec![value1.clone()])
				.await
				.context("failed to put first value")?;

			tracing::info!("put first value");

			// Get and verify first value
			let response1 = config
				.send_kv_get(vec![key.clone()])
				.await
				.context("failed to get first value")?;

			let retrieved1 = response1
				.values
				.first()
				.context("expected first value to exist")?;

			if *retrieved1 != value1 {
				bail!(
					"first value mismatch: expected {:?}, got {:?}",
					String::from_utf8_lossy(&value1),
					String::from_utf8_lossy(retrieved1)
				);
			}

			tracing::info!("verified first value");

			// Put second value (overwrite)
			config
				.send_kv_put(vec![key.clone()], vec![value2.clone()])
				.await
				.context("failed to put second value")?;

			tracing::info!("put second value (overwrite)");

			// Get and verify second value
			let response2 = config
				.send_kv_get(vec![key.clone()])
				.await
				.context("failed to get second value")?;

			let retrieved2 = response2
				.values
				.first()
				.context("expected second value to exist")?;

			if *retrieved2 != value2 {
				bail!(
					"second value mismatch: expected {:?}, got {:?}",
					String::from_utf8_lossy(&value2),
					String::from_utf8_lossy(retrieved2)
				);
			}

			tracing::info!("verified second value overwrote first");
			Result::Ok(KvTestResult::Success)
		}
		.await;

		// Notify test of result
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
		"PutOverwriteActor"
	}
}

/// Actor that puts a key, verifies it exists, then deletes it
struct DeleteKeyActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl DeleteKeyActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for DeleteKeyActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "delete key actor starting");

		let result = async {
			let key = make_key("delete-key");
			let value = make_value("delete-value");

			// Put a key-value pair
			config
				.send_kv_put(vec![key.clone()], vec![value.clone()])
				.await
				.context("failed to put key-value")?;

			tracing::info!("put key-value pair");

			// Verify key exists
			let response1 = config
				.send_kv_get(vec![key.clone()])
				.await
				.context("failed to get key before delete")?;

			if response1.values.first().is_none() {
				bail!("key should exist before delete");
			}

			tracing::info!("verified key exists");

			// Delete the key
			config
				.send_kv_delete(vec![key.clone()])
				.await
				.context("failed to delete key")?;

			tracing::info!("deleted key");

			// Verify key no longer exists
			let response2 = config
				.send_kv_get(vec![key.clone()])
				.await
				.context("failed to get key after delete")?;

			if response2.values.first().is_some() {
				bail!(
					"key should not exist after delete, got value: {:?}",
					response2.values.first()
				);
			}

			tracing::info!("verified key deleted successfully");
			Result::Ok(KvTestResult::Success)
		}
		.await;

		// Notify test of result
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
		"DeleteKeyActor"
	}
}

/// Actor that attempts to delete a key that doesn't exist
struct DeleteNonexistentKeyActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl DeleteNonexistentKeyActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for DeleteNonexistentKeyActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "delete nonexistent key actor starting");

		let result = async {
			// Try to delete a key that was never put
			let key = make_key("nonexistent-delete-key");

			config
				.send_kv_delete(vec![key.clone()])
				.await
				.context("delete should succeed even for nonexistent key")?;

			tracing::info!("successfully deleted nonexistent key (no error)");
			Ok(())
		}
		.await;

		// Notify test of result
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
		"DeleteNonexistentKeyActor"
	}
}

// MARK: Basic CRUD Tests

#[test]
fn basic_kv_put_and_get() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-put-get", move |_| {
				Box::new(PutAndGetActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-put-get",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to complete KV operations
		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "basic put and get test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("basic put and get test failed: {}", msg);
			}
		}
	});
}

#[test]
fn kv_get_nonexistent_key() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-get-nonexistent", move |_| {
				Box::new(GetNonexistentKeyActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-get-nonexistent",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to complete KV operations
		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "get nonexistent key test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("get nonexistent key test failed: {}", msg);
			}
		}
	});
}

#[test]
fn kv_put_overwrite_existing() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-overwrite", move |_| {
				Box::new(PutOverwriteActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-overwrite",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to complete KV operations
		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "put overwrite test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("put overwrite test failed: {}", msg);
			}
		}
	});
}

#[test]
fn kv_delete_existing_key() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-delete", move |_| {
				Box::new(DeleteKeyActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-delete",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to complete KV operations
		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "delete key test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("delete key test failed: {}", msg);
			}
		}
	});
}

#[test]
fn kv_delete_nonexistent_key() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-delete-nonexistent", move |_| {
				Box::new(DeleteNonexistentKeyActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-delete-nonexistent",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		// Wait for actor to complete KV operations
		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "delete nonexistent key test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("delete nonexistent key test failed: {}", msg);
			}
		}
	});
}
// MARK: Batch Operations Tests

/// Actor that puts multiple key-value pairs in one operation
struct BatchPutActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl BatchPutActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for BatchPutActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "batch put actor starting");

		let result = async {
			// Put 10 key-value pairs in single operation
			let mut keys = Vec::new();
			let mut values = Vec::new();
			for i in 0..10 {
				keys.push(make_key(&format!("batch-key-{}", i)));
				values.push(make_value(&format!("batch-value-{}", i)));
			}

			config
				.send_kv_put(keys.clone(), values.clone())
				.await
				.context("failed to put multiple keys")?;

			tracing::info!("put 10 key-value pairs");

			// Get all 10 keys individually to verify
			for i in 0..10 {
				let key = make_key(&format!("batch-key-{}", i));
				let expected_value = make_value(&format!("batch-value-{}", i));

				let response = config
					.send_kv_get(vec![key.clone()])
					.await
					.context(format!("failed to get key {}", i))?;

				let retrieved_value = response
					.values
					.first()
					.context(format!("key {} not found", i))?;

				if *retrieved_value != expected_value {
					bail!(
						"key {} value mismatch: expected {:?}, got {:?}",
						i,
						String::from_utf8_lossy(&expected_value),
						String::from_utf8_lossy(retrieved_value)
					);
				}
			}

			tracing::info!("verified all 10 keys");
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
		"BatchPutActor"
	}
}

/// Actor that gets multiple keys in one operation
struct BatchGetActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl BatchGetActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for BatchGetActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "batch get actor starting");

		let result = async {
			// Put 5 key-value pairs individually
			for i in 0..5 {
				let key = make_key(&format!("get-key-{}", i));
				let value = make_value(&format!("get-value-{}", i));

				config
					.send_kv_put(vec![key], vec![value])
					.await
					.context(format!("failed to put key {}", i))?;
			}

			tracing::info!("put 5 keys individually");

			// Get all 5 keys in single operation
			let keys: Vec<Vec<u8>> = (0..5)
				.map(|i| make_key(&format!("get-key-{}", i)))
				.collect();

			let response = config
				.send_kv_get(keys.clone())
				.await
				.context("failed to get multiple keys")?;

			tracing::info!(?response, "got batch response");

			// Verify all 5 values returned correctly
			if response.values.len() != 5 {
				bail!("expected 5 values, got {}", response.values.len());
			}

			for i in 0..5 {
				let expected_value = make_value(&format!("get-value-{}", i));
				let retrieved_value = &response.values[i];

				if *retrieved_value != expected_value {
					bail!(
						"key {} value mismatch: expected {:?}, got {:?}",
						i,
						String::from_utf8_lossy(&expected_value),
						String::from_utf8_lossy(retrieved_value)
					);
				}
			}

			tracing::info!("verified all 5 values from batch get");
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
		"BatchGetActor"
	}
}

/// Actor that deletes multiple keys in one operation
struct BatchDeleteActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl BatchDeleteActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for BatchDeleteActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "batch delete actor starting");

		let result = async {
			// Put 5 key-value pairs
			let mut keys = Vec::new();
			let mut values = Vec::new();
			for i in 0..5 {
				keys.push(make_key(&format!("del-key-{}", i)));
				values.push(make_value(&format!("del-value-{}", i)));
			}

			config
				.send_kv_put(keys.clone(), values)
				.await
				.context("failed to put keys")?;

			tracing::info!("put 5 keys");

			// Delete all 5 keys in single operation
			config
				.send_kv_delete(keys.clone())
				.await
				.context("failed to delete keys")?;

			tracing::info!("deleted 5 keys");

			// Try to get all 5 keys - should all return empty
			let response = config
				.send_kv_get(keys)
				.await
				.context("failed to get keys after delete")?;

			// TODO: Engine returns empty arrays for nonexistent keys
			// Should return 5 values (could be empty or some other indicator)
			// Currently returns: keys: [], values: []
			if !response.values.is_empty() {
				bail!(
					"expected empty values after delete, got {} values",
					response.values.len()
				);
			}

			tracing::info!("verified all keys deleted (empty response)");
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
		"BatchDeleteActor"
	}
}

#[test]
fn kv_put_multiple_keys() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-batch-put", move |_| {
				Box::new(BatchPutActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-batch-put",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "batch put test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("batch put test failed: {}", msg);
			}
		}
	});
}

#[test]
fn kv_get_multiple_keys() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-batch-get", move |_| {
				Box::new(BatchGetActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-batch-get",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "batch get test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("batch get test failed: {}", msg);
			}
		}
	});
}

#[test]
fn kv_delete_multiple_keys() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-batch-delete", move |_| {
				Box::new(BatchDeleteActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-batch-delete",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "batch delete test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("batch delete test failed: {}", msg);
			}
		}
	});
}
