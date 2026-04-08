mod common;

use anyhow::{Result, bail};
use axum::{
	Json, Router,
	extract::State,
	http::StatusCode,
	routing::{get, post},
};
use reqwest::Method;
use rivet_engine_runner::{
	ActorContext, ActorRequestContext, AxumActorDefinition, AxumRunnerApp, Runner, RunnerConfig,
};
use serde_json::{Value, json};
use std::{collections::HashSet, sync::Arc, time::{Duration, Instant}};
use tokio::sync::Mutex;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn counter_actor_runner_http_kv_e2e() -> Result<()> {
	let _test_lock = common::acquire_test_lock().await?;
	let engine = common::EngineProcess::start().await?;
	let namespace = "default".to_string();

	let runner_name = common::random_name("rust-counter-runner");
	let runner_key = common::random_name("key");
	let actor_key = common::random_name("counter");
	let actor_registry = Arc::new(Mutex::new(HashSet::<String>::new()));

	let runner = Runner::builder(
		RunnerConfig::builder()
			.endpoint(engine.guard_url())
			.namespace(namespace.clone())
			.runner_name(runner_name.clone())
			.runner_key(runner_key)
			.token("dev")
			.total_slots(16)
			.build()?,
	)
	.app(build_counter_app(actor_registry.clone()))
	.build()?;

	runner.start().await?;
	runner.wait_ready().await?;

	let actor_id = engine
		.create_actor(&namespace, "counter", &runner_name, Some(&actor_key))
		.await?;
	wait_for_actor_presence(&actor_registry, &actor_id, true, Duration::from_secs(30)).await?;
	let actor = engine
		.get_actor(&namespace, &actor_id)
		.await?
		.ok_or_else(|| anyhow::anyhow!("actor missing after create: {actor_id}"))?;
	if actor.get("destroy_ts").is_some_and(|x| !x.is_null()) {
		bail!("actor is already destroyed before first request: {actor}");
	}

	let count = match engine
		.actor_request_json(Method::GET, &actor_id, "/count", None)
		.await
	{
		Ok(value) => value,
		Err(err) => {
			let actor = engine.get_actor(&namespace, &actor_id).await?;
			bail!("initial actor request failed actor={actor:?}: {err}");
		}
	};
	assert_count(&count, 0)?;

	let incremented = match engine
		.actor_request_json(Method::POST, &actor_id, "/increment", None)
		.await
	{
		Ok(value) => value,
		Err(err) => {
			let actor = engine.get_actor(&namespace, &actor_id).await?;
			bail!("first increment request failed actor={actor:?}: {err}");
		}
	};
	assert_count(&incremented, 1)?;

	let incremented_again = match engine
		.actor_request_json(Method::POST, &actor_id, "/increment", None)
		.await
	{
		Ok(value) => value,
		Err(err) => {
			let actor = engine.get_actor(&namespace, &actor_id).await?;
			bail!("second increment request failed actor={actor:?}: {err}");
		}
	};
	assert_count(&incremented_again, 2)?;

	runner.handle().sleep_actor(&actor_id, None).await?;
	wait_for_actor_presence(&actor_registry, &actor_id, false, Duration::from_secs(30)).await?;

	let persisted = match engine
		.actor_request_json(Method::GET, &actor_id, "/count", None)
		.await
	{
		Ok(value) => value,
		Err(err) => {
			let actor = engine.get_actor(&namespace, &actor_id).await?;
			bail!("persisted count request failed actor={actor:?}: {err}");
		}
	};
	assert_count(&persisted, 2)?;
	wait_for_actor_presence(&actor_registry, &actor_id, true, Duration::from_secs(30)).await?;

	runner.shutdown(true).await?;

	Ok(())
}

fn build_counter_app(actor_registry: Arc<Mutex<HashSet<String>>>) -> AxumRunnerApp {
	let on_start_registry = actor_registry.clone();
	let on_stop_registry = actor_registry;

	AxumRunnerApp::new().with_actor(
		"counter",
		AxumActorDefinition::new(
			Router::new()
				.route("/count", get(get_count))
				.route("/increment", post(increment)),
		)
		.on_start(move |ctx: ActorContext| {
			let actor_registry = on_start_registry.clone();
			async move {
				actor_registry.lock().await.insert(ctx.actor_id);
				Ok(())
			}
		})
		.on_stop(move |ctx: ActorContext| {
			let actor_registry = on_stop_registry.clone();
			async move {
				actor_registry.lock().await.remove(&ctx.actor_id);
				Ok(())
			}
		}),
	)
}

async fn get_count(
	State(ctx): State<ActorRequestContext>,
) -> Result<Json<Value>, StatusCode> {
	let count = ctx
		.kv_get_u64("count")
		.await
		.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
		.unwrap_or(0);
	Ok(Json(json!({ "count": count })))
}

async fn increment(
	State(ctx): State<ActorRequestContext>,
) -> Result<Json<Value>, StatusCode> {
	let count = ctx
		.kv_get_u64("count")
		.await
		.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
		.unwrap_or(0)
		+ 1;
	ctx.kv_put_u64("count", count)
		.await
		.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
	Ok(Json(json!({ "count": count })))
}

fn assert_count(value: &Value, expected: u64) -> Result<()> {
	let actual = value
		.get("count")
		.and_then(Value::as_u64)
		.ok_or_else(|| anyhow::anyhow!("response missing `count` field: {value}"))?;
	if actual != expected {
		bail!("count mismatch: expected {expected}, got {actual}");
	}
	Ok(())
}

async fn wait_for_actor_presence(
	actor_registry: &Arc<Mutex<HashSet<String>>>,
	actor_id: &str,
	expected: bool,
	timeout: Duration,
) -> Result<()> {
	let deadline = Instant::now() + timeout;
	loop {
		let present = actor_registry.lock().await.contains(actor_id);
		if present == expected {
			return Ok(());
		}
		if Instant::now() >= deadline {
			bail!(
				"timed out waiting for actor presence state actor_id={} expected_present={} actual_present={}",
				actor_id,
				expected,
				present
			);
		}
		tokio::time::sleep(Duration::from_millis(100)).await;
	}
}
