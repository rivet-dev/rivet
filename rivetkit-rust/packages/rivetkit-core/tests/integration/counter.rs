use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use rivet_actor_test_plugin::counter_actor;
use rivetkit_core::{
	ActorConfig, ActorFactory, CoreRegistry, build_native_plugin_factory,
	build_portable_native_actor_factory,
};
use serde_json::{Value as JsonValue, json};

use crate::common::ctx::IntegrationCtx;

const ACTOR_NAME: &str = "counter";

#[derive(Clone, Copy, Debug)]
enum Backend {
	Native,
	Dylib,
}

#[tokio::test(flavor = "multi_thread")]
async fn counter_actor_has_backend_parity_through_engine() -> Result<()> {
	let dylib = build_fixture();

	for backend in [Backend::Native, Backend::Dylib] {
		scenario_counter(backend, dylib.clone())
			.await
			.with_context(|| format!("counter parity for {backend:?}"))?;
	}

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn counter_actor_persists_state_with_backend_parity() -> Result<()> {
	let dylib = build_fixture();

	for backend in [Backend::Native, Backend::Dylib] {
		scenario_counter_state_persistence(backend, dylib.clone())
			.await
			.with_context(|| format!("counter state parity for {backend:?}"))?;
	}

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn counter_actor_has_set_state_abort_snapshot_backend_parity() -> Result<()> {
	let dylib = build_fixture();

	for backend in [Backend::Native, Backend::Dylib] {
		scenario_counter_set_state_abort_snapshot(backend, dylib.clone())
			.await
			.with_context(|| format!("counter set-state/abort parity for {backend:?}"))?;
	}

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn counter_actor_has_identity_backend_parity() -> Result<()> {
	let dylib = build_fixture();

	for backend in [Backend::Native, Backend::Dylib] {
		scenario_counter_identity(backend, dylib.clone())
			.await
			.with_context(|| format!("counter identity parity for {backend:?}"))?;
	}

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn counter_actor_observes_connections_with_backend_parity() -> Result<()> {
	let dylib = build_fixture();

	for backend in [Backend::Native, Backend::Dylib] {
		scenario_counter_connections(backend, dylib.clone())
			.await
			.with_context(|| format!("counter connection parity for {backend:?}"))?;
	}

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn counter_actor_has_kv_backend_parity() -> Result<()> {
	let dylib = build_fixture();

	for backend in [Backend::Native, Backend::Dylib] {
		scenario_counter_kv(backend, dylib.clone())
			.await
			.with_context(|| format!("counter kv parity for {backend:?}"))?;
	}

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn counter_actor_has_sqlite_backend_parity() -> Result<()> {
	let dylib = build_fixture();

	for backend in [Backend::Native, Backend::Dylib] {
		scenario_counter_sqlite(backend, dylib.clone())
			.await
			.with_context(|| format!("counter sqlite parity for {backend:?}"))?;
	}

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn counter_actor_has_scheduling_backend_parity() -> Result<()> {
	let dylib = build_fixture();

	for backend in [Backend::Native, Backend::Dylib] {
		scenario_counter_scheduling(backend, dylib.clone())
			.await
			.with_context(|| format!("counter scheduling parity for {backend:?}"))?;
	}

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn counter_actor_can_request_sleep_with_backend_parity() -> Result<()> {
	let dylib = build_fixture();

	for backend in [Backend::Native, Backend::Dylib] {
		scenario_counter_sleep(backend, dylib.clone())
			.await
			.with_context(|| format!("counter sleep parity for {backend:?}"))?;
	}

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn counter_actor_has_keep_awake_backend_parity() -> Result<()> {
	let dylib = build_fixture();

	for backend in [Backend::Native, Backend::Dylib] {
		scenario_counter_keep_awake(backend, dylib.clone())
			.await
			.with_context(|| format!("counter keep-awake parity for {backend:?}"))?;
	}

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn counter_actor_has_fanout_alarm_backend_parity() -> Result<()> {
	let dylib = build_fixture();

	for backend in [Backend::Native, Backend::Dylib] {
		scenario_counter_fanout_alarm(backend, dylib.clone())
			.await
			.with_context(|| format!("counter fanout/alarm parity for {backend:?}"))?;
	}

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn counter_actor_has_queue_send_backend_parity() -> Result<()> {
	let dylib = build_fixture();

	for backend in [Backend::Native, Backend::Dylib] {
		scenario_counter_queue_send(backend, dylib.clone())
			.await
			.with_context(|| format!("counter queue send parity for {backend:?}"))?;
	}

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn counter_actor_has_hibernation_ack_backend_parity() -> Result<()> {
	let dylib = build_fixture();

	for backend in [Backend::Native, Backend::Dylib] {
		scenario_counter_hibernation_ack(backend, dylib.clone())
			.await
			.with_context(|| format!("counter hibernation ack parity for {backend:?}"))?;
	}

	Ok(())
}

async fn scenario_counter(backend: Backend, dylib: PathBuf) -> Result<()> {
	let ctx = IntegrationCtx::builder().start().await?;
	ctx.create_default_namespace().await?;
	let mut registry = CoreRegistry::new();
	registry.register(ACTOR_NAME, factory_for(backend, dylib)?);
	let registry_task = ctx.serve_registry(registry);

	ctx.wait_for_envoy_ready().await?;
	let actor = ctx.create_actor(ACTOR_NAME).await?;

	let first = ctx
		.wait_for_json_action(&actor.actor_id, "increment")
		.await
		.context("first increment")?;
	let second = ctx
		.wait_for_json_action(&actor.actor_id, "increment")
		.await
		.context("second increment")?;
	let current = ctx
		.wait_for_json_action(&actor.actor_id, "get")
		.await
		.context("get count")?;

	assert_eq!(action_output(&first)?, json!(1));
	assert_eq!(action_output(&second)?, json!(2));
	assert_eq!(action_output(&current)?, json!(2));

	registry_task.shutdown().await?;
	ctx.shutdown().await?;

	Ok(())
}

async fn scenario_counter_identity(backend: Backend, dylib: PathBuf) -> Result<()> {
	let ctx = IntegrationCtx::builder().start().await?;
	ctx.create_default_namespace().await?;
	let mut registry = CoreRegistry::new();
	registry.register(ACTOR_NAME, factory_for(backend, dylib)?);
	let registry_task = ctx.serve_registry(registry);

	ctx.wait_for_envoy_ready().await?;
	let actor = ctx
		.create_actor_with_key(ACTOR_NAME, Some("portable-identity"))
		.await?;

	let report = ctx
		.wait_for_json_action(&actor.actor_id, "identity_report")
		.await
		.context("identity report")?;
	let report = action_output(&report)?;
	assert_eq!(report["actorId"], json!(actor.actor_id));
	assert_eq!(report["name"], json!(ACTOR_NAME));
	assert_eq!(report["key"], json!("portable-identity"));
	assert!(report["region"].is_string());
	assert_eq!(report["input"], JsonValue::Null);
	assert_eq!(report["hasState"], json!(false));

	registry_task.shutdown().await?;
	ctx.shutdown().await?;

	Ok(())
}

async fn scenario_counter_hibernation_ack(backend: Backend, dylib: PathBuf) -> Result<()> {
	let ctx = IntegrationCtx::builder().start().await?;
	ctx.create_default_namespace().await?;
	let mut registry = CoreRegistry::new();
	registry.register(ACTOR_NAME, factory_for(backend, dylib)?);
	let registry_task = ctx.serve_registry(registry);

	ctx.wait_for_envoy_ready().await?;
	let actor = ctx.create_actor(ACTOR_NAME).await?;

	let report = ctx
		.wait_for_json_action(&actor.actor_id, "ack_invalid")
		.await
		.context("invalid hibernation ack")?;
	let report = action_output(&report)?;
	assert_eq!(report["ok"], json!(false));
	assert!(
		report["error"]
			.as_str()
			.unwrap_or_default()
			.contains("gateway_id must be exactly 4 bytes"),
		"unexpected ack error for {backend:?}: {report}"
	);

	registry_task.shutdown().await?;
	ctx.shutdown().await?;

	Ok(())
}

async fn scenario_counter_set_state_abort_snapshot(backend: Backend, dylib: PathBuf) -> Result<()> {
	let ctx = IntegrationCtx::builder().start().await?;
	ctx.create_default_namespace().await?;
	let mut registry = CoreRegistry::new();
	registry.register(ACTOR_NAME, factory_for(backend, dylib)?);
	let registry_task = ctx.serve_registry(registry);

	ctx.wait_for_envoy_ready().await?;
	let actor = ctx.create_actor(ACTOR_NAME).await?;

	let state_report = ctx
		.wait_for_json_action(&actor.actor_id, "set_state_report")
		.await
		.context("set-state report")?;
	let state_report = action_output(&state_report)?;
	assert_eq!(state_report["count"], json!(41));

	let abort_report = ctx
		.wait_for_json_action(&actor.actor_id, "abort_snapshot")
		.await
		.context("abort snapshot")?;
	let abort_report = action_output(&abort_report)?;
	assert_eq!(abort_report["aborted"], json!(false));

	registry_task.shutdown().await?;
	ctx.shutdown().await?;

	Ok(())
}

async fn scenario_counter_queue_send(backend: Backend, dylib: PathBuf) -> Result<()> {
	let ctx = IntegrationCtx::builder().start().await?;
	ctx.create_default_namespace().await?;
	let mut registry = CoreRegistry::new();
	registry.register(ACTOR_NAME, factory_for(backend, dylib)?);
	let registry_task = ctx.serve_registry(registry);

	ctx.wait_for_envoy_ready().await?;
	let actor = ctx.create_actor(ACTOR_NAME).await?;

	let body = send_json_queue(
		&ctx,
		&actor.actor_id,
		"portable-queue",
		json!({ "kind": "portable", "backend": format!("{backend:?}") }),
	)
	.await
	.context("queue send")?;
	let body: JsonValue = serde_json::from_str(&body).context("decode queue response")?;

	assert_eq!(body["status"], json!("completed"));
	assert_eq!(body["response"]["name"], json!("portable-queue"));
	assert_eq!(body["response"]["body"]["kind"], json!("portable"));
	assert_eq!(body["response"]["wait"], json!(true));
	assert_eq!(body["response"]["timeoutMs"], json!(2_000));
	assert_eq!(body["response"]["conn"]["isHibernatable"], json!(false));
	assert!(body["response"]["conn"]["id"].is_string());

	registry_task.shutdown().await?;
	ctx.shutdown().await?;

	Ok(())
}

async fn scenario_counter_scheduling(backend: Backend, dylib: PathBuf) -> Result<()> {
	let ctx = IntegrationCtx::builder().start().await?;
	ctx.create_default_namespace().await?;
	let mut registry = CoreRegistry::new();
	registry.register(ACTOR_NAME, factory_for(backend, dylib)?);
	let registry_task = ctx.serve_registry(registry);

	ctx.wait_for_envoy_ready().await?;
	let actor = ctx.create_actor(ACTOR_NAME).await?;

	let scheduled = ctx
		.wait_for_json_action(&actor.actor_id, "schedule_once")
		.await
		.context("schedule once")?;
	let scheduled = action_output(&scheduled)?;
	assert_eq!(scheduled["pendingCount"], json!(1));
	assert_eq!(scheduled["firstAction"], json!("scheduled_increment"));
	wait_for_scheduled_count(&ctx, &actor.actor_id, 1)
		.await
		.context("scheduled after action")?;

	let scheduled_at = ctx
		.wait_for_json_action(&actor.actor_id, "schedule_at_once")
		.await
		.context("schedule at once")?;
	let scheduled_at = action_output(&scheduled_at)?;
	assert_eq!(scheduled_at["pendingCount"], json!(1));
	assert_eq!(scheduled_at["firstAction"], json!("scheduled_increment"));
	assert_eq!(scheduled_at["firstTimestampAtOrAfter"], json!(true));
	wait_for_scheduled_count(&ctx, &actor.actor_id, 2)
		.await
		.context("scheduled at action")?;

	registry_task.shutdown().await?;
	ctx.shutdown().await?;

	Ok(())
}

async fn scenario_counter_sleep(backend: Backend, dylib: PathBuf) -> Result<()> {
	let ctx = IntegrationCtx::builder().start().await?;
	ctx.create_default_namespace().await?;
	let mut registry = CoreRegistry::new();
	registry.register(ACTOR_NAME, factory_for(backend, dylib)?);
	let registry_task = ctx.serve_registry(registry);

	ctx.wait_for_envoy_ready().await?;
	let actor = ctx.create_actor(ACTOR_NAME).await?;

	let before = ctx
		.wait_for_json_action(&actor.actor_id, "sleep_marker")
		.await
		.context("sleep marker before request")?;
	let before = action_output(&before)?;
	assert_eq!(before["sleepCleanupObserved"], json!(false));

	let requested = ctx
		.wait_for_json_action(&actor.actor_id, "sleep_now")
		.await
		.context("request sleep")?;
	let requested = action_output(&requested)?;
	assert_eq!(requested["requested"], json!(true));

	wait_for_sleep_marker(&ctx, &actor.actor_id).await?;

	registry_task.shutdown().await?;
	ctx.shutdown().await?;

	Ok(())
}

async fn scenario_counter_keep_awake(backend: Backend, dylib: PathBuf) -> Result<()> {
	let ctx = IntegrationCtx::builder().start().await?;
	ctx.create_default_namespace().await?;
	let mut registry = CoreRegistry::new();
	registry.register(ACTOR_NAME, factory_for(backend, dylib)?);
	let registry_task = ctx.serve_registry(registry);

	ctx.wait_for_envoy_ready().await?;
	let actor = ctx.create_actor(ACTOR_NAME).await?;

	let report = ctx
		.wait_for_json_action(&actor.actor_id, "keep_awake_report")
		.await
		.context("keep-awake report")?;
	let report = action_output(&report)?;

	assert_eq!(report["before"], json!(0));
	assert_eq!(report["during"], json!(1));
	assert_eq!(report["after"], json!(0));

	registry_task.shutdown().await?;
	ctx.shutdown().await?;

	Ok(())
}

async fn scenario_counter_fanout_alarm(backend: Backend, dylib: PathBuf) -> Result<()> {
	let ctx = IntegrationCtx::builder().start().await?;
	ctx.create_default_namespace().await?;
	let mut registry = CoreRegistry::new();
	registry.register(ACTOR_NAME, factory_for(backend, dylib)?);
	let registry_task = ctx.serve_registry(registry);

	ctx.wait_for_envoy_ready().await?;
	let actor = ctx.create_actor(ACTOR_NAME).await?;

	let report = ctx
		.wait_for_json_action(&actor.actor_id, "fanout_alarm_report")
		.await
		.context("fanout/alarm report")?;
	let report = action_output(&report)?;

	assert_eq!(report["broadcasted"], json!(true));
	assert_eq!(report["alarmTimestampFuture"], json!(true));
	assert_eq!(report["alarmCleared"], json!(true));

	registry_task.shutdown().await?;
	ctx.shutdown().await?;

	Ok(())
}

async fn scenario_counter_kv(backend: Backend, dylib: PathBuf) -> Result<()> {
	let ctx = IntegrationCtx::builder().start().await?;
	ctx.create_default_namespace().await?;
	let mut registry = CoreRegistry::new();
	registry.register(ACTOR_NAME, factory_for(backend, dylib)?);
	let registry_task = ctx.serve_registry(registry);

	ctx.wait_for_envoy_ready().await?;
	let actor = ctx.create_actor(ACTOR_NAME).await?;

	let report = ctx
		.wait_for_json_action(&actor.actor_id, "kv_roundtrip")
		.await
		.context("kv roundtrip")?;
	let report = action_output(&report)?;

	assert_eq!(report["got"], json!("one"));
	assert_eq!(report["batch"], json!(["one", "two", null]));
	assert_eq!(
		report["prefix"],
		json!([
			{ "key": "portable-kv/a", "value": "one" },
			{ "key": "portable-kv/b", "value": "two" }
		])
	);
	assert_eq!(
		report["range"],
		json!([{ "key": "portable-kv/b", "value": "two" }])
	);
	assert_eq!(report["afterDelete"], json!([]));

	registry_task.shutdown().await?;
	ctx.shutdown().await?;

	Ok(())
}

async fn scenario_counter_sqlite(backend: Backend, dylib: PathBuf) -> Result<()> {
	let ctx = IntegrationCtx::builder().enable_sqlite().start().await?;
	ctx.create_default_namespace().await?;
	let mut registry = CoreRegistry::new();
	registry.register(ACTOR_NAME, factory_for_with_database(backend, dylib, true)?);
	let registry_task = ctx.serve_registry(registry);

	ctx.wait_for_envoy_ready().await?;
	let actor = ctx.create_actor(ACTOR_NAME).await?;

	let report = ctx
		.wait_for_json_action(&actor.actor_id, "sqlite_roundtrip")
		.await
		.context("sqlite roundtrip")?;
	let report = action_output(&report)?;

	assert_eq!(report["enabled"], json!(true));
	assert_eq!(
		report["rows"],
		json!([{ "id": 1, "value": "native-dylib-parity" }])
	);

	registry_task.shutdown().await?;
	ctx.shutdown().await?;

	Ok(())
}

async fn scenario_counter_connections(backend: Backend, dylib: PathBuf) -> Result<()> {
	let ctx = IntegrationCtx::builder().start().await?;
	ctx.create_default_namespace().await?;
	let mut registry = CoreRegistry::new();
	registry.register(ACTOR_NAME, factory_for(backend, dylib)?);
	let registry_task = ctx.serve_registry(registry);

	ctx.wait_for_envoy_ready().await?;
	let actor = ctx.create_actor(ACTOR_NAME).await?;

	let first = send_json_action_with_conn_params(
		&ctx,
		&actor.actor_id,
		"conn_report",
		json!({ "source": "parity", "backend": format!("{backend:?}") }),
	)
	.await
	.context("first connection report")?;
	let first = action_output(&first)?;
	assert_eq!(first["preflightCount"], json!(1));
	assert_eq!(first["openCount"], json!(1));
	assert_eq!(first["lastPreflightParams"]["source"], json!("parity"));
	assert_eq!(first["lastOpen"]["params"]["source"], json!("parity"));
	assert_eq!(
		first["lastPreflight"]["id"], first["lastOpen"]["id"],
		"preflight/open should describe the same connection"
	);
	assert_eq!(first["lastOpen"]["isHibernatable"], json!(false));
	assert_eq!(first["disconnectMissingOk"], json!(true));
	assert_eq!(first["sendOk"], json!(false));
	assert!(
		first["sendError"]
			.as_str()
			.unwrap_or_default()
			.contains("Connection event sender is not configured"),
		"unexpected send error: {first}"
	);
	assert!(
		conn_list_contains(&first, first["lastOpen"]["id"].as_str().unwrap_or_default()),
		"conn_list should include the current action connection"
	);

	let second = send_json_action_with_conn_params(
		&ctx,
		&actor.actor_id,
		"conn_report",
		json!({ "source": "parity-2" }),
	)
	.await
	.context("second connection report")?;
	let second = action_output(&second)?;
	assert_eq!(second["preflightCount"], json!(2));
	assert_eq!(second["openCount"], json!(2));
	assert_eq!(second["lastPreflightParams"]["source"], json!("parity-2"));
	assert!(
		second["closedCount"].as_u64().unwrap_or_default() >= 1,
		"second report should observe the first HTTP action connection closing"
	);
	assert!(second["lastClosed"]["id"].is_string());
	assert_eq!(second["disconnectMissingOk"], json!(true));
	assert_eq!(second["sendOk"], json!(false));
	assert!(
		second["sendError"]
			.as_str()
			.unwrap_or_default()
			.contains("Connection event sender is not configured"),
		"unexpected send error: {second}"
	);
	assert!(
		conn_list_contains(
			&second,
			second["lastOpen"]["id"].as_str().unwrap_or_default()
		),
		"conn_list should include the current action connection"
	);

	registry_task.shutdown().await?;
	ctx.shutdown().await?;

	Ok(())
}

async fn scenario_counter_state_persistence(backend: Backend, dylib: PathBuf) -> Result<()> {
	let ctx = IntegrationCtx::builder().start().await?;
	ctx.create_default_namespace().await?;

	let mut registry = CoreRegistry::new();
	registry.register(ACTOR_NAME, factory_for(backend, dylib)?);
	let registry_task = ctx.serve_registry(registry);

	ctx.wait_for_envoy_ready().await?;
	let actor = ctx
		.create_or_get_actor_with_key(ACTOR_NAME, "portable-state")
		.await?;

	let saved = ctx
		.wait_for_json_action(&actor.actor_id, "save_wait")
		.await
		.context("request save and wait")?;
	assert_eq!(action_output(&saved)?, json!(1));
	wait_for_save_wait_status(&ctx, &actor.actor_id)
		.await
		.context("save wait completion")?;

	let sleep = ctx
		.wait_for_json_action(&actor.actor_id, "sleep_now")
		.await
		.context("request sleep before state restore")?;
	assert_eq!(action_output(&sleep)?["requested"], json!(true));
	wait_for_sleep_marker(&ctx, &actor.actor_id)
		.await
		.context("sleep/wake before state restore")?;

	let restored = ctx
		.wait_for_json_action(&actor.actor_id, "get")
		.await
		.context("restored count")?;
	assert_eq!(action_output(&restored)?, json!(1));

	registry_task.shutdown().await?;
	ctx.shutdown().await?;

	Ok(())
}

async fn wait_for_save_wait_status(ctx: &IntegrationCtx, actor_id: &str) -> Result<()> {
	let deadline = Instant::now() + Duration::from_secs(10);
	let mut last_status = JsonValue::Null;
	while Instant::now() < deadline {
		let status = ctx
			.wait_for_json_action(actor_id, "save_wait_status")
			.await
			.context("save wait status")?;
		last_status = action_output(&status)?;
		if last_status["done"] == json!(true) {
			if last_status["ok"] == json!(true) {
				return Ok(());
			}
			anyhow::bail!("request_save_and_wait failed: {last_status}");
		}
		tokio::time::sleep(Duration::from_millis(100)).await;
	}

	anyhow::bail!("request_save_and_wait did not complete; last status: {last_status}");
}

async fn wait_for_sleep_marker(ctx: &IntegrationCtx, actor_id: &str) -> Result<()> {
	let deadline = Instant::now() + Duration::from_secs(10);
	let mut last_report = JsonValue::Null;
	while Instant::now() < deadline {
		let report = ctx
			.wait_for_json_action(actor_id, "sleep_marker")
			.await
			.context("sleep marker")?;
		last_report = action_output(&report)?;
		if last_report["sleepCleanupObserved"] == json!(true) {
			return Ok(());
		}
		tokio::time::sleep(Duration::from_millis(100)).await;
	}

	anyhow::bail!("sleep cleanup marker was not observed; last report: {last_report}");
}

async fn wait_for_scheduled_count(
	ctx: &IntegrationCtx,
	actor_id: &str,
	expected: i64,
) -> Result<()> {
	let deadline = Instant::now() + Duration::from_secs(10);
	let mut last_report = JsonValue::Null;
	while Instant::now() < deadline {
		let report = ctx
			.wait_for_json_action(actor_id, "schedule_report")
			.await
			.context("schedule report")?;
		last_report = action_output(&report)?;
		if last_report["scheduledCount"] == json!(expected) {
			return Ok(());
		}
		tokio::time::sleep(Duration::from_millis(100)).await;
	}

	anyhow::bail!("scheduled count did not reach {expected}; last report: {last_report}");
}

fn factory_for(backend: Backend, dylib: PathBuf) -> Result<ActorFactory> {
	factory_for_with_database(backend, dylib, false)
}

fn factory_for_with_database(
	backend: Backend,
	dylib: PathBuf,
	has_database: bool,
) -> Result<ActorFactory> {
	let mut config = ActorConfig::default();
	config.has_database = has_database;
	config.remote_sqlite = has_database;
	Ok(match backend {
		Backend::Native => build_portable_native_actor_factory(config, counter_actor),
		Backend::Dylib => build_native_plugin_factory(&dylib, "{}", "", config)
			.context("load counter dylib fixture")?,
	})
}

fn build_fixture() -> PathBuf {
	let status = Command::new(env!("CARGO"))
		.args(["build", "-p", "rivet-actor-test-plugin"])
		.status()
		.expect("spawn cargo build for fixture plugin");
	assert!(status.success(), "fixture plugin build failed");

	let target = std::env::var("CARGO_TARGET_DIR")
		.unwrap_or_else(|_| format!("{}/../../../target", env!("CARGO_MANIFEST_DIR")));
	let lib = if cfg!(target_os = "macos") {
		"librivet_actor_test_plugin.dylib"
	} else if cfg!(target_os = "windows") {
		"rivet_actor_test_plugin.dll"
	} else {
		"librivet_actor_test_plugin.so"
	};
	let path = PathBuf::from(format!("{target}/debug/{lib}"));
	assert!(
		path.exists(),
		"built fixture not found at {}",
		path.display()
	);
	path
}

fn action_output(body: &str) -> Result<JsonValue> {
	let value: JsonValue = serde_json::from_str(body).context("decode action response")?;
	Ok(value.get("output").cloned().unwrap_or(JsonValue::Null))
}

fn conn_list_contains(report: &JsonValue, conn_id: &str) -> bool {
	report["connList"]
		.as_array()
		.map(|conns| {
			conns
				.iter()
				.any(|conn| conn.get("id").and_then(JsonValue::as_str) == Some(conn_id))
		})
		.unwrap_or(false)
}

async fn send_json_action_with_conn_params(
	ctx: &IntegrationCtx,
	actor_id: &str,
	action: &str,
	conn_params: JsonValue,
) -> Result<String> {
	let response = ctx
		.client()
		.post(format!(
			"{}/gateway/{}/action/{}",
			ctx.endpoint(),
			actor_id,
			action
		))
		.header("x-rivet-encoding", "json")
		.header("content-type", "application/json")
		.header("x-rivet-conn-params", serde_json::to_string(&conn_params)?)
		.body(r#"{"args":[]}"#)
		.send()
		.await
		.context("send actor action with connection params")?;
	let status = response.status();
	let body = response
		.text()
		.await
		.context("read actor action response")?;
	if !status.is_success() {
		anyhow::bail!(
			"actor action failed with {status}: {body}\n\nengine stdout:\n{}\n\nengine stderr:\n{}",
			ctx.engine_stdout_tail(),
			ctx.engine_stderr_tail()
		);
	}
	Ok(body)
}

async fn send_json_queue(
	ctx: &IntegrationCtx,
	actor_id: &str,
	queue: &str,
	body: JsonValue,
) -> Result<String> {
	let response = ctx
		.client()
		.post(format!(
			"{}/gateway/{}/queue/{}",
			ctx.endpoint(),
			actor_id,
			queue
		))
		.header("x-rivet-encoding", "json")
		.header("content-type", "application/json")
		.body(serde_json::to_string(&json!({
			"body": body,
			"wait": true,
			"timeout": 2_000,
		}))?)
		.send()
		.await
		.context("send actor queue request")?;
	let status = response.status();
	let body = response.text().await.context("read actor queue response")?;
	if !status.is_success() {
		anyhow::bail!(
			"actor queue failed with {status}: {body}\n\nengine stdout:\n{}\n\nengine stderr:\n{}",
			ctx.engine_stdout_tail(),
			ctx.engine_stderr_tail()
		);
	}
	Ok(body)
}
