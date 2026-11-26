//! Rust SDK side of the SDK comparison benchmark (`mise run bench:compare` in
//! rivetkit-java). Serves a `counter` actor whose `increment(amount)` adds to an
//! in-memory count and returns it, the same shape as the Java and TypeScript bench
//! servers.
//!
//! None of the three persist the count on the request path: this one and the Java
//! one keep it in memory, and the TypeScript one uses actor `state` (deprecated in
//! RivetKit 3.0; saved off the request path, throttled). Per-call work is comparable.
//!
//! Env: RIVET_ENDPOINT (default http://127.0.0.1:6420), RIVET_POOL (required),
//! BENCH_SPAWN_ENGINE=1 to spawn the local engine and keep it alive while this runs
//! (the compare script uses one such process as the shared engine holder).

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use async_trait::async_trait;
use rivetkit::prelude::*;
use rivetkit::{Action, Handler, action};
use rivetkit_core::{EngineSpawnMode, ServeConfig};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

struct Counter {
	count: AtomicI64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Increment {
	amount: i64,
}

impl Action for Increment {
	type Output = i64;
	const NAME: &'static str = "increment";
}

#[async_trait]
impl Actor for Counter {
	type Input = ();
	type Actions = (Increment,);
	type Events = ();
	type ConnParams = ();
	type Action = action::Raw;

	async fn create(_ctx: &Ctx<Self>) -> Result<Self> {
		Ok(Self {
			count: AtomicI64::new(0),
		})
	}
}

impl Handler<Increment> for Counter {
	async fn handle(self: Arc<Self>, _ctx: Ctx<Self>, action: Increment) -> Result<i64> {
		Ok(self.count.fetch_add(action.amount, Ordering::SeqCst) + action.amount)
	}
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
	let endpoint =
		std::env::var("RIVET_ENDPOINT").unwrap_or_else(|_| "http://127.0.0.1:6420".into());
	let pool = std::env::var("RIVET_POOL").expect("RIVET_POOL is required");
	let mut registry = Registry::new();
	registry.register_actor::<Counter>("counter");

	let shutdown = CancellationToken::new();
	let serve = tokio::spawn({
		let shutdown = shutdown.clone();
		registry.serve_with_config(
			ServeConfig {
				endpoint,
				token: Some("default".into()),
				namespace: "default".into(),
				pool_name: pool,
				engine_spawn: if std::env::var("BENCH_SPAWN_ENGINE").as_deref() == Ok("1") {
					EngineSpawnMode::Always
				} else {
					EngineSpawnMode::Never
				},
				..ServeConfig::default()
			},
			shutdown,
		)
	});
	println!("bench server ready (rust)");
	tokio::signal::ctrl_c().await?;
	shutdown.cancel();
	serve.await?
}
