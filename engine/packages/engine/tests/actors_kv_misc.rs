use anyhow::*;
use async_trait::async_trait;
use common::test_runner::*;
use rivet_runner_protocol as rp;
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

// MARK: Actor Behaviors for Binary Data Tests

/// Actor that tests binary keys and values
struct BinaryDataActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl BinaryDataActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for BinaryDataActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "binary data actor starting");

		let result = async {
			// Create binary data with null bytes and non-UTF8 data
			let key = vec![0x00, 0xFF, 0xAB, 0xCD, 0x00, 0x42];
			let value = vec![0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0xFF, 0x12, 0x34];

			config
				.send_kv_put(vec![key.clone()], vec![value.clone()])
				.await
				.context("failed to put binary data")?;

			tracing::info!("put binary key-value pair");

			// Get the key back
			let response = config
				.send_kv_get(vec![key.clone()])
				.await
				.context("failed to get binary key")?;

			// Verify binary data is preserved exactly
			if response.values.len() != 1 {
				bail!("expected 1 value, got {}", response.values.len());
			}

			let retrieved_value = response.values.first().context("expected value to exist")?;

			if *retrieved_value != value {
				bail!(
					"binary value mismatch: expected {:?}, got {:?}",
					value,
					retrieved_value
				);
			}

			tracing::info!("verified binary data preserved exactly");
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
		"BinaryDataActor"
	}
}

/// Actor that tests empty value is rejected (engine doesn't support zero-length values)
struct EmptyValueActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl EmptyValueActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for EmptyValueActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "empty value actor starting");

		let result = async {
			// First, put a normal key-value pair
			let key = make_key("empty-value-key");
			let initial_value = make_value("initial");

			config
				.send_kv_put(vec![key.clone()], vec![initial_value])
				.await
				.context("failed to put initial key-value")?;

			tracing::info!("put initial key with value");

			// Try to put key with empty value (0 bytes)
			// Engine behavior: empty values are not supported and will fail or be ignored
			let empty_value = Vec::new();
			let put_result = config
				.send_kv_put(vec![key.clone()], vec![empty_value])
				.await;

			// Engine rejects empty values, so put should fail
			if put_result.is_ok() {
				bail!("expected put with empty value to fail, but it succeeded");
			}

			tracing::info!(
				"verified empty value put was rejected (engine doesn't support empty values)"
			);

			// Verify original value still exists
			let response = config
				.send_kv_get(vec![key.clone()])
				.await
				.context("failed to get key after empty value rejection")?;

			if response.values.is_empty() {
				bail!("key should still exist with original value");
			}

			let retrieved_value = response.values.first().context("expected value to exist")?;

			if retrieved_value != &make_value("initial") {
				bail!(
					"expected original value 'initial', got {:?}",
					String::from_utf8_lossy(retrieved_value)
				);
			}

			tracing::info!("verified original value preserved after rejected empty value put");
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
		"EmptyValueActor"
	}
}

/// Actor that tests large value (1MB)
struct LargeValueActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl LargeValueActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for LargeValueActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "large value actor starting");

		let result = async {
			// Create 1MB value
			let key = make_key("large-value-key");
			let value: Vec<u8> = (0..1024 * 1024).map(|i| (i % 256) as u8).collect();

			tracing::info!(value_size = value.len(), "putting large value");

			config
				.send_kv_put(vec![key.clone()], vec![value.clone()])
				.await
				.context("failed to put large value")?;

			tracing::info!("put large value");

			// Get the key
			let response = config
				.send_kv_get(vec![key.clone()])
				.await
				.context("failed to get large value")?;

			// Verify full value returned
			if response.values.len() != 1 {
				bail!("expected 1 value, got {}", response.values.len());
			}

			let retrieved_value = response.values.first().context("expected value to exist")?;

			if retrieved_value.len() != value.len() {
				bail!(
					"value size mismatch: expected {} bytes, got {} bytes",
					value.len(),
					retrieved_value.len()
				);
			}

			if *retrieved_value != value {
				bail!("large value content mismatch");
			}

			tracing::info!("verified large value stored and retrieved correctly");
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
		"LargeValueActor"
	}
}

// MARK: Actor Behaviors for Edge Case Tests

/// Actor that tests get with empty keys array
struct GetEmptyKeysActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl GetEmptyKeysActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for GetEmptyKeysActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "get empty keys actor starting");

		let result = async {
			// Call get with empty array
			let response = config
				.send_kv_get(Vec::new())
				.await
				.context("get with empty keys should not error")?;

			// Verify operation completes (returns empty array)
			if !response.keys.is_empty() {
				bail!(
					"expected empty keys for empty get, got {}",
					response.keys.len()
				);
			}

			if !response.values.is_empty() {
				bail!(
					"expected empty values for empty get, got {}",
					response.values.len()
				);
			}

			tracing::info!("verified get with empty keys returns empty result");
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
		"GetEmptyKeysActor"
	}
}

/// Actor that tests list with limit=0
struct ListLimitZeroActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl ListLimitZeroActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for ListLimitZeroActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "list limit zero actor starting");

		let result = async {
			// Put some keys
			let mut keys = Vec::new();
			let mut values = Vec::new();
			for i in 0..5 {
				keys.push(make_key(&format!("key-{}", i)));
				values.push(make_value(&format!("value-{}", i)));
			}

			config
				.send_kv_put(keys, values)
				.await
				.context("failed to put keys")?;

			tracing::info!("put 5 keys");

			// Call listAll with limit=0
			let response = config
				.send_kv_list(rp::KvListQuery::KvListAllQuery, None, Some(0))
				.await
				.context("list with limit=0 should not error")?;

			// Verify returns empty array
			if !response.keys.is_empty() {
				bail!(
					"expected empty keys for limit=0, got {}",
					response.keys.len()
				);
			}

			if !response.values.is_empty() {
				bail!(
					"expected empty values for limit=0, got {}",
					response.values.len()
				);
			}

			tracing::info!("verified limit=0 returns empty result");
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
		"ListLimitZeroActor"
	}
}

/// Actor that tests key ordering is lexicographic
struct KeyOrderingActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl KeyOrderingActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for KeyOrderingActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "key ordering actor starting");

		let result = async {
			// Put keys in random order
			let key_names = vec!["z", "a", "m", "b", "x"];
			let mut keys = Vec::new();
			let mut values = Vec::new();

			for name in &key_names {
				keys.push(make_key(name));
				values.push(make_value(&format!("value-{}", name)));
			}

			config
				.send_kv_put(keys, values)
				.await
				.context("failed to put keys")?;

			tracing::info!("put keys in random order: z, a, m, b, x");

			// Call listAll
			let response = config
				.send_kv_list(rp::KvListQuery::KvListAllQuery, None, None)
				.await
				.context("failed to list all")?;

			tracing::info!(?response, "list all response");

			// Verify keys returned in lexicographic order: a, b, m, x, z
			let expected_order = vec!["a", "b", "m", "x", "z"];

			if response.keys.len() != expected_order.len() {
				bail!(
					"expected {} keys, got {}",
					expected_order.len(),
					response.keys.len()
				);
			}

			for (i, expected_name) in expected_order.iter().enumerate() {
				let expected_key = make_key(expected_name);
				if response.keys[i] != expected_key {
					bail!(
						"key at position {} expected {:?}, got {:?}",
						i,
						expected_name,
						String::from_utf8_lossy(&response.keys[i])
					);
				}
			}

			tracing::info!("verified lexicographic ordering: a, b, m, x, z");
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
		"KeyOrderingActor"
	}
}

/// Actor that stores many keys (1000+)
struct ManyKeysActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl ManyKeysActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for ManyKeysActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "many keys actor starting");

		let result = async {
			// Put 1000 key-value pairs
			let mut keys = Vec::new();
			let mut values = Vec::new();
			for i in 0..1000 {
				keys.push(make_key(&format!("many-key-{:04}", i)));
				values.push(make_value(&format!("many-value-{}", i)));
			}

			config
				.send_kv_put(keys.clone(), values.clone())
				.await
				.context("failed to put 1000 keys")?;

			tracing::info!("put 1000 keys");

			// Call listAll
			let response = config
				.send_kv_list(rp::KvListQuery::KvListAllQuery, None, None)
				.await
				.context("failed to list all 1000 keys")?;

			// Verify all 1000 pairs present
			if response.keys.len() != 1000 {
				bail!("expected 1000 keys, got {}", response.keys.len());
			}

			if response.values.len() != 1000 {
				bail!("expected 1000 values, got {}", response.values.len());
			}

			tracing::info!("verified 1000 keys present in list");

			// Get random sample of keys to verify values
			for i in &[0, 100, 500, 750, 999] {
				let key = make_key(&format!("many-key-{:04}", i));
				let expected_value = make_value(&format!("many-value-{}", i));

				let get_response = config
					.send_kv_get(vec![key.clone()])
					.await
					.context(format!("failed to get key {}", i))?;

				let retrieved_value = get_response
					.values
					.first()
					.context(format!("key {} not found", i))?;

				if *retrieved_value != expected_value {
					bail!("key {} value mismatch", i);
				}
			}

			tracing::info!("verified random sample of keys have correct values");
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
		"ManyKeysActor"
	}
}

// MARK: Tests

#[test]
fn kv_binary_keys_and_values() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-binary", move |_| {
				Box::new(BinaryDataActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-binary",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "binary data test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("binary data test failed: {}", msg);
			}
		}
	});
}

#[test]
#[ignore]
fn kv_empty_value() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-empty-value", move |_| {
				Box::new(EmptyValueActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-empty-value",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "empty value test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("empty value test failed: {}", msg);
			}
		}
	});
}

#[test]
#[ignore]
fn kv_large_value() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-large-value", move |_| {
				Box::new(LargeValueActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-large-value",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "large value test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("large value test failed: {}", msg);
			}
		}
	});
}

#[test]
fn kv_get_with_empty_keys_array() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-get-empty", move |_| {
				Box::new(GetEmptyKeysActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-get-empty",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "get empty keys test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("get empty keys test failed: {}", msg);
			}
		}
	});
}

#[test]
fn kv_list_with_limit_zero() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-list-limit-zero", move |_| {
				Box::new(ListLimitZeroActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-list-limit-zero",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "list limit zero test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("list limit zero test failed: {}", msg);
			}
		}
	});
}

#[test]
fn kv_key_ordering_lexicographic() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-key-ordering", move |_| {
				Box::new(KeyOrderingActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-key-ordering",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "key ordering test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("key ordering test failed: {}", msg);
			}
		}
	});
}

#[test]
#[ignore]
fn kv_many_keys_storage() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-many-keys", move |_| {
				Box::new(ManyKeysActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-many-keys",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "many keys storage test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("many keys storage test failed: {}", msg);
			}
		}
	});
}
