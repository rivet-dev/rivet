//! Phase 1b end-to-end gate. Constructs `NapiActorFactory` via the NAPI
//! `from_agent_os` factory method (the same path JS uses) and drives a
//! real action through the inner `CoreActorFactory` against a live
//! agent-os sidecar. Verifies:
//!
//! 1. `from_agent_os` builds a factory whose `start(...)` actually runs
//!    the actor's event loop (not a no-op shell).
//! 2. A `writeFile` -> `readFile` round-trip dispatched through that loop
//!    lands at `actions::dispatch` and replies with the wrapped
//!    `["$Uint8Array", base64]` payload.
//! 3. The factory drains cleanly when the event channel closes.
//!
//! Sidecar-gated: skips when `AGENT_OS_SIDECAR_BIN` is unset.

mod e2e {
	use std::io::Cursor;
	use std::path::PathBuf;

	use base64::Engine as _;
	use base64::engine::general_purpose::STANDARD as BASE64;
	use rivetkit_core::{ActorContext, ActorEvent, ActorStart, Reply};
	use tokio::sync::{mpsc, oneshot};

	use crate::actor_factory::NapiActorFactory;
	use crate::agent_os::NapiAgentOsOptions;

	fn sidecar_available() -> bool {
		if std::env::var("AGENT_OS_SIDECAR_BIN").is_err() {
			let candidate = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
				.join("../../../target/debug/agent-os-sidecar");
			if candidate.exists() {
				// SAFETY: tests run single-process; env mutation here is fine.
				unsafe {
					std::env::set_var("AGENT_OS_SIDECAR_BIN", candidate);
				}
			}
		}
		std::env::var("AGENT_OS_SIDECAR_BIN")
			.map(|path| PathBuf::from(path).exists())
			.unwrap_or(false)
	}

	fn encode_cbor<T: serde::Serialize>(value: &T) -> Vec<u8> {
		let mut buf = Vec::new();
		ciborium::into_writer(value, &mut buf).expect("encode CBOR");
		buf
	}

	#[tokio::test]
	async fn napi_factory_dispatches_write_then_read_file() {
		if !sidecar_available() {
			eprintln!("skipping: AGENT_OS_SIDECAR_BIN not present");
			return;
		}

		// Build the factory the same way JS does: through the NAPI
		// `from_agent_os` static. The underlying Rust fn is callable
		// directly; `#[napi(factory)]` only adds the JS export.
		let napi_factory = NapiActorFactory::from_agent_os(
			NapiAgentOsOptions {
				config_json: Some("{}".to_owned()),
			},
			None,
		)
		.expect("from_agent_os ok");
		let core_factory = napi_factory.actor_factory();

		// Queue two actions on the actor's event channel:
		// 1. writeFile(path, bytes)
		// 2. readFile(path) — must return the bytes from step 1.
		let path = "/home/user/napi-factory-roundtrip.txt";
		let payload = b"NAPI factory e2e payload".to_vec();
		let write_args = encode_cbor(&(
			path.to_owned(),
			serde_bytes::ByteBuf::from(payload.clone()),
		));
		let read_args = encode_cbor(&(path.to_owned(),));

		let (write_reply_tx, write_reply_rx) = oneshot::channel();
		let (read_reply_tx, read_reply_rx) = oneshot::channel();
		let (event_tx, event_rx) = mpsc::unbounded_channel::<ActorEvent>();
		event_tx
			.send(ActorEvent::Action {
				name: "writeFile".to_owned(),
				args: write_args,
				conn: None,
				reply: Reply::from(write_reply_tx),
			})
			.expect("queue writeFile");
		event_tx
			.send(ActorEvent::Action {
				name: "readFile".to_owned(),
				args: read_args,
				conn: None,
				reply: Reply::from(read_reply_tx),
			})
			.expect("queue readFile");

		let (startup_tx, startup_rx) = oneshot::channel();
		let start = ActorStart {
			ctx: ActorContext::new(
				"napi-factory-e2e",
				"agent-os",
				Vec::new(),
				"local",
			),
			input: None,
			snapshot: None,
			hibernated: Vec::new(),
			events: event_rx.into(),
			startup_ready: Some(startup_tx),
		};

		// Spawn the factory's entry future. It drives ensure_vm on the
		// first action, then dispatches both actions in order.
		let factory = core_factory.clone();
		let join = tokio::spawn(async move { factory.start(start).await });

		// Confirm startup before draining replies.
		startup_rx
			.await
			.expect("recv startup signal")
			.expect("startup ok");

		// writeFile replies with `()` — CBOR-encoded as `null` (0xF6).
		// We don't assert the exact bytes; just that the reply arrived
		// without error (the round-trip is verified by readFile below).
		let write_reply = write_reply_rx
			.await
			.expect("recv writeFile reply")
			.expect("writeFile ok");
		assert!(
			!write_reply.is_empty(),
			"writeFile reply should encode at least the unit value"
		);

		// readFile reply: ciborium decode -> ["$Uint8Array", base64].
		let read_reply = read_reply_rx
			.await
			.expect("recv readFile reply")
			.expect("readFile ok");
		let intermediate: serde_json::Value =
			ciborium::from_reader(Cursor::new(&read_reply))
				.expect("decode readFile reply CBOR");
		assert!(
			intermediate.is_array(),
			"expected wrapped Uint8Array, got {intermediate:?}"
		);
		assert_eq!(intermediate[0], "$Uint8Array");
		let base64 = intermediate[1].as_str().expect("base64 element");
		let decoded = BASE64.decode(base64).expect("decode base64");
		assert_eq!(decoded, payload, "readFile bytes match writeFile bytes");

		// Drain the loop: closing the event channel triggers the loop's
		// shutdown path (shutdown_vm with reason="error").
		drop(event_tx);
		let _ = tokio::time::timeout(std::time::Duration::from_secs(30), join)
			.await
			.expect("factory task joins within 30s")
			.expect("factory task didn't panic");
	}

	#[tokio::test]
	async fn napi_factory_rejects_unknown_action_through_loop() {
		if !sidecar_available() {
			eprintln!("skipping: AGENT_OS_SIDECAR_BIN not present");
			return;
		}

		let napi_factory = NapiActorFactory::from_agent_os(
			NapiAgentOsOptions {
				config_json: Some("{}".to_owned()),
			},
			None,
		)
		.expect("from_agent_os ok");
		let core_factory = napi_factory.actor_factory();

		let (reply_tx, reply_rx) = oneshot::channel();
		let (event_tx, event_rx) = mpsc::unbounded_channel::<ActorEvent>();
		let args = encode_cbor(&Vec::<serde_json::Value>::new());
		event_tx
			.send(ActorEvent::Action {
				name: "totallyMadeUp".to_owned(),
				args,
				conn: None,
				reply: Reply::from(reply_tx),
			})
			.expect("queue action");

		let (startup_tx, startup_rx) = oneshot::channel();
		let start = ActorStart {
			ctx: ActorContext::new(
				"napi-factory-unknown",
				"agent-os",
				Vec::new(),
				"local",
			),
			input: None,
			snapshot: None,
			hibernated: Vec::new(),
			events: event_rx.into(),
			startup_ready: Some(startup_tx),
		};

		let factory = core_factory.clone();
		let join = tokio::spawn(async move { factory.start(start).await });
		startup_rx
			.await
			.expect("recv startup signal")
			.expect("startup ok");

		let error = reply_rx
			.await
			.expect("recv reply")
			.expect_err("unknown action should error");
		let msg = error.to_string();
		assert!(
			msg.contains("not implemented yet") || msg.contains("totallyMadeUp"),
			"expected not-implemented error, got: {msg}"
		);

		drop(event_tx);
		let _ = tokio::time::timeout(std::time::Duration::from_secs(30), join)
			.await
			.expect("factory task joins within 30s")
			.expect("factory task didn't panic");
	}
}
