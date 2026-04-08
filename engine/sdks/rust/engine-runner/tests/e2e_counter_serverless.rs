mod common;

use anyhow::{Context, Result, bail};
use axum::{
	Json, Router,
	extract::State,
	http::StatusCode,
	routing::{get, post},
};
use reqwest::Method;
use rivet_engine_runner::{
	ActorContext, ActorRequestContext, AxumActorDefinition, AxumRunnerApp, ServerlessConfig,
	ServerlessRunner,
};
use serde_json::{Value, json};
use std::{collections::HashSet, sync::Arc, time::{Duration, Instant}};
use tokio::sync::{Mutex, oneshot};

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn counter_actor_serverless_http_kv_e2e() -> Result<()> {
	let _test_lock = common::acquire_test_lock().await?;
	let engine = common::EngineProcess::start().await?;
	let namespace = "default".to_string();

	let runner_name = common::random_name("rust-counter-serverless");
	let runner_key = common::random_name("key");
	let actor_key = common::random_name("counter");
	let actor_registry = Arc::new(Mutex::new(HashSet::<String>::new()));

	let serverless_runner = ServerlessRunner::builder(
		ServerlessConfig::builder()
			.endpoint(engine.guard_url())
			.namespace(namespace.clone())
			.runner_name(runner_name.clone())
			.runner_key(runner_key)
			.prepopulate_actor_name("counter", json!({}))
			.token("dev")
			.total_slots(1)
			.max_runners(1000)
			.slots_per_runner(1)
			.request_lifespan(300)
			.build()?,
	)
	.app(build_counter_app(actor_registry.clone()))
	.build()?;

	let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
		.await
		.context("failed to bind serverless test listener")?;
	let addr = listener.local_addr().context("missing listener local addr")?;
	let serverless_url = format!("http://localhost:{}", addr.port());

	let routes = Arc::new(serverless_runner.clone()).axum_routes();
	let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
	let mut server_task = tokio::spawn(async move {
		axum::serve(listener, routes)
			.with_graceful_shutdown(async move {
				let _ = shutdown_rx.await;
			})
			.await
			.context("serverless axum server exited with error")
	});

	let metadata_response = reqwest::get(format!("{serverless_url}/api/rivet/metadata"))
		.await
		.context("failed to call serverless metadata endpoint")?;
	if metadata_response.status() != reqwest::StatusCode::OK {
		bail!("metadata endpoint returned {}", metadata_response.status());
	}

	let start_response = reqwest::Client::new()
		.get(format!("{serverless_url}/api/rivet/start"))
		.send()
		.await?;
	if start_response.status() != reqwest::StatusCode::OK {
		bail!("serverless start endpoint returned {}", start_response.status());
	}

	let actor_id = engine
		.create_actor(&namespace, "counter", &runner_name, Some(&actor_key))
		.await?;

	let count = engine
		.actor_request_json(Method::GET, &actor_id, "/count", None)
		.await?;
	assert_count(&count, 0)?;

	let incremented = engine
		.actor_request_json(Method::POST, &actor_id, "/increment", None)
		.await?;
	assert_count(&incremented, 1)?;

	let incremented_again = engine
		.actor_request_json(Method::POST, &actor_id, "/increment", None)
		.await?;
	assert_count(&incremented_again, 2)?;

	tokio::time::timeout(Duration::from_secs(30), serverless_runner.runner().wait_ready())
		.await
		.context("timed out waiting for serverless runner init")??;

	wait_for_actor_presence(&actor_registry, &actor_id, true, Duration::from_secs(30)).await?;

	serverless_runner
		.runner()
		.handle()
		.sleep_actor(&actor_id, None)
		.await?;
	wait_for_actor_presence(&actor_registry, &actor_id, false, Duration::from_secs(30)).await?;

	let persisted = engine
		.actor_request_json(Method::GET, &actor_id, "/count", None)
		.await?;
	assert_count(&persisted, 2)?;
	wait_for_actor_presence(&actor_registry, &actor_id, true, Duration::from_secs(30)).await?;

	serverless_runner.runner().shutdown(true).await?;

	let _ = shutdown_tx.send(());
	if tokio::time::timeout(Duration::from_secs(10), &mut server_task)
		.await
		.is_err()
	{
		server_task.abort();
	}
	let _ = server_task.await;

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
