//! End-to-end integration test for the native-actor-plugin host loader.
//!
//! Builds the `rivet-actor-test-plugin` cdylib FIXTURE, loads it through the
//! real `build_native_plugin_factory` (dlopen + ABI magic/version check +
//! symbol cache), drives a full actor lifecycle (startup-ready signal → Action
//! dispatch → reply slab → reply), and asserts the portable counter actor
//! replies correctly. Exercises the generic ABI + host adapter path with no
//! product-specific plugin package and no sidecar.
#![cfg(feature = "native-runtime")]

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use anyhow::{Context, Result};
use rivet_actor_test_plugin::counter_actor;
use rivetkit_core::{
	ActorConfig, ActorContext, ActorEvent, ActorFactory, ActorStart, ConnHandle, CoreRegistry,
	QueueSendStatus, Reply, Request, SerializeStateReason, ShutdownKind, StateDelta, WebSocket,
	build_native_plugin_factory, build_portable_native_actor_factory,
};
use serde_json::{Value as JsonValue, json};
use tokio::sync::{mpsc, oneshot};

use crate::common::ctx::IntegrationCtx;

/// Build the fixture cdylib and return the path to the built `.so`.
fn build_fixture() -> PathBuf {
	let status = Command::new(env!("CARGO"))
		.args(["build", "-p", "rivet-actor-test-plugin"])
		.status()
		.expect("spawn cargo build for fixture plugin");
	assert!(status.success(), "fixture plugin build failed");

	let target = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| {
		// <manifest>/../../../target  (manifest = .../packages/rivetkit-core)
		format!("{}/../../../target", env!("CARGO_MANIFEST_DIR"))
	});
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

#[tokio::test]
async fn native_plugin_counter_round_trips_action_through_host_loader() {
	let so = build_fixture();

	let mut config = ActorConfig::default();
	config.has_database = false;
	let factory = build_native_plugin_factory(&so, "{}", "", config)
		.expect("load + construct native plugin factory");

	// Queue one Action; its reply channel receives the portable counter output.
	let args = Vec::new();
	let (reply_tx, reply_rx) = oneshot::channel();
	let (event_tx, event_rx) = mpsc::unbounded_channel::<ActorEvent>();
	event_tx
		.send(ActorEvent::Action {
			name: "get".to_owned(),
			args: args.clone(),
			conn: None,
			reply: Reply::from(reply_tx),
		})
		.expect("queue action");

	let (startup_tx, startup_rx) = oneshot::channel();
	let start = ActorStart {
		ctx: ActorContext::new("native-plugin-e2e", "test", Vec::new(), "local"),
		is_new: true,
		input: None,
		snapshot: None,
		hibernated: Vec::new(),
		events: event_rx.into(),
		startup_ready: Some(startup_tx),
	};

	let join = tokio::spawn(async move { factory.start(start).await });

	// The fixture must signal startup (manual startup-ready) before the host
	// resolves this — proves the startup handshake works.
	tokio::time::timeout(Duration::from_secs(10), startup_rx)
		.await
		.expect("startup signal within 10s")
		.expect("startup channel open")
		.expect("startup ok");

	// The fixture returns the counter value through the reply slab.
	let reply = tokio::time::timeout(Duration::from_secs(10), reply_rx)
		.await
		.expect("reply within 10s")
		.expect("reply channel open")
		.expect("action reply ok");
	assert_eq!(decode_cbor_json(&reply), json!(0));

	// Closing the event stream ends the actor; the run future must join cleanly.
	drop(event_tx);
	tokio::time::timeout(Duration::from_secs(10), join)
		.await
		.expect("actor task joins within 10s")
		.expect("actor task not panicked")
		.expect("actor run ok");
}

#[tokio::test]
async fn native_plugin_websocket_open_round_trips_through_host_loader() {
	let so = build_fixture();

	let mut config = ActorConfig::default();
	config.has_database = false;
	let factory = build_native_plugin_factory(&so, "{}", "", config)
		.expect("load + construct native plugin factory");

	let (reply_tx, reply_rx) = oneshot::channel();
	let (event_tx, event_rx) = mpsc::unbounded_channel::<ActorEvent>();
	event_tx
		.send(ActorEvent::WebSocketOpen {
			conn: ConnHandle::new("ws-conn", Vec::new(), Vec::new(), false),
			ws: WebSocket::new(),
			request: None,
			reply: Reply::from(reply_tx),
		})
		.expect("queue websocket open");

	let (startup_tx, startup_rx) = oneshot::channel();
	let start = ActorStart {
		ctx: ActorContext::new("native-plugin-ws-e2e", "test", Vec::new(), "local"),
		is_new: true,
		input: None,
		snapshot: None,
		hibernated: Vec::new(),
		events: event_rx.into(),
		startup_ready: Some(startup_tx),
	};

	let join = tokio::spawn(async move { factory.start(start).await });

	tokio::time::timeout(Duration::from_secs(10), startup_rx)
		.await
		.expect("startup signal within 10s")
		.expect("startup channel open")
		.expect("startup ok");

	tokio::time::timeout(Duration::from_secs(10), reply_rx)
		.await
		.expect("reply within 10s")
		.expect("reply channel open")
		.expect("websocket open reply ok");

	drop(event_tx);
	tokio::time::timeout(Duration::from_secs(10), join)
		.await
		.expect("actor task joins within 10s")
		.expect("actor task not panicked")
		.expect("actor run ok");
}

#[tokio::test]
async fn native_plugin_forwards_opaque_factory_config() {
	let so = build_fixture();

	let mut config = ActorConfig::default();
	config.has_database = false;
	let config_json = r#"{"package":"native-plugin-test","nested":{"ok":true}}"#;
	let sidecar_path = "/tmp/native-plugin-sidecar-for-forwarding-test";
	let factory = build_native_plugin_factory(&so, config_json, sidecar_path, config)
		.expect("load + construct native plugin factory");

	let (reply_tx, reply_rx) = oneshot::channel();
	let (event_tx, event_rx) = mpsc::unbounded_channel::<ActorEvent>();
	event_tx
		.send(ActorEvent::Action {
			name: "factory_config_report".to_owned(),
			args: Vec::new(),
			conn: None,
			reply: Reply::from(reply_tx),
		})
		.expect("queue action");

	let (startup_tx, startup_rx) = oneshot::channel();
	let start = ActorStart {
		ctx: ActorContext::new("native-plugin-config-e2e", "test", Vec::new(), "local"),
		is_new: true,
		input: None,
		snapshot: None,
		hibernated: Vec::new(),
		events: event_rx.into(),
		startup_ready: Some(startup_tx),
	};

	let join = tokio::spawn(async move { factory.start(start).await });

	tokio::time::timeout(Duration::from_secs(10), startup_rx)
		.await
		.expect("startup signal within 10s")
		.expect("startup channel open")
		.expect("startup ok");

	let reply = tokio::time::timeout(Duration::from_secs(10), reply_rx)
		.await
		.expect("reply within 10s")
		.expect("reply channel open")
		.expect("action reply ok");
	assert_eq!(
		decode_cbor_json(&reply),
		json!({
			"configJson": config_json,
			"sidecarPath": sidecar_path,
		})
	);

	drop(event_tx);
	tokio::time::timeout(Duration::from_secs(10), join)
		.await
		.expect("actor task joins within 10s")
		.expect("actor task not panicked")
		.expect("actor run ok");
}

#[derive(Clone, Copy, Debug)]
enum PortableBackend {
	Native,
	Dylib,
}

#[tokio::test]
async fn portable_actor_waits_for_abort_on_both_backends() -> Result<()> {
	let so = build_fixture();

	for backend in [PortableBackend::Native, PortableBackend::Dylib] {
		let factory = portable_factory(backend, &so)?;
		assert_abort_wait(factory, backend).await?;
	}

	Ok(())
}

#[tokio::test]
async fn portable_actor_forwards_http_and_subscribe_on_both_backends() -> Result<()> {
	let so = build_fixture();

	for backend in [PortableBackend::Native, PortableBackend::Dylib] {
		let factory = portable_factory(backend, &so)?;
		assert_http_and_subscribe(factory, backend).await?;
	}

	Ok(())
}

#[tokio::test]
async fn portable_actor_forwards_serialize_state_and_destroy_on_both_backends() -> Result<()> {
	let so = build_fixture();

	for backend in [PortableBackend::Native, PortableBackend::Dylib] {
		let factory = portable_factory(backend, &so)?;
		assert_serialize_state_and_destroy(factory, backend).await?;
	}

	Ok(())
}

#[tokio::test]
async fn portable_actor_forwards_conn_queue_ws_on_both_backends() -> Result<()> {
	let so = build_fixture();

	for backend in [PortableBackend::Native, PortableBackend::Dylib] {
		let factory = portable_factory(backend, &so)?;
		assert_conn_queue_ws(factory, backend).await?;
	}

	Ok(())
}

#[tokio::test]
async fn portable_actor_rejects_double_reply_on_both_backends() -> Result<()> {
	let so = build_fixture();

	for backend in [PortableBackend::Native, PortableBackend::Dylib] {
		let factory = portable_factory(backend, &so)?;
		assert_double_reply_rejected(factory, backend).await?;
	}

	Ok(())
}

#[tokio::test]
async fn portable_actor_drops_unanswered_replies_on_both_backends() -> Result<()> {
	let so = build_fixture();

	for backend in [PortableBackend::Native, PortableBackend::Dylib] {
		let factory = portable_factory(backend, &so)?;
		assert_unanswered_reply_dropped(factory, backend).await?;
	}

	Ok(())
}

#[tokio::test]
async fn portable_actor_propagates_reply_err_on_both_backends() -> Result<()> {
	let so = build_fixture();

	for backend in [PortableBackend::Native, PortableBackend::Dylib] {
		let factory = portable_factory(backend, &so)?;
		assert_reply_err_propagated(factory, backend).await?;
	}

	Ok(())
}

fn portable_factory(backend: PortableBackend, so: &PathBuf) -> Result<ActorFactory> {
	let mut config = ActorConfig::default();
	config.has_database = false;
	Ok(match backend {
		PortableBackend::Native => build_portable_native_actor_factory(config, counter_actor),
		PortableBackend::Dylib => build_native_plugin_factory(so, "{}", "", config)
			.context("load counter dylib fixture")?,
	})
}

async fn assert_abort_wait(factory: ActorFactory, backend: PortableBackend) -> Result<()> {
	let (reply_tx, reply_rx) = oneshot::channel();
	let (event_tx, event_rx) = mpsc::unbounded_channel::<ActorEvent>();
	event_tx
		.send(ActorEvent::Action {
			name: "wait_abort".to_owned(),
			args: Vec::new(),
			conn: None,
			reply: Reply::from(reply_tx),
		})
		.expect("queue wait_abort action");

	let (startup_tx, startup_rx) = oneshot::channel();
	let actor_ctx = ActorContext::new("portable-abort-e2e", "test", Vec::new(), "local");
	let start = ActorStart {
		ctx: actor_ctx.clone(),
		is_new: true,
		input: None,
		snapshot: None,
		hibernated: Vec::new(),
		events: event_rx.into(),
		startup_ready: Some(startup_tx),
	};

	let join = tokio::spawn(async move { factory.start(start).await });
	tokio::time::timeout(Duration::from_secs(10), startup_rx)
		.await
		.with_context(|| format!("startup signal within 10s for {backend:?}"))?
		.context("startup channel open")?
		.context("startup ok")?;

	actor_ctx.cancel_actor_abort_signal();
	let reply = tokio::time::timeout(Duration::from_secs(10), reply_rx)
		.await
		.with_context(|| format!("abort reply within 10s for {backend:?}"))?
		.context("reply channel open")?
		.context("action reply ok")?;
	assert_eq!(decode_cbor_json(&reply), json!({ "aborted": true }));

	drop(event_tx);
	tokio::time::timeout(Duration::from_secs(10), join)
		.await
		.with_context(|| format!("actor task joins within 10s for {backend:?}"))?
		.context("actor task not panicked")?
		.context("actor run ok")?;

	Ok(())
}

async fn assert_double_reply_rejected(
	factory: ActorFactory,
	backend: PortableBackend,
) -> Result<()> {
	let (probe_reply_tx, probe_reply_rx) = oneshot::channel();
	let (status_reply_tx, status_reply_rx) = oneshot::channel();
	let (event_tx, event_rx) = mpsc::unbounded_channel::<ActorEvent>();
	event_tx
		.send(ActorEvent::Action {
			name: "double_reply_probe".to_owned(),
			args: Vec::new(),
			conn: None,
			reply: Reply::from(probe_reply_tx),
		})
		.expect("queue double-reply probe");
	event_tx
		.send(ActorEvent::Action {
			name: "double_reply_status".to_owned(),
			args: Vec::new(),
			conn: None,
			reply: Reply::from(status_reply_tx),
		})
		.expect("queue double-reply status");

	let (startup_tx, startup_rx) = oneshot::channel();
	let start = ActorStart {
		ctx: ActorContext::new("portable-double-reply-e2e", "test", Vec::new(), "local"),
		is_new: true,
		input: None,
		snapshot: None,
		hibernated: Vec::new(),
		events: event_rx.into(),
		startup_ready: Some(startup_tx),
	};

	let join = tokio::spawn(async move { factory.start(start).await });
	tokio::time::timeout(Duration::from_secs(10), startup_rx)
		.await
		.with_context(|| format!("startup signal within 10s for {backend:?}"))?
		.context("startup channel open")?
		.context("startup ok")?;

	let first = tokio::time::timeout(Duration::from_secs(10), probe_reply_rx)
		.await
		.with_context(|| format!("double-reply first response within 10s for {backend:?}"))?
		.context("double-reply first channel open")?
		.context("double-reply first ok")?;
	assert_eq!(decode_cbor_json(&first), json!({ "first": true }));

	let status = tokio::time::timeout(Duration::from_secs(10), status_reply_rx)
		.await
		.with_context(|| format!("double-reply status within 10s for {backend:?}"))?
		.context("double-reply status channel open")?
		.context("double-reply status ok")?;
	let status = decode_cbor_json(&status);
	assert_eq!(status["secondOk"], json!(false));
	assert!(
		status["secondError"].as_str().is_some_and(|error| {
			error.contains("already answered") || error.contains("reply_ok failed")
		}),
		"unexpected double-reply error for {backend:?}: {status}"
	);

	drop(event_tx);
	tokio::time::timeout(Duration::from_secs(10), join)
		.await
		.with_context(|| format!("actor task joins within 10s for {backend:?}"))?
		.context("actor task not panicked")?
		.context("actor run ok")?;

	Ok(())
}

async fn assert_unanswered_reply_dropped(
	factory: ActorFactory,
	backend: PortableBackend,
) -> Result<()> {
	let (reply_tx, reply_rx) = oneshot::channel();
	let (event_tx, event_rx) = mpsc::unbounded_channel::<ActorEvent>();
	event_tx
		.send(ActorEvent::Action {
			name: "drop_reply_probe".to_owned(),
			args: Vec::new(),
			conn: None,
			reply: Reply::from(reply_tx),
		})
		.expect("queue drop-reply probe");

	let (startup_tx, startup_rx) = oneshot::channel();
	let start = ActorStart {
		ctx: ActorContext::new("portable-drop-reply-e2e", "test", Vec::new(), "local"),
		is_new: true,
		input: None,
		snapshot: None,
		hibernated: Vec::new(),
		events: event_rx.into(),
		startup_ready: Some(startup_tx),
	};

	let join = tokio::spawn(async move { factory.start(start).await });
	tokio::time::timeout(Duration::from_secs(10), startup_rx)
		.await
		.with_context(|| format!("startup signal within 10s for {backend:?}"))?
		.context("startup channel open")?
		.context("startup ok")?;

	drop(event_tx);

	let result = tokio::time::timeout(Duration::from_secs(10), reply_rx)
		.await
		.with_context(|| format!("dropped reply within 10s for {backend:?}"))?
		.context("drop-reply channel open")?;
	let error = result.expect_err("drop-reply probe should return an error");
	let error = format!("{error:#}");
	assert!(
		error.contains("dropped_reply")
			|| error.contains("dropped without a response")
			|| error.contains("DroppedReply"),
		"unexpected dropped-reply error for {backend:?}: {error}"
	);

	tokio::time::timeout(Duration::from_secs(10), join)
		.await
		.with_context(|| format!("actor task joins within 10s for {backend:?}"))?
		.context("actor task not panicked")?
		.context("actor run ok")?;

	Ok(())
}

async fn assert_reply_err_propagated(
	factory: ActorFactory,
	backend: PortableBackend,
) -> Result<()> {
	let (reply_tx, reply_rx) = oneshot::channel();
	let (event_tx, event_rx) = mpsc::unbounded_channel::<ActorEvent>();
	event_tx
		.send(ActorEvent::Action {
			name: "reply_err_probe".to_owned(),
			args: Vec::new(),
			conn: None,
			reply: Reply::from(reply_tx),
		})
		.expect("queue reply_err probe");

	let (startup_tx, startup_rx) = oneshot::channel();
	let start = ActorStart {
		ctx: ActorContext::new("portable-reply-err-e2e", "test", Vec::new(), "local"),
		is_new: true,
		input: None,
		snapshot: None,
		hibernated: Vec::new(),
		events: event_rx.into(),
		startup_ready: Some(startup_tx),
	};

	let join = tokio::spawn(async move { factory.start(start).await });
	tokio::time::timeout(Duration::from_secs(10), startup_rx)
		.await
		.with_context(|| format!("startup signal within 10s for {backend:?}"))?
		.context("startup channel open")?
		.context("startup ok")?;

	let result = tokio::time::timeout(Duration::from_secs(10), reply_rx)
		.await
		.with_context(|| format!("reply_err response within 10s for {backend:?}"))?
		.context("reply_err channel open")?;
	let error = result.expect_err("reply_err probe should return an error");
	let error = format!("{error:#}");
	assert!(
		error.contains("portable reply error"),
		"unexpected reply_err error for {backend:?}: {error}"
	);

	drop(event_tx);
	tokio::time::timeout(Duration::from_secs(10), join)
		.await
		.with_context(|| format!("actor task joins within 10s for {backend:?}"))?
		.context("actor task not panicked")?
		.context("actor run ok")?;

	Ok(())
}

async fn assert_serialize_state_and_destroy(
	factory: ActorFactory,
	backend: PortableBackend,
) -> Result<()> {
	let (increment_reply_tx, increment_reply_rx) = oneshot::channel();
	let (serialize_reply_tx, serialize_reply_rx) = oneshot::channel();
	let (destroy_reply_tx, destroy_reply_rx) = oneshot::channel();
	let (event_tx, event_rx) = mpsc::unbounded_channel::<ActorEvent>();
	event_tx
		.send(ActorEvent::Action {
			name: "increment".to_owned(),
			args: Vec::new(),
			conn: None,
			reply: Reply::from(increment_reply_tx),
		})
		.expect("queue increment action");
	event_tx
		.send(ActorEvent::SerializeState {
			reason: SerializeStateReason::Save,
			reply: Reply::from(serialize_reply_tx),
		})
		.expect("queue serialize state");
	event_tx
		.send(ActorEvent::RunGracefulCleanup {
			reason: ShutdownKind::Destroy,
			reply: Reply::from(destroy_reply_tx),
		})
		.expect("queue destroy cleanup");

	let (startup_tx, startup_rx) = oneshot::channel();
	let start = ActorStart {
		ctx: ActorContext::new(
			"portable-serialize-destroy-e2e",
			"test",
			Vec::new(),
			"local",
		),
		is_new: true,
		input: None,
		snapshot: None,
		hibernated: Vec::new(),
		events: event_rx.into(),
		startup_ready: Some(startup_tx),
	};

	let join = tokio::spawn(async move { factory.start(start).await });
	tokio::time::timeout(Duration::from_secs(10), startup_rx)
		.await
		.with_context(|| format!("startup signal within 10s for {backend:?}"))?
		.context("startup channel open")?
		.context("startup ok")?;

	let increment_reply = tokio::time::timeout(Duration::from_secs(10), increment_reply_rx)
		.await
		.with_context(|| format!("increment reply within 10s for {backend:?}"))?
		.context("increment reply channel open")?
		.context("increment reply ok")?;
	assert_eq!(decode_cbor_json(&increment_reply), json!(1));

	let deltas = tokio::time::timeout(Duration::from_secs(10), serialize_reply_rx)
		.await
		.with_context(|| format!("serialize-state reply within 10s for {backend:?}"))?
		.context("serialize-state reply channel open")?
		.context("serialize-state reply ok")?;
	assert_eq!(deltas.len(), 1);
	match &deltas[0] {
		StateDelta::ActorState(bytes) => {
			assert_eq!(decode_cbor_json(bytes), json!({ "count": 1 }));
		}
		other => anyhow::bail!("unexpected state delta for {backend:?}: {other:?}"),
	}

	tokio::time::timeout(Duration::from_secs(10), destroy_reply_rx)
		.await
		.with_context(|| format!("destroy reply within 10s for {backend:?}"))?
		.context("destroy reply channel open")?
		.context("destroy reply ok")?;

	drop(event_tx);
	tokio::time::timeout(Duration::from_secs(10), join)
		.await
		.with_context(|| format!("actor task joins within 10s for {backend:?}"))?
		.context("actor task not panicked")?
		.context("actor run ok")?;

	Ok(())
}

async fn assert_conn_queue_ws(factory: ActorFactory, backend: PortableBackend) -> Result<()> {
	let conn = ConnHandle::new(
		format!("rich-conn-{backend:?}"),
		encode_cbor_json(&json!({ "backend": format!("{backend:?}") })),
		vec![1, 2, 3],
		true,
	);
	let (preflight_reply_tx, preflight_reply_rx) = oneshot::channel();
	let (open_reply_tx, open_reply_rx) = oneshot::channel();
	let (queue_reply_tx, queue_reply_rx) = oneshot::channel();
	let (ws_reply_tx, ws_reply_rx) = oneshot::channel();
	let (report_reply_tx, report_reply_rx) = oneshot::channel();
	let (event_tx, event_rx) = mpsc::unbounded_channel::<ActorEvent>();

	event_tx
		.send(ActorEvent::ConnectionPreflight {
			conn: conn.clone(),
			params: encode_cbor_json(&json!({
				"phase": "preflight",
				"backend": format!("{backend:?}"),
			})),
			request: Some(
				Request::from_parts(
					"GET",
					"/portable-preflight",
					HashMap::from([("x-portable-test".to_owned(), format!("{backend:?}"))]),
					Vec::new(),
				)
				.context("build preflight request")?,
			),
			reply: Reply::from(preflight_reply_tx),
		})
		.expect("queue connection preflight");
	event_tx
		.send(ActorEvent::ConnectionOpen {
			conn: conn.clone(),
			request: Some(
				Request::from_parts(
					"GET",
					"/portable-open",
					HashMap::from([("x-portable-test".to_owned(), format!("{backend:?}"))]),
					Vec::new(),
				)
				.context("build connection-open request")?,
			),
			reply: Reply::from(open_reply_tx),
		})
		.expect("queue connection open");
	event_tx
		.send(ActorEvent::QueueSend {
			name: "portable-queue-direct".to_owned(),
			body: encode_cbor_json(&json!({
				"kind": "direct",
				"backend": format!("{backend:?}"),
			})),
			conn: conn.clone(),
			request: Request::from_parts(
				"POST",
				"/portable-queue",
				HashMap::from([("x-portable-test".to_owned(), format!("{backend:?}"))]),
				encode_cbor_json(&json!({ "request": "queue" })),
			)
			.context("build queue request")?,
			wait: true,
			timeout_ms: Some(3_456),
			reply: Reply::from(queue_reply_tx),
		})
		.expect("queue queue-send event");
	event_tx
		.send(ActorEvent::WebSocketOpen {
			conn: conn.clone(),
			ws: WebSocket::new(),
			request: Some(
				Request::from_parts(
					"GET",
					"/portable-ws",
					HashMap::from([("x-portable-test".to_owned(), format!("{backend:?}"))]),
					Vec::new(),
				)
				.context("build websocket request")?,
			),
			reply: Reply::from(ws_reply_tx),
		})
		.expect("queue websocket open");
	event_tx
		.send(ActorEvent::ConnectionClosed { conn: conn.clone() })
		.expect("queue connection closed");
	event_tx
		.send(ActorEvent::Action {
			name: "conn_report".to_owned(),
			args: Vec::new(),
			conn: None,
			reply: Reply::from(report_reply_tx),
		})
		.expect("queue connection report");

	let (startup_tx, startup_rx) = oneshot::channel();
	let start = ActorStart {
		ctx: ActorContext::new("portable-rich-events-e2e", "test", Vec::new(), "local"),
		is_new: true,
		input: None,
		snapshot: None,
		hibernated: Vec::new(),
		events: event_rx.into(),
		startup_ready: Some(startup_tx),
	};

	let join = tokio::spawn(async move { factory.start(start).await });
	tokio::time::timeout(Duration::from_secs(10), startup_rx)
		.await
		.with_context(|| format!("startup signal within 10s for {backend:?}"))?
		.context("startup channel open")?
		.context("startup ok")?;

	tokio::time::timeout(Duration::from_secs(10), preflight_reply_rx)
		.await
		.with_context(|| format!("preflight reply within 10s for {backend:?}"))?
		.context("preflight reply channel open")?
		.context("preflight reply ok")?;
	tokio::time::timeout(Duration::from_secs(10), open_reply_rx)
		.await
		.with_context(|| format!("open reply within 10s for {backend:?}"))?
		.context("open reply channel open")?
		.context("open reply ok")?;

	let queue = tokio::time::timeout(Duration::from_secs(10), queue_reply_rx)
		.await
		.with_context(|| format!("queue reply within 10s for {backend:?}"))?
		.context("queue reply channel open")?
		.context("queue reply ok")?;
	assert_eq!(queue.status, QueueSendStatus::Completed);
	let queue_response = decode_cbor_json(queue.response.as_deref().context("queue response")?);
	assert_eq!(queue_response["name"], json!("portable-queue-direct"));
	assert_eq!(queue_response["body"]["kind"], json!("direct"));
	assert_eq!(
		queue_response["body"]["backend"],
		json!(format!("{backend:?}"))
	);
	assert_eq!(queue_response["conn"]["id"], json!(conn.id()));
	assert_eq!(
		queue_response["conn"]["params"]["backend"],
		json!(format!("{backend:?}"))
	);
	assert_eq!(queue_response["conn"]["state"], json!([1, 2, 3]));
	assert_eq!(queue_response["conn"]["isHibernatable"], json!(true));
	assert_eq!(queue_response["request"]["method"], json!("POST"));
	assert_eq!(queue_response["request"]["uri"], json!("/portable-queue"));
	assert_eq!(
		queue_response["request"]["headers"]["x-portable-test"],
		json!(format!("{backend:?}"))
	);
	assert_eq!(queue_response["request"]["body"]["request"], json!("queue"));
	assert_eq!(queue_response["wait"], json!(true));
	assert_eq!(queue_response["timeoutMs"], json!(3_456));

	tokio::time::timeout(Duration::from_secs(10), ws_reply_rx)
		.await
		.with_context(|| format!("websocket reply within 10s for {backend:?}"))?
		.context("websocket reply channel open")?
		.context("websocket reply ok")?;

	let report = tokio::time::timeout(Duration::from_secs(10), report_reply_rx)
		.await
		.with_context(|| format!("connection report within 10s for {backend:?}"))?
		.context("connection report channel open")?
		.context("connection report ok")?;
	let report = decode_cbor_json(&report);
	assert_eq!(report["preflightCount"], json!(1));
	assert_eq!(report["openCount"], json!(1));
	assert_eq!(report["closedCount"], json!(1));
	assert_eq!(report["lastPreflight"]["id"], json!(conn.id()));
	assert_eq!(
		report["lastPreflight"]["params"]["backend"],
		json!(format!("{backend:?}"))
	);
	assert_eq!(report["lastPreflight"]["state"], json!([1, 2, 3]));
	assert_eq!(report["lastPreflight"]["isHibernatable"], json!(true));
	assert_eq!(report["lastPreflightParams"]["phase"], json!("preflight"));
	assert_eq!(
		report["lastPreflightParams"]["backend"],
		json!(format!("{backend:?}"))
	);
	assert_eq!(report["lastOpen"]["id"], json!(conn.id()));
	assert_eq!(report["lastClosed"]["id"], json!(conn.id()));
	assert_eq!(report["wsOpenCount"], json!(1));
	assert_eq!(report["lastWsOpen"]["id"], json!(conn.id()));
	assert_eq!(report["lastWsRequest"]["method"], json!("GET"));
	assert_eq!(report["lastWsRequest"]["uri"], json!("/portable-ws"));
	assert_eq!(
		report["lastWsRequest"]["headers"]["x-portable-test"],
		json!(format!("{backend:?}"))
	);

	drop(event_tx);
	tokio::time::timeout(Duration::from_secs(10), join)
		.await
		.with_context(|| format!("actor task joins within 10s for {backend:?}"))?
		.context("actor task not panicked")?
		.context("actor run ok")?;

	Ok(())
}

async fn assert_http_and_subscribe(factory: ActorFactory, backend: PortableBackend) -> Result<()> {
	let subscribe_conn = ConnHandle::new(
		format!("subscribe-conn-{backend:?}"),
		encode_cbor_json(&json!({ "backend": format!("{backend:?}") })),
		vec![4, 5, 6],
		false,
	);
	let (http_reply_tx, http_reply_rx) = oneshot::channel();
	let (subscribe_reply_tx, subscribe_reply_rx) = oneshot::channel();
	let (report_reply_tx, report_reply_rx) = oneshot::channel();
	let (event_tx, event_rx) = mpsc::unbounded_channel::<ActorEvent>();
	event_tx
		.send(ActorEvent::HttpRequest {
			request: Request::from_parts(
				"POST",
				"/portable-http",
				HashMap::from([("x-portable-test".to_owned(), format!("{backend:?}"))]),
				encode_cbor_json(&json!({ "backend": format!("{backend:?}") })),
			)
			.context("build portable http request")?,
			reply: Reply::from(http_reply_tx),
		})
		.expect("queue http request");
	event_tx
		.send(ActorEvent::SubscribeRequest {
			conn: subscribe_conn.clone(),
			event_name: "portable.event".to_owned(),
			reply: Reply::from(subscribe_reply_tx),
		})
		.expect("queue subscribe request");
	event_tx
		.send(ActorEvent::Action {
			name: "conn_report".to_owned(),
			args: Vec::new(),
			conn: None,
			reply: Reply::from(report_reply_tx),
		})
		.expect("queue connection report");

	let (startup_tx, startup_rx) = oneshot::channel();
	let start = ActorStart {
		ctx: ActorContext::new("portable-http-subscribe-e2e", "test", Vec::new(), "local"),
		is_new: true,
		input: None,
		snapshot: None,
		hibernated: Vec::new(),
		events: event_rx.into(),
		startup_ready: Some(startup_tx),
	};

	let join = tokio::spawn(async move { factory.start(start).await });
	tokio::time::timeout(Duration::from_secs(10), startup_rx)
		.await
		.with_context(|| format!("startup signal within 10s for {backend:?}"))?
		.context("startup channel open")?
		.context("startup ok")?;

	let http_response = tokio::time::timeout(Duration::from_secs(10), http_reply_rx)
		.await
		.with_context(|| format!("http reply within 10s for {backend:?}"))?
		.context("http reply channel open")?
		.context("http reply ok")?;
	assert_eq!(http_response.status().as_u16(), 207);
	assert_eq!(
		http_response
			.headers()
			.get("x-portable-fixture")
			.and_then(|value| value.to_str().ok()),
		Some("http")
	);
	let body = decode_cbor_json(http_response.body());
	assert_eq!(body["method"], json!("POST"));
	assert_eq!(body["uri"], json!("/portable-http"));
	assert_eq!(body["header"], json!(format!("{backend:?}")));
	assert_eq!(body["body"]["backend"], json!(format!("{backend:?}")));

	tokio::time::timeout(Duration::from_secs(10), subscribe_reply_rx)
		.await
		.with_context(|| format!("subscribe reply within 10s for {backend:?}"))?
		.context("subscribe reply channel open")?
		.context("subscribe reply ok")?;

	let report = tokio::time::timeout(Duration::from_secs(10), report_reply_rx)
		.await
		.with_context(|| format!("subscribe report within 10s for {backend:?}"))?
		.context("subscribe report channel open")?
		.context("subscribe report ok")?;
	let report = decode_cbor_json(&report);
	assert_eq!(report["subscribeCount"], json!(1));
	assert_eq!(report["lastSubscribe"]["id"], json!(subscribe_conn.id()));
	assert_eq!(
		report["lastSubscribe"]["params"]["backend"],
		json!(format!("{backend:?}"))
	);
	assert_eq!(report["lastSubscribe"]["state"], json!([4, 5, 6]));
	assert_eq!(report["lastSubscribeEventName"], json!("portable.event"));

	drop(event_tx);
	tokio::time::timeout(Duration::from_secs(10), join)
		.await
		.with_context(|| format!("actor task joins within 10s for {backend:?}"))?
		.context("actor task not panicked")?
		.context("actor run ok")?;

	Ok(())
}

fn action_output(body: &str) -> Result<JsonValue> {
	let value: JsonValue = serde_json::from_str(body).context("decode action response")?;
	Ok(value.get("output").cloned().unwrap_or(JsonValue::Null))
}

/// Full-stack: the SAME harness `counter` uses (IntegrationCtx + CoreRegistry +
/// the real `rivet-engine` binary), but the registered actor is backed by a
/// native plugin loaded via `build_native_plugin_factory`. Proves a native
/// plugin actor runs through the real engine -> runtime -> actor -> reply path
/// identically to an in-process Rust actor — not just the host loader in
/// isolation. Requires the engine binary (target/debug/rivet-engine or
/// RIVET_ENGINE_BINARY_PATH), exactly like `counter`.
#[tokio::test(flavor = "multi_thread")]
async fn native_plugin_counter_actor_runs_through_engine() -> Result<()> {
	const ACTOR_NAME: &str = "native-plugin-counter";
	let so = build_fixture();

	let ctx = IntegrationCtx::builder().start().await?;
	ctx.create_default_namespace().await?;

	let mut registry = CoreRegistry::new();
	let mut config = ActorConfig::default();
	config.has_database = false;
	let factory =
		build_native_plugin_factory(&so, "{}", "", config).expect("load native plugin factory");
	registry.register(ACTOR_NAME, factory);
	let registry_task = ctx.serve_registry(registry);

	ctx.wait_for_envoy_ready().await?;
	let actor = ctx.create_actor(ACTOR_NAME).await?;

	let body = ctx
		.wait_for_json_action(&actor.actor_id, "increment")
		.await
		.context("increment action through engine")?;
	assert_eq!(
		action_output(&body)?,
		json!(1),
		"native plugin reply via engine"
	);

	registry_task.shutdown().await?;
	ctx.shutdown().await?;
	Ok(())
}

fn decode_cbor_json(bytes: &[u8]) -> JsonValue {
	ciborium::from_reader(std::io::Cursor::new(bytes)).expect("decode cbor json")
}

fn encode_cbor_json(value: &JsonValue) -> Vec<u8> {
	let mut out = Vec::new();
	ciborium::into_writer(value, &mut out).expect("encode cbor json");
	out
}
