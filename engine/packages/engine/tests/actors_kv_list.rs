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

/// Actor that calls listAll on empty store
struct ListAllEmptyActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl ListAllEmptyActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for ListAllEmptyActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "list all empty actor starting");

		let result = async {
			// Call listAll on fresh store
			let response = config
				.send_kv_list(rp::KvListQuery::KvListAllQuery, None, None)
				.await
				.context("failed to list all on empty store")?;

			tracing::info!(?response, "list all response");

			// Verify empty result
			if !response.keys.is_empty() {
				bail!("expected empty keys, got {} keys", response.keys.len());
			}

			if !response.values.is_empty() {
				bail!(
					"expected empty values, got {} values",
					response.values.len()
				);
			}

			tracing::info!("verified empty list on fresh store");
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
		"ListAllEmptyActor"
	}
}

/// Actor that lists all keys after putting some
struct ListAllKeysActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl ListAllKeysActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for ListAllKeysActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "list all keys actor starting");

		let result = async {
			// Put 5 key-value pairs
			let mut keys = Vec::new();
			let mut values = Vec::new();
			for i in 0..5 {
				keys.push(make_key(&format!("list-key-{}", i)));
				values.push(make_value(&format!("list-value-{}", i)));
			}

			config
				.send_kv_put(keys.clone(), values.clone())
				.await
				.context("failed to put keys")?;

			tracing::info!("put 5 keys");

			// Call listAll
			let response = config
				.send_kv_list(rp::KvListQuery::KvListAllQuery, None, None)
				.await
				.context("failed to list all")?;

			tracing::info!(?response, "list all response");

			// Verify all 5 pairs returned
			if response.keys.len() != 5 {
				bail!("expected 5 keys, got {}", response.keys.len());
			}

			if response.values.len() != 5 {
				bail!("expected 5 values, got {}", response.values.len());
			}

			// Verify each key-value pair
			for i in 0..5 {
				let expected_key = &keys[i];
				let expected_value = &values[i];

				if !response.keys.contains(expected_key) {
					bail!("missing key: {:?}", String::from_utf8_lossy(expected_key));
				}

				// Find the index of this key and verify the value
				if let Some(idx) = response.keys.iter().position(|k| k == expected_key) {
					if response.values[idx] != *expected_value {
						bail!(
							"value mismatch for key {:?}: expected {:?}, got {:?}",
							String::from_utf8_lossy(expected_key),
							String::from_utf8_lossy(expected_value),
							String::from_utf8_lossy(&response.values[idx])
						);
					}
				}
			}

			tracing::info!("verified all 5 key-value pairs present");
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
		"ListAllKeysActor"
	}
}

/// Actor that tests listAll with limit parameter
struct ListAllLimitActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl ListAllLimitActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for ListAllLimitActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "list all limit actor starting");

		let result = async {
			// Put 10 key-value pairs
			let mut keys = Vec::new();
			let mut values = Vec::new();
			for i in 0..10 {
				keys.push(make_key(&format!("limit-key-{:02}", i)));
				values.push(make_value(&format!("limit-value-{}", i)));
			}

			config
				.send_kv_put(keys, values)
				.await
				.context("failed to put keys")?;

			tracing::info!("put 10 keys");

			// Call listAll with limit=5
			let response = config
				.send_kv_list(rp::KvListQuery::KvListAllQuery, None, Some(5))
				.await
				.context("failed to list all with limit")?;

			tracing::info!(?response, "list all with limit response");

			// Verify exactly 5 pairs returned
			if response.keys.len() != 5 {
				bail!("expected 5 keys with limit, got {}", response.keys.len());
			}

			if response.values.len() != 5 {
				bail!(
					"expected 5 values with limit, got {}",
					response.values.len()
				);
			}

			tracing::info!("verified limit=5 returned exactly 5 results");
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
		"ListAllLimitActor"
	}
}

/// Actor that tests listAll with reverse parameter
struct ListAllReverseActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl ListAllReverseActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for ListAllReverseActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "list all reverse actor starting");

		let result = async {
			// Put keys in specific order
			let key_names = vec!["a", "b", "c", "d", "e"];
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

			tracing::info!("put keys in order: a, b, c, d, e");

			// Call listAll with reverse=true
			let response = config
				.send_kv_list(rp::KvListQuery::KvListAllQuery, Some(true), None)
				.await
				.context("failed to list all with reverse")?;

			tracing::info!(?response, "list all reverse response");

			// Verify order is reversed: e, d, c, b, a
			let expected_order = vec!["e", "d", "c", "b", "a"];

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

			tracing::info!("verified reverse order: e, d, c, b, a");
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
		"ListAllReverseActor"
	}
}

/// Actor that tests listRange with inclusive bounds
struct ListRangeInclusiveActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl ListRangeInclusiveActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for ListRangeInclusiveActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "list range inclusive actor starting");

		let result = async {
			// Put keys: a, b, c, d, e
			let key_names = vec!["a", "b", "c", "d", "e"];
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

			tracing::info!("put keys: a, b, c, d, e");

			// Call listRange(start="b", end="d", exclusive=false)
			let response = config
				.send_kv_list(
					rp::KvListQuery::KvListRangeQuery(rp::KvListRangeQuery {
						start: make_key("b"),
						end: make_key("d"),
						exclusive: false,
					}),
					None,
					None,
				)
				.await
				.context("failed to list range")?;

			tracing::info!(?response, "list range response");

			// Verify returns: b, c, d (inclusive)
			let expected_keys = vec!["b", "c", "d"];

			if response.keys.len() != expected_keys.len() {
				bail!(
					"expected {} keys, got {}",
					expected_keys.len(),
					response.keys.len()
				);
			}

			for (i, expected_name) in expected_keys.iter().enumerate() {
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

			tracing::info!("verified inclusive range: b, c, d");
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
		"ListRangeInclusiveActor"
	}
}

/// Actor that tests listRange with exclusive end (half-open range)
struct ListRangeExclusiveActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl ListRangeExclusiveActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for ListRangeExclusiveActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "list range exclusive actor starting");

		let result = async {
			// Put keys: a, b, c, d, e
			let key_names = vec!["a", "b", "c", "d", "e"];
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

			tracing::info!("put keys: a, b, c, d, e");

			// Call listRange(start="b", end="d", exclusive=true) - half-open range [b, d)
			let response = config
				.send_kv_list(
					rp::KvListQuery::KvListRangeQuery(rp::KvListRangeQuery {
						start: make_key("b"),
						end: make_key("d"),
						exclusive: true,
					}),
					None,
					None,
				)
				.await
				.context("failed to list range")?;

			tracing::info!(?response, "list range exclusive response");

			// Verify returns: b, c (includes start, excludes end - half-open range [b, d))
			let expected_keys = vec!["b", "c"];

			if response.keys.len() != expected_keys.len() {
				bail!(
					"expected {} keys, got {}",
					expected_keys.len(),
					response.keys.len()
				);
			}

			for (i, expected_name) in expected_keys.iter().enumerate() {
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

			tracing::info!("verified exclusive range: b, c (half-open range [b, d))");
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
		"ListRangeExclusiveActor"
	}
}

/// Actor that tests listPrefix with matching keys
struct ListPrefixActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl ListPrefixActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for ListPrefixActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "list prefix actor starting");

		let result = async {
			// Put keys with different prefixes
			let key_names = vec!["user:1", "user:2", "user:3", "admin:1", "admin:2"];
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

			tracing::info!("put keys with user: and admin: prefixes");

			// Call listPrefix(prefix="user:")
			let response = config
				.send_kv_list(
					rp::KvListQuery::KvListPrefixQuery(rp::KvListPrefixQuery {
						key: make_key("user:"),
					}),
					None,
					None,
				)
				.await
				.context("failed to list prefix")?;

			tracing::info!(?response, "list prefix response");

			// Verify returns only: user:1, user:2, user:3
			let expected_keys = vec!["user:1", "user:2", "user:3"];

			if response.keys.len() != expected_keys.len() {
				bail!(
					"expected {} keys, got {}",
					expected_keys.len(),
					response.keys.len()
				);
			}

			for expected_name in &expected_keys {
				let expected_key = make_key(expected_name);
				if !response.keys.contains(&expected_key) {
					bail!("missing key with prefix user:: {:?}", expected_name);
				}
			}

			// Verify admin keys are not present
			for admin_key in &["admin:1", "admin:2"] {
				let key = make_key(admin_key);
				if response.keys.contains(&key) {
					bail!(
						"admin key should not be in user: prefix results: {:?}",
						admin_key
					);
				}
			}

			tracing::info!("verified only user: prefixed keys returned");
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
		"ListPrefixActor"
	}
}

/// Actor that tests listPrefix with no matching keys
struct ListPrefixNoMatchActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl ListPrefixNoMatchActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for ListPrefixNoMatchActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		tracing::info!(actor_id = ?config.actor_id, generation = config.generation, "list prefix no match actor starting");

		let result = async {
			// Put keys with user: prefix only
			let key_names = vec!["user:1", "user:2"];
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

			tracing::info!("put keys with user: prefix");

			// Call listPrefix(prefix="admin:")
			let response = config
				.send_kv_list(
					rp::KvListQuery::KvListPrefixQuery(rp::KvListPrefixQuery {
						key: make_key("admin:"),
					}),
					None,
					None,
				)
				.await
				.context("failed to list prefix")?;

			tracing::info!(?response, "list prefix no match response");

			// Verify empty result
			if !response.keys.is_empty() {
				bail!(
					"expected empty keys for non-matching prefix, got {}",
					response.keys.len()
				);
			}

			if !response.values.is_empty() {
				bail!(
					"expected empty values for non-matching prefix, got {}",
					response.values.len()
				);
			}

			tracing::info!("verified empty result for non-matching prefix");
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
		"ListPrefixNoMatchActor"
	}
}

// MARK: Tests

#[test]
fn kv_list_all_empty_store() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-list-empty", move |_| {
				Box::new(ListAllEmptyActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-list-empty",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "list all empty test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("list all empty test failed: {}", msg);
			}
		}
	});
}

#[test]
fn kv_list_all_with_keys() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-list-keys", move |_| {
				Box::new(ListAllKeysActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-list-keys",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "list all with keys test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("list all with keys test failed: {}", msg);
			}
		}
	});
}

#[test]
fn kv_list_all_with_limit() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-list-limit", move |_| {
				Box::new(ListAllLimitActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-list-limit",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "list all with limit test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("list all with limit test failed: {}", msg);
			}
		}
	});
}

#[test]
fn kv_list_all_reverse() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-list-reverse", move |_| {
				Box::new(ListAllReverseActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-list-reverse",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "list all reverse test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("list all reverse test failed: {}", msg);
			}
		}
	});
}

#[test]
fn kv_list_range_inclusive() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-range-inclusive", move |_| {
				Box::new(ListRangeInclusiveActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-range-inclusive",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "list range inclusive test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("list range inclusive test failed: {}", msg);
			}
		}
	});
}

#[test]
fn kv_list_range_exclusive() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-range-exclusive", move |_| {
				Box::new(ListRangeExclusiveActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-range-exclusive",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "list range exclusive test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("list range exclusive test failed: {}", msg);
			}
		}
	});
}

#[test]
fn kv_list_prefix_match() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-prefix-match", move |_| {
				Box::new(ListPrefixActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-prefix-match",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "list prefix match test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("list prefix match test failed: {}", msg);
			}
		}
	});
}

#[test]
fn kv_list_prefix_no_matches() {
	common::run(common::TestOpts::new(1), |ctx| async move {
		let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

		let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
		let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

		let runner = common::setup_runner(ctx.leader_dc(), &namespace, |builder| {
			builder.with_actor_behavior("kv-prefix-no-match", move |_| {
				Box::new(ListPrefixNoMatchActor::new(notify_tx.clone()))
			})
		})
		.await;

		let res = common::create_actor(
			ctx.leader_dc().guard_port(),
			&namespace,
			"kv-prefix-no-match",
			runner.name(),
			rivet_types::actors::CrashPolicy::Destroy,
		)
		.await;

		let actor_id = res.actor.actor_id.to_string();

		let result = notify_rx.await.expect("actor should send test result");

		match result {
			KvTestResult::Success => {
				tracing::info!(?actor_id, "list prefix no matches test succeeded");
			}
			KvTestResult::Failure(msg) => {
				panic!("list prefix no matches test failed: {}", msg);
			}
		}
	});
}
