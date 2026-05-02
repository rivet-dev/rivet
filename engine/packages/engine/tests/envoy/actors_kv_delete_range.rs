use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use common::test_envoy::*;
use rivet_runner_protocol::mk2 as rp;
use std::sync::{Arc, Mutex};

use super::super::common;

fn make_key(s: &str) -> Vec<u8> {
	s.as_bytes().to_vec()
}

fn make_value(s: &str) -> Vec<u8> {
	s.as_bytes().to_vec()
}

#[derive(Debug, Clone)]
enum KvTestResult {
	Success,
	Failure(String),
}

struct DeleteRangeActor {
	notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>,
}

impl DeleteRangeActor {
	fn new(notify_tx: Arc<Mutex<Option<tokio::sync::oneshot::Sender<KvTestResult>>>>) -> Self {
		Self { notify_tx }
	}
}

#[async_trait]
impl TestActor for DeleteRangeActor {
	async fn on_start(&mut self, config: ActorConfig) -> Result<ActorStartResult> {
		let result = async {
			let keys = vec!["a", "b", "c", "d"]
				.into_iter()
				.map(make_key)
				.collect::<Vec<_>>();
			let values = vec!["alpha", "bravo", "charlie", "delta"]
				.into_iter()
				.map(make_value)
				.collect::<Vec<_>>();

			config
				.send_kv_put(keys, values)
				.await
				.context("failed to seed KV data")?;

			config
				.send_kv_delete_range(make_key("b"), make_key("d"))
				.await
				.context("failed to delete KV range")?;

			let response = config
				.send_kv_list(rp::KvListQuery::KvListAllQuery, None, None)
				.await
				.context("failed to list KV after delete range")?;

			if response.keys != vec![make_key("a"), make_key("d")] {
				bail!(
					"unexpected keys after delete range: {:?}",
					response
						.keys
						.iter()
						.map(|key| String::from_utf8_lossy(key).to_string())
						.collect::<Vec<_>>()
				);
			}

			Result::Ok(KvTestResult::Success)
		}
		.await;

		let test_result = match result {
			Result::Ok(ok) => ok,
			Result::Err(err) => KvTestResult::Failure(err.to_string()),
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
		"DeleteRangeActor"
	}
}

#[test]
fn kv_delete_range_removes_half_open_range() {
	common::run(
		common::TestOpts::new(1).with_timeout(30),
		|ctx| async move {
			let (namespace, _) = common::setup_test_namespace(ctx.leader_dc()).await;

			let (notify_tx, notify_rx) = tokio::sync::oneshot::channel();
			let notify_tx = Arc::new(Mutex::new(Some(notify_tx)));

			let runner = common::setup_envoy(ctx.leader_dc(), &namespace, |builder| {
				builder.with_actor_behavior("kv-delete-range", move |_| {
					Box::new(DeleteRangeActor::new(notify_tx.clone()))
				})
			})
			.await;

			common::create_actor(
				ctx.leader_dc().guard_port(),
				&namespace,
				"kv-delete-range",
				runner.pool_name(),
				rivet_types::actors::CrashPolicy::Destroy,
			)
			.await;

			match notify_rx.await.expect("actor should send test result") {
				KvTestResult::Success => {}
				KvTestResult::Failure(msg) => panic!("kv delete range test failed: {}", msg),
			}
		},
	);
}
