mod common;

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use rivet_engine_runner::{
	ActorContext, HibernatingWebSocketMetadata, Runner, RunnerApp, RunnerConfig, RunnerHandle,
	ServerlessConfig, ServerlessRunner, WebSocketContext, WebSocketMessage,
};
use serde_json::json;
use std::{
	collections::{HashMap, HashSet},
	sync::Arc,
	time::{Duration, Instant},
};
use tokio::sync::{Mutex, oneshot};
use tokio_tungstenite::tungstenite::Message;

#[derive(Clone, Default)]
struct EchoWebSocketApp {
	actors: Arc<Mutex<HashSet<String>>>,
	closes: Arc<Mutex<Vec<String>>>,
	hibernating_metadata:
		Arc<Mutex<HashMap<String, HashMap<([u8; 4], [u8; 4]), HibernatingWebSocketMetadata>>>>,
}

#[async_trait]
impl RunnerApp for EchoWebSocketApp {
	async fn on_actor_start(&self, runner: RunnerHandle, ctx: ActorContext) -> Result<()> {
		self.actors.lock().await.insert(ctx.actor_id.clone());
		let metadata = self
			.hibernating_metadata
			.lock()
			.await
			.get(&ctx.actor_id)
			.map(|entries| entries.values().cloned().collect())
			.unwrap_or_default();
		runner
			.restore_hibernating_requests(&ctx.actor_id, metadata)
			.await?;
		Ok(())
	}

	async fn on_actor_stop(&self, _runner: RunnerHandle, ctx: ActorContext) -> Result<()> {
		self.actors.lock().await.remove(&ctx.actor_id);
		Ok(())
	}

	async fn websocket(&self, _runner: RunnerHandle, ctx: WebSocketContext) -> Result<()> {
		self.hibernating_metadata
			.lock()
			.await
			.entry(ctx.actor_id.clone())
			.or_default()
			.insert(
				(ctx.gateway_id, ctx.request_id),
				HibernatingWebSocketMetadata {
					gateway_id: ctx.gateway_id,
					request_id: ctx.request_id,
					client_message_index: 0,
					server_message_index: 0,
					path: ctx.path,
					headers: ctx.headers,
				},
			);
		Ok(())
	}

	async fn websocket_message(
		&self,
		runner: RunnerHandle,
		ctx: WebSocketContext,
		message: WebSocketMessage,
	) -> Result<()> {
		if ctx.is_hibernatable {
			runner
				.send_hibernatable_websocket_message_ack(
					ctx.gateway_id,
					ctx.request_id,
					message.message_index,
				)
				.await?;
		}

		let response_data = message.data.clone();
		let response_binary = message.binary;
		runner
			.send_websocket_message(
				ctx.gateway_id,
				ctx.request_id,
				response_data,
				response_binary,
			)
			.await?;

		if let Some(actor_entries) = self
			.hibernating_metadata
			.lock()
			.await
			.get_mut(&ctx.actor_id)
		{
			if let Some(meta) = actor_entries.get_mut(&(ctx.gateway_id, ctx.request_id)) {
				meta.server_message_index = message.message_index;
				meta.client_message_index = meta.client_message_index.wrapping_add(1);
			}
		}

		Ok(())
	}

	async fn websocket_close(
		&self,
		_runner: RunnerHandle,
		ctx: WebSocketContext,
		_code: Option<u16>,
		_reason: Option<String>,
	) -> Result<()> {
		let actor_id = ctx.actor_id.clone();
		self.closes.lock().await.push(actor_id.clone());
		if let Some(actor_entries) = self
			.hibernating_metadata
			.lock()
			.await
			.get_mut(&actor_id)
		{
			actor_entries.remove(&(ctx.gateway_id, ctx.request_id));
		}
		Ok(())
	}

	fn can_hibernate(&self, _ctx: &WebSocketContext) -> bool {
		true
	}
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn websocket_runner_e2e() -> Result<()> {
	let _test_lock = common::acquire_test_lock().await?;
	let engine = common::EngineProcess::start().await?;
	let namespace = "default".to_string();
	let runner_name = common::random_name("rust-ws-runner");
	let actor_key = common::random_name("ws");
	let app = EchoWebSocketApp::default();

	let runner = Runner::builder(
		RunnerConfig::builder()
			.endpoint(engine.guard_url())
			.namespace(namespace.clone())
			.runner_name(runner_name.clone())
			.runner_key(common::random_name("key"))
			.token("dev")
			.total_slots(16)
			.build()?,
	)
	.app(app.clone())
	.build()?;
	runner.start().await?;
	runner.wait_ready().await?;

	let actor_id = engine
		.create_actor(&namespace, "ws-echo", &runner_name, Some(&actor_key))
		.await?;
	wait_for_actor_presence(&app.actors, &actor_id, true, Duration::from_secs(30)).await?;

	let mut ws = engine.actor_websocket_connect(&actor_id, "/ws").await?;
	ws.send(Message::Text("ping".to_string().into())).await?;
	let echoed = ws
		.next()
		.await
		.context("missing echoed text frame")??;
	assert_text_message(&echoed, "ping")?;

	ws.send(Message::Binary(vec![1u8, 2, 3].into())).await?;
	let echoed_binary = ws
		.next()
		.await
		.context("missing echoed binary frame")??;
	assert_binary_message(&echoed_binary, &[1, 2, 3])?;

	let mut large_payload = vec![0u8; 64 * 1024];
	for (idx, byte) in large_payload.iter_mut().enumerate() {
		*byte = (idx % 251) as u8;
	}
	ws.send(Message::Binary(large_payload.clone().into())).await?;
	let echoed_large_binary = ws
		.next()
		.await
		.context("missing echoed large binary frame")??;
	assert_binary_message(&echoed_large_binary, &large_payload)?;

	ws.close(None).await?;
	wait_for_close(&app.closes, &actor_id, Duration::from_secs(10)).await?;

	runner.handle().sleep_actor(&actor_id, None).await?;
	wait_for_actor_presence(&app.actors, &actor_id, false, Duration::from_secs(30)).await?;
	runner.shutdown(true).await?;
	Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn websocket_hibernation_restore_runner_e2e() -> Result<()> {
	let _test_lock = common::acquire_test_lock().await?;
	let engine = common::EngineProcess::start().await?;
	let namespace = "default".to_string();
	let runner_name = common::random_name("rust-ws-hibernation-runner");
	let actor_key = common::random_name("ws");
	let app = EchoWebSocketApp::default();

	let runner = Runner::builder(
		RunnerConfig::builder()
			.endpoint(engine.guard_url())
			.namespace(namespace.clone())
			.runner_name(runner_name.clone())
			.runner_key(common::random_name("key"))
			.token("dev")
			.total_slots(16)
			.build()?,
	)
	.app(app.clone())
	.build()?;
	runner.start().await?;
	runner.wait_ready().await?;

	let actor_id = engine
		.create_actor(&namespace, "ws-echo", &runner_name, Some(&actor_key))
		.await?;
	wait_for_actor_presence(&app.actors, &actor_id, true, Duration::from_secs(30)).await?;

	let mut ws = engine.actor_websocket_connect(&actor_id, "/ws").await?;
	ws.send(Message::Text("before-sleep".to_string().into())).await?;
	let echoed = ws
		.next()
		.await
		.context("missing echoed before-sleep frame")??;
	assert_text_message(&echoed, "before-sleep")?;

	runner.handle().sleep_actor(&actor_id, None).await?;
	wait_for_actor_presence(&app.actors, &actor_id, false, Duration::from_secs(30)).await?;

	ws.send(Message::Text("after-sleep".to_string().into())).await?;
	let echoed_after_sleep = ws
		.next()
		.await
		.context("missing echoed after-sleep frame")??;
	assert_text_message(&echoed_after_sleep, "after-sleep")?;
	wait_for_actor_presence(&app.actors, &actor_id, true, Duration::from_secs(30)).await?;

	ws.close(None).await?;
	wait_for_close(&app.closes, &actor_id, Duration::from_secs(10)).await?;
	runner.shutdown(true).await?;
	Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn websocket_serverless_e2e() -> Result<()> {
	let _test_lock = common::acquire_test_lock().await?;
	let engine = common::EngineProcess::start().await?;
	let namespace = "default".to_string();
	let runner_name = common::random_name("rust-ws-serverless");
	let actor_key = common::random_name("ws");
	let app = EchoWebSocketApp::default();

	let serverless_runner = ServerlessRunner::builder(
		ServerlessConfig::builder()
			.endpoint(engine.guard_url())
			.namespace(namespace.clone())
			.runner_name(runner_name.clone())
			.runner_key(common::random_name("key"))
			.token("dev")
			.prepopulate_actor_name("ws-echo", json!({}))
			.total_slots(1)
			.max_runners(1000)
			.slots_per_runner(1)
			.request_lifespan(300)
			.build()?,
	)
	.app(app.clone())
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

	let start_response = reqwest::Client::new()
		.get(format!("{serverless_url}/api/rivet/start"))
		.send()
		.await?;
	if start_response.status() != reqwest::StatusCode::OK {
		bail!("serverless start endpoint returned {}", start_response.status());
	}

	let actor_id = engine
		.create_actor(&namespace, "ws-echo", &runner_name, Some(&actor_key))
		.await?;
	wait_for_actor_presence(&app.actors, &actor_id, true, Duration::from_secs(30)).await?;

	let mut ws = engine.actor_websocket_connect(&actor_id, "/ws").await?;
	ws.send(Message::Text("pong".to_string().into())).await?;
	let echoed = ws
		.next()
		.await
		.context("missing echoed text frame")??;
	assert_text_message(&echoed, "pong")?;

	ws.close(None).await?;
	wait_for_close(&app.closes, &actor_id, Duration::from_secs(10)).await?;

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

fn assert_text_message(message: &Message, expected: &str) -> Result<()> {
	match message {
		Message::Text(text) if text.as_str() == expected => Ok(()),
		_ => bail!("expected text websocket message `{expected}`, got `{message:?}`"),
	}
}

fn assert_binary_message(message: &Message, expected: &[u8]) -> Result<()> {
	match message {
		Message::Binary(data) if data.as_ref() == expected => Ok(()),
		_ => bail!("expected binary websocket message `{expected:?}`, got `{message:?}`"),
	}
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

async fn wait_for_close(
	close_registry: &Arc<Mutex<Vec<String>>>,
	actor_id: &str,
	timeout: Duration,
) -> Result<()> {
	let deadline = Instant::now() + timeout;
	loop {
		if close_registry.lock().await.iter().any(|x| x == actor_id) {
			return Ok(());
		}
		if Instant::now() >= deadline {
			bail!("timed out waiting for websocket close callback actor_id={actor_id}");
		}
		tokio::time::sleep(Duration::from_millis(100)).await;
	}
}
